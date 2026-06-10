use parity_scale_codec::{
    Decode, DecodeWithMemTracking, Encode, Error as CodecError, Input, MaxEncodedLen,
};
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
    Clone,
    Copy,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Encode,
    Decode,
    DecodeWithMemTracking,
    Debug,
    TypeInfo,
    MaxEncodedLen,
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

impl core::fmt::Display for AccountId32 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        for b in &self.0 {
            write!(f, "{:02x}", b)?;
        }
        Ok(())
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

impl From<AccountName> for [u8; 10] {
    fn from(name: AccountName) -> [u8; 10] {
        name.0
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

impl TryFrom<&str> for AccountName {
    type Error = &'static str;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        AccountName::new(s.as_bytes())
    }
}

impl core::str::FromStr for AccountName {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        AccountName::new(s.as_bytes())
    }
}

impl core::fmt::Display for AccountName {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // All allowed characters are ASCII, so this conversion is always valid.
        f.write_str(
            core::str::from_utf8(&self.0).expect("AccountName bytes are always valid ASCII"),
        )
    }
}

impl Decode for AccountName {
    fn decode<I: Input>(input: &mut I) -> Result<Self, CodecError> {
        let mut buf = [0u8; 10];
        input.read(&mut buf)?;
        AccountName::try_from(buf).map_err(CodecError::from)
    }
}

/// Sound because the manual `Decode` implementation above reads into a fixed-size stack
/// buffer and never allocates heap memory.
impl DecodeWithMemTracking for AccountName {}

/// Error returned by the typed `decode_*` functions when the account is not of the expected
/// keyless variant.
#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    PartialEq,
    Encode,
    Decode,
    DecodeWithMemTracking,
    TypeInfo,
    MaxEncodedLen,
)]
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
#[derive(
    Eq,
    PartialEq,
    Debug,
    Clone,
    Copy,
    Encode,
    Decode,
    DecodeWithMemTracking,
    TypeInfo,
    MaxEncodedLen,
)]
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

    /// Returns `true` if this is a keyless account (App Agent, Transactional, or Named).
    pub fn is_keyless_account(&self) -> bool {
        !matches!(self, Self::Regular { .. })
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

    #[test]
    fn test_try_from_owned_array() {
        let arr: [u8; 10] = *b"hot-wallet";
        let name = AccountName::try_from(arr).unwrap();
        assert_eq!(name.as_bytes(), b"hot-wallet");

        let invalid: [u8; 10] = *b"invalid\x00xx";
        assert!(AccountName::try_from(invalid).is_err());
    }

    #[test]
    fn test_from_str() {
        let name: AccountName = "hot-wallet".parse().unwrap();
        assert_eq!(name.as_bytes(), b"hot-wallet");

        assert!("too-short".parse::<AccountName>().is_err());
        assert!("invalid@xxx".parse::<AccountName>().is_err());
    }

    #[test]
    fn test_into_array() {
        let name = AccountName::new(b"hot-wallet").unwrap();
        let arr: [u8; 10] = name.into();
        assert_eq!(&arr, b"hot-wallet");
    }
}

#[cfg(test)]
mod tests_display_and_codec {
    use super::*;
    use parity_scale_codec::{Decode, Encode};

    #[test]
    fn test_account_id32_display() {
        let account = AccountId32::new([0u8; 32]);
        assert_eq!(format!("{}", account), "0".repeat(64));

        let mut bytes = [0u8; 32];
        bytes[0] = 0xAB;
        bytes[31] = 0xCD;
        let s = format!("{}", AccountId32::new(bytes));
        assert!(s.starts_with("ab"));
        assert!(s.ends_with("cd"));
        assert_eq!(s.len(), 64);
    }

    #[test]
    fn test_account_name_display() {
        let name = AccountName::new(b"hot-wallet").unwrap();
        assert_eq!(format!("{}", name), "hot-wallet");
    }

    #[test]
    fn test_keyless_decode_error_display() {
        assert_eq!(
            format!("{}", KeylessDecodeError::NotAnAppAgentAccount),
            "Account is not an App Agent root account"
        );
        assert_eq!(
            format!("{}", KeylessDecodeError::NotATransactionalAccount),
            "Account is not a Transactional account"
        );
        assert_eq!(
            format!("{}", KeylessDecodeError::NotANamedAccount),
            "Account is not a Named account"
        );
    }

    #[test]
    fn test_account_name_scale_codec() {
        let name = AccountName::new(b"hot-wallet").unwrap();
        let encoded = name.encode();
        assert_eq!(encoded.len(), 10);
        let decoded = AccountName::decode(&mut &encoded[..]).unwrap();
        assert_eq!(decoded, name);
    }

    #[test]
    fn test_account_id32_scale_codec() {
        let bytes: [u8; 32] = core::array::from_fn(|i| i as u8);
        let account = AccountId32::new(bytes);
        let encoded = account.encode();
        assert_eq!(encoded.len(), 32);
        let decoded = AccountId32::decode(&mut &encoded[..]).unwrap();
        assert_eq!(decoded, account);
    }

    #[test]
    fn test_keyless_decode_error_scale_codec() {
        for error in [
            KeylessDecodeError::NotAnAppAgentAccount,
            KeylessDecodeError::NotATransactionalAccount,
            KeylessDecodeError::NotANamedAccount,
        ] {
            let encoded = error.encode();
            assert_eq!(encoded.len(), 1);
            let decoded = KeylessDecodeError::decode(&mut &encoded[..]).unwrap();
            assert_eq!(decoded, error);
        }
    }

    #[test]
    fn test_blockchain_account_info_scale_codec() {
        let account_id = AccountId32::new(core::array::from_fn(|i| i as u8));
        for info in [
            BlockchainAccountInfo::Regular { account_id },
            BlockchainAccountInfo::AppAgent {
                account_id,
                app_agent_id: 7,
            },
            BlockchainAccountInfo::Transactional {
                account_id,
                app_agent_id: 7,
                ta_id: 9,
            },
            BlockchainAccountInfo::Named {
                account_id,
                app_agent_id: 7,
                account_name: AccountName::new(b"hot-wallet").unwrap(),
            },
        ] {
            let encoded = info.encode();
            let decoded = BlockchainAccountInfo::decode(&mut &encoded[..]).unwrap();
            assert_eq!(decoded, info);
        }
    }

    /// Compile-time check that every public type of the crate supports
    /// `codec::DecodeWithMemTracking`.
    #[test]
    fn test_all_types_implement_decode_with_mem_tracking() {
        fn assert_impl<T: DecodeWithMemTracking>() {}
        assert_impl::<AppAgentId>();
        assert_impl::<TransactionalId>();
        assert_impl::<AccountId32>();
        assert_impl::<AccountName>();
        assert_impl::<KeylessDecodeError>();
        assert_impl::<BlockchainAccountInfo>();
    }
}
