//! The header every application image carries, and the check the bootloader
//! runs on it before it starts the application.
//!
//! The header sits at [`HEADER_OFFSET`] from the start of the application
//! region, right after the vector table. The application reserves it with
//! [`image_header!`](crate::image_header) and links it there; the image tool
//! fills in the length and CRC after linking ([`seal`]). Until then the two
//! fields read as erased flash, so an image that was never sealed fails
//! validation the same way a missing one does.
//!
//! All fields are little-endian, as the MCU reads them.

use crate::SemVer;

/// Where the header starts, counted from the start of the application
/// region. The vector table of every supported chip ends before it.
pub const HEADER_OFFSET: usize = 0x200;

/// Size of the header in bytes.
pub const HEADER_LEN: usize = 16;

/// The smallest image: everything up to the end of the header.
pub const MIN_IMAGE_LEN: usize = HEADER_OFFSET + HEADER_LEN;

/// "AEVI", Aphid EV image, as a little-endian word.
pub const IMAGE_MAGIC: u32 = u32::from_le_bytes(*b"AEVI");

/// Where the CRC field sits in the image. The CRC covers every byte of the
/// image except these four.
const CRC_OFFSET: usize = HEADER_OFFSET + 8;

/// What an unsealed length or CRC field holds: erased flash.
const UNSEALED: u32 = 0xffff_ffff;

/// CRC-32/ISO-HDLC, the common CRC-32 of zlib and Ethernet. The STM32G4 CRC
/// unit computes it with input and output reversed, so the bootloader does not
/// need this software implementation.
const CRC32: crc::Crc<u32, crc::NoTable> =
    crc::Crc::<u32, crc::NoTable>::new(&crc::CRC_32_ISO_HDLC);

/// The application image header.
///
/// `repr(C)` with four-byte fields only, so the layout is the same in the
/// application's static and in [`ImageHeader::to_bytes`].
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageHeader {
    magic: u32,
    /// Bytes from the start of the application region to the end of the
    /// image, a multiple of four.
    length: u32,
    crc: u32,
    /// Major, minor, patch and one reserved byte.
    version: [u8; 4],
}

impl ImageHeader {
    /// The header an application links in, with the length and CRC left for
    /// the image tool to fill in.
    pub const fn unsealed(version: SemVer) -> Self {
        Self {
            magic: IMAGE_MAGIC,
            length: UNSEALED,
            crc: UNSEALED,
            version: [version.major, version.minor, version.patch, 0xff],
        }
    }

    pub fn from_bytes(bytes: &[u8; HEADER_LEN]) -> Self {
        let word =
            |n: usize| u32::from_le_bytes([bytes[n], bytes[n + 1], bytes[n + 2], bytes[n + 3]]);

        Self {
            magic: word(0),
            length: word(4),
            crc: word(8),
            version: [bytes[12], bytes[13], bytes[14], bytes[15]],
        }
    }

    pub fn to_bytes(&self) -> [u8; HEADER_LEN] {
        let mut bytes = [0u8; HEADER_LEN];
        bytes[0..4].copy_from_slice(&self.magic.to_le_bytes());
        bytes[4..8].copy_from_slice(&self.length.to_le_bytes());
        bytes[8..12].copy_from_slice(&self.crc.to_le_bytes());
        bytes[12..16].copy_from_slice(&self.version);
        bytes
    }

    pub fn version(&self) -> SemVer {
        let [major, minor, patch, _] = self.version;
        SemVer {
            major,
            minor,
            patch,
        }
    }

    pub fn length(&self) -> u32 {
        self.length
    }

    pub fn crc(&self) -> u32 {
        self.crc
    }

    /// Reads the header out of an image or a whole application region.
    fn read(image: &[u8]) -> Option<Self> {
        let bytes = image.get(HEADER_OFFSET..MIN_IMAGE_LEN)?.try_into().ok()?;
        Some(Self::from_bytes(bytes))
    }
}

/// Why an image failed validation. The discriminants are the wire values of
/// the status byte in **Wakeup**, where 0 means the image is valid.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageError {
    /// No header magic: erased flash, or an image built without a header.
    Missing = 1,
    /// The length is not sealed, is shorter than the header, runs past the
    /// region or is not a multiple of four.
    Length = 2,
    /// The image does not match its CRC: an interrupted or corrupted update.
    Checksum = 3,
}

