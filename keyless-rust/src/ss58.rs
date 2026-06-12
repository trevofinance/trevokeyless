//! SS58 address encoding and decoding for [`AccountId32`] (feature `ss58`).
//!
//! SS58 is the address format used by Substrate-based chains: a base58 string carrying a
//! network format prefix, the 32-byte account id, and a 2-byte checksum
//! (`blake2b_512(b"SS58PRE" ++ prefix ++ account_id)[0..2]`). See the
//! [SS58 specification](https://docs.substrate.io/reference/address-formats/) for details.
//!
//! The Trevo chain uses the SS58 format [`SS58_FORMAT_TREVO_ASSET_HUB`] (5335).

// The prefix and checksum scheme follows the SS58 codec of Substrate's sp-core (Apache-2.0):
// https://github.com/paritytech/polkadot-sdk/blob/polkadot-stable2407-2/substrate/primitives/core/src/crypto.rs

use crate::types::AccountId32;
use alloc::string::String;
use alloc::vec::Vec;

/// SS58 format (network prefix) of the Trevo asset hub.
pub const SS58_FORMAT_TREVO_ASSET_HUB: u16 = 5335;

/// Largest valid SS58 format: the prefix encodes 14 bits.
const SS58_FORMAT_MAX: u16 = 0b0011_1111_1111_1111;

/// Checksum length for 32-byte account ids.
const SS58_CHECKSUM_LENGTH: usize = 2;

/// Maximum length in characters of a valid SS58 address for a 32-byte account id: the
/// payload is at most 2 (prefix) + 32 (account id) + 2 (checksum) = 36 bytes = 288 bits,
/// and each base58 character carries log2(58) ≈ 5.858 bits, so a valid address has at most
/// ceil(288 / 5.858) = 50 characters. [`AccountId32::from_ss58check_with_version`] rejects
/// longer input before base58 decoding, whose cost grows quadratically with the input
/// length; callers handling untrusted address strings can apply the same bound up front.
///
/// The Python and TypeScript trevo-keyless libraries export the same constant.
pub const SS58_MAX_ADDRESS_LENGTH: usize = 50;

/// Error returned by the SS58 decoding (and encoding, for invalid formats) functions.
///
/// Deliberately exhaustive (no `#[non_exhaustive]`): the five variants are the contractual
/// error taxonomy shared with the Python (`Ss58DecodeError.kind`) and TypeScript
/// (`Ss58Error.kind`) implementations and pinned by the conformance fixture. Adding a variant
/// is a breaking change of that cross-language contract, not a routine extension.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum Ss58Error {
    /// The string is not valid base58.
    InvalidBase58,
    /// The address string is longer than any valid SS58-encoded 32-byte account id, or the
    /// decoded payload does not have the expected length.
    InvalidLength,
    /// The checksum does not match the payload.
    InvalidChecksum,
    /// The SS58 format is reserved (46, 47), out of range (> 16383), or its prefix byte
    /// cannot be parsed.
    InvalidFormat,
    /// The address is a valid SS58 string but uses a different format than expected.
    UnexpectedFormat {
        /// The format required by the caller.
        expected: u16,
        /// The format found in the address.
        found: u16,
    },
}

#[cfg(feature = "std")]
impl std::error::Error for Ss58Error {}

impl core::fmt::Display for Ss58Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidBase58 => f.write_str("Address is not valid base58"),
            Self::InvalidLength => f.write_str("Address has an invalid length"),
            Self::InvalidChecksum => f.write_str("Address has an invalid checksum"),
            Self::InvalidFormat => f.write_str("SS58 format is invalid or reserved"),
            Self::UnexpectedFormat { expected, found } => {
                write!(f, "Expected SS58 format {expected} but found {found}")
            }
        }
    }
}

/// Returns `true` for SS58 formats that can never be used: reserved (46, 47) or out of the
/// 14-bit range.
fn format_invalid(format: u16) -> bool {
    format > SS58_FORMAT_MAX || format == 46 || format == 47
}

/// Computes the 64-byte Blake2b hash of `b"SS58PRE" ++ data`, of which the first
/// [`SS58_CHECKSUM_LENGTH`] bytes are the SS58 checksum.
fn ss58hash(data: &[u8]) -> [u8; 64] {
    blake2b_simd::Params::new()
        .hash_length(64)
        .to_state()
        .update(b"SS58PRE")
        .update(data)
        .finalize()
        .as_bytes()
        .try_into()
        .expect("slice is always the necessary length")
}

