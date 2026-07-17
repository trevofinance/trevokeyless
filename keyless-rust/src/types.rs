#[cfg(feature = "scale")]
use parity_scale_codec::{
    Decode, DecodeWithMemTracking, Encode, EncodeLike, Error as CodecError, Input, MaxEncodedLen,
    Output,
};
#[cfg(feature = "scale")]
use scale_info::TypeInfo;

macro_rules! identifier_type {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[repr(transparent)]
        #[derive(Clone, Copy, Default, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
        #[cfg_attr(
            feature = "scale",
            derive(Encode, Decode, DecodeWithMemTracking, TypeInfo, MaxEncodedLen)
        )]
        #[cfg_attr(
            feature = "serde",
            derive(serde::Serialize, serde::Deserialize),
            serde(transparent)
        )]
        pub struct $name(u32);

        impl $name {
            /// Creates an identifier from its numeric `u32` value.
            pub const fn new(value: u32) -> Self {
                Self(value)
            }

            /// Returns the identifier's numeric `u32` value.
            pub const fn get(self) -> u32 {
                self.0
            }

            pub(crate) const fn to_le_bytes(self) -> [u8; 4] {
                self.0.to_le_bytes()
            }
        }

        impl From<u32> for $name {
            fn from(value: u32) -> Self {
                Self::new(value)
            }
        }

        impl From<$name> for u32 {
            fn from(value: $name) -> Self {
                value.get()
            }
        }

        impl core::fmt::Display for $name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                self.0.fmt(f)
            }
        }
    };
}

identifier_type!(
    /// Numeric identifier of an App Agent (`u32`, little-endian in keyless account bytes
    /// `[0..=3]`). A distinct type prevents accidental transposition with a
    /// [`TransactionalId`].
    AppAgentId
);

identifier_type!(
    /// Numeric identifier of a Transactional sub-account (`u32`, little-endian in keyless
    /// account bytes `[5..=8]`). A distinct type prevents accidental transposition with an
    /// [`AppAgentId`].
    TransactionalId
);

/// The length in bytes of an [`AccountId32`] — the width of every on-chain account
/// identifier, keyless or regular.
///
/// The counterpart of the Python and TypeScript `ACCOUNT_ID_LENGTH` constants.
pub const ACCOUNT_ID_LENGTH: usize = 32;

/// The length in bytes of an [`AccountName`] — the fixed width of the name embedded in a
/// Named keyless account.
///
/// The name is validated (and stored) as raw bytes; every allowed character is one-byte
/// ASCII, so this is equivalently the number of allowed characters. The counterpart of the
/// Python and TypeScript `ACCOUNT_NAME_LENGTH` constants.
pub const ACCOUNT_NAME_LENGTH: usize = 10;

/// Byte `[4]` discriminant for an App Agent root keyless account.
pub const APP_AGENT_ACCOUNT_IDENTIFIER: u8 = 1;
/// Byte `[4]` discriminant for a Transactional keyless account.
pub const TRANSACTIONAL_ACCOUNT_IDENTIFIER: u8 = 2;
/// Byte `[4]` discriminant for a Named keyless account.
pub const NAMED_ACCOUNT_IDENTIFIER: u8 = 3;

/// Error returned when converting a byte slice of the wrong length into an [`AccountId32`].
///
/// The only failure mode: an account id is exactly [`ACCOUNT_ID_LENGTH`] (32) bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct InvalidAccountIdLength {
    length: usize,
}

impl InvalidAccountIdLength {
    /// Returns the length of the offending slice.
    pub const fn length(self) -> usize {
        self.length
    }
}

impl core::error::Error for InvalidAccountIdLength {}

impl core::fmt::Display for InvalidAccountIdLength {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "Account id must be exactly {ACCOUNT_ID_LENGTH} bytes long, got {}",
            self.length
        )
    }
}

