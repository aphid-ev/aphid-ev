use crate::{
    BLOCK_DATA_BASE_ID, CATCH_BASE_ID, COMMAND_BASE_ID, ERASE_KEY, Error, ImageError, NodeId,
    RESPONSE_ID, Result, SUB_BLOCK_LEN, SemVer, WAKEUP_ID,
};

/// A classic CAN data frame with a standard identifier, the only kind the
/// protocol uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Frame {
    id: u16,
    len: usize,
    data: [u8; 8],
}

impl Frame {
    pub const MAX_STD_ID: u16 = 0x7ff;

    /// `None` when the identifier is not a standard one or the payload is
    /// longer than eight bytes.
    pub fn new(id: u16, data: &[u8]) -> Option<Self> {
        if id > Self::MAX_STD_ID || data.len() > 8 {
            return None;
        }

        let mut buffer = [0u8; 8];
        buffer[..data.len()].copy_from_slice(data);

        Some(Self {
            id,
            len: data.len(),
            data: buffer,
        })
    }

    pub fn id(&self) -> u16 {
        self.id
    }

    pub fn data(&self) -> &[u8] {
        &self.data[..self.len]
    }

    /// **Wakeup**, sent by a node on startup: the result of checking its
    /// application image. The version reads 0.0.0 when the image is invalid.
    pub fn wakeup(node: NodeId, image: &core::result::Result<SemVer, ImageError>) -> Self {
        let (version, status) = match image {
            Ok(version) => (*version, 0),
            Err(error) => (SemVer::NONE, *error as u8),
        };

        Self::fixed(
            WAKEUP_ID,
            [
                node.get(),
                version.major,
                version.minor,
                version.patch,
                status,
            ],
        )
    }

    /// Decodes a **Wakeup**, as the programming device sees it. `None` for
    /// any other frame, a node ID above 15 or an unknown status.
    pub fn as_wakeup(&self) -> Option<(NodeId, core::result::Result<SemVer, ImageError>)> {
        let [node, major, minor, patch, status] = *self.data() else {
            return None;
        };

        if self.id != WAKEUP_ID {
            return None;
        }

        let image = match status {
            0 => Ok(SemVer {
                major,
                minor,
                patch,
            }),
            code => Err(ImageError::from_code(code)?),
        };

        Some((NodeId::new(node)?, image))
    }

    /// **Catch bootloader**, sent by the programming device.
    pub fn catch(node: NodeId) -> Self {
        Self::fixed(CATCH_BASE_ID + node.get() as u16, [])
    }

    /// **Command response**: empty on success, one error code otherwise.
    pub fn response(result: Result<()>) -> Self {
        match result {
            Ok(()) => Self::fixed(RESPONSE_ID, []),
            Err(error) => Self::fixed(RESPONSE_ID, [error as u8]),
        }
    }

    /// A **Bootloader command** for `node`.
    pub fn command(node: NodeId, command: &Command) -> Self {
        let id = COMMAND_BASE_ID + node.get() as u16;

        match *command {
            Command::Start => Self::fixed(id, []),
            Command::Erase => Self::fixed(id, ERASE_KEY.to_be_bytes()),
            Command::Program { offset, len, crc } => {
                let [o0, o1, o2, o3] = offset.to_be_bytes();
                let [l0, l1] = len.to_be_bytes();
                let [c0, c1] = crc.to_be_bytes();

                Self::fixed(id, [o0, o1, o2, o3, l0, l1, c0, c1])
            }
        }
    }

    /// **Block data** for `node`.
    pub fn block_data(node: NodeId, sub_block: &SubBlock) -> Self {
        let mut data = [0u8; 8];
        data[0] = sub_block.index;
        data[1..=sub_block.len].copy_from_slice(sub_block.data());

        Self {
            id: BLOCK_DATA_BASE_ID + node.get() as u16,
            len: 1 + sub_block.len,
            data,
        }
    }

    fn fixed<const N: usize>(id: u16, payload: [u8; N]) -> Self {
        const { assert!(N <= 8) };

        let mut data = [0u8; 8];
        data[..N].copy_from_slice(&payload);

        Self { id, len: N, data }
    }

    pub fn is_catch(&self, node: NodeId) -> bool {
        self.id == CATCH_BASE_ID + node.get() as u16 && self.len == 0
    }

