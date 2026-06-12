use parity_scale_codec::{
    Decode, DecodeWithMemTracking, Encode, Error as CodecError, Input, MaxEncodedLen,
};
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
#[derive(
    Clone,
    Copy,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    Encode,
    Decode,
    DecodeWithMemTracking,
    Debug,
    TypeInfo,
    MaxEncodedLen,
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
#[derive(
    Clone, Encode, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Copy, TypeInfo, MaxEncodedLen,
)]
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
#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    PartialEq,
    Hash,
    Encode,
    Decode,
    DecodeWithMemTracking,
    TypeInfo,
    MaxEncodedLen,
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
/// This is the Rust counterpart of the `AddressType` enum of the Python and TypeScript
/// trevo-keyless libraries; the variant order (Regular = 0, AppAgent = 1, Transactional = 2,
/// Named = 3) matches their numeric values, so the keyless variants coincide with the type
/// discriminant stored at byte `[4]` of a keyless account.
#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    Encode,
    Decode,
    DecodeWithMemTracking,
    TypeInfo,
    MaxEncodedLen,
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
/// Each variant carries the actual classification of the account in its `found` field, as
/// determined by [`decode_account_info`](crate::decode_account_info).
#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    PartialEq,
    Hash,
    Encode,
    Decode,
    DecodeWithMemTracking,
    TypeInfo,
    MaxEncodedLen,
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum KeylessDecodeError {
    /// The account does not decode as an App Agent root account.
    NotAnAppAgentAccount {
        /// The actual classification of the account.
        found: AccountType,
    },
    /// The account does not decode as a Transactional account.
    NotATransactionalAccount {
        /// The actual classification of the account.
        found: AccountType,
    },
    /// The account does not decode as a Named account.
    NotANamedAccount {
        /// The actual classification of the account.
        found: AccountType,
    },
}

#[cfg(feature = "std")]
impl std::error::Error for KeylessDecodeError {}

impl core::fmt::Display for KeylessDecodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NotAnAppAgentAccount { found } => {
                write!(
                    f,
                    "Account is not an App Agent root account (found: {found})"
                )
            }
            Self::NotATransactionalAccount { found } => {
                write!(f, "Account is not a Transactional account (found: {found})")
            }
            Self::NotANamedAccount { found } => {
                write!(f, "Account is not a Named account (found: {found})")
            }
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
/// trevo-keyless libraries; only SS58 address strings and raw account-id bytes are
/// cross-language interchange formats.
#[derive(
    Eq,
    PartialEq,
    Debug,
    Clone,
    Copy,
    Hash,
    Encode,
    Decode,
    DecodeWithMemTracking,
    TypeInfo,
    MaxEncodedLen,
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