/// A 32-byte on-chain account identifier.
///
/// For keyless accounts the bytes follow the layout described in the crate documentation.
/// For regular accounts the bytes are an opaque public key or hash with no special structure.
#[derive(Clone, Copy, Default, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[cfg_attr(
    feature = "scale",
    derive(Encode, Decode, DecodeWithMemTracking, TypeInfo, MaxEncodedLen)
)]
pub struct AccountId32([u8; 32]);

impl AccountId32 {
    /// Creates a new `AccountId32` from its raw 32-byte representation.
    ///
    /// Equivalent to the `From<[u8; 32]>` implementation; provided as a `const fn` because
    /// trait implementations cannot be `const`.
    pub const fn new(inner: [u8; 32]) -> Self {
        Self(inner)
    }

    /// Returns the raw 32-byte representation of the account id.
    ///
    /// The unambiguous way to reach the bytes. This type implements both
    /// `AsRef<[u8; 32]>` and `AsRef<[u8]>` — each useful on its own, but together they make
    /// a bare `account.as_ref()` fail to infer (`error[E0283]`), forcing every caller to
    /// write a turbofish or a type ascription. Mirrors [`AccountName::as_bytes`].
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Returns the account id as a byte slice.
    #[must_use]
    pub const fn as_slice(&self) -> &[u8] {
        &self.0
    }
}

impl AsRef<[u8; 32]> for AccountId32 {
    fn as_ref(&self) -> &[u8; 32] {
        &self.0
    }
}

impl From<[u8; 32]> for AccountId32 {
    fn from(x: [u8; 32]) -> Self {
        Self::new(x)
    }
}

impl From<AccountId32> for [u8; 32] {
    fn from(x: AccountId32) -> [u8; 32] {
        x.0
    }
}

/// Accepts a slice of exactly [`ACCOUNT_ID_LENGTH`] (32) bytes — the shape account ids arrive
/// in from a database row or a chain query. Mirrors [`AccountName`]'s `TryFrom<&[u8]>`.
impl TryFrom<&[u8]> for AccountId32 {
    type Error = InvalidAccountIdLength;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let bytes: [u8; ACCOUNT_ID_LENGTH] =
            value.try_into().map_err(|_| InvalidAccountIdLength {
                length: value.len(),
            })?;
        Ok(Self::new(bytes))
    }
}

impl AsRef<[u8]> for AccountId32 {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

/// Renders the account id in the cross-language raw interchange form: `0x` followed by
/// 64 lowercase hex characters — the exact string accepted by this type's [`FromStr`]
/// implementation and by the Python `decode_account_id` and TypeScript
/// `decodeAccountId` entry points.
///
/// [`FromStr`]: core::str::FromStr
impl core::fmt::Display for AccountId32 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("0x")?;
        for b in &self.0 {
            write!(f, "{:02x}", b)?;
        }
        Ok(())
    }
}

/// Error returned when parsing an [`AccountId32`] from its hex string form fails.
///
/// The accepted form is exactly `0x` followed by 64 hex characters (both digit cases)
/// — the contract shared with the Python `decode_account_id` and TypeScript
/// `decodeAccountId` entry points and pinned by the `hex_valid` / `hex_invalid`
/// groups of the conformance fixture. The display message matches theirs.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct ParseAccountIdError {
    expected_hex_characters: usize,
}

impl ParseAccountIdError {
    pub(crate) const fn new(expected_hex_characters: usize) -> Self {
        Self {
            expected_hex_characters,
        }
    }

    /// Returns the required number of hexadecimal characters after the `0x` prefix.
    pub const fn expected_hex_characters(self) -> usize {
        self.expected_hex_characters
    }
}

impl core::error::Error for ParseAccountIdError {}

impl core::fmt::Display for ParseAccountIdError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "Invalid hex account id: expected 0x followed by {} hex characters",
            self.expected_hex_characters
        )
    }
}

