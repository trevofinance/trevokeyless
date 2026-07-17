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

/// Conservative pre-decode length bound for an SS58 address of a 32-byte account id.
/// The generic 36-byte payload-size calculation gives 50 base58 characters;
/// [`AccountId32::from_ss58check_with_version`] rejects longer input before base58
/// decoding, whose cost grows quadratically with the input length. Callers handling
/// untrusted address strings can apply the same bound up front.
///
/// The bound is applied to the **UTF-8 byte length** of the input (`str::len`), which is
/// identical to the character count for the all-ASCII base58 alphabet; the Python and
/// TypeScript trevokeyless libraries export the same constant and apply the same
/// byte-length rule, so the rejection kind of non-ASCII garbage cannot drift.
///
/// This is deliberately a safe over-approximation, not the maximum achievable valid
/// spelling: the prefix limits the payload below 2²⁸⁷, making the true maximum 49
/// characters (pinned by the all-`0xFF`/format-16383 conformance vector).
pub const SS58_MAX_ADDRESS_LENGTH: usize = 50;

/// Error returned by the SS58 decoding (and encoding, for invalid formats) functions.
///
/// Deliberately exhaustive (no `#[non_exhaustive]`): the five variants are the contractual
/// error taxonomy shared with the Python and TypeScript implementations (`Ss58Error.kind`
/// in both) and pinned by the conformance fixture. Adding a variant
/// is a breaking change of that cross-language contract, not a routine extension.
//
// Intentionally derives neither the SCALE (`Encode`/`Decode`/…) nor the serde traits that
// `AccountNameError` and `KeylessDecodeError` carry: an `Ss58Error` describes a *decode-time*
// failure of an address *string*, not a value that is ever stored or transmitted on the
// SCALE/serde wire. The other two errors can surface as pallet errors and so must round-trip;
// SS58 decoding happens off-chain, so this one has no wire form to define.
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

impl core::error::Error for Ss58Error {}

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

/// Error returned by [`AccountId32::to_ss58check_with_version`] — the requested SS58 format is
/// reserved (46, 47) or outside the 14-bit range — and carried by
/// [`Ss58ExpectingError::InvalidExpectedFormat`] when the *decoder's* `expected_format`
/// argument is out of range.
///
/// This is the *only* way SS58 encoding can fail. It is deliberately not an [`Ss58Error`]:
/// a rejected format is a rejected *argument*, not a statement about an address. Encoding has
/// no base58 to parse, no checksum to verify and no expected format to compare
/// against, so three of that enum's five variants are unreachable for an encoder and callers
/// should not have to match on them. The Python
/// and TypeScript implementations classify it the same way: a `ValueError` and a `RangeError`
/// respectively, never their `Ss58Error`, which reports on an *address*. That holds for
/// **both** ways a format can be unusable at encode time — reserved (46, 47) and outside the
/// 14-bit range — so a caller catching the argument-error type catches every rejected format.
/// Pinned by the `format_invalid_encode` and `format_invalid_decode` conformance groups, which
/// for that reason pin the rejection but not the classification.
///
/// Note the encoder and the decoder reject different sets. A reserved format can never be
/// encoded, but it is still a well-formed *expected* format for a decoder — the answer is
/// simply that no address matches, i.e. [`Ss58Error::UnexpectedFormat`]; only an
/// `expected_format` outside the 14-bit range is a rejected decoder argument. See
/// [`AccountId32::from_ss58check_expecting`].
///
/// Converts into [`Ss58Error::InvalidFormat`] via [`From`] for callers that want a single error
/// type across encoding and decoding, and renders the same message.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct InvalidSs58Format {
    format: u16,
}

impl InvalidSs58Format {
    pub(crate) const fn new(format: u16) -> Self {
        Self { format }
    }

    /// Returns the rejected format.
    pub const fn format(self) -> u16 {
        self.format
    }
}

impl core::error::Error for InvalidSs58Format {}

/// The message is shared verbatim with [`Ss58Error::InvalidFormat`] and with the Python and
/// TypeScript implementations.
impl core::fmt::Display for InvalidSs58Format {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("SS58 format is invalid or reserved")
    }
}

impl From<InvalidSs58Format> for Ss58Error {
    fn from(_: InvalidSs58Format) -> Self {
        Ss58Error::InvalidFormat
    }
}

