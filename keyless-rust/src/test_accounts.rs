//! Fixed-width account ids for Substrate runtime tests (feature `test-accounts`).
//!
//! [`AccountId64`] and [`AccountId128`] let a Substrate runtime instantiate its
//! `frame_system::Config::AccountId` at widths other than the production 32 bytes, for its
//! general (non-keyless) test suites. They are not part of the cross-language contract — the
//! Python and TypeScript ports have no counterpart — and carry no SS58 codec.
//!
//! Each wraps a 32-byte keyless account carried in its leading 32 bytes, right-padded with
//! zeros:
//!
//! ```text
//! [ AccountId32 (32 bytes) | 0 0 0 … 0 ]
//! ```
//!
//! - **Encode** (zero-pad): `AccountId64::from(core)` / `core.into()` — the `From<AccountId32>`
//!   impl.
//! - **Decode** (unpad): [`AccountId64::to_account_id32`] returns the leading
//!   [`AccountId32`](crate::AccountId32) iff the trailing bytes are the canonical zero padding,
//!   otherwise `None`. Feed the result to the ordinary
//!   [`decode_account_info`](crate::decode_account_info) /
//!   [`is_keyless_account`](crate::is_keyless_account) entry points — the wide types reuse the
//!   audited 32-byte codec unchanged.
//!
//! Requiring zero padding on decode keeps the encoding canonical (so encode→decode is an exact
//! round-trip) and preserves the crate's "corrupting any byte demotes a keyless account to
//! Regular" guarantee across the full width.

use crate::{AccountId32, ParseAccountIdError};

#[cfg(feature = "scale")]
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
#[cfg(feature = "scale")]
use scale_info::TypeInfo;

/// Decodes one hex nibble (both digit cases). Mirrors the private helper in `types.rs`,
/// duplicated here so this test-only feature leaves the production module untouched.
fn nibble(c: u8) -> Result<u8, ParseAccountIdError> {
    match c {
        b'0'..=b'9' => Ok(c - b'0'),
        b'a'..=b'f' => Ok(c - b'a' + 10),
        b'A'..=b'F' => Ok(c - b'A' + 10),
        _ => Err(ParseAccountIdError),
    }
}

/// Generates a fixed-width padded account-id newtype around `[u8; $bytes]` (with `$bytes >= 32`)
/// carrying the trait surface a Substrate `frame_system::Config::AccountId` requires, plus the
/// zero-pad / strict-unpad conversions to [`AccountId32`]. `$hex_len` is `2 * $bytes` (the
/// number of hex characters in the `Display`/`FromStr` form).
macro_rules! define_padded_account_id {
    ($name:ident, $bytes:literal, $hex_len:literal) => {
        #[doc = concat!(
            "A ", stringify!($bytes),
            "-byte account id for Substrate runtime tests: a 32-byte keyless account in the \
             leading bytes, right-padded with zeros. See the module-level documentation for the \
             encode/decode contract."
        )]
        #[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
        #[cfg_attr(
            feature = "scale",
            derive(Encode, Decode, DecodeWithMemTracking, TypeInfo, MaxEncodedLen)
        )]
        pub struct $name([u8; $bytes]);

        impl $name {
            #[doc = concat!("Creates a `", stringify!($name), "` from its raw bytes.")]
            pub const fn new(inner: [u8; $bytes]) -> Self {
                Self(inner)
            }

            /// Returns the leading 32 bytes as an [`AccountId32`](crate::AccountId32), **iff** the
            /// trailing bytes are the canonical zero padding; otherwise `None`.
            ///
            /// This is the decode direction: pass the returned [`AccountId32`](crate::AccountId32)
            /// to [`decode_account_info`](crate::decode_account_info) or
            /// [`is_keyless_account`](crate::is_keyless_account). A non-`None` result is exactly
            /// the account a prior `from(core)` zero-padded.
            pub fn to_account_id32(&self) -> Option<AccountId32> {
                if self.0[32..].iter().all(|&b| b == 0) {
                    let mut core = [0u8; 32];
                    core.copy_from_slice(&self.0[..32]);
                    Some(AccountId32::new(core))
                } else {
                    None
                }
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self([0u8; $bytes])
            }
        }

        /// Zero-pads a canonical 32-byte keyless account to this width (the encode direction).
        impl From<AccountId32> for $name {
            fn from(core: AccountId32) -> Self {
                let core_bytes: [u8; 32] = core.into();
                let mut bytes = [0u8; $bytes];
                bytes[..32].copy_from_slice(&core_bytes);
                Self(bytes)
            }
        }

        impl From<[u8; $bytes]> for $name {
            fn from(x: [u8; $bytes]) -> Self {
                Self(x)
            }
        }

        impl From<$name> for [u8; $bytes] {
            fn from(x: $name) -> [u8; $bytes] {
                x.0
            }
        }

        impl AsRef<[u8; $bytes]> for $name {
            fn as_ref(&self) -> &[u8; $bytes] {
                &self.0
            }
        }

        impl AsRef<[u8]> for $name {
            fn as_ref(&self) -> &[u8] {
                &self.0
            }
        }

        /// Renders as `0x` followed by the lowercase hex of every byte — the same interchange
        /// form as [`AccountId32`](crate::AccountId32), widened to this type's length.
        impl core::fmt::Display for $name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                f.write_str("0x")?;
                for b in &self.0 {
                    write!(f, "{:02x}", b)?;
                }
                Ok(())
            }
        }

        /// Parses the `0x`-prefixed hex form produced by [`Display`](core::fmt::Display): `0x`
        /// followed by exactly this type's width in hex characters (both digit cases). Reuses
        /// [`ParseAccountIdError`](crate::ParseAccountIdError) (whose message names the 32-byte
        /// length, but the parser enforces this type's own width).
        impl core::str::FromStr for $name {
            type Err = ParseAccountIdError;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                let hex = s.strip_prefix("0x").ok_or(ParseAccountIdError)?.as_bytes();
                if hex.len() != $hex_len {
                    return Err(ParseAccountIdError);
                }
                let mut bytes = [0u8; $bytes];
                for (i, byte) in bytes.iter_mut().enumerate() {
                    *byte = (nibble(hex[2 * i])? << 4) | nibble(hex[2 * i + 1])?;
                }
                Ok(Self(bytes))
            }
        }

        /// Serializes as the `0x`-prefixed hex string of [`Display`](core::fmt::Display).
        #[cfg(feature = "serde")]
        impl serde::Serialize for $name {
            fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.collect_str(self)
            }
        }

        /// Deserializes from the `0x`-prefixed hex string accepted by
        /// [`FromStr`](core::str::FromStr).
        #[cfg(feature = "serde")]
        impl<'de> serde::Deserialize<'de> for $name {
            fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let s = alloc::string::String::deserialize(deserializer)?;
                s.parse().map_err(serde::de::Error::custom)
            }
        }
    };
}

define_padded_account_id!(AccountId64, 64, 128);
define_padded_account_id!(AccountId128, 128, 256);