/// Parses the cross-language raw interchange form produced by `Display`: exactly `0x`
/// (lowercase prefix) followed by 64 hex characters, both digit cases accepted —
/// the same contract as the Python `decode_account_id` and TypeScript
/// `decodeAccountId` entry points (pinned by the `hex_valid` / `hex_invalid`
/// conformance groups). No whitespace or other deviation is tolerated.
impl core::str::FromStr for AccountId32 {
    type Err = ParseAccountIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        /// Two hex characters per account id byte.
        const HEX_CHARACTERS: usize = ACCOUNT_ID_LENGTH * 2;

        fn nibble(c: u8) -> Result<u8, ParseAccountIdError> {
            match c {
                b'0'..=b'9' => Ok(c - b'0'),
                b'a'..=b'f' => Ok(c - b'a' + 10),
                b'A'..=b'F' => Ok(c - b'A' + 10),
                _ => Err(ParseAccountIdError::new(HEX_CHARACTERS)),
            }
        }

        let error = ParseAccountIdError::new(HEX_CHARACTERS);
        let hex = s.strip_prefix("0x").ok_or(error)?.as_bytes();
        if hex.len() != HEX_CHARACTERS {
            // Byte length: multi-byte UTF-8 input of 64 bytes is caught by `nibble`.
            return Err(error);
        }
        let mut bytes = [0u8; ACCOUNT_ID_LENGTH];
        for (i, byte) in bytes.iter_mut().enumerate() {
            *byte = (nibble(hex[2 * i])? << 4) | nibble(hex[2 * i + 1])?;
        }
        Ok(Self(bytes))
    }
}

impl TryFrom<&str> for AccountId32 {
    type Error = ParseAccountIdError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        s.parse()
    }
}

/// A fixed 10-byte account name used to identify Named keyless accounts.
///
/// Only the characters `0-9`, `a-z`, `A-Z`, `-`, and `#` (64 characters total) are allowed.
/// The name is stored verbatim in bytes [5..=14] of the Named account's open part.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(feature = "scale", derive(Encode, TypeInfo, MaxEncodedLen))]
pub struct AccountName([u8; 10]);

impl AccountName {
    /// Lookup table: `VALID_CHARS[b]` is `true` iff byte `b` is in the allowed set
    /// (`0-9 a-z A-Z - #`). Built at compile time with zero heap allocation.
    const VALID_CHARS: [bool; 256] = {
        let mut t = [false; 256];
        let mut i = b'0';
        while i <= b'9' {
            t[i as usize] = true;
            i += 1;
        }
        let mut i = b'a';
        while i <= b'z' {
            t[i as usize] = true;
            i += 1;
        }
        let mut i = b'A';
        while i <= b'Z' {
            t[i as usize] = true;
            i += 1;
        }
        t[b'-' as usize] = true;
        t[b'#' as usize] = true;
        t
    };

    /// Creates an `AccountName` from a byte slice.
    ///
    /// Returns an error if `input` is not exactly [`ACCOUNT_NAME_LENGTH`] (10) bytes or
    /// contains characters outside the allowed set (`0-9 a-z A-Z - #`). The length is
    /// checked first, so a non-ASCII input is judged by its UTF-8 *byte* length — the rule
    /// the Python and TypeScript implementations reproduce, pinned per name by the
    /// `named_invalid_encode` conformance group.
    pub fn new(input: &[u8]) -> Result<Self, AccountNameError> {
        if input.len() != ACCOUNT_NAME_LENGTH {
            return Err(AccountNameError::InvalidLength);
        }
        if input.iter().all(|&b| AccountName::VALID_CHARS[b as usize]) {
            let mut array = [0u8; ACCOUNT_NAME_LENGTH];
            array.copy_from_slice(input);
            Ok(AccountName(array))
        } else {
            Err(AccountNameError::InvalidCharacter)
        }
    }