/// Error returned by [`AccountId32::from_ss58check_expecting`]: either the `expected_format`
/// argument was itself rejected, or the address failed.
///
/// The split keeps the decoder's taxonomy aligned with the encoder's ([`InvalidSs58Format`])
/// and with the Python and TypeScript implementations, whose decode entry points raise their
/// parameter-validation errors — a `ValueError` and a `RangeError` — for a bad `ss58_format`
/// argument, never their `Ss58Error`, which reports on an *address*. An `expected_format`
/// outside the 14-bit range is the caller's own bug and is reported as
/// [`InvalidExpectedFormat`](Self::InvalidExpectedFormat) before the address is even looked
/// at; everything the *address* can be blamed for — including a valid address of a different
/// format — arrives as [`Address`](Self::Address).
///
/// The reserved formats 46 and 47 are accepted as a parameter: they are well-formed things to
/// *expect*. No valid address can carry them, so the result is the address's own validation
/// error or an [`Ss58Error::UnexpectedFormat`], matching the other implementations.
///
/// Converts into [`Ss58Error`] via [`From`] for callers that want the single address-level
/// taxonomy: the parameter branch folds into [`Ss58Error::InvalidFormat`], exactly like
/// [`InvalidSs58Format`]'s own conversion.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum Ss58ExpectingError {
    /// The `expected_format` argument is outside the 14-bit range: a rejected *argument*,
    /// not a statement about the address (which was not examined).
    InvalidExpectedFormat(InvalidSs58Format),
    /// The address failed validation (structure, length, checksum), or is valid but carries
    /// a format other than the expected one.
    Address(Ss58Error),
}

impl core::error::Error for Ss58ExpectingError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::InvalidExpectedFormat(error) => Some(error),
            Self::Address(error) => Some(error),
        }
    }
}

/// Renders the inner error's message, so every wording stays the shared cross-language one.
impl core::fmt::Display for Ss58ExpectingError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidExpectedFormat(error) => error.fmt(f),
            Self::Address(error) => error.fmt(f),
        }
    }
}

impl From<InvalidSs58Format> for Ss58ExpectingError {
    fn from(error: InvalidSs58Format) -> Self {
        Self::InvalidExpectedFormat(error)
    }
}

impl From<Ss58Error> for Ss58ExpectingError {
    fn from(error: Ss58Error) -> Self {
        Self::Address(error)
    }
}

