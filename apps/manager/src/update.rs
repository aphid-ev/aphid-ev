//! Firmware update over the CAN bootloader, the programming device's side
//! of `boot-protocol`.
//!
//! The sequence is catch, erase, program every block, start. The node checks
//! the image itself before it starts it, so a run that stops anywhere leaves
//! the node in its bootloader, and running the update again resumes it: the
//! catch step also finds a node that is already waiting in its bootloader.

use std::time::Duration;

use boot_protocol::image::{self, SoftwareCrc32};
use boot_protocol::{
    COMMAND_BASE_ID, Command, ERASE_KEY, Error, ImageError, MAX_BLOCK_LEN, NodeId, SUB_BLOCK_LEN,
    SemVer, SubBlock, checksum,
};
use tokio::time::{Instant, timeout_at};

use crate::bus::{Bus, BusError, Frame};

/// Bytes per **Program block**: the largest block, which is a whole number
/// of double words, so every block offset stays aligned.
const BLOCK_LEN: usize = MAX_BLOCK_LEN;
const _: () = assert!(BLOCK_LEN.is_multiple_of(boot_protocol::PROGRAM_ALIGNMENT as usize));

/// The key of the probe: an **Erase flash** with the wrong key. A node in
/// its bootloader refuses it with [`Error::InvalidKey`] and erases nothing,
/// which tells the programmer the node is listening.
const PROBE_KEY: u32 = !ERASE_KEY;

/// A sealed application image, checked before anything is sent.
#[derive(Debug, Clone)]
pub struct Image {
    bytes: Vec<u8>,
    version: SemVer,
}

impl Image {
    /// Takes the raw image the image tool writes (`can-echo.bin`). Refuses
    /// an image the bootloader would refuse after flashing: unsealed, cut
    /// short or corrupted.
    pub fn new(bytes: Vec<u8>) -> Result<Self, ImageError> {
        let version = image::validate(&bytes, &mut SoftwareCrc32)?;

        Ok(Self { bytes, version })
    }

    pub fn version(&self) -> SemVer {
        self.version
    }

    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// The blocks to program, as offsets and data. Blocks that are all
    /// `0xff` are skipped: erased flash already holds them.
    fn blocks(&self) -> impl Iterator<Item = (u32, &[u8])> {
        self.bytes
            .chunks(BLOCK_LEN)
            .enumerate()
            .map(|(n, data)| ((n * BLOCK_LEN) as u32, data))
            .filter(|(_, data)| data.iter().any(|byte| *byte != 0xff))
    }
}

/// How long each step may take.
#[derive(Debug, Clone, Copy)]
pub struct Timeouts {
    /// Waiting for the node: for its wakeup frame after a reset, or for an
    /// answer to the probe from a node already in its bootloader.
    pub catch: Duration,
    /// Between probes while catching.
    pub probe_interval: Duration,
    /// **Erase flash** over the whole application region.
    pub erase: Duration,
    /// One **Program block**, from its command to the response.
    pub block: Duration,
    /// How long to wait for a refusal after **Start application**. A node
    /// with a valid image starts it without answering.
    pub start: Duration,
}

impl Default for Timeouts {
    fn default() -> Self {
        Self {
            catch: Duration::from_secs(30),
            probe_interval: Duration::from_millis(100),
            // 252 pages of up to 40 ms each on the STM32G491 (RM0440 tERASE).
            erase: Duration::from_secs(15),
            block: Duration::from_secs(2),
            start: Duration::from_millis(300),
        }
    }
}

/// Where an update is, for progress reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// Probing for the node. Reset or power-cycle it now.
    Catching,
    /// The node answered from its bootloader. `wakeup` is what its wakeup
    /// frame reported, if the programmer saw one.
    Caught {
        wakeup: Option<Result<SemVer, ImageError>>,
    },
    Erasing,
    /// `done` of `total` bytes are programmed.
    Programming {
        done: usize,
        total: usize,
    },
    Starting,
    Done,
}

