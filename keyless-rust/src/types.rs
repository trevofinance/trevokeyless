#[cfg(feature = "scale")]
use parity_scale_codec::{
    Decode, DecodeWithMemTracking, Encode, Error as CodecError, Input, MaxEncodedLen,
};
#[cfg(feature = "scale")]
use scale_info::TypeInfo;

/// Numeric identifier of an App Agent (u32, little-endian in keyless account bytes [0..=3]).
pub type AppAgentId = u32;

/// Numeric identifier of a Transactional sub-account (u32, little-endian in bytes [5..=8]).
pub type TransactionalId = u32;

/// Raw type for the one-byte account type discriminant stored at byte `[4]` of a keyless
/// account.
pub(crate) type AccountIdentifierType = u8;

/// Byte `[4]` discriminant for an App Agent root keyless account.
pub const APP_AGENT_ACCOUNT_IDENTIFIER: AccountIdentifierType = 1;
/// Byte `[4]` discriminant for a Transactional keyless account.
pub const TRANSACTIONAL_ACCOUNT_IDENTIFIER: AccountIdentifierType = 2;
/// Byte `[4]` discriminant for a Named keyless account.
pub const NAMED_ACCOUNT_IDENTIFIER: AccountIdentifierType = 3;

/// A 32-byte on-chain account identifier.
///
/// For keyless accounts the bytes follow the layout described in the crate documentation.
/// For regular accounts the bytes are an opaque public key or hash with no special structure.
#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
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
pub struct ParseAccountIdError;

#[cfg(feature = "std")]
impl std::error::Error for ParseAccountIdError {}

impl core::fmt::Display for ParseAccountIdError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("Invalid hex account id: expected 0x followed by 64 hex characters")
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
        fn nibble(c: u8) -> Result<u8, ParseAccountIdError> {
            match c {
                b'0'..=b'9' => Ok(c - b'0'),
                b'a'..=b'f' => Ok(c - b'a' + 10),
                b'A'..=b'F' => Ok(c - b'A' + 10),
                _ => Err(ParseAccountIdError),
            }
        }

        let hex = s.strip_prefix("0x").ok_or(ParseAccountIdError)?.as_bytes();
        if hex.len() != 64 {
            // Byte length: multi-byte UTF-8 input of 64 bytes is caught by `nibble`.
            return Err(ParseAccountIdError);
        }
        let mut bytes = [0u8; 32];
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
    /// Returns an error if `input` is not exactly 10 bytes or contains characters outside the
    /// allowed set (`0-9 a-z A-Z - #`).
    pub fn new(input: &[u8]) -> Result<Self, AccountNameError> {
        if input.len() != 10 {
            return Err(AccountNameError::InvalidLength);
        }
        if input.iter().all(|&b| AccountName::VALID_CHARS[b as usize]) {
            let mut array = [0u8; 10];
            array.copy_from_slice(input);
            Ok(AccountName(array))
        } else {
            Err(AccountNameError::InvalidCharacter)
        }
    }

    /// Returns the raw 10-byte representation of the name.
    pub fn as_bytes(&self) -> &[u8; 10] {
        &self.0
    }

    /// Returns the name as a string slice.
    ///
    /// All allowed characters are ASCII, so the conversion never fails.
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
impl DecodeWithMemTracking for AccountName {}