    /// Decodes a **Bootloader command**. Commands are told apart by length.
    ///
    /// `None` means the frame is not a command for this node. `Some(Err(..))`
    /// means it is one but has to be refused; the node owes the sender a
    /// response either way.
    pub fn as_command(&self, node: NodeId) -> Option<Result<Command>> {
        if self.id != COMMAND_BASE_ID + node.get() as u16 {
            return None;
        }

        let [d0, d1, d2, d3, d4, d5, d6, d7] = self.data;

        match self.len {
            0 => Some(Ok(Command::Start)),
            4 if u32::from_be_bytes([d0, d1, d2, d3]) == ERASE_KEY => Some(Ok(Command::Erase)),
            4 => Some(Err(Error::InvalidKey)),
            8 => Some(Ok(Command::Program {
                offset: u32::from_be_bytes([d0, d1, d2, d3]),
                len: u16::from_be_bytes([d4, d5]),
                crc: u16::from_be_bytes([d6, d7]),
            })),
            _ => None,
        }
    }

    /// Decodes **Block data** for this node, `None` for anything else.
    pub fn as_block_data(&self, node: NodeId) -> Option<SubBlock> {
        if self.id != BLOCK_DATA_BASE_ID + node.get() as u16 || self.len < 2 {
            return None;
        }

        let len = self.len - 1;
        let mut data = [0u8; SUB_BLOCK_LEN];
        data[..len].copy_from_slice(&self.data[1..self.len]);

        Some(SubBlock {
            index: self.data[0],
            len,
            data,
        })
    }

    /// Decodes a **Command response**, as the programming device sees it.
    /// `None` for any other frame or an unknown error code.
    pub fn as_response(&self) -> Option<Result<()>> {
        if self.id != RESPONSE_ID {
            return None;
        }

        match self.data() {
            [] => Some(Ok(())),
            [code] => Error::from_code(*code).map(Err),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    /// Jump to the application.
    Start,
    /// Erase the whole application region.
    Erase,
    /// Announce a block: `offset` into the application region, `len` bytes,
    /// CRC-16/IBM-SDLC over them.
    Program { offset: u32, len: u16, crc: u16 },
}

/// One **Block data** payload: up to seven bytes at `index * 7` in the block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubBlock {
    pub index: u8,
    len: usize,
    data: [u8; SUB_BLOCK_LEN],
}

impl SubBlock {
    /// `None` for an empty payload or one longer than seven bytes.
    pub fn new(index: u8, data: &[u8]) -> Option<Self> {
        if data.is_empty() || data.len() > SUB_BLOCK_LEN {
            return None;
        }

        let mut buffer = [0u8; SUB_BLOCK_LEN];
        buffer[..data.len()].copy_from_slice(data);

        Some(Self {
            index,
            len: data.len(),
            data: buffer,
        })
    }

