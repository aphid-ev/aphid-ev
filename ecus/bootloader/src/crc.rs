use crate::pac::{CRC, RCC};
use boot_protocol::image::Crc32;

/// The CRC calculation unit, set up for CRC-32/ISO-HDLC.
///
/// The unit's default polynomial (0x04C11DB7) and initial value (0xFFFFFFFF)
/// are the right ones. Reversing the input by word and the output gives the
/// reflected CRC-32 for little-endian words read from flash, and the final
/// XOR is done in software. `boot-protocol` checks this configuration against
/// a model of the unit in its host tests.
pub struct Crc {
    crc: CRC,
}

impl Crc {
    pub fn new(crc: CRC, rcc: &mut RCC) -> Self {
        rcc.ahb1enr().modify(|_, w| w.crcen().set_bit());

        crc.cr().write(|w| {
            w.polysize()
                .polysize32()
                .rev_in()
                .word()
                .rev_out()
                .reversed()
        });

        Self { crc }
    }

    /// `validate` only passes whole words. A stray tail byte would only
    /// make the CRC wrong, so it is dropped rather than turned into a panic:
    /// this runs on every reset, and a panic here would halt the node before
    /// it can be reached over CAN.
    fn feed(&mut self, data: &[u8]) {
        let (words, _) = data.as_chunks::<4>();

        for word in words {
            self.crc
                .dr()
                .write(|w| unsafe { w.bits(u32::from_le_bytes(*word)) });
        }
    }
}

impl Crc {
    /// Puts the CRC unit back in its reset state and stops its clock.
    pub fn reset(self, rcc: &mut RCC) {
        rcc.ahb1rstr().modify(|_, w| w.crcrst().set_bit());
        rcc.ahb1rstr().modify(|_, w| w.crcrst().clear_bit());
        rcc.ahb1enr().modify(|_, w| w.crcen().clear_bit());
    }
}

impl Crc32 for Crc {
    fn checksum(&mut self, head: &[u8], tail: &[u8]) -> u32 {
        self.crc.cr().modify(|_, w| w.reset().set_bit());

        self.feed(head);
        self.feed(tail);

        !self.crc.dr().read().bits()
    }
}
