//! CAN protocol between the ECU bootloader and the programming device.
//!
//! The specification is the wiki page at <https://bladlus.se/wiki/bootloader/>;
//! this crate is its single implementation. Bootloaders in the field cannot be
//! updated over CAN, so extend the protocol compatibly and never change what an
//! existing frame means.
//!
//! Everything here is plain data and decoding. The bootloader moves [`Frame`]s
//! between this crate and its CAN peripheral, writes assembled [`Block`]s to
//! flash, and checks the application [`image`] before it starts it.

// Tests link the standard test harness, so std is only excluded otherwise.
#![cfg_attr(not(test), no_std)]

mod block;
mod frame;
pub mod image;
mod timeout;

pub use block::*;
pub use frame::*;
pub use image::{ImageError, ImageHeader, parse_version_part};
pub use timeout::*;

/// Sent by a node on startup. Carries no node ID in the identifier.
pub const WAKEUP_ID: u16 = 0x7ff;
/// Catch bootloader, `CATCH_BASE_ID + node`.
pub const CATCH_BASE_ID: u16 = 0x7f0;
/// Command response, shared by every node: `0x7ef + node` would collide with
/// the catch frames.
pub const RESPONSE_ID: u16 = 0x7ef;
/// Bootloader command, `COMMAND_BASE_ID + node`.
pub const COMMAND_BASE_ID: u16 = 0x7e0;
/// Block data, `BLOCK_DATA_BASE_ID + node`.
pub const BLOCK_DATA_BASE_ID: u16 = 0x7d0;

/// Not a bootloader frame: kept free for bus tests, such as `can-usb ping`
/// against a node running `can-echo`. It sits just below the bootloader's
/// identifiers, so test traffic never looks like a bootloader frame, and it
/// is not one of the OBD-II diagnostic identifiers (`0x7df`, `0x7e0` to
/// `0x7ef`).
pub const BUS_TEST_ID: u16 = 0x7cf;

/// The answer to a [`BUS_TEST_ID`] frame: `can-echo` sends the same data
/// back on this identifier.
///
/// A test node streams on [`BUS_TEST_ID`] as fast as the bus takes frames.
/// If the echo went out on the same identifier, the two nodes would start
/// frames together, tie in arbitration and then differ in the data, which
/// is a bit error for the one that loses. A different identifier settles
/// arbitration instead.
pub const BUS_TEST_ECHO_ID: u16 = 0x7ce;

/// The standard identifier `can-echo` answers a frame on `id` with:
/// [`BUS_TEST_ECHO_ID`] for [`BUS_TEST_ID`], `id` itself for everything
/// else.
pub const fn echo_id(id: u16) -> u16 {
    if id == BUS_TEST_ID {
        BUS_TEST_ECHO_ID
    } else {
        id
    }
}

/// How long a node waits for **Catch bootloader** after its wakeup frame
/// before it starts the application.
pub const WAKEUP_WINDOW_MS: u32 = 500;

/// Guards **Erase flash**, so a stray frame cannot wipe the application.
pub const ERASE_KEY: u32 = 0x1234_5678;

/// Payload bytes in one **Block data** frame; the first byte is the index.
pub const SUB_BLOCK_LEN: usize = 7;

/// The largest block: 256 sub-blocks, as many as a one-byte index can name.
pub const MAX_BLOCK_LEN: usize = 256 * SUB_BLOCK_LEN;

/// Block offsets are multiples of this, the flash programming granularity.
pub const PROGRAM_ALIGNMENT: u32 = 8;

/// Codes carried by an error response. The discriminants are wire values.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// Block data ran past the length given in **Program block**, or the
    /// block is empty, longer than [`MAX_BLOCK_LEN`] or does not fit in the
    /// application region.
    Overflow = 0,
    /// The assembled block did not match the CRC in **Program block**.
    Checksum = 1,
    /// The node could not build a response frame.
    Header = 2,
    /// The CAN peripheral rejected a transmission.
    FdCan = 3,
    /// **Erase flash** carried the wrong key.
    InvalidKey = 4,
    /// Block offset is not aligned to [`PROGRAM_ALIGNMENT`].
    Alignment = 5,
    /// The flash peripheral reported an erase or programming failure.
    Program = 6,
    /// **Start application** found no valid image, so the node stays in the
    /// bootloader.
    InvalidImage = 7,
    /// No **Block data** arrived for [`BLOCK_TIMEOUT_MS`] while the node
    /// collected a block. The block is dropped and nothing is written.
    Timeout = 8,
}

