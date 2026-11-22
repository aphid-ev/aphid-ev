use core::{error, fmt};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum BusError {
    Stuff,
    Form,
    Acknowledge,
    BitRecessive,
    BitDominant,
    Crc,
    Software,
}

impl fmt::Display for BusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl error::Error for BusError {}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [BusError; 7] = [
        BusError::Stuff,
        BusError::Form,
        BusError::Acknowledge,
        BusError::BitRecessive,
        BusError::BitDominant,
        BusError::Crc,
        BusError::Software,
    ];

    #[test]
    fn every_variant_round_trips_through_postcard() {
        for error in ALL {
            let bytes = postcard::to_allocvec(&error).unwrap();

            assert_eq!(postcard::from_bytes::<BusError>(&bytes).unwrap(), error);
        }
    }

    /// Variants are encoded by position, so reordering or inserting one
    /// changes the meaning of existing values on the wire.
    #[test]
    fn wire_encoding_is_stable() {
        for (discriminant, error) in ALL.iter().enumerate() {
            assert_eq!(
                postcard::to_allocvec(error).unwrap(),
                vec![discriminant as u8]
            );
        }
    }

    #[test]
    fn deserialize_rejects_unknown_discriminants() {
        assert!(postcard::from_bytes::<BusError>(&[ALL.len() as u8]).is_err());
    }

    #[test]
    fn display_names_the_variant() {
        assert_eq!(BusError::Crc.to_string(), "Crc");
        assert_eq!(BusError::BitRecessive.to_string(), "BitRecessive");
    }
}