impl ImageError {
    pub const fn from_code(code: u8) -> Option<Self> {
        Some(match code {
            1 => Self::Missing,
            2 => Self::Length,
            3 => Self::Checksum,
            _ => return None,
        })
    }
}

/// A CRC-32/ISO-HDLC engine: the CRC unit in the bootloader, software on the
/// host.
pub trait Crc32 {
    /// The CRC of `head` followed by `tail`. Both have a length that is a
    /// multiple of four, and on the target both are word aligned.
    fn checksum(&mut self, head: &[u8], tail: &[u8]) -> u32;
}

/// [`Crc32`] in software, for the image tool and the tests.
pub struct SoftwareCrc32;

impl Crc32 for SoftwareCrc32 {
    fn checksum(&mut self, head: &[u8], tail: &[u8]) -> u32 {
        let mut digest = CRC32.digest();
        digest.update(head);
        digest.update(tail);
        digest.finalize()
    }
}

/// The CRC of `image` without its CRC field.
fn image_crc(image: &[u8], crc: &mut impl Crc32) -> u32 {
    crc.checksum(&image[..CRC_OFFSET], &image[CRC_OFFSET + 4..])
}

/// Checks the image at the start of `region`, the whole application region,
/// and returns its version. The bootloader starts the application only when
/// this succeeds.
pub fn validate(region: &[u8], crc: &mut impl Crc32) -> Result<SemVer, ImageError> {
    let header = ImageHeader::read(region).ok_or(ImageError::Missing)?;

    if header.magic != IMAGE_MAGIC {
        return Err(ImageError::Missing);
    }

    let length = header.length as usize;

    if length < MIN_IMAGE_LEN || length > region.len() || !length.is_multiple_of(4) {
        return Err(ImageError::Length);
    }

    if image_crc(&region[..length], crc) != header.crc {
        return Err(ImageError::Checksum);
    }

    Ok(header.version())
}

/// Fills in the length and CRC of a linked image, `image` being exactly the
/// bytes from the start of the application region to the end of the image.
/// The caller pads the image to a multiple of four with `0xff`.
pub fn seal(image: &mut [u8]) -> Result<SemVer, ImageError> {
    let mut header = ImageHeader::read(image).ok_or(ImageError::Length)?;

    if header.magic != IMAGE_MAGIC {
        return Err(ImageError::Missing);
    }

    let length = u32::try_from(image.len()).map_err(|_| ImageError::Length)?;

    if !image.len().is_multiple_of(4) {
        return Err(ImageError::Length);
    }

    header.length = length;
    image[HEADER_OFFSET..MIN_IMAGE_LEN].copy_from_slice(&header.to_bytes());
    header.crc = image_crc(image, &mut SoftwareCrc32);
    image[HEADER_OFFSET..MIN_IMAGE_LEN].copy_from_slice(&header.to_bytes());

    Ok(header.version())
}

/// Parses one component of a version at compile time, for
/// [`image_header!`](crate::image_header). Fails the build on anything that is
/// not a number from 0 to 255.
#[doc(hidden)]
pub const fn parse_version_part(text: &str) -> u8 {
    let bytes = text.as_bytes();
    assert!(!bytes.is_empty(), "empty version component");

    let mut value: u32 = 0;
    let mut n = 0;
    while n < bytes.len() {
        let digit = bytes[n];
        assert!(digit.is_ascii_digit(), "version component is not a number");
        value = value * 10 + (digit - b'0') as u32;
        assert!(value <= u8::MAX as u32, "version component above 255");
        n += 1;
    }

    value as u8
}

