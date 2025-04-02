use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;

// Types of identifiers
pub type AppAgentId = u32;
pub type TransactionalId = u32;

// identifiers for account type
pub type AddressIdentifierType = u8;
pub const APP_AGENT_ADDRESS_IDENTIFIER: AddressIdentifierType = 1;
pub const TRANSACTIONAL_ADDRESS_IDENTIFIER: AddressIdentifierType = 2;
pub const NAMED_ADDRESS_IDENTIFIER: AddressIdentifierType = 3;

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
pub struct AddressName([u8; 10]);

impl AddressName {
    const ALLOWED_CHARS: &'static [u8] =
        b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ-#";

    pub fn new(input: &[u8]) -> Result<Self, &'static str> {
        if input.len() != 10 {
            return Err("Input must be exactly 10 bytes long");
        }
        if input
            .iter()
            .all(|&b| AddressName::ALLOWED_CHARS.contains(&b))
        {
            let mut array = [0u8; 10];
            array.copy_from_slice(input);
            Ok(AddressName(array))
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

impl TryFrom<&[u8]> for AddressName {
    type Error = &'static str;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        AddressName::new(value)
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
pub enum AddressType {
    Regular,
    AppAgent,
    Transactional,
    Named,
}

/// Contains information related to a blockchain account.
///
/// Fields:
/// - `account_id`: The account ID associated with the blockchain address, represented as a vector of bytes.
/// - `address_type`: The type of the blockchain account.
/// - `app_agent_id`: The application agent ID associated with the account, if applicable.
/// - `ta_id`: The transactional account ID, if applicable.
/// - `account_name`: The name of the account, if applicable.
#[derive(Eq, PartialEq, Debug)]
pub struct BlockchainAddressInfo {
    pub account_id: AccountId32,
    pub address_type: AddressType,
    pub app_agent_id: Option<AppAgentId>,
    pub ta_id: Option<TransactionalId>,
    pub address_name: Option<AddressName>,
}

impl BlockchainAddressInfo {
    /// Helper function to create a new `BlockchainAddressInfo` with the given account_id.
    /// The `address_type` will be set to `AddressType::Regular`, and other optional fields will be set to `None`.
    pub fn from_regular_account(account_id: AccountId32) -> Self {
        Self {
            account_id,
            address_type: AddressType::Regular,
            app_agent_id: None,
            ta_id: None,
            address_name: None,
        }
    }

    /// Helper function to create a new `BlockchainAddressInfo` with the given account_id, and app_agent_id.
    /// The `address_type` will be set to `AddressType::AppAgent`, and other optional fields will be set to `None`.
    pub fn from_app_agent_account(account_id: AccountId32, app_agent_id: AppAgentId) -> Self {
        Self {
            account_id,
            address_type: AddressType::AppAgent,
            app_agent_id: Some(app_agent_id),
            ta_id: None,
            address_name: None,
        }
    }

    /// Helper function to create a new `BlockchainAddressInfo` with the given account_id, ta_id and app_agent_id.
    /// The `address_type` will be set to `AddressType::Transactional`, and other optional fields will be set to `None`.
    pub fn from_ta_account(
        account_id: AccountId32,
        app_agent_id: AppAgentId,
        ta_id: TransactionalId,
    ) -> Self {
        Self {
            account_id,
            address_type: AddressType::Transactional,
            app_agent_id: Some(app_agent_id),
            ta_id: Some(ta_id),
            address_name: None,
        }
    }

    /// Helper function to create a new `BlockchainAddressInfo` with the given account_id, name and app_agent_id.
    /// The `address_type` will be set to `AddressType::Named`, and other optional fields will be set to `None`.
    pub fn from_named_account(
        account_id: AccountId32,
        app_agent_id: AppAgentId,
        name: AddressName,
    ) -> Self {
        Self {
            account_id,
            address_type: AddressType::Named,
            app_agent_id: Some(app_agent_id),
            ta_id: None,
            address_name: Some(name),
        }
    }

    /// Returns true if the address is a keyless address, false otherwise.
    pub fn is_keyless(&self) -> bool {
        self.address_type != AddressType::Regular
    }
}

/// Internal enum to carry the type and IDs of account
pub(super) enum BlockchainAccountIds {
    Regular,
    AppAgent(AppAgentId),
    Transactional((AppAgentId, TransactionalId)),
    Named((AppAgentId, AddressName)),
}

#[cfg(test)]
mod tests_address_name {
    use super::*;

    #[test]
    fn test_valid_address_name() {
        let valid_address = b"abcdEF1234";
        assert!(AddressName::new(valid_address).is_ok());
    }

    #[test]
    fn test_invalid_length() {
        let short_address = b"abcdEF123";
        let long_address = b"abcdEF12345";
        assert_eq!(
            AddressName::new(short_address).unwrap_err(),
            "Input must be exactly 10 bytes long"
        );
        assert_eq!(
            AddressName::new(long_address).unwrap_err(),
            "Input must be exactly 10 bytes long"
        );
    }

    #[test]
    fn test_invalid_characters() {
        let invalid_address1 = b"abcdEF@123";
        let invalid_address2 = b"abcdEF*123";
        let invalid_address3 = b"abcdEF/123";
        assert_eq!(
            AddressName::new(invalid_address1).unwrap_err(),
            "Input contains invalid characters"
        );
        assert_eq!(
            AddressName::new(invalid_address2).unwrap_err(),
            "Input contains invalid characters"
        );
        assert_eq!(
            AddressName::new(invalid_address3).unwrap_err(),
            "Input contains invalid characters"
        );
    }

    #[test]
    fn test_edge_cases() {
        let valid_address1 = b"0000000000";
        let valid_address2 = b"##########";
        let valid_address3 = b"abcdEFghij";
        let valid_address4 = b"1234567890";

        assert!(AddressName::new(valid_address1).is_ok());
        assert!(AddressName::new(valid_address2).is_ok());
        assert!(AddressName::new(valid_address3).is_ok());
        assert!(AddressName::new(valid_address4).is_ok());
    }
}
