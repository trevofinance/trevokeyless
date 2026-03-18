use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;

// Types of identifiers
pub type AppAgentId = u32;
pub type TransactionalId = u32;

// identifiers for account type
pub type AccountIdentifierType = u8;
pub const APP_AGENT_ACCOUNT_IDENTIFIER: AccountIdentifierType = 1;
pub const TRANSACTIONAL_ACCOUNT_IDENTIFIER: AccountIdentifierType = 2;
pub const NAMED_ACCOUNT_IDENTIFIER: AccountIdentifierType = 3;

/// An opaque 32-byte cryptographic identifier.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Encode, Decode, Debug, TypeInfo, MaxEncodedLen)]
#[cfg_attr(feature = "std", derive(Hash))]
pub struct AccountId32([u8; 32]);

impl AccountId32 {
    /// Create a new instance from its raw inner byte value.
    ///
    /// Equivalent to this types `From<[u8; 32]>` implementation. For the lack of const
    /// support in traits we have this constructor.
    pub const fn new(inner: [u8; 32]) -> Self {
        Self(inner)
    }

    pub fn inner(&self) -> &[u8; 32] {
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

#[derive(
    Clone,
    Encode,
    Decode,
    Debug,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Default,
    Copy,
    TypeInfo,
    MaxEncodedLen,
)]
pub struct AccountName([u8; 10]);

impl AccountName {
    const ALLOWED_CHARS: &'static [u8] =
        b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ-#";

    pub fn new(input: &[u8]) -> Result<Self, &'static str> {
        if input.len() != 10 {
            return Err("Input must be exactly 10 bytes long");
        }
        if input
            .iter()
            .all(|&b| AccountName::ALLOWED_CHARS.contains(&b))
        {
            let mut array = [0u8; 10];
            array.copy_from_slice(input);
            Ok(AccountName(array))
        } else {
            Err("Input contains invalid characters")
        }
    }

    pub fn as_bytes(&self) -> &[u8; 10] {
        &self.0
    }

    pub fn starts_with(&self, prefix: &[u8]) -> bool {
        self.0.starts_with(prefix)
    }
}

impl TryFrom<&[u8]> for AccountName {
    type Error = &'static str;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        AccountName::new(value)
    }
}

/// Represents the type of a blockchain account.
///
/// Variants:
/// - `Regular`: A standard blockchain account.
/// - `AppAgent`: An account associated with an application agent.
/// - `Transactional`: An account used for transactional purposes.
/// - `Named`: An account that has a specific name associated with it.
#[derive(Eq, PartialEq, Debug, Clone, Copy)]
pub enum AccountType {
    Regular,
    AppAgent,
    Transactional,
    Named,
}

/// Contains information related to a blockchain account.
///
/// Fields:
/// - `account_id`: The account ID associated with the blockchain account, represented as a vector of bytes.
/// - `account_type`: The type of the blockchain account.
/// - `app_agent_id`: The application agent ID associated with the account, if applicable.
/// - `ta_id`: The transactional account ID, if applicable.
/// - `account_name`: The name of the account, if applicable.
#[derive(Eq, PartialEq, Debug)]
pub struct BlockchainAccountInfo {
    pub account_id: AccountId32,
    pub account_type: AccountType,
    pub app_agent_id: Option<AppAgentId>,
    pub ta_id: Option<TransactionalId>,
    pub account_name: Option<AccountName>,
}

impl BlockchainAccountInfo {
    /// Helper function to create a new `BlockchainAccountInfo` with the given account_id.
    /// The `account_type` will be set to `AccountType::Regular`, and other optional fields will be set to `None`.
    pub fn from_regular_account(account_id: AccountId32) -> Self {
        Self {
            account_id,
            account_type: AccountType::Regular,
            app_agent_id: None,
            ta_id: None,
            account_name: None,
        }
    }

    /// Helper function to create a new `BlockchainAccountInfo` with the given account_id, and app_agent_id.
    /// The `account_type` will be set to `AccountType::AppAgent`, and other optional fields will be set to `None`.
    pub fn from_app_agent_account(account_id: AccountId32, app_agent_id: AppAgentId) -> Self {
        Self {
            account_id,
            account_type: AccountType::AppAgent,
            app_agent_id: Some(app_agent_id),
            ta_id: None,
            account_name: None,
        }
    }

    /// Helper function to create a new `BlockchainAccountInfo` with the given account_id, ta_id and app_agent_id.
    /// The `account_type` will be set to `AccountType::Transactional`, and other optional fields will be set to `None`.
    pub fn from_ta_account(
        account_id: AccountId32,
        app_agent_id: AppAgentId,
        ta_id: TransactionalId,
    ) -> Self {
        Self {
            account_id,
            account_type: AccountType::Transactional,
            app_agent_id: Some(app_agent_id),
            ta_id: Some(ta_id),
            account_name: None,
        }
    }

    /// Helper function to create a new `BlockchainAccountInfo` with the given account_id, name and app_agent_id.
    /// The `account_type` will be set to `AccountType::Named`, and other optional fields will be set to `None`.
    pub fn from_named_account(
        account_id: AccountId32,
        app_agent_id: AppAgentId,
        name: AccountName,
    ) -> Self {
        Self {
            account_id,
            account_type: AccountType::Named,
            app_agent_id: Some(app_agent_id),
            ta_id: None,
            account_name: Some(name),
        }
    }

    /// Returns true if the account is a keyless account, false otherwise.
    pub fn is_keyless(&self) -> bool {
        self.account_type != AccountType::Regular
    }
}

/// Internal enum to carry the type and IDs of account
pub(super) enum BlockchainAccountIds {
    Regular,
    AppAgent(AppAgentId),
    Transactional((AppAgentId, TransactionalId)),
    Named((AppAgentId, AccountName)),
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