/// Reserves the image header in an application, with the version from the
/// application's `Cargo.toml`.
///
/// The application's `memory.x` must place the `.image_header` section at
/// [`HEADER_OFFSET`] from the start of flash; `ecus/bootloader/can-echo`
/// shows how. The image tool then seals the linked image.
#[macro_export]
macro_rules! image_header {
    () => {
        #[unsafe(link_section = ".image_header")]
        #[used]
        static IMAGE_HEADER: $crate::ImageHeader = $crate::ImageHeader::unsealed($crate::SemVer {
            major: $crate::parse_version_part(env!("CARGO_PKG_VERSION_MAJOR")),
            minor: $crate::parse_version_part(env!("CARGO_PKG_VERSION_MINOR")),
            patch: $crate::parse_version_part(env!("CARGO_PKG_VERSION_PATCH")),
        });
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    const REGION_LEN: usize = 4096;

    const VERSION: SemVer = SemVer {
        major: 1,
        minor: 2,
        patch: 3,
    };

    /// A linked but unsealed image of `len` bytes: a fake vector table,
    /// the header, then code.
    fn unsealed(len: usize) -> Vec<u8> {
        let mut image: Vec<u8> = (0..len).map(|n| (n * 7 + 3) as u8).collect();
        image[HEADER_OFFSET..MIN_IMAGE_LEN]
            .copy_from_slice(&ImageHeader::unsealed(VERSION).to_bytes());
        image
    }

    fn sealed(len: usize) -> Vec<u8> {
        let mut image = unsealed(len);
        assert_eq!(seal(&mut image), Ok(VERSION));
        image
    }

    /// The application region as flash holds it after an update: the image,
    /// then erased flash.
    fn region(image: &[u8]) -> Vec<u8> {
        let mut region = vec![0xff; REGION_LEN];
        region[..image.len()].copy_from_slice(image);
        region
    }

    #[test]
    fn crc_is_iso_hdlc() {
        assert_eq!(SoftwareCrc32.checksum(b"1234", b"56789"), 0xcbf4_3926);
    }

    #[test]
    fn header_is_sixteen_bytes() {
        assert_eq!(core::mem::size_of::<ImageHeader>(), HEADER_LEN);
        assert_eq!(&IMAGE_MAGIC.to_le_bytes(), b"AEVI");
    }

    #[test]
    fn header_round_trips() {
        let header = ImageHeader::unsealed(VERSION);
        let bytes = header.to_bytes();

        assert_eq!(&bytes[..4], b"AEVI");
        assert_eq!(&bytes[4..12], &[0xff; 8]);
        assert_eq!(&bytes[12..], &[1, 2, 3, 0xff]);
        assert_eq!(ImageHeader::from_bytes(&bytes), header);
        assert_eq!(header.version(), VERSION);
    }

    #[test]
    fn a_sealed_image_validates() {
        for len in [MIN_IMAGE_LEN, 1024, REGION_LEN] {
            let image = sealed(len);
            let header = ImageHeader::read(&image).unwrap();

            assert_eq!(header.length() as usize, len);
            assert_eq!(validate(&region(&image), &mut SoftwareCrc32), Ok(VERSION));
        }
    }

    #[test]
    fn erased_flash_has_no_image() {
        assert_eq!(
            validate(&[0xff; REGION_LEN], &mut SoftwareCrc32),
            Err(ImageError::Missing)
        );
    }

    #[test]
    fn a_region_too_small_for_the_header_has_no_image() {
        assert_eq!(
            validate(&[0xff; 0x100], &mut SoftwareCrc32),
            Err(ImageError::Missing)
        );
    }

    /// An image flashed straight from the linker, without the image tool.
    #[test]
    fn an_unsealed_image_fails() {
        assert_eq!(
            validate(&region(&unsealed(1024)), &mut SoftwareCrc32),
            Err(ImageError::Length)
        );
    }

    #[test]
    fn an_image_without_a_header_fails() {
        let mut image = sealed(1024);
        image[HEADER_OFFSET] ^= 1;

        assert_eq!(
            validate(&region(&image), &mut SoftwareCrc32),
            Err(ImageError::Missing)
        );
        assert_eq!(seal(&mut image), Err(ImageError::Missing));
    }

    /// An update cut off partway leaves the rest of the region erased. The
    /// header itself is in the first block, so it is already written and
    /// claims the full length.
    #[test]
    fn an_interrupted_update_fails() {
        let image = sealed(2048);

        for written in [MIN_IMAGE_LEN, 1024, 2040] {
            assert_eq!(
                validate(&region(&image[..written]), &mut SoftwareCrc32),
                Err(ImageError::Checksum)
            );
        }
    }

    /// The same update interrupted in any order: one block that was never
    /// written, anywhere in the image, is enough to fail.
    #[test]
    fn a_missing_block_anywhere_fails() {
        let image = sealed(2048);

        for start in (0..2048)
            .step_by(256)
            .filter(|start| *start != HEADER_OFFSET)
        {
            let mut flash = region(&image);
            flash[start..start + 256].fill(0xff);

            assert_eq!(
                validate(&flash, &mut SoftwareCrc32),
                Err(ImageError::Checksum)
            );
        }
    }

    #[test]
    fn any_flipped_bit_fails() {
        let image = sealed(MIN_IMAGE_LEN + 64);

        for byte in (0..image.len()).filter(|n| !(HEADER_OFFSET..CRC_OFFSET).contains(n)) {
            let mut flash = region(&image);
            flash[byte] ^= 0x10;

            assert_eq!(
                validate(&flash, &mut SoftwareCrc32),
                Err(ImageError::Checksum),
                "byte {byte}"
            );
        }
    }

    #[test]
    fn lengths_outside_the_region_fail() {
        let image = sealed(1024);

        for length in [
            0,
            4,
            MIN_IMAGE_LEN as u32 - 4,
            1022,
            REGION_LEN as u32 + 4,
            u32::MAX - 3,
        ] {
            let mut flash = region(&image);
            let mut header = ImageHeader::read(&flash).unwrap();
            header.length = length;
            flash[HEADER_OFFSET..MIN_IMAGE_LEN].copy_from_slice(&header.to_bytes());

            assert_eq!(
                validate(&flash, &mut SoftwareCrc32),
                Err(ImageError::Length),
                "length {length}"
            );
        }
    }

    /// The image may fill the region exactly, but not one word more.
    #[test]
    fn an_image_may_fill_the_region() {
        let image = sealed(REGION_LEN);

        assert_eq!(validate(&image, &mut SoftwareCrc32), Ok(VERSION));
        assert_eq!(
            validate(&image[..REGION_LEN - 4], &mut SoftwareCrc32),
            Err(ImageError::Length)
        );
    }

    #[test]
    fn seal_needs_a_padded_image() {
        assert_eq!(seal(&mut unsealed(1025)), Err(ImageError::Length));
        assert_eq!(
            seal(&mut unsealed(MIN_IMAGE_LEN)[..MIN_IMAGE_LEN - 4]),
            Err(ImageError::Length)
        );
    }

    #[test]
    fn image_error_codes_round_trip() {
        for code in 1..=3 {
            assert_eq!(ImageError::from_code(code).map(|e| e as u8), Some(code));
        }
        assert_eq!(ImageError::from_code(0), None);
        assert_eq!(ImageError::from_code(4), None);
    }

    #[test]
    fn version_parts_parse() {
        assert_eq!(parse_version_part("0"), 0);
        assert_eq!(parse_version_part("42"), 42);
        assert_eq!(parse_version_part("255"), 255);
    }

    #[test]
    #[should_panic]
    fn version_parts_above_255_fail() {
        parse_version_part("256");
    }

    image_header!();

    #[test]
    fn the_macro_takes_the_crate_version() {
        let version = IMAGE_HEADER.version();

        assert_eq!(version.major.to_string(), env!("CARGO_PKG_VERSION_MAJOR"));
        assert_eq!(version.minor.to_string(), env!("CARGO_PKG_VERSION_MINOR"));
        assert_eq!(version.patch.to_string(), env!("CARGO_PKG_VERSION_PATCH"));
        assert_eq!(IMAGE_HEADER.length(), 0xffff_ffff);
    }

    /// A bit-level model of the STM32G4 CRC unit (RM0440, CRC calculation
    /// unit) as the bootloader configures it: 32-bit polynomial 0x04C11DB7,
    /// initial value 0xFFFFFFFF, input bit-reversed by word, output reversed.
    /// The bootloader writes little-endian words read from flash and inverts
    /// the result. This checks that the configuration computes the same CRC as
    /// the image tool, which cannot be tested on the host otherwise.
    fn stm32_crc_unit(data: &[u8]) -> u32 {
        let mut state: u32 = 0xffff_ffff;

        for word in data.as_chunks::<4>().0 {
            let input = u32::from_le_bytes(*word).reverse_bits();
            state ^= input;
            for _ in 0..32 {
                state = if state & 0x8000_0000 != 0 {
                    (state << 1) ^ 0x04c1_1db7
                } else {
                    state << 1
                };
            }
        }

        !state.reverse_bits()
    }

    #[test]
    fn the_stm32_crc_unit_matches_the_image_crc() {
        let data: Vec<u8> = (0..1024u32).map(|n| (n * 31 + 7) as u8).collect();

        assert_eq!(
            stm32_crc_unit(b"12345678"),
            SoftwareCrc32.checksum(b"1234", b"5678")
        );
        assert_eq!(
            stm32_crc_unit(&data),
            SoftwareCrc32.checksum(&data[..520], &data[520..])
        );
    }
}