impl From<Ss58ExpectingError> for Ss58Error {
    fn from(error: Ss58ExpectingError) -> Self {
        match error {
            Ss58ExpectingError::InvalidExpectedFormat(error) => error.into(),
            Ss58ExpectingError::Address(error) => error,
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

#[cfg_attr(docsrs, doc(cfg(feature = "ss58")))]
impl AccountId32 {
    /// Encodes the account id as an SS58 address with the given format.
    ///
    /// Returns [`InvalidSs58Format`] if `format` is reserved (46, 47) or greater than 16383 —
    /// the only way encoding can fail. It converts into [`Ss58Error::InvalidFormat`] with `?`
    /// or `.into()` if the caller works in terms of [`Ss58Error`].
    pub fn to_ss58check_with_version(&self, format: u16) -> Result<String, InvalidSs58Format> {
        if format_invalid(format) {
            return Err(InvalidSs58Format::new(format));
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
        let account_bytes = self.as_bytes();
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
    /// Input longer than the maximum length of a valid address (50 UTF-8 bytes, see
    /// [`SS58_MAX_ADDRESS_LENGTH`]) is rejected with [`Ss58Error::InvalidLength`] before
    /// base58 decoding, so the cost of decoding untrusted input stays bounded.
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

    /// Decodes an SS58 address, requiring the given expected format.
    ///
    /// The `expected_format` argument is validated first: a value outside the 14-bit range
    /// returns [`Ss58ExpectingError::InvalidExpectedFormat`] before the address is examined
    /// (a caller parameter is a programmer error; the address is data, often untrusted —
    /// the caller's own bug is reported first, exactly like the Python and TypeScript
    /// parameter-validation errors). The reserved formats 46 and 47 are accepted as a
    /// parameter — no valid address can carry them, so the result is the address's own
    /// validation error or an `UnexpectedFormat`, matching the other implementations.
    ///
    /// The address is then fully validated — structure, length, checksum — exactly like
    /// [`from_ss58check_with_version`](Self::from_ss58check_with_version); only then is
    /// the embedded format compared, so an invalid address reports its validation error,
    /// never a misleading [`Ss58Error::UnexpectedFormat`] (the format carried by an
    /// address that fails validation is not trustworthy information). This is the
    /// construction behind the `ss58_format` parameter of the Python and TypeScript
    /// decode entry points and behind the conformance fixture's rejection vectors.
    pub fn from_ss58check_expecting(
        address: &str,
        expected_format: u16,
    ) -> Result<Self, Ss58ExpectingError> {
        if expected_format > SS58_FORMAT_MAX {
            return Err(Ss58ExpectingError::InvalidExpectedFormat(
                InvalidSs58Format::new(expected_format),
            ));
        }
        Self::decode_expecting_valid(address, expected_format).map_err(Ss58ExpectingError::Address)
    }

    /// The shared decode-and-compare step behind [`from_ss58check_expecting`]
    /// (which validates the format argument first) and [`from_ss58check`] (whose constant
    /// format needs no validation, keeping its error the plain address-level [`Ss58Error`]).
    fn decode_expecting_valid(address: &str, expected_format: u16) -> Result<Self, Ss58Error> {
        let (account, found) = Self::from_ss58check_with_version(address)?;
        if found != expected_format {
            return Err(Ss58Error::UnexpectedFormat {
                expected: expected_format,
                found,
            });
        }
        Ok(account)
    }

    /// Decodes an SS58 address with the Trevo format ([`SS58_FORMAT_TREVO_ASSET_HUB`]).
    ///
    /// Returns [`Ss58Error::UnexpectedFormat`] if the address is valid but carries a
    /// different format. Behaves like
    /// [`from_ss58check_expecting`](Self::from_ss58check_expecting) with
    /// [`SS58_FORMAT_TREVO_ASSET_HUB`], but keeps the plain [`Ss58Error`]: the format is a
    /// known-valid constant, so the parameter-rejection branch cannot occur.
    pub fn from_ss58check(address: &str) -> Result<Self, Ss58Error> {
        Self::decode_expecting_valid(address, SS58_FORMAT_TREVO_ASSET_HUB)
    }
}

/// Into a **human-readable** format (JSON, YAML, TOML…), `AccountId32` serializes as its
/// SS58 address string in the Trevo format ([`SS58_FORMAT_TREVO_ASSET_HUB`]) — the address
/// representation used by the Python and TypeScript trevokeyless libraries, and the one a
/// person reading the document expects.
///
/// Into a **binary** format (bincode, postcard, MessagePack…), it serializes as the raw 32
/// account-id bytes. Those formats are for machines: the 49-character base58 text is four
/// times the size, costs a base58 encode and a Blake2b checksum per value, and — for a
/// format that never sees a human — encodes nothing the bytes do not. Some binary formats
/// cannot even represent the string form usefully.
///
/// `is_human_readable()` is serde's standard hook for exactly this split. Getting it wrong
/// is not merely wasteful: it silently locks every `AccountId32` field of every downstream
/// struct into a text encoding on the wire, which is the sort of thing that is very hard to
/// change later.
#[cfg(feature = "serde")]
#[cfg_attr(docsrs, doc(cfg(feature = "serde")))]
impl serde::Serialize for AccountId32 {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if serializer.is_human_readable() {
            serializer.serialize_str(&self.to_ss58check())
        } else {
            serializer.serialize_bytes(self.as_bytes())
        }
    }
}

/// The mirror of [`Serialize`](serde::Serialize): from a human-readable format an SS58
/// address string in the Trevo format ([`SS58_FORMAT_TREVO_ASSET_HUB`]); from a binary
/// format the raw 32 account-id bytes.
#[cfg(feature = "serde")]
#[cfg_attr(docsrs, doc(cfg(feature = "serde")))]
impl<'de> serde::Deserialize<'de> for AccountId32 {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        if deserializer.is_human_readable() {
            let s = String::deserialize(deserializer)?;
            AccountId32::from_ss58check(&s).map_err(serde::de::Error::custom)
        } else {
            deserializer.deserialize_bytes(AccountIdBytesVisitor)
        }
    }
}

/// Accepts the raw account-id bytes from a binary format. `visit_seq` is needed as well as
/// `visit_bytes`: several binary formats (bincode among them) hand a byte sequence to the
/// visitor element by element rather than as one borrowed slice.
#[cfg(feature = "serde")]
struct AccountIdBytesVisitor;

#[cfg(feature = "serde")]
impl<'de> serde::de::Visitor<'de> for AccountIdBytesVisitor {
    type Value = AccountId32;

    fn expecting(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            formatter,
            "{} raw account id bytes",
            crate::types::ACCOUNT_ID_LENGTH
        )
    }

    fn visit_bytes<E: serde::de::Error>(self, value: &[u8]) -> Result<Self::Value, E> {
        AccountId32::try_from(value).map_err(serde::de::Error::custom)
    }

    fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let mut bytes = [0u8; crate::types::ACCOUNT_ID_LENGTH];
        for (index, byte) in bytes.iter_mut().enumerate() {
            *byte = seq
                .next_element()?
                .ok_or_else(|| serde::de::Error::invalid_length(index, &self))?;
        }
        if seq.next_element::<u8>()?.is_some() {
            return Err(serde::de::Error::invalid_length(
                crate::types::ACCOUNT_ID_LENGTH + 1,
                &self,
            ));
        }
        Ok(AccountId32::new(bytes))
    }
}