#[cfg(all(test, feature = "std"))]
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
            AccountNameError::InvalidLength
        );
        assert_eq!(
            AccountName::new(long_account).unwrap_err(),
            AccountNameError::InvalidLength
        );
    }

    #[test]
    fn test_invalid_characters() {
        let invalid_account1 = b"abcdEF@123";
        let invalid_account2 = b"abcdEF*123";
        let invalid_account3 = b"abcdEF/123";
        assert_eq!(
            AccountName::new(invalid_account1).unwrap_err(),
            AccountNameError::InvalidCharacter
        );
        assert_eq!(
            AccountName::new(invalid_account2).unwrap_err(),
            AccountNameError::InvalidCharacter
        );
        assert_eq!(
            AccountName::new(invalid_account3).unwrap_err(),
            AccountNameError::InvalidCharacter
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

#[cfg(all(test, feature = "std"))]
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
    fn test_account_name_as_str() {
        let name = AccountName::new(b"hot-wallet").unwrap();
        assert_eq!(name.as_str(), "hot-wallet");
    }

    #[test]
    fn test_account_type_display() {
        assert_eq!(format!("{}", AccountType::Regular), "Regular");
        assert_eq!(format!("{}", AccountType::AppAgent), "AppAgent");
        assert_eq!(format!("{}", AccountType::Transactional), "Transactional");
        assert_eq!(format!("{}", AccountType::Named), "Named");
    }

    #[test]
    fn test_account_name_error_display() {
        assert_eq!(
            format!("{}", AccountNameError::InvalidLength),
            "Account name must be exactly 10 bytes long"
        );
        assert_eq!(
            format!("{}", AccountNameError::InvalidCharacter),
            "Account name contains invalid characters"
        );
    }

    #[test]
    fn test_keyless_decode_error_display() {
        assert_eq!(
            format!(
                "{}",
                KeylessDecodeError::NotAnAppAgentAccount {
                    found: AccountType::Regular
                }
            ),
            "Account is not an App Agent root account (found: Regular)"
        );
        assert_eq!(
            format!(
                "{}",
                KeylessDecodeError::NotATransactionalAccount {
                    found: AccountType::Named
                }
            ),
            "Account is not a Transactional account (found: Named)"
        );
        assert_eq!(
            format!(
                "{}",
                KeylessDecodeError::NotANamedAccount {
                    found: AccountType::AppAgent
                }
            ),
            "Account is not a Named account (found: AppAgent)"
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
    fn test_account_name_scale_decode_rejects_invalid_characters() {
        // Exactly 10 bytes, but the first one (0x00) is outside the allowed charset.
        let bytes = *b"\x00ot-wallet";
        assert!(AccountName::decode(&mut &bytes[..]).is_err());
    }

    #[test]
    fn test_account_name_scale_decode_rejects_truncated_input() {
        // Fewer than the 10 bytes required by the fixed-size encoding.
        assert!(AccountName::decode(&mut &b"short"[..]).is_err());
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
            KeylessDecodeError::NotAnAppAgentAccount {
                found: AccountType::Regular,
            },
            KeylessDecodeError::NotATransactionalAccount {
                found: AccountType::AppAgent,
            },
            KeylessDecodeError::NotANamedAccount {
                found: AccountType::Transactional,
            },
        ] {
            let encoded = error.encode();
            // One byte for the error variant, one for the embedded `AccountType`.
            assert_eq!(encoded.len(), 2);
            let decoded = KeylessDecodeError::decode(&mut &encoded[..]).unwrap();
            assert_eq!(decoded, error);
        }
    }

    #[test]
    fn test_account_type_scale_codec() {
        for (account_type, expected_index) in [
            (AccountType::Regular, 0u8),
            (AccountType::AppAgent, 1),
            (AccountType::Transactional, 2),
            (AccountType::Named, 3),
        ] {
            let encoded = account_type.encode();
            // The SCALE variant index matches the numeric values of the Python/TypeScript
            // `AddressType` enums (and, for keyless variants, the byte [4] discriminant).
            assert_eq!(encoded, [expected_index]);
            let decoded = AccountType::decode(&mut &encoded[..]).unwrap();
            assert_eq!(decoded, account_type);
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

    /// Pins the documented contract that SCALE decoding performs **no** consistency
    /// validation (unlike serde deserialization, which re-validates): a value whose
    /// identifiers do not match its `account_id` round-trips through SCALE unchanged.
    #[test]
    fn test_blockchain_account_info_scale_decode_performs_no_validation() {
        let inconsistent = BlockchainAccountInfo::AppAgent {
            account_id: AccountId32::new([0u8; 32]),
            app_agent_id: 999,
        };
        assert_ne!(
            crate::keyless::decode_account_info(&inconsistent.account_id()),
            inconsistent,
            "self check: the crafted value must actually be inconsistent"
        );

        let encoded = inconsistent.encode();
        let decoded = BlockchainAccountInfo::decode(&mut &encoded[..]).unwrap();
        assert_eq!(
            decoded, inconsistent,
            "SCALE decode must accept inconsistent values; see the type documentation"
        );
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
        assert_impl::<AccountNameError>();
        assert_impl::<AccountType>();
        assert_impl::<KeylessDecodeError>();
        assert_impl::<BlockchainAccountInfo>();
    }

    /// Compile-time check that every public type of the crate supports `core::hash::Hash`,
    /// independently of the `std` feature.
    #[test]
    fn test_all_types_implement_hash() {
        fn assert_impl<T: core::hash::Hash>() {}
        assert_impl::<AppAgentId>();
        assert_impl::<TransactionalId>();
        assert_impl::<AccountId32>();
        assert_impl::<AccountName>();
        assert_impl::<AccountNameError>();
        assert_impl::<AccountType>();
        assert_impl::<KeylessDecodeError>();
        assert_impl::<BlockchainAccountInfo>();
    }
}