    /// Returns the raw 10-byte representation of the name.
    ///
    /// The unambiguous way to reach the bytes: like [`AccountId32`], this type implements
    /// both `AsRef<[u8; 10]>` and `AsRef<[u8]>`, so a bare `name.as_ref()` cannot infer.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 10] {
        &self.0
    }

    /// Returns the name as a byte slice.
    #[must_use]
    pub const fn as_slice(&self) -> &[u8] {
        &self.0
    }

    /// Returns the name as a string slice.
    ///
    /// All allowed characters are ASCII, so the conversion never fails.
    #[must_use]
    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.0).expect("AccountName bytes are always valid ASCII")
    }
}

impl AsRef<[u8; 10]> for AccountName {
    fn as_ref(&self) -> &[u8; 10] {
        &self.0
    }
}

impl AsRef<[u8]> for AccountName {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl From<AccountName> for [u8; 10] {
    fn from(name: AccountName) -> [u8; 10] {
        name.0
    }
}

impl TryFrom<&[u8]> for AccountName {
    type Error = AccountNameError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        AccountName::new(value)
    }
}

impl TryFrom<[u8; 10]> for AccountName {
    type Error = AccountNameError;

    fn try_from(value: [u8; 10]) -> Result<Self, Self::Error> {
        AccountName::new(&value)
    }
}

impl TryFrom<&str> for AccountName {
    type Error = AccountNameError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        AccountName::new(s.as_bytes())
    }
}

impl core::str::FromStr for AccountName {
    type Err = AccountNameError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        AccountName::new(s.as_bytes())
    }
}

impl core::fmt::Display for AccountName {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(feature = "scale")]
#[cfg_attr(docsrs, doc(cfg(feature = "scale")))]
impl Decode for AccountName {
    fn decode<I: Input>(input: &mut I) -> Result<Self, CodecError> {
        let mut buf = [0u8; 10];
        input.read(&mut buf)?;
        // The length error is unreachable here: the buffer is always exactly 10 bytes.
        AccountName::new(&buf).map_err(|_| "Account name contains invalid characters".into())
    }
}

/// Sound because the manual `Decode` implementation above reads into a fixed-size stack
/// buffer and never allocates heap memory.
#[cfg(feature = "scale")]
#[cfg_attr(docsrs, doc(cfg(feature = "scale")))]
impl DecodeWithMemTracking for AccountName {}

#[cfg(feature = "serde")]
#[cfg_attr(docsrs, doc(cfg(feature = "serde")))]
impl serde::Serialize for AccountName {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

#[cfg(feature = "serde")]
#[cfg_attr(docsrs, doc(cfg(feature = "serde")))]
impl<'de> serde::Deserialize<'de> for AccountName {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = alloc::string::String::deserialize(deserializer)?;
        AccountName::new(s.as_bytes()).map_err(serde::de::Error::custom)
    }
}

/// Error returned by [`AccountName`] constructors when the input is not a valid account name.
///
/// Deliberately exhaustive (no `#[non_exhaustive]`): the name validation rules are part of
/// the frozen encoding, so the failure taxonomy is not expected to grow.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
#[cfg_attr(
    feature = "scale",
    derive(Encode, Decode, DecodeWithMemTracking, TypeInfo, MaxEncodedLen)
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(u8)]
pub enum AccountNameError {
    /// The input is not exactly [`ACCOUNT_NAME_LENGTH`] (10) bytes long.
    #[cfg_attr(feature = "scale", codec(index = 0))]
    InvalidLength = 0,
    /// The input contains a character outside the allowed set (`0-9 a-z A-Z - #`).
    #[cfg_attr(feature = "scale", codec(index = 1))]
    InvalidCharacter = 1,
}

impl core::error::Error for AccountNameError {}

/// The messages are shared verbatim with the Python and TypeScript implementations
/// (their name-validation `ValueError`/`Error` texts). The length message says
/// "characters" although the validated quantity is the UTF-8 *byte* length: every allowed
/// character is one-byte ASCII, so for any accepted name the two counts coincide, and the
/// message states the requirement.
///
/// The validation rule is the same in all three implementations — UTF-8 byte length first,
/// then the charset — so a rejected non-ASCII name yields the same failure *kind*
/// everywhere: `"café12345"` (9 characters, 10 bytes) is an `InvalidCharacter`, while
/// `"héllo12345"` (10 characters, 11 bytes) is an `InvalidLength`. Each vector of the
/// `named_invalid_encode` conformance group pins the kind (`error_kind`), so the
/// classification cannot drift between the implementations.
impl core::fmt::Display for AccountNameError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidLength => f.write_str("Account name must be exactly 10 characters long"),
            Self::InvalidCharacter => f.write_str("Account name contains invalid characters"),
        }
    }
}

