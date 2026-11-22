/// Builds a CRC-8 lookup table for `polynomial` at compile time, with no
/// reflection and no final XOR.
pub const fn crc8_table(polynomial: u8) -> [u8; 256] {
    let mut table = [0u8; 256];
    let mut i = 0;

    while i < 256 {
        let mut value = i as u8;
        let mut bit = 0;

        while bit < 8 {
            value = if value & 0x80 != 0 {
                (value << 1) ^ polynomial
            } else {
                value << 1
            };
            bit += 1;
        }

        table[i] = value;
        i += 1;
    }

    table
}

/// CRC-8 of `bytes` with initial value 0, using a table from [`crc8_table`].
pub fn crc8(bytes: &[u8], table: &[u8; 256]) -> u8 {
    bytes
        .iter()
        .fold(0, |crc, byte| table[(crc ^ byte) as usize])
}

const NISSAN_TABLE: [u8; 256] = crc8_table(0x85);

/// The CRC Nissan puts in the last byte of several EV-CAN messages:
/// polynomial `0x85`, initial value `0x00`.
pub fn nissan_crc(bytes: &[u8]) -> u8 {
    crc8(bytes, &NISSAN_TABLE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_matches_known_values() {
        let table = crc8_table(0x07);

        assert_eq!(table[0], 0x00);
        assert_eq!(table[8], 0x38);
        assert_eq!(table[16], 0x70);
        assert_eq!(table[255], 0xf3);
    }

    #[test]
    fn crc8_check_value() {
        // The standard check value for CRC-8 with polynomial 0x07.
        assert_eq!(crc8(b"123456789", &crc8_table(0x07)), 0xf4);
    }

    /// 0x1D4 frames with their CRCs, from dalathegreat's Battery-Emulator
    /// (`NISSAN-LEAF-BATTERY.h` and `.cpp`), which runs against real Leaf
    /// hardware.
    #[test]
    fn nissan_crc_matches_captured_frames() {
        assert_eq!(
            nissan_crc(&[0x6e, 0x6e, 0x00, 0x04, 0x03, 0x06, 0xe0]),
            0x69
        );

        for (counter, crc) in [0x12, 0xd5, 0x19, 0xde].into_iter().enumerate() {
            let byte4 = ((counter as u8) << 6) | 0x07;

            assert_eq!(
                nissan_crc(&[0x6e, 0x6e, 0x00, 0x04, byte4, 0x46, 0xe0]),
                crc
            );
        }
    }

    #[test]
    fn nissan_table_starts_like_battery_emulators() {
        assert_eq!(
            NISSAN_TABLE[..16],
            [
                0, 133, 143, 10, 155, 30, 20, 145, 179, 54, 60, 185, 40, 173, 167, 34
            ]
        );
    }
}
