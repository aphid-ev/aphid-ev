use crate::{Error, Result, SUB_BLOCK_LEN, SubBlock};

/// CRC-16/IBM-SDLC without a lookup table, to keep the bootloader small.
const CRC: crc::Crc<u16, crc::NoTable> = crc::Crc::<u16, crc::NoTable>::new(&crc::CRC_16_IBM_SDLC);

/// The checksum **Program block** carries for `data`.
pub fn checksum(data: &[u8]) -> u16 {
    CRC.checksum(data)
}

/// A block being assembled from **Block data** frames, `N` bytes at most.
pub struct Block<const N: usize> {
    offset: u32,
    len: usize,
    data: [u8; N],
}

impl<const N: usize> Block<N> {
    /// Starts a block of `len` bytes at `offset` into the application region.
    pub fn new(offset: u32, len: usize) -> Result<Self> {
        if len > N {
            return Err(Error::Overflow);
        }

        Ok(Self {
            offset,
            len,
            data: [0u8; N],
        })
    }

    /// Places a sub-block by its index and reports whether the block is now
    /// complete: true when this sub-block ends exactly at the block's length.
    pub fn insert(&mut self, sub_block: &SubBlock) -> Result<bool> {
        let data = sub_block.data();
        let start = sub_block.index as usize * SUB_BLOCK_LEN;
        let end = start + data.len();

        if end > self.len {
            return Err(Error::Overflow);
        }

        self.data[start..end].copy_from_slice(data);

        Ok(end == self.len)
    }

    /// Checks the assembled bytes against the CRC from **Program block**.
    pub fn validate(&self, crc: u16) -> Result<()> {
        if checksum(self.data()) == crc {
            Ok(())
        } else {
            Err(Error::Checksum)
        }
    }

    pub fn offset(&self) -> u32 {
        self.offset
    }

    pub fn data(&self) -> &[u8] {
        &self.data[..self.len]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MAX_BLOCK_LEN;

    fn sub_blocks(data: &[u8]) -> Vec<SubBlock> {
        data.chunks(SUB_BLOCK_LEN)
            .enumerate()
            .map(|(index, chunk)| SubBlock::new(index as u8, chunk).unwrap())
            .collect()
    }

    fn assemble(data: &[u8]) -> Result<Block<MAX_BLOCK_LEN>> {
        let mut block = Block::new(0x100, data.len())?;
        let parts = sub_blocks(data);

        for (n, part) in parts.iter().enumerate() {
            assert_eq!(block.insert(part)?, n == parts.len() - 1);
        }

        Ok(block)
    }

    #[test]
    fn crc_is_ibm_sdlc() {
        // The standard check value for CRC-16/IBM-SDLC.
        assert_eq!(checksum(b"123456789"), 0x906e);
    }

    #[test]
    fn assembles_blocks_of_every_tail_length() {
        for len in [1, 6, 7, 8, 13, 14, 100, MAX_BLOCK_LEN] {
            let data: Vec<u8> = (0..len).map(|n| n as u8).collect();
            let block = assemble(&data).unwrap();

            assert_eq!(block.offset(), 0x100);
            assert_eq!(block.data(), data.as_slice());
            assert_eq!(block.validate(checksum(&data)), Ok(()));
        }
    }

    #[test]
    fn rejects_a_wrong_crc() {
        let block = assemble(&[1, 2, 3]).unwrap();

        assert_eq!(block.validate(checksum(&[1, 2, 4])), Err(Error::Checksum));
    }

    #[test]
    fn rejects_blocks_longer_than_the_buffer() {
        assert!(matches!(Block::<16>::new(0, 17), Err(Error::Overflow)));
        assert!(Block::<16>::new(0, 16).is_ok());
    }

    #[test]
    fn rejects_data_past_the_length() {
        let mut block = Block::<MAX_BLOCK_LEN>::new(0, 10).unwrap();
        let past = SubBlock::new(1, &[0; 7]).unwrap();

        assert_eq!(block.insert(&past), Err(Error::Overflow));
    }

    /// Sub-blocks are placed by index, so order does not matter until the
    /// final one arrives.
    #[test]
    fn places_sub_blocks_by_index() {
        let data: Vec<u8> = (0..20).collect();
        let mut parts = sub_blocks(&data);
        let last = parts.pop().unwrap();
        let mut block = Block::<MAX_BLOCK_LEN>::new(0, data.len()).unwrap();

        for part in parts.iter().rev() {
            assert_eq!(block.insert(part), Ok(false));
        }
        assert_eq!(block.insert(&last), Ok(true));
        assert_eq!(block.data(), data.as_slice());
    }

    /// A final sub-block that arrives early ends the block with a hole,
    /// which the CRC then catches.
    #[test]
    fn an_early_final_sub_block_fails_the_crc() {
        let data: Vec<u8> = (1..=20).collect();
        let parts = sub_blocks(&data);
        let mut block = Block::<MAX_BLOCK_LEN>::new(0, data.len()).unwrap();

        assert_eq!(block.insert(&parts[0]), Ok(false));
        assert_eq!(block.insert(&parts[2]), Ok(true));
        assert_eq!(block.validate(checksum(&data)), Err(Error::Checksum));
    }
}