/// Classification of a blockchain account, as determined by
/// [`decode_account_info`](crate::decode_account_info).
///
/// This is the Rust counterpart of the `AccountType` enum of the Python and TypeScript
/// trevokeyless libraries; the variant order (Regular = 0, AppAgent = 1, Transactional = 2,
/// Named = 3) matches their numeric values, so the keyless variants coincide with the type
/// discriminant stored at byte `[4]` of a keyless account.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(
    feature = "scale",
    derive(Encode, Decode, DecodeWithMemTracking, TypeInfo, MaxEncodedLen)
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(u8)]
pub enum AccountType {
    /// A regular account: no keyless identifiers are embedded in its bytes.
    #[cfg_attr(feature = "scale", codec(index = 0))]
    Regular = 0,
    /// The root keyless account of an App Agent.
    #[cfg_attr(feature = "scale", codec(index = 1))]
    AppAgent = 1,
    /// A transactional keyless sub-account of an App Agent.
    #[cfg_attr(feature = "scale", codec(index = 2))]
    Transactional = 2,
    /// A named keyless sub-account of an App Agent.
    #[cfg_attr(feature = "scale", codec(index = 3))]
    Named = 3,
}

impl core::fmt::Display for AccountType {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::Regular => "Regular",
            Self::AppAgent => "AppAgent",
            Self::Transactional => "Transactional",
            Self::Named => "Named",
        })
    }
}

/// Error returned by [`AccountType::try_from`]: the value is not one of the four account-type
/// values (0–3).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct InvalidAccountType {
    value: u8,
}

impl InvalidAccountType {
    /// Returns the rejected value.
    pub const fn value(self) -> u8 {
        self.value
    }
}

impl core::error::Error for InvalidAccountType {}

impl core::fmt::Display for InvalidAccountType {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Account type must be 0, 1, 2, or 3, got {}", self.value)
    }
}

/// Checked conversion from the numeric account-type value — the inverse of `AccountType as
/// u8`.
///
/// The values (`Regular = 0`, `AppAgent = 1`, `Transactional = 2`, `Named = 3`) are a
/// cross-language contract, and for the keyless variants they equal the discriminant stored
/// at byte `[4]` of the account id. Python's `AccountType(value)` (an `IntEnum` call) is the
/// same value→member lookup; TypeScript needs none, its `AccountType` values being the plain
/// numbers themselves. A Rust consumer holding the raw discriminant byte no longer
/// hand-writes the match here.
impl TryFrom<u8> for AccountType {
    type Error = InvalidAccountType;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Regular),
            1 => Ok(Self::AppAgent),
            2 => Ok(Self::Transactional),
            3 => Ok(Self::Named),
            value => Err(InvalidAccountType { value }),
        }
    }
}

/// Error returned by the typed `decode_*` functions when the account is not of the expected
/// keyless variant.
///
/// Carries both the keyless type the called decoder is for (`expected`) and the account's
/// actual classification (`found`), as determined by
/// [`decode_account_info`](crate::decode_account_info) — the same `(expected, found)` pair
/// the Python/TypeScript `AccountTypeMismatchError` exposes.
///
/// Deliberately exhaustive (no `#[non_exhaustive]`): the single variant is closed over the
/// account-type taxonomy, so it does not grow if the frozen encoding gains a type.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
#[cfg_attr(
    feature = "scale",
    derive(Encode, Decode, DecodeWithMemTracking, TypeInfo, MaxEncodedLen)
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum KeylessDecodeError {
    /// The account does not decode as the keyless type the called decoder expects.
    #[cfg_attr(feature = "scale", codec(index = 0))]
    WrongAccountType {
        /// The keyless type the called decoder is for (`AppAgent`, `Transactional`, or
        /// `Named`).
        expected: AccountType,
        /// The actual classification of the account.
        found: AccountType,
    },
}

