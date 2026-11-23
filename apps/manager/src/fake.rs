//! A fake bus with one node that runs the bootloader's logic on the host,
//! built from the same `boot-protocol` pieces as the real bootloader.
//!
//! Time does not pass for the node: it stays in its wakeup window until it
//! is caught or the test calls [`FakeNode::let_window_close`].

use std::collections::VecDeque;

use boot_protocol::image::{self, SoftwareCrc32};
use boot_protocol::{
    Block, Command, Error, ImageError, MAX_BLOCK_LEN, NodeId, SemVer, check_program,
};

use crate::bus::{Bus, BusError, Frame};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeState {
    /// Without power: it neither sends nor hears anything.
    Off,
    /// Just reset, waiting for **Catch bootloader**.
    Wakeup,
    Bootloader,
    /// Running an application, which ignores the bootloader's frames.
    Application,
}

pub struct FakeNode {
    id: NodeId,
    flash: Vec<u8>,
    state: NodeState,
    outbox: VecDeque<Frame>,
    block: Option<(Block<MAX_BLOCK_LEN>, u16)>,
    wakeup: Option<Result<SemVer, ImageError>>,
    running: Option<SemVer>,
    blocks_written: usize,
    erases: usize,
    power_loss_after: Option<usize>,
    corrupt_writes: bool,
    fail_writes: bool,
}

impl FakeNode {
    /// A node with an erased application region of `region_len` bytes.
    /// [`NodeState::Wakeup`] powers it on, so it has sent its wakeup frame.
    pub fn new(id: NodeId, region_len: usize, state: NodeState) -> Self {
        let mut node = Self {
            id,
            flash: vec![0xff; region_len],
            state,
            outbox: VecDeque::new(),
            block: None,
            wakeup: None,
            running: None,
            blocks_written: 0,
            erases: 0,
            power_loss_after: None,
            corrupt_writes: false,
            fail_writes: false,
        };

        if state == NodeState::Wakeup {
            node.power_on();
        }

        node
    }

    /// Resets the node: it checks its image and sends a wakeup frame.
    pub fn power_on(&mut self) {
        self.outbox.clear();
        self.block = None;
        self.running = None;
        self.state = NodeState::Wakeup;

        let image = self.validate();
        self.wakeup = Some(image);
        self.outbox.push_back(Frame::wakeup(self.id, &image));
    }

    /// Ends the wakeup window without a catch.
    pub fn let_window_close(&mut self) {
        if self.state == NodeState::Wakeup {
            match self.validate() {
                Ok(version) => self.start(version),
                Err(_) => self.state = NodeState::Bootloader,
            }
        }
    }

    /// Puts an image in flash, as if a probe had written it.
    pub fn install(&mut self, image: &[u8]) {
        self.flash[..image.len()].copy_from_slice(image);
    }

    pub fn lose_power_after_blocks(&mut self, blocks: usize) {
        self.power_loss_after = Some(blocks);
    }

    /// Flips a bit in every block after its CRC check, like a bad write.
    pub fn corrupt_writes(&mut self) {
        self.corrupt_writes = true;
    }

    /// Makes the flash peripheral report a failure on every write.
    pub fn fail_writes(&mut self) {
        self.fail_writes = true;
    }

    pub fn state(&self) -> NodeState {
        self.state
    }

    pub fn flash(&self) -> &[u8] {
        &self.flash
    }

    /// What the last wakeup frame reported.
    pub fn wakeup(&self) -> Option<Result<SemVer, ImageError>> {
        self.wakeup
    }

    /// The version of the application the node started, if it is running one.
    pub fn running(&self) -> Option<SemVer> {
        self.running
    }

    pub fn blocks_written(&self) -> usize {
        self.blocks_written
    }

    pub fn erases(&self) -> usize {
        self.erases
    }

    fn validate(&self) -> Result<SemVer, ImageError> {
        image::validate(&self.flash, &mut SoftwareCrc32)
    }

    fn start(&mut self, version: SemVer) {
        self.state = NodeState::Application;
        self.running = Some(version);
    }

    fn respond(&mut self, result: boot_protocol::Result<()>) {
        self.outbox.push_back(Frame::response(result));
    }

    fn handle(&mut self, frame: &Frame) {
        match self.state {
            NodeState::Off | NodeState::Application => {}
            NodeState::Wakeup => {
                if frame.is_catch(self.id) {
                    self.state = NodeState::Bootloader;
                }
            }
            NodeState::Bootloader if self.block.is_some() => self.collect(frame),
            NodeState::Bootloader => self.command(frame),
        }
    }

    fn command(&mut self, frame: &Frame) {
        match frame.as_command(self.id) {
            Some(Ok(Command::Start)) => match self.validate() {
                Ok(version) => self.start(version),
                Err(_) => self.respond(Err(Error::InvalidImage)),
            },
            Some(Ok(Command::Erase)) => {
                self.flash.fill(0xff);
                self.erases += 1;
                self.respond(Ok(()));
            }
            Some(Ok(Command::Program { offset, len, crc })) => {
                match check_program(offset, len, self.flash.len() as u32)
                    .and_then(|()| Block::new(offset, len as usize))
                {
                    Ok(block) => self.block = Some((block, crc)),
                    Err(error) => self.respond(Err(error)),
                }
            }
            Some(Err(error)) => self.respond(Err(error)),
            None => {}
        }
    }

    fn collect(&mut self, frame: &Frame) {
        let Some(sub_block) = frame.as_block_data(self.id) else {
            return;
        };
        let Some((block, crc)) = self.block.as_mut() else {
            return;
        };

        let result = match block.insert(&sub_block) {
            Ok(false) => return,
            Ok(true) => block.validate(*crc),
            Err(error) => Err(error),
        };
        let (block, _) = self.block.take().expect("collecting a block");

        let result = result.and_then(|()| self.write(&block));
        if self.state != NodeState::Off {
            self.respond(result);
        }
    }

    /// Writes like flash does: only over erased bytes.
    fn write(&mut self, block: &Block<MAX_BLOCK_LEN>) -> boot_protocol::Result<()> {
        let start = block.offset() as usize;
        let target = &mut self.flash[start..start + block.data().len()];

        if self.fail_writes || target.iter().any(|byte| *byte != 0xff) {
            return Err(Error::Program);
        }

        target.copy_from_slice(block.data());
        if self.corrupt_writes {
            target[0] ^= 0x01;
        }

        self.blocks_written += 1;
        if self.power_loss_after == Some(self.blocks_written) {
            self.power_loss_after = None;
            self.state = NodeState::Off;
        }

        Ok(())
    }
}

/// A bus with one [`FakeNode`] on it. Frames reach the node at once, and
/// its answers queue up for [`Bus::receive`].
pub struct FakeBus {
    pub node: FakeNode,
}

impl FakeBus {
    pub fn new(node: FakeNode) -> Self {
        Self { node }
    }

    /// The programmer opens the interface again: frames the node sent
    /// before then are gone.
    pub fn reconnect(&mut self) {
        self.node.outbox.clear();
    }
}

impl Bus for FakeBus {
    async fn send(&mut self, frame: &Frame) -> Result<(), BusError> {
        self.node.handle(frame);
        Ok(())
    }

    async fn receive(&mut self) -> Result<Frame, BusError> {
        match self.node.outbox.pop_front() {
            Some(frame) => Ok(frame),
            None => std::future::pending().await,
        }
    }
}
