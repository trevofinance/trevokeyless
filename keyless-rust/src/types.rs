use parity_scale_codec::{Decode, Encode, Error as CodecError, Input, MaxEncodedLen};
use scale_info::TypeInfo;

/// Numeric identifier of an App Agent (u32, little-endian in keyless account bytes [0..=3]).
pub type AppAgentId = u32;

/// Numeric identifier of a Transactional sub-account (u32, little-endian in bytes [5..=8]).
pub type TransactionalId = u32;

/// Raw type for the one-byte account type discriminant stored at byte [4] of a keyless account.
pub(crate) type AccountIdentifierType = u8;

/// Byte [4] discriminant for an App Agent root keyless account.
pub const APP_AGENT_ACCOUNT_IDENTIFIER: AccountIdentifierType = 1;
/// Byte [4] discriminant for a Transactional keyless account.
pub const TRANSACTIONAL_ACCOUNT_IDENTIFIER: AccountIdentifierType = 2;
/// Byte [4] discriminant for a Named keyless account.
pub const NAMED_ACCOUNT_IDENTIFIER: AccountIdentifierType = 3;

/// A 32-byte on-chain account identifier.
///
/// For keyless accounts the bytes follow the layout described in the crate documentation.
/// For regular accounts the bytes are an opaque public key or hash with no special structure.
#[derive(
    Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Encode, Decode, Debug, TypeInfo, MaxEncodedLen,
)]
#[cfg_attr(feature = "std", derive(Hash))]
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

/// A fixed 10-byte account name used to identify Named keyless accounts.
///
/// Only the characters `0-9`, `a-z`, `A-Z`, `-`, and `#` (64 characters total) are allowed.
/// The name is stored verbatim in bytes [5..=14] of the Named account's open part.
#[derive(Clone, Encode, Debug, Eq, PartialEq, Ord, PartialOrd, Copy, TypeInfo, MaxEncodedLen)]
#[cfg_attr(feature = "std", derive(Hash))]
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
    pub fn new(input: &[u8]) -> Result<Self, &'static str> {
        if input.len() != 10 {
            return Err("Input must be exactly 10 bytes long");
        }
        if input.iter().all(|&b| AccountName::VALID_CHARS[b as usize]) {
            let mut array = [0u8; 10];
            array.copy_from_slice(input);
            Ok(AccountName(array))
        } else {
            Err("Input contains invalid characters")
        }
    }

    /// Returns the raw 10-byte representation of the name.
    pub fn as_bytes(&self) -> &[u8; 10] {
        &self.0
    }

    /// Returns `true` if the name begins with `prefix`.
    pub fn starts_with(&self, prefix: &[u8]) -> bool {
        self.0.starts_with(prefix)
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

impl TryFrom<&[u8]> for AccountName {
    type Error = &'static str;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        AccountName::new(value)
    }
}

impl TryFrom<[u8; 10]> for AccountName {
    type Error = &'static str;

    fn try_from(value: [u8; 10]) -> Result<Self, Self::Error> {
        AccountName::new(&value)
    }
}

impl Decode for AccountName {
    fn decode<I: Input>(input: &mut I) -> Result<Self, CodecError> {
        let mut buf = [0u8; 10];
        input.read(&mut buf)?;
        AccountName::try_from(buf).map_err(CodecError::from)
    }
}

/// Error returned by the typed `decode_*` functions when the account is not of the expected
/// keyless variant.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KeylessDecodeError {
    /// The account does not decode as an App Agent root account.
    NotAnAppAgentAccount,
    /// The account does not decode as a Transactional account.
    NotATransactionalAccount,
    /// The account does not decode as a Named account.
    NotANamedAccount,
}

#[cfg(feature = "std")]
impl std::error::Error for KeylessDecodeError {}

impl core::fmt::Display for KeylessDecodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NotAnAppAgentAccount => f.write_str("Account is not an App Agent root account"),
            Self::NotATransactionalAccount => f.write_str("Account is not a Transactional account"),
            Self::NotANamedAccount => f.write_str("Account is not a Named account"),
        }
    }
}

/// Decoded identity of a blockchain account.
///
/// Returned by [`decode_account_info`](crate::decode_account_info). The caller can
/// pattern-match on this enum to determine whether an account is a regular (non-keyless)
/// account or one of the three Trevo keyless variants, and to extract the embedded identifiers.
#[derive(Eq, PartialEq, Debug, Clone, Copy)]
#[cfg_attr(feature = "std", derive(Hash))]
pub enum BlockchainAccountInfo {
    /// A regular account whose bytes do not match any valid keyless encoding. Its private
    /// key may or may not exist — the point is that no keyless identifiers are embedded.
    Regular { account_id: AccountId32 },
    /// The root keyless account of an App Agent. No private key is known; the App Agent
    /// acts on its behalf via Clearing Transactions.
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
    /// account bytes and can be used to distinguish well-known sub-accounts (e.g. a hot
    /// wallet with name `"hot-wallet"` or a cold wallet whose name starts with `"cold"`).
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

    /// Returns `true` if this is a `Regular` (non-keyless) account.
    pub fn is_regular(&self) -> bool {
        matches!(self, Self::Regular { .. })
    }
}

#[cfg(test)]
mod tests_account_name {
    use super::*;

    #[test]
    fn test_valid_account_name() {
        let valid_account = b"abcdEF1234";
        assert!(AccountName::new(valid_account).is_ok());
    }

    #[test]
    fn test_invalid_length() {
        let short_account = b"abcdEF123";
        let long_account = b"abcdEF12345";
        assert_eq!(
            AccountName::new(short_account).unwrap_err(),
            "Input must be exactly 10 bytes long"
        );
        assert_eq!(
            AccountName::new(long_account).unwrap_err(),
            "Input must be exactly 10 bytes long"
        );
    }

    #[test]
    fn test_invalid_characters() {
        let invalid_account1 = b"abcdEF@123";
        let invalid_account2 = b"abcdEF*123";
        let invalid_account3 = b"abcdEF/123";
        assert_eq!(
            AccountName::new(invalid_account1).unwrap_err(),
            "Input contains invalid characters"
        );
        assert_eq!(
            AccountName::new(invalid_account2).unwrap_err(),
            "Input contains invalid characters"
        );
        assert_eq!(
            AccountName::new(invalid_account3).unwrap_err(),
            "Input contains invalid characters"
        );
    }

    #[test]
    fn test_starts_with() {
        let name = AccountName::new(b"hot-wallet").unwrap();
        assert!(name.starts_with(b"hot"), "must match known prefix");
        assert!(
            name.starts_with(b"hot-wallet"),
            "full name is its own prefix"
        );
        assert!(name.starts_with(b""), "empty prefix always matches");
        assert!(
            !name.starts_with(b"cold"),
            "non-matching prefix must return false"
        );
        assert!(
            !name.starts_with(b"hot-wallet1"),
            "longer-than-name prefix must return false"
        );
    }

    #[test]
    fn test_edge_cases() {
        let valid_account1 = b"0000000000";
        let valid_account2 = b"##########";
        let valid_account3 = b"abcdEFghij";
        let valid_account4 = b"1234567890";

        assert!(AccountName::new(valid_account1).is_ok());
        assert!(AccountName::new(valid_account2).is_ok());
        assert!(AccountName::new(valid_account3).is_ok());
        assert!(AccountName::new(valid_account4).is_ok());
    }
}