impl core::error::Error for KeylessDecodeError {}

impl core::fmt::Display for KeylessDecodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let Self::WrongAccountType { expected, found } = self;
        let expected_phrase = match expected {
            AccountType::Regular => "a Regular account",
            AccountType::AppAgent => "an App Agent root account",
            AccountType::Transactional => "a Transactional account",
            AccountType::Named => "a Named account",
        };
        write!(f, "Account is not {expected_phrase} (found: {found})")
    }
}

/// The identifiers that define a Transactional keyless account.
///
/// Passed to [`encode_transactional_account`](crate::encode_transactional_account) and returned
/// by [`decode_transactional_account`](crate::decode_transactional_account). A named struct plus
/// distinct identifier newtypes prevents the App Agent and Transactional IDs from being silently
/// transposed. Mirrors the object-shaped TypeScript encoder input and the keyword-only Python
/// encoder arguments.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
#[cfg_attr(
    feature = "scale",
    derive(Encode, Decode, DecodeWithMemTracking, TypeInfo, MaxEncodedLen)
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TransactionalAccountIdentifiers {
    /// The App Agent that controls the account.
    pub app_agent_id: AppAgentId,
    /// The transactional sub-account id.
    pub ta_id: TransactionalId,
}

impl TransactionalAccountIdentifiers {
    /// Creates the identifiers that define a Transactional keyless account.
    pub const fn new(app_agent_id: AppAgentId, ta_id: TransactionalId) -> Self {
        Self {
            app_agent_id,
            ta_id,
        }
    }
}

/// The identifiers that define a Named keyless account.
///
/// Passed to [`encode_named_account`](crate::encode_named_account) and returned by
/// [`decode_named_account`](crate::decode_named_account). The object shape prevents positional
/// confusion and mirrors the TypeScript and Python encoder APIs.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
#[cfg_attr(
    feature = "scale",
    derive(Encode, Decode, DecodeWithMemTracking, TypeInfo, MaxEncodedLen)
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NamedAccountIdentifiers {
    /// The App Agent that controls the account.
    pub app_agent_id: AppAgentId,
    /// The name embedded in the account bytes.
    pub account_name: AccountName,
}

impl NamedAccountIdentifiers {
    /// Creates the identifiers that define a Named keyless account.
    pub const fn new(app_agent_id: AppAgentId, account_name: AccountName) -> Self {
        Self {
            app_agent_id,
            account_name,
        }
    }
}