#[cfg(feature = "serde")]
impl serde::Serialize for AccountName {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

#[cfg(feature = "serde")]
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
pub enum AccountNameError {
    /// The input is not exactly 10 bytes long.
    InvalidLength,
    /// The input contains a character outside the allowed set (`0-9 a-z A-Z - #`).
    InvalidCharacter,
}

#[cfg(feature = "std")]
impl std::error::Error for AccountNameError {}

impl core::fmt::Display for AccountNameError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidLength => f.write_str("Account name must be exactly 10 bytes long"),
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
pub enum AccountType {
    /// A regular account: no keyless identifiers are embedded in its bytes.
    Regular,
    /// The root keyless account of an App Agent.
    AppAgent,
    /// A transactional keyless sub-account of an App Agent.
    Transactional,
    /// A named keyless sub-account of an App Agent.
    Named,
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
    WrongAccountType {
        /// The keyless type the called decoder is for (`AppAgent`, `Transactional`, or
        /// `Named`).
        expected: AccountType,
        /// The actual classification of the account.
        found: AccountType,
    },
}

#[cfg(feature = "std")]
impl std::error::Error for KeylessDecodeError {}

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

/// The identifiers embedded in a Transactional keyless account, returned by
/// [`decode_transactional_account`](crate::decode_transactional_account).
///
/// A named struct rather than a bare `(AppAgentId, TransactionalId)` tuple, so the two
/// `u32` identifiers cannot be silently transposed at the call site. Mirrors the object the
/// TypeScript `decodeTransactionalAddress` returns and the Python `DecodedTransactionalAddress`
/// named tuple.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
#[cfg_attr(
    feature = "scale",
    derive(Encode, Decode, DecodeWithMemTracking, TypeInfo, MaxEncodedLen)
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DecodedTransactionalAccount {
    /// The App Agent that controls the account.
    pub app_agent_id: AppAgentId,
    /// The transactional sub-account id.
    pub ta_id: TransactionalId,
}

/// The identifiers embedded in a Named keyless account, returned by
/// [`decode_named_account`](crate::decode_named_account).
///
/// A named struct rather than a bare `(AppAgentId, AccountName)` tuple. Mirrors the object the
/// TypeScript `decodeNamedAddress` returns and the Python `DecodedNamedAddress` named tuple.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
#[cfg_attr(
    feature = "scale",
    derive(Encode, Decode, DecodeWithMemTracking, TypeInfo, MaxEncodedLen)
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DecodedNamedAccount {
    /// The App Agent that controls the account.
    pub app_agent_id: AppAgentId,
    /// The name embedded in the account bytes.
    pub account_name: AccountName,
}

/// Decoded identity of a blockchain account.
///
/// Returned by [`decode_account_info`](crate::decode_account_info). The caller can
/// pattern-match on this enum to determine whether an account is a regular (non-keyless)
/// account or one of the three Trevo keyless variants, and to extract the embedded identifiers.
///
/// # Consistency guarantees
///
/// Values produced by [`decode_account_info`](crate::decode_account_info) are always
/// internally consistent: the identifiers carried by the variant are exactly the ones embedded
/// in `account_id`. The serde `Deserialize` implementation re-validates this invariant and
/// rejects inconsistent input. The SCALE [`Decode`] implementation performs **no** such
/// validation (keeping the usual cost model of SCALE decoding), so SCALE-decoded values from
/// untrusted sources must not be treated as verified claims — re-derive them with
/// [`decode_account_info`](crate::decode_account_info) if in doubt.
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
    feature = "scale",
    derive(Encode, Decode, DecodeWithMemTracking, TypeInfo, MaxEncodedLen)
)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(remote = "Self")
)]
pub enum BlockchainAccountInfo {
    /// A regular account whose bytes do not match any valid keyless encoding. Its private
    /// key may or may not exist — the point is that no keyless identifiers are embedded.
    Regular { account_id: AccountId32 },
    /// The root keyless account of an App Agent. No private key is known; the App Agent
    /// acts on its behalf via App Transactions.
    AppAgent {
        account_id: AccountId32,
        app_agent_id: AppAgentId,
    },
    /// A transactional sub-account of an App Agent. Controlled exclusively by the App Agent
    /// identified by `app_agent_id`.
    Transactional {
        account_id: AccountId32,
        app_agent_id: AppAgentId,
        ta_id: TransactionalId,
    },
    /// A named sub-account of an App Agent. The `account_name` is embedded verbatim in the
    /// account bytes.
    Named {
        account_id: AccountId32,
        app_agent_id: AppAgentId,
        account_name: AccountName,
    },
}

impl BlockchainAccountInfo {
    /// Returns the underlying `AccountId32` for any variant.
    pub fn account_id(&self) -> AccountId32 {
        match self {
            Self::Regular { account_id }
            | Self::AppAgent { account_id, .. }
            | Self::Transactional { account_id, .. }
            | Self::Named { account_id, .. } => *account_id,
        }
    }

    /// Returns the `AppAgentId` if this is a keyless account, or `None` for `Regular`.
    pub fn app_agent_id(&self) -> Option<AppAgentId> {
        match self {
            Self::Regular { .. } => None,
            Self::AppAgent { app_agent_id, .. }
            | Self::Transactional { app_agent_id, .. }
            | Self::Named { app_agent_id, .. } => Some(*app_agent_id),
        }
    }

    /// Returns the [`AccountType`] classification of this account.
    pub fn account_type(&self) -> AccountType {
        match self {
            Self::Regular { .. } => AccountType::Regular,
            Self::AppAgent { .. } => AccountType::AppAgent,
            Self::Transactional { .. } => AccountType::Transactional,
            Self::Named { .. } => AccountType::Named,
        }
    }

    /// Returns the `TransactionalId` if this is a Transactional account, or `None` otherwise.
    pub fn ta_id(&self) -> Option<TransactionalId> {
        match self {
            Self::Transactional { ta_id, .. } => Some(*ta_id),
            _ => None,
        }
    }

    /// Returns the [`AccountName`] if this is a Named account, or `None` otherwise.
    pub fn account_name(&self) -> Option<AccountName> {
        match self {
            Self::Named { account_name, .. } => Some(*account_name),
            _ => None,
        }
    }

    /// Returns `true` if this is a keyless account (App Agent, Transactional, or Named).
    pub fn is_keyless_account(&self) -> bool {
        !matches!(self, Self::Regular { .. })
    }
}

/// `BlockchainAccountInfo` serializes via the derived externally tagged enum representation;
/// see the type-level documentation for the exact shape.
#[cfg(feature = "serde")]
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
