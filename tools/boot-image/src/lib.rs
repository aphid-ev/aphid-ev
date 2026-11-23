//! Turns a linked application ELF into the image the bootloader accepts.
//!
//! The application links an unsealed header (`boot_protocol::image_header!`).
//! This tool lays the ELF's loadable segments out as they will sit in flash,
//! fills the header's length and CRC, and writes the sealed header back into
//! the ELF as well, so an image flashed with a debug probe passes the same
//! check as one flashed over CAN.

use boot_protocol::image::{self, HEADER_LEN, HEADER_OFFSET};
use boot_protocol::{ImageError, SemVer};
use object::elf::PT_LOAD;
use object::read::elf::{ElfFile32, ProgramHeader};

/// Larger than any application region, so a segment linked into RAM rather
/// than flash shows up as an error instead of a buffer of gigabytes.
const MAX_IMAGE_LEN: usize = 1024 * 1024;

/// Value of flash that nothing was written to.
const ERASED: u8 = 0xff;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("not a 32-bit little-endian ELF: {0}")]
    Elf(#[from] object::Error),
    #[error("the ELF has no loadable segments")]
    Empty,
    #[error("a segment's data lies outside the file")]
    Truncated,
    #[error("segments at {0:#010x} and {1:#010x} overlap")]
    Overlap(u32, u32),
    #[error("the image spans {0} bytes; is a segment linked outside flash?")]
    TooLarge(usize),
    #[error(
        "no image header at offset {HEADER_OFFSET:#x}: call boot_protocol::image_header!() and place .image_header in memory.x"
    )]
    MissingHeader,
    #[error("the image cannot be sealed: {0:?}")]
    Seal(ImageError),
}

/// One loadable segment: the bytes that go to flash at `address`, found at
/// `file_offset` in the ELF.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Segment<'a> {
    pub address: u32,
    pub data: &'a [u8],
    pub file_offset: usize,
}

/// A sealed image, as the programmer sends it.
#[derive(Debug)]
pub struct Sealed {
    /// Flash address of the first byte, the start of the application region.
    pub start: u32,
    pub image: Vec<u8>,
    pub version: SemVer,
    pub crc: u32,
}

/// The loadable segments of `elf` with data in them, by physical (load)
/// address: `.data` counts at its place in flash, not in RAM.
pub fn segments(elf: &[u8]) -> Result<Vec<Segment<'_>>, Error> {
    let file = ElfFile32::<object::Endianness>::parse(elf)?;
    let endian = file.endian();
    let mut segments = Vec::new();

    for header in file.elf_program_headers() {
        if header.p_type(endian) != PT_LOAD || header.p_filesz(endian) == 0 {
            continue;
        }

        segments.push(Segment {
            address: header.p_paddr(endian),
            data: header.data(endian, elf).map_err(|()| Error::Truncated)?,
            file_offset: header.p_offset(endian) as usize,
        });
    }

    Ok(segments)
}

/// Lays `segments` out as flash holds them: from the lowest address, gaps
/// left erased, padded with erased bytes to a whole word.
pub fn layout(segments: &[Segment]) -> Result<(u32, Vec<u8>), Error> {
    let mut sorted = segments.to_vec();
    sorted.sort_by_key(|segment| segment.address);

    let start = sorted.first().ok_or(Error::Empty)?.address;
    let mut image = Vec::new();

    for pair in sorted.windows(2) {
        if pair[0].address as u64 + pair[0].data.len() as u64 > pair[1].address as u64 {
            return Err(Error::Overlap(pair[0].address, pair[1].address));
        }
    }

    for segment in &sorted {
        let offset = (segment.address - start) as usize;
        let end = offset + segment.data.len();

        if end > MAX_IMAGE_LEN {
            return Err(Error::TooLarge(end));
        }

        image.resize(end.max(image.len()), ERASED);
        image[offset..end].copy_from_slice(segment.data);
    }

    image.resize(image.len().next_multiple_of(4), ERASED);

    Ok((start, image))
}

/// Seals the image in `elf` and patches the sealed header back into it.
pub fn seal(elf: &mut [u8]) -> Result<Sealed, Error> {
    let segments = segments(elf)?;
    let (start, mut image) = layout(&segments)?;

    let version = image::seal(&mut image).map_err(|error| match error {
        ImageError::Missing => Error::MissingHeader,
        error => Error::Seal(error),
    })?;

    let header = &image[HEADER_OFFSET..HEADER_OFFSET + HEADER_LEN];
    let address = start + HEADER_OFFSET as u32;
    let file_offset = segments
        .iter()
        .find(|segment| {
            address >= segment.address
                && (address - segment.address) as usize + HEADER_LEN <= segment.data.len()
        })
        .map(|segment| segment.file_offset + (address - segment.address) as usize)
        .ok_or(Error::MissingHeader)?;

    let crc =
        boot_protocol::ImageHeader::from_bytes(header.try_into().expect("sixteen bytes")).crc();
    elf[file_offset..file_offset + HEADER_LEN].copy_from_slice(header);

    Ok(Sealed {
        start,
        image,
        version,
        crc,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn segment(address: u32, data: &[u8]) -> Segment<'_> {
        Segment {
            address,
            data,
            file_offset: 0,
        }
    }

    #[test]
    fn gaps_and_the_tail_are_erased() {
        let (start, image) =
            layout(&[segment(0x0800_2008, &[3]), segment(0x0800_2000, &[1, 2])]).unwrap();

        assert_eq!(start, 0x0800_2000);
        assert_eq!(
            image,
            [
                1, 2, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 3, 0xff, 0xff, 0xff
            ]
        );
    }

    #[test]
    fn an_aligned_image_is_not_padded() {
        let (_, image) = layout(&[segment(0x0800_2000, &[0; 8])]).unwrap();

        assert_eq!(image.len(), 8);
    }

    #[test]
    fn overlapping_segments_fail() {
        let result = layout(&[segment(0x0800_2000, &[0; 8]), segment(0x0800_2004, &[0; 4])]);

        assert!(matches!(
            result,
            Err(Error::Overlap(0x0800_2000, 0x0800_2004))
        ));
    }

    #[test]
    fn a_segment_in_ram_fails() {
        let result = layout(&[segment(0x0800_2000, &[0; 8]), segment(0x2000_0000, &[0; 4])]);

        assert!(matches!(result, Err(Error::TooLarge(_))));
    }

    #[test]
    fn no_segments_fail() {
        assert!(matches!(layout(&[]), Err(Error::Empty)));
    }

    #[test]
    fn not_an_elf_fails() {
        assert!(matches!(seal(&mut [0u8; 64]), Err(Error::Elf(_))));
    }
}