/// Decoded identity of a blockchain account.
///
/// Returned by [`decode_account_info`](crate::decode_account_info). The caller can
/// pattern-match on this enum to determine whether an account is a regular (non-keyless)
/// account or one of the three Trevo keyless variants, and to extract the embedded identifiers.
///
/// # Consistency guarantees
///
/// Values produced through the public API are internally consistent: the identifiers carried by
/// the variant are exactly the ones embedded in `account_id`. The struct variants are
/// `#[non_exhaustive]`, preventing downstream crates from constructing them directly. Both serde
/// deserialization and SCALE decoding validate the invariant. The SCALE representation contains
/// only the 32-byte [`AccountId32`]; decoding derives the enum variant and identifiers from those
/// bytes.
///
/// ```compile_fail
/// use trevokeyless::{AccountId32, BlockchainAccountInfo};
///
/// // Downstream code cannot bypass classification by constructing a variant directly.
/// let _ = BlockchainAccountInfo::Regular {
///     account_id: AccountId32::new([0; 32]),
/// };
/// ```
///
/// # Cross-language note (serde)
///
/// The serde representation is the externally tagged Rust enum, e.g.
/// `{"Named":{"account_id":"ttow…","app_agent_id":123,"account_name":"example123"}}`.
/// This shape differs from the flat info objects returned by the Python and TypeScript
/// trevokeyless libraries; only SS58 address strings and raw account-id bytes are
/// cross-language interchange formats.
#[derive(Eq, PartialEq, Debug, Clone, Copy, Hash)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(remote = "Self")
)]
pub enum BlockchainAccountInfo {
    /// A regular account whose bytes do not match any valid keyless encoding. Its private
    /// key may or may not exist — the point is that no keyless identifiers are embedded.
    #[non_exhaustive]
    Regular {
        /// The 32-byte account id.
        account_id: AccountId32,
    },
    /// The root keyless account of an App Agent. No private key is known; the App Agent
    /// acts on its behalf via App Transactions.
    #[non_exhaustive]
    AppAgent {
        /// The 32-byte account id.
        account_id: AccountId32,
        /// The App Agent that controls the account.
        app_agent_id: AppAgentId,
    },
    /// A transactional sub-account of an App Agent. Controlled exclusively by the App Agent
    /// identified by `app_agent_id`.
    #[non_exhaustive]
    Transactional {
        /// The 32-byte account id.
        account_id: AccountId32,
        /// The App Agent that controls the account.
        app_agent_id: AppAgentId,
        /// The transactional sub-account id.
        ta_id: TransactionalId,
    },
    /// A named sub-account of an App Agent. The `account_name` is embedded verbatim in the
    /// account bytes.
    #[non_exhaustive]
    Named {
        /// The 32-byte account id.
        account_id: AccountId32,
        /// The App Agent that controls the account.
        app_agent_id: AppAgentId,
        /// The name embedded in the account bytes.
        account_name: AccountName,
    },
}

impl BlockchainAccountInfo {
    /// Returns the underlying `AccountId32` for any variant.
    #[must_use]
    pub fn account_id(&self) -> AccountId32 {
        match self {
            Self::Regular { account_id }
            | Self::AppAgent { account_id, .. }
            | Self::Transactional { account_id, .. }
            | Self::Named { account_id, .. } => *account_id,
        }
    }

    /// Returns the `AppAgentId` if this is a keyless account, or `None` for `Regular`.
    #[must_use]
    pub fn app_agent_id(&self) -> Option<AppAgentId> {
        match self {
            Self::Regular { .. } => None,
            Self::AppAgent { app_agent_id, .. }
            | Self::Transactional { app_agent_id, .. }
            | Self::Named { app_agent_id, .. } => Some(*app_agent_id),
        }
    }

    /// Returns the [`AccountType`] classification of this account.
    #[must_use]
    pub fn account_type(&self) -> AccountType {
        match self {
            Self::Regular { .. } => AccountType::Regular,
            Self::AppAgent { .. } => AccountType::AppAgent,
            Self::Transactional { .. } => AccountType::Transactional,
            Self::Named { .. } => AccountType::Named,
        }
    }

    /// Returns the `TransactionalId` if this is a Transactional account, or `None` otherwise.
    #[must_use]
    pub fn ta_id(&self) -> Option<TransactionalId> {
        match self {
            Self::Transactional { ta_id, .. } => Some(*ta_id),
            _ => None,
        }
    }

    /// Returns the [`AccountName`] if this is a Named account, or `None` otherwise.
    #[must_use]
    pub fn account_name(&self) -> Option<AccountName> {
        match self {
            Self::Named { account_name, .. } => Some(*account_name),
            _ => None,
        }
    }

    /// Returns `true` if this is a keyless account (App Agent, Transactional, or Named).
    #[must_use]
    pub fn is_keyless_account(&self) -> bool {
        !matches!(self, Self::Regular { .. })
    }
}

/// The SCALE ABI of `BlockchainAccountInfo` is exactly one `AccountId32` (32 bytes). The
/// classification and all identifiers are derived while decoding, so SCALE can never construct an
/// internally inconsistent value from redundant fields.
#[cfg(feature = "scale")]
#[cfg_attr(docsrs, doc(cfg(feature = "scale")))]
impl Encode for BlockchainAccountInfo {
    fn size_hint(&self) -> usize {
        self.account_id().size_hint()
    }