impl Error {
    /// Decodes an error code, as the programming device sees it.
    pub const fn from_code(code: u8) -> Option<Self> {
        Some(match code {
            0 => Self::Overflow,
            1 => Self::Checksum,
            2 => Self::Header,
            3 => Self::FdCan,
            4 => Self::InvalidKey,
            5 => Self::Alignment,
            6 => Self::Program,
            7 => Self::InvalidImage,
            8 => Self::Timeout,
            _ => return None,
        })
    }
}

pub type Result<T> = core::result::Result<T, Error>;

/// A node's address on the bus, 0 to 15.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeId(u8);

impl NodeId {
    /// The bus master.
    pub const MASTER: Self = Self(0);
    /// The programming device. It never runs a bootloader, so its catch and
    /// command identifiers may alias [`WAKEUP_ID`] and [`RESPONSE_ID`].
    pub const PROGRAMMER: Self = Self(15);

    /// `None` above 15, where the per-node identifiers would run into the
    /// next range.
    pub const fn new(id: u8) -> Option<Self> {
        if id < 16 { Some(Self(id)) } else { None }
    }

    pub const fn get(self) -> u8 {
        self.0
    }
}

/// An application version, from its image header and reported in the wakeup
/// frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SemVer {
    pub major: u8,
    pub minor: u8,
    pub patch: u8,
}

impl SemVer {
    /// What **Wakeup** reports when the node has no valid image.
    pub const NONE: Self = Self {
        major: 0,
        minor: 0,
        patch: 0,
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_ids_stop_at_fifteen() {
        assert_eq!(NodeId::new(15), Some(NodeId::PROGRAMMER));
        assert_eq!(NodeId::new(16), None);
    }

    /// The identifiers of the nodes that run a bootloader must not overlap
    /// each other or the shared ones.
    #[test]
    fn identifier_ranges_do_not_overlap() {
        let ranges = [BLOCK_DATA_BASE_ID, COMMAND_BASE_ID, CATCH_BASE_ID];
        let mut ids: Vec<u16> = ranges.iter().flat_map(|base| *base..base + 15).collect();
        ids.extend([RESPONSE_ID, WAKEUP_ID]);
        let count = ids.len();

        ids.sort_unstable();
        ids.dedup();

        assert_eq!(ids.len(), count);
        assert!(ids.iter().all(|id| *id <= 0x7ff));
    }

    /// The bus test identifier must not be mistaken for a bootloader frame
    /// of any node, the programming device included.
    #[test]
    fn bus_test_identifier_is_outside_the_bootloader_range() {
        let bootloader = BLOCK_DATA_BASE_ID..=WAKEUP_ID;

        assert!(!bootloader.contains(&BUS_TEST_ID));
    }

    /// The echo is a bus test frame too, and must not tie with the frame it
    /// answers.
    #[test]
    fn bus_test_echo_identifier_is_outside_the_bootloader_range_and_its_own() {
        let bootloader = BLOCK_DATA_BASE_ID..=WAKEUP_ID;

        assert!(!bootloader.contains(&BUS_TEST_ECHO_ID));
        assert_ne!(BUS_TEST_ECHO_ID, BUS_TEST_ID);
    }

    #[test]
    fn only_the_bus_test_identifier_is_echoed_on_another() {
        assert_eq!(echo_id(BUS_TEST_ID), BUS_TEST_ECHO_ID);
        assert_eq!(echo_id(BUS_TEST_ECHO_ID), BUS_TEST_ECHO_ID);
        assert_eq!(echo_id(0x123), 0x123);
        assert_eq!(echo_id(0), 0);
        assert_eq!(echo_id(0x7ff), 0x7ff);
    }

    /// The programming device never runs a bootloader, which is what makes
    /// its own per-node identifiers safe to alias the shared ones.
    #[test]
    fn programmer_identifiers_alias_the_shared_ones() {
        let programmer = NodeId::PROGRAMMER.get() as u16;

        assert_eq!(CATCH_BASE_ID + programmer, WAKEUP_ID);
        assert_eq!(COMMAND_BASE_ID + programmer, RESPONSE_ID);
    }

    #[test]
    fn error_codes_round_trip() {
        for code in 0..=8 {
            assert_eq!(Error::from_code(code).map(|e| e as u8), Some(code));
        }
        assert_eq!(Error::from_code(9), None);
    }

    #[test]
    fn max_block_fits_the_index() {
        assert_eq!(MAX_BLOCK_LEN / SUB_BLOCK_LEN, u8::MAX as usize + 1);
    }
}