#[derive(Debug, thiserror::Error)]
pub enum UpdateError {
    #[error(transparent)]
    Bus(#[from] BusError),
    #[error("node {0} did not answer from its bootloader; reset it while the update waits")]
    NotCaught(u8),
    #[error("no response to {0}")]
    NoResponse(&'static str),
    #[error("{command} refused: {error:?}")]
    Refused { command: &'static str, error: Error },
}

/// Flashes `image` into `node` and starts it. On any error the node is left
/// in its bootloader, or in its old application if the catch failed, and
/// running the update again starts over.
pub async fn update<B: Bus>(
    bus: &mut B,
    node: NodeId,
    image: &Image,
    timeouts: &Timeouts,
    mut progress: impl FnMut(Step),
) -> Result<(), UpdateError> {
    progress(Step::Catching);
    let wakeup = catch(bus, node, timeouts).await?;
    progress(Step::Caught { wakeup });

    progress(Step::Erasing);
    command(bus, node, &Command::Erase, "Erase flash", timeouts.erase).await?;

    let total = image.len();
    progress(Step::Programming { done: 0, total });
    for (offset, data) in image.blocks() {
        program(bus, node, offset, data, timeouts.block).await?;
        progress(Step::Programming {
            done: offset as usize + data.len(),
            total,
        });
    }

    progress(Step::Starting);
    start(bus, node, timeouts.start).await?;
    progress(Step::Done);

    Ok(())
}

/// Probes until the node answers from its bootloader, and catches it when
/// its wakeup frame shows it has just reset.
async fn catch<B: Bus>(
    bus: &mut B,
    node: NodeId,
    timeouts: &Timeouts,
) -> Result<Option<Result<SemVer, ImageError>>, UpdateError> {
    let deadline = Instant::now() + timeouts.catch;
    let probe = Frame::new(
        COMMAND_BASE_ID + node.get() as u16,
        &PROBE_KEY.to_be_bytes(),
    )
    .expect("a command identifier is a standard one");
    let mut wakeup = None;

    while Instant::now() < deadline {
        bus.send(&probe).await?;
        let listen_until = (Instant::now() + timeouts.probe_interval).min(deadline);

        while let Some(frame) = receive_until(bus, listen_until).await? {
            if let Some((sender, image)) = frame.as_wakeup()
                && sender == node
            {
                bus.send(&Frame::catch(node)).await?;
                wakeup = Some(image);
            }

            if frame.as_response() == Some(Err(Error::InvalidKey)) {
                return Ok(wakeup);
            }
        }
    }

    Err(UpdateError::NotCaught(node.get()))
}

async fn program<B: Bus>(
    bus: &mut B,
    node: NodeId,
    offset: u32,
    data: &[u8],
    timeout: Duration,
) -> Result<(), UpdateError> {
    let deadline = Instant::now() + timeout;
    let command = Command::Program {
        offset,
        len: data.len() as u16,
        crc: checksum(data),
    };
    bus.send(&Frame::command(node, &command)).await?;

    for (index, chunk) in data.chunks(SUB_BLOCK_LEN).enumerate() {
        let sub_block = SubBlock::new(index as u8, chunk).expect("one to seven bytes");
        bus.send(&Frame::block_data(node, &sub_block)).await?;
    }

    response(bus, deadline, "Program block").await
}

/// **Start application**: silence means the node started the image.
async fn start<B: Bus>(bus: &mut B, node: NodeId, timeout: Duration) -> Result<(), UpdateError> {
    let deadline = Instant::now() + timeout;
    bus.send(&Frame::command(node, &Command::Start)).await?;

    match response(bus, deadline, "Start application").await {
        Err(UpdateError::NoResponse(_)) | Ok(()) => Ok(()),
        Err(error) => Err(error),
    }
}

async fn command<B: Bus>(
    bus: &mut B,
    node: NodeId,
    command: &Command,
    name: &'static str,
    timeout: Duration,
) -> Result<(), UpdateError> {
    let deadline = Instant::now() + timeout;
    bus.send(&Frame::command(node, command)).await?;

    response(bus, deadline, name).await
}

/// Waits for the next **Command response**, ignoring every other frame.
async fn response<B: Bus>(
    bus: &mut B,
    deadline: Instant,
    command: &'static str,
) -> Result<(), UpdateError> {
    while let Some(frame) = receive_until(bus, deadline).await? {
        match frame.as_response() {
            Some(Ok(())) => return Ok(()),
            Some(Err(error)) => return Err(UpdateError::Refused { command, error }),
            None => {}
        }
    }

    Err(UpdateError::NoResponse(command))
}

/// The next frame, or `None` once `deadline` passes.
async fn receive_until<B: Bus>(bus: &mut B, deadline: Instant) -> Result<Option<Frame>, BusError> {
    match timeout_at(deadline, bus.receive()).await {
        Ok(frame) => frame.map(Some),
        Err(_) => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::{FakeBus, FakeNode, NodeState};
    use boot_protocol::image::{HEADER_OFFSET, ImageHeader, MIN_IMAGE_LEN};

    const NODE: NodeId = NodeId::MASTER;

    const VERSION: SemVer = SemVer {
        major: 2,
        minor: 1,
        patch: 0,
    };

    /// A sealed image of `len` bytes, with a run of erased bytes in the
    /// middle that the programmer can skip.
    fn image(len: usize) -> Image {
        let mut bytes: Vec<u8> = (0..len).map(|n| (n * 13 + 1) as u8).collect();
        bytes[HEADER_OFFSET..MIN_IMAGE_LEN]
            .copy_from_slice(&ImageHeader::unsealed(VERSION).to_bytes());
        if len > 3 * BLOCK_LEN {
            bytes[BLOCK_LEN..2 * BLOCK_LEN].fill(0xff);
        }
        image::seal(&mut bytes).unwrap();
        Image::new(bytes).unwrap()
    }

    fn bus(state: NodeState) -> FakeBus {
        FakeBus::new(FakeNode::new(NODE, 120 * 1024, state))
    }

    async fn run(bus: &mut FakeBus, image: &Image) -> (Result<(), UpdateError>, Vec<Step>) {
        let mut steps = Vec::new();
        let result = update(bus, NODE, image, &Timeouts::default(), |step| {
            steps.push(step)
        })
        .await;
        (result, steps)
    }

    #[tokio::test(start_paused = true)]
    async fn updates_a_node_that_just_reset() {
        let image = image(8000);
        let mut bus = bus(NodeState::Wakeup);

        let (result, steps) = run(&mut bus, &image).await;

        result.unwrap();
        assert_eq!(bus.node.running(), Some(VERSION));
        assert_eq!(&bus.node.flash()[..image.len()], image.bytes.as_slice());
        assert!(steps.contains(&Step::Caught {
            wakeup: Some(Err(ImageError::Missing))
        }));
        assert_eq!(steps.last(), Some(&Step::Done));
    }

    /// The node replaces an older image, and reports its version on wakeup.
    #[tokio::test(start_paused = true)]
    async fn reports_the_old_version_when_caught() {
        let old = SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        };
        let mut bus = bus(NodeState::Wakeup);
        bus.node.install(&image_with_version(old, 4000));
        bus.node.power_on();

        let (result, steps) = run(&mut bus, &image(4000)).await;

        result.unwrap();
        assert!(steps.contains(&Step::Caught {
            wakeup: Some(Ok(old))
        }));
        assert_eq!(bus.node.running(), Some(VERSION));
    }

    fn image_with_version(version: SemVer, len: usize) -> Vec<u8> {
        let mut bytes = vec![0u8; len];
        bytes[HEADER_OFFSET..MIN_IMAGE_LEN]
            .copy_from_slice(&ImageHeader::unsealed(version).to_bytes());
        image::seal(&mut bytes).unwrap();
        bytes
    }

    #[tokio::test(start_paused = true)]
    async fn skips_erased_blocks() {
        let image = image(5 * BLOCK_LEN);
        let mut bus = bus(NodeState::Wakeup);

        run(&mut bus, &image).await.0.unwrap();

        assert_eq!(bus.node.blocks_written(), 4);
        assert_eq!(bus.node.running(), Some(VERSION));
    }

    /// An update cut off partway leaves the node in its bootloader. The next
    /// run finds it there without a reset and finishes the job.
    #[tokio::test(start_paused = true)]
    async fn resumes_after_an_interrupted_update() {
        let image = image(10_000);
        let mut bus = bus(NodeState::Wakeup);
        bus.node.lose_power_after_blocks(2);

        let (result, _) = run(&mut bus, &image).await;
        assert!(matches!(
            result,
            Err(UpdateError::NoResponse("Program block"))
        ));

        bus.node.power_on();
        assert_eq!(bus.node.wakeup(), Some(Err(ImageError::Checksum)));
        bus.node.let_window_close();
        assert_eq!(bus.node.state(), NodeState::Bootloader);
        bus.reconnect();

        let (result, steps) = run(&mut bus, &image).await;

        result.unwrap();
        assert!(steps.contains(&Step::Caught { wakeup: None }));
        assert_eq!(bus.node.running(), Some(VERSION));
    }

    #[tokio::test(start_paused = true)]
    async fn gives_up_on_a_node_that_never_answers() {
        let mut bus = bus(NodeState::Application);

        let (result, _) = run(&mut bus, &image(4000)).await;

        assert!(matches!(result, Err(UpdateError::NotCaught(0))));
        assert_eq!(bus.node.erases(), 0);
    }

    #[tokio::test(start_paused = true)]
    async fn reports_a_refused_start() {
        let mut bus = bus(NodeState::Wakeup);
        bus.node.corrupt_writes();

        let (result, _) = run(&mut bus, &image(4000)).await;

        assert!(matches!(
            result,
            Err(UpdateError::Refused {
                command: "Start application",
                error: Error::InvalidImage
            })
        ));
        assert_eq!(bus.node.state(), NodeState::Bootloader);
    }

    #[tokio::test(start_paused = true)]
    async fn reports_a_refused_block() {
        let mut bus = bus(NodeState::Wakeup);
        bus.node.fail_writes();

        let (result, _) = run(&mut bus, &image(4000)).await;

        assert!(matches!(
            result,
            Err(UpdateError::Refused {
                command: "Program block",
                error: Error::Program
            })
        ));
    }

    #[test]
    fn refuses_an_unsealed_image() {
        let mut bytes = vec![0u8; 4000];
        bytes[HEADER_OFFSET..MIN_IMAGE_LEN]
            .copy_from_slice(&ImageHeader::unsealed(VERSION).to_bytes());

        assert_eq!(Image::new(bytes).unwrap_err(), ImageError::Length);
    }

    #[test]
    fn refuses_an_image_without_a_header() {
        assert_eq!(
            Image::new(vec![0xff; 4000]).unwrap_err(),
            ImageError::Missing
        );
    }

    #[test]
    fn blocks_are_aligned_and_cover_the_image() {
        let image = image(BLOCK_LEN * 2 + 100);
        let blocks: Vec<_> = image.blocks().collect();

        assert_eq!(blocks.len(), 3);
        assert!(blocks.iter().all(|(offset, _)| offset % 8 == 0));
        assert_eq!(blocks.last().unwrap().1.len(), 100);
    }
}