    fn encode_to<T: Output + ?Sized>(&self, dest: &mut T) {
        self.account_id().encode_to(dest);
    }
}

#[cfg(feature = "scale")]
#[cfg_attr(docsrs, doc(cfg(feature = "scale")))]
impl Decode for BlockchainAccountInfo {
    fn decode<I: Input>(input: &mut I) -> Result<Self, CodecError> {
        let account_id = AccountId32::decode(input)?;
        Ok(crate::keyless::decode_account_info(&account_id))
    }
}

/// The manual decoder reads one fixed-width account id and performs no allocation.
#[cfg(feature = "scale")]
#[cfg_attr(docsrs, doc(cfg(feature = "scale")))]
impl DecodeWithMemTracking for BlockchainAccountInfo {}

/// The derive macro supplies `EncodeLike` alongside `Encode`; the hand-written `Encode` above
/// must therefore declare it explicitly. Without it this type cannot satisfy
/// `frame_support::Parameter` (`Codec + EncodeLike + ...`) or `FullCodec`, i.e. it could not be
/// a pallet call argument, event field, or storage value — the crate's primary use.
///
/// Sound because encoding is deterministic and `Self::decode` of `Self::encode` reproduces the
/// value: the SCALE form is exactly the 32-byte account id, from which decoding re-derives the
/// classification.
#[cfg(feature = "scale")]
#[cfg_attr(docsrs, doc(cfg(feature = "scale")))]
impl EncodeLike for BlockchainAccountInfo {}

#[cfg(feature = "scale")]
#[cfg_attr(docsrs, doc(cfg(feature = "scale")))]
impl MaxEncodedLen for BlockchainAccountInfo {
    fn max_encoded_len() -> usize {
        AccountId32::max_encoded_len()
    }
}

#[cfg(feature = "scale")]
#[cfg_attr(docsrs, doc(cfg(feature = "scale")))]
impl TypeInfo for BlockchainAccountInfo {
    type Identity = Self;

    fn type_info() -> scale_info::Type {
        scale_info::Type::builder()
            .path(scale_info::Path::new(
                "BlockchainAccountInfo",
                module_path!(),
            ))
            .composite(
                scale_info::build::Fields::unnamed()
                    .field(|field| field.ty::<AccountId32>().type_name("AccountId32")),
            )
    }
}

/// `BlockchainAccountInfo` serializes via the derived externally tagged enum representation;
/// see the type-level documentation for the exact shape.
#[cfg(feature = "serde")]
#[cfg_attr(docsrs, doc(cfg(feature = "serde")))]
impl serde::Serialize for BlockchainAccountInfo {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        // Delegates to the inherent function generated by `#[serde(remote = "Self")]`,
        // which holds the derived implementation.
        Self::serialize(self, serializer)
    }
}

/// Validating deserializer: after deserializing via the derived representation, the value is
/// checked for internal consistency — re-classifying `account_id` must reproduce the
/// deserialized value exactly. Inconsistent input (e.g. an `AppAgent` claim over an account id
/// that does not embed that `app_agent_id`, or a `Regular` claim over a keyless account) is
/// rejected.
#[cfg(feature = "serde")]
#[cfg_attr(docsrs, doc(cfg(feature = "serde")))]
impl<'de> serde::Deserialize<'de> for BlockchainAccountInfo {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        // The inherent function generated by `#[serde(remote = "Self")]` holds the derived
        // implementation.
        let unchecked = Self::deserialize(deserializer)?;
        if crate::keyless::decode_account_info(&unchecked.account_id()) != unchecked {
            return Err(serde::de::Error::custom(
                "inconsistent BlockchainAccountInfo: the identifiers do not match account_id",
            ));
        }
        Ok(unchecked)
    }
}