impl AccountId32 {
    /// Encodes the account id as an SS58 address with the given format.
    ///
    /// Returns [`Ss58Error::InvalidFormat`] if `format` is reserved (46, 47) or
    /// greater than 16383.
    pub fn to_ss58check_with_version(&self, format: u16) -> Result<String, Ss58Error> {
        if format_invalid(format) {
            return Err(Ss58Error::InvalidFormat);
        }

        let mut payload: Vec<u8> = Vec::with_capacity(2 + 32 + SS58_CHECKSUM_LENGTH);
        if format < 64 {
            payload.push(format as u8);
        } else {
            // Two-byte prefix: the first byte carries bits 2..=7 of the format (tagged with
            // 0b01 in its upper two bits), the second byte carries bits 8..=13 in its lower
            // six bits and bits 0..=1 in its upper two bits.
            payload.push((((format & 0b0000_0000_1111_1100) >> 2) | 0b0100_0000) as u8);
            payload.push(((format >> 8) | ((format & 0b0000_0000_0000_0011) << 6)) as u8);
        }
        let account_bytes: &[u8; 32] = self.as_ref();
        payload.extend_from_slice(account_bytes);

        let checksum = ss58hash(&payload);
        payload.extend_from_slice(&checksum[0..SS58_CHECKSUM_LENGTH]);

        Ok(bs58::encode(payload).into_string())
    }

    /// Encodes the account id as an SS58 address with the Trevo format
    /// ([`SS58_FORMAT_TREVO_ASSET_HUB`]).
    #[must_use]
    pub fn to_ss58check(&self) -> String {
        self.to_ss58check_with_version(SS58_FORMAT_TREVO_ASSET_HUB)
            .expect("SS58_FORMAT_TREVO_ASSET_HUB is a valid format")
    }

    /// Decodes an SS58 address of any (non-reserved) format, returning the account id and
    /// the format found in the address.
    ///
    /// Input longer than the maximum length of a valid address (50 characters) is rejected
    /// with [`Ss58Error::InvalidLength`] before base58 decoding, so the cost of decoding
    /// untrusted input stays bounded.
    pub fn from_ss58check_with_version(address: &str) -> Result<(Self, u16), Ss58Error> {
        if address.len() > SS58_MAX_ADDRESS_LENGTH {
            return Err(Ss58Error::InvalidLength);
        }
        let data = bs58::decode(address)
            .into_vec()
            .map_err(|_| Ss58Error::InvalidBase58)?;
        if data.is_empty() {
            return Err(Ss58Error::InvalidLength);
        }

        let (prefix_length, format) = match data[0] {
            0..=63 => (1, data[0] as u16),
            64..=127 => {
                if data.len() < 2 {
                    return Err(Ss58Error::InvalidLength);
                }
                let format = (((data[0] & 0b0011_1111) as u16) << 2)
                    | ((data[1] >> 6) as u16)
                    | (((data[1] & 0b0011_1111) as u16) << 8);
                (2, format)
            }
            _ => return Err(Ss58Error::InvalidFormat),
        };
        if format_invalid(format) {
            return Err(Ss58Error::InvalidFormat);
        }
        if data.len() != prefix_length + 32 + SS58_CHECKSUM_LENGTH {
            return Err(Ss58Error::InvalidLength);
        }

        let checksum_start = data.len() - SS58_CHECKSUM_LENGTH;
        let expected_checksum = ss58hash(&data[..checksum_start]);
        if data[checksum_start..] != expected_checksum[0..SS58_CHECKSUM_LENGTH] {
            return Err(Ss58Error::InvalidChecksum);
        }

        let account_bytes: [u8; 32] = data[prefix_length..checksum_start]
            .try_into()
            .expect("length checked above");
        Ok((Self::new(account_bytes), format))
    }

    /// Decodes an SS58 address with the Trevo format ([`SS58_FORMAT_TREVO_ASSET_HUB`]).
    ///
    /// Returns [`Ss58Error::UnexpectedFormat`] if the address is valid but carries a
    /// different format.
    pub fn from_ss58check(address: &str) -> Result<Self, Ss58Error> {
        let (account, format) = Self::from_ss58check_with_version(address)?;
        if format != SS58_FORMAT_TREVO_ASSET_HUB {
            return Err(Ss58Error::UnexpectedFormat {
                expected: SS58_FORMAT_TREVO_ASSET_HUB,
                found: format,
            });
        }
        Ok(account)
    }
}

/// `AccountId32` serializes as its SS58 address string in the Trevo format
/// ([`SS58_FORMAT_TREVO_ASSET_HUB`]), matching the address representation used by the
/// Python and TypeScript trevo-keyless libraries.
#[cfg(feature = "serde")]
impl serde::Serialize for AccountId32 {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_ss58check())
    }
}

/// `AccountId32` deserializes from an SS58 address string, requiring the Trevo format
/// ([`SS58_FORMAT_TREVO_ASSET_HUB`]).
#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for AccountId32 {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        AccountId32::from_ss58check(&s).map_err(serde::de::Error::custom)
    }
}