    pub fn data(&self) -> &[u8] {
        &self.data[..self.len]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NODE: NodeId = NodeId::MASTER;

    fn node(id: u8) -> NodeId {
        NodeId::new(id).unwrap()
    }

    #[test]
    fn wakeup_carries_node_version_and_status() {
        let version = SemVer {
            major: 1,
            minor: 2,
            patch: 3,
        };
        let frame = Frame::wakeup(node(5), &Ok(version));

        assert_eq!(frame.id(), 0x7ff);
        assert_eq!(frame.data(), &[5, 1, 2, 3, 0]);
        assert_eq!(frame.as_wakeup(), Some((node(5), Ok(version))));
    }

    #[test]
    fn wakeup_without_a_valid_image_reports_why() {
        for error in [
            ImageError::Missing,
            ImageError::Length,
            ImageError::Checksum,
        ] {
            let frame = Frame::wakeup(node(2), &Err(error));

            assert_eq!(frame.data(), &[2, 0, 0, 0, error as u8]);
            assert_eq!(frame.as_wakeup(), Some((node(2), Err(error))));
        }
    }

    #[test]
    fn other_frames_are_not_wakeups() {
        assert_eq!(Frame::new(0x7ff, &[0, 1, 2, 3]).unwrap().as_wakeup(), None);
        assert_eq!(
            Frame::new(0x7fe, &[0, 1, 2, 3, 0]).unwrap().as_wakeup(),
            None
        );
        assert_eq!(
            Frame::new(0x7ff, &[0, 1, 2, 3, 9]).unwrap().as_wakeup(),
            None
        );
        assert_eq!(
            Frame::new(0x7ff, &[16, 1, 2, 3, 0]).unwrap().as_wakeup(),
            None
        );
    }

    #[test]
    fn catch_is_empty_and_addressed() {
        let frame = Frame::catch(node(3));

        assert_eq!(frame.id(), 0x7f3);
        assert!(frame.data().is_empty());
        assert!(frame.is_catch(node(3)));
        assert!(!frame.is_catch(node(4)));
    }

    #[test]
    fn catch_with_a_payload_is_not_a_catch() {
        assert!(!Frame::new(0x7f0, &[0]).unwrap().is_catch(NODE));
    }

    #[test]
    fn responses_are_empty_or_one_code() {
        assert_eq!(Frame::response(Ok(())).data(), &[] as &[u8]);
        assert_eq!(Frame::response(Err(Error::Checksum)).data(), &[1]);
        assert_eq!(Frame::response(Ok(())).id(), 0x7ef);

        for result in [Ok(()), Err(Error::Overflow), Err(Error::InvalidImage)] {
            assert_eq!(Frame::response(result).as_response(), Some(result));
        }
    }

    #[test]
    fn start_is_an_empty_command() {
        let frame = Frame::new(0x7e2, &[]).unwrap();

        assert_eq!(frame.as_command(node(2)), Some(Ok(Command::Start)));
    }

    #[test]
    fn erase_needs_the_key() {
        let good = Frame::new(0x7e0, &[0x12, 0x34, 0x56, 0x78]).unwrap();
        let bad = Frame::new(0x7e0, &[0x12, 0x34, 0x56, 0x79]).unwrap();

        assert_eq!(good.as_command(NODE), Some(Ok(Command::Erase)));
        assert_eq!(bad.as_command(NODE), Some(Err(Error::InvalidKey)));
    }

    #[test]
    fn program_block_is_big_endian() {
        let frame = Frame::new(0x7e0, &[0x00, 0x01, 0x02, 0x08, 0x06, 0xf9, 0xab, 0xcd]).unwrap();

        assert_eq!(
            frame.as_command(NODE),
            Some(Ok(Command::Program {
                offset: 0x0001_0208,
                len: 1785,
                crc: 0xabcd,
            }))
        );
    }

    #[test]
    fn commands_for_other_nodes_or_lengths_are_ignored() {
        assert_eq!(Frame::new(0x7e1, &[]).unwrap().as_command(NODE), None);
        assert_eq!(Frame::new(0x7e0, &[0; 3]).unwrap().as_command(NODE), None);
    }

    #[test]
    fn commands_round_trip() {
        for command in [
            Command::Start,
            Command::Erase,
            Command::Program {
                offset: 0x800,
                len: 7,
                crc: 0x1234,
            },
        ] {
            let frame = Frame::command(node(7), &command);

            assert_eq!(frame.as_command(node(7)), Some(Ok(command)));
        }
    }

    /// The final sub-block of most blocks is shorter than seven bytes. The
    /// original bootloader copied seven bytes into a shorter slice here and
    /// panicked.
    #[test]
    fn block_data_may_be_short() {
        for len in 1..=SUB_BLOCK_LEN {
            let payload: Vec<u8> = (1..=len as u8).collect();
            let mut bytes = vec![9];
            bytes.extend(&payload);
            let frame = Frame::new(0x7d0, &bytes).unwrap();
            let sub_block = frame.as_block_data(NODE).unwrap();

            assert_eq!(sub_block.index, 9);
            assert_eq!(sub_block.data(), payload.as_slice());
        }
    }

    #[test]
    fn block_data_needs_a_payload() {
        assert_eq!(Frame::new(0x7d0, &[0]).unwrap().as_block_data(NODE), None);
        assert_eq!(
            Frame::new(0x7d1, &[0, 1]).unwrap().as_block_data(NODE),
            None
        );
    }

    #[test]
    fn block_data_round_trips() {
        let sub_block = SubBlock::new(42, &[1, 2, 3]).unwrap();
        let frame = Frame::block_data(node(1), &sub_block);

        assert_eq!(frame.id(), 0x7d1);
        assert_eq!(frame.data(), &[42, 1, 2, 3]);
        assert_eq!(frame.as_block_data(node(1)), Some(sub_block));
    }

    #[test]
    fn frames_are_classic_and_standard() {
        assert!(Frame::new(0x800, &[]).is_none());
        assert!(Frame::new(0x7ff, &[0; 9]).is_none());
        assert!(SubBlock::new(0, &[]).is_none());
        assert!(SubBlock::new(0, &[0; 8]).is_none());
    }
}
