use crate::types::*;
use parity_scale_codec::alloc::string::ToString;
use scale_info::prelude::string::String;

// Original implementation of "blake2" and "blake2_256":
// https://github.com/paritytech/polkadot-sdk/blob/polkadot-stable2407-2/substrate/primitives/crypto/hashing/src/lib.rs

#[inline(always)]
fn blake2<const N: usize>(data: &[u8]) -> [u8; N] {
    blake2b_simd::Params::new()
        .hash_length(N)
        .hash(data)
        .as_bytes()
        .try_into()
        .expect("slice is always the necessary length")
}

/// Do a Blake2 256-bit hash and return result.
pub(crate) fn blake2_256(data: &[u8]) -> [u8; 32] {
    blake2(data)
}

/// Builds a keyless account by concatenating open part of the account with the checksum.
///
/// # Arguments
///
/// `open_part` - The open part of the account.
///
/// # Returns
///
/// Returns the built keyless account as an `AccountId32`
fn _encode_account(open_part: &[u8]) -> AccountId32
where
    AccountId32: From<[u8; 32]>,
{
    let mut checksum = blake2_256(open_part);
    checksum[0..open_part.len()].clone_from_slice(open_part);
    return AccountId32::from(checksum);
}

/// Builds a keyless account for the specified `AppAgentId`.
///
/// ********************************
/// The structure of an AppAgent account:
/// [0..=3] -> AppAgentId
/// [4..4] -> Address type identifier (1)
/// [5..=31] -> Checksum
/// ********************************
///
/// # Arguments
///
/// * `app_agent_id` - The `AppAgentId` used to build the keyless account.
///
/// # Returns
///
/// Returns the built keyless account as an `AccountId32`
pub fn encode_app_agent_account(app_agent_id: &AppAgentId) -> AccountId32
where
    AccountId32: From<[u8; 32]>,
{
    let mut open_part: [u8; 5] = [0u8; 5];
    open_part[0..4].copy_from_slice(&app_agent_id.to_le_bytes());
    open_part[4] = APP_AGENT_ADDRESS_IDENTIFIER;

    return _encode_account(&open_part);
}

/// Builds a keyless account for the specified `AppAgentId` and `TransactionalId`.
///
/// ********************************
/// The structure of a Transactional account:
/// [0..=3] -> AppAgentId
/// [4..4] -> Address type identifier (2)
/// [5..=8] -> TransactionalId
/// [9..=31] -> Checksum
/// ********************************
///
/// # Arguments
///
/// * `app_agent_id` - The `AppAgentId` used to build the keyless account.
/// * `ta_id` - The `TransactionalId` used to build the keyless account.
///
/// # Returns
///
/// Returns the built keyless account as an `AccountId32`
pub fn encode_transactional_account(
    app_agent_id: &AppAgentId,
    ta_id: &TransactionalId,
) -> AccountId32
where
    AppAgentId: Into<u32>,
    TransactionalId: Into<u32>,
    AccountId32: From<[u8; 32]>,
{
    let mut open_part: [u8; 9] = [0u8; 9];
    open_part[0..4].copy_from_slice(&app_agent_id.to_le_bytes());
    open_part[4] = TRANSACTIONAL_ADDRESS_IDENTIFIER;
    open_part[5..9].copy_from_slice(&ta_id.to_le_bytes());

    return _encode_account(&open_part);
}

/// Builds a keyless account for the specified `AppAgentId` and `Name`.
///
/// ********************************
/// The structure of a Named account:
/// [0..=3] -> AppAgentId
/// [4..4] -> Address type identifier (3)
/// [5..=14] -> Address name
/// [15..=31] -> Checksum
/// ********************************
///
/// # Arguments
///
/// * `app_agent_id` - The `AppAgentId` used to build the keyless account.
/// * `name` - The `AccountId32` used to build the keyless account.
///
/// # Returns
///
/// Returns a `Result` containing the built keyless account as an `AccountId32` if successful,
/// or a `String` if an error occurs during the building process.
pub fn encode_named_account(app_agent_id: &AppAgentId, name: &AddressName) -> AccountId32
where
    AppAgentId: Into<u32>,
    AccountId32: From<[u8; 32]>,
{
    let mut open_part: [u8; 15] = [0u8; 15];
    open_part[0..4].copy_from_slice(&app_agent_id.to_le_bytes());
    open_part[4] = NAMED_ADDRESS_IDENTIFIER;
    open_part[5..15].copy_from_slice(name.as_bytes());

    return _encode_account(&open_part);
}

/// Decodes and verifies the correctness of a keyless account.
/// Provides type of account and keyless IDs.
fn decode_account_ids(account: &AccountId32) -> BlockchainAccountIds
where
    AccountId32: From<[u8; 32]>,
{
    let account_bytes: &[u8; 32] = account.as_ref();

    let type_identifier: AddressIdentifierType = account_bytes[4];
    match type_identifier {
        APP_AGENT_ADDRESS_IDENTIFIER => {
            // Verify checksum
            let open_part: &[u8; 5] = &account_bytes[0..5].try_into().unwrap();
            let checksum: &[u8; 27] = &account_bytes[5..32].try_into().unwrap();
            let calculated_checksum: [u8; 32] = blake2_256(open_part);
            let calculated_checksum_trimmed: &[u8; 27] =
                (&calculated_checksum[5..32]).try_into().unwrap();

            if checksum == calculated_checksum_trimmed {
                // Extract and decode IDs
                let app_agent_id_bytes = account_bytes[0..4].try_into().unwrap();
                let app_agent_id: AppAgentId = u32::from_le_bytes(app_agent_id_bytes);

                return BlockchainAccountIds::AppAgent(app_agent_id);
            };
        }
        TRANSACTIONAL_ADDRESS_IDENTIFIER => {
            // Verify checksum
            let open_part: &[u8; 9] = &account_bytes[0..9].try_into().unwrap();
            let checksum: &[u8; 23] = &account_bytes[9..32].try_into().unwrap();
            let calculated_checksum: [u8; 32] = blake2_256(open_part);
            let calculated_checksum_trimmed: &[u8; 23] =
                (&calculated_checksum[9..32]).try_into().unwrap();

            if checksum == calculated_checksum_trimmed {
                // Extract and decode IDs
                let app_agent_id_bytes = account_bytes[0..4].try_into().unwrap();
                let app_agent_id: AppAgentId = u32::from_le_bytes(app_agent_id_bytes);
                // 5 = appagent_id_size + address_identifier_size
                // 9 = appagent_id_size + address_identifier_size + ta_id_size
                let ta_id_bytes = account_bytes[5..9].try_into().unwrap();
                let ta_id: TransactionalId = u32::from_le_bytes(ta_id_bytes);

                return BlockchainAccountIds::Transactional((app_agent_id, ta_id));
            };
        }
        NAMED_ADDRESS_IDENTIFIER => {
            // Verify checksum
            let open_part: &[u8; 15] = &account_bytes[0..15].try_into().unwrap();
            let checksum: &[u8; 17] = &account_bytes[15..32].try_into().unwrap();
            let calculated_checksum: [u8; 32] = blake2_256(open_part);
            let calculated_checksum_trimmed: &[u8; 17] =
                (&calculated_checksum[15..32]).try_into().unwrap();

            if checksum == calculated_checksum_trimmed {
                // Extract and decode IDs
                let app_agent_id_bytes = account_bytes[..4].try_into().unwrap();
                let app_agent_id: AppAgentId = u32::from_le_bytes(app_agent_id_bytes);
                let address_name = account_bytes[5..15].try_into().unwrap();

                return BlockchainAccountIds::Named((app_agent_id, address_name));
            };
        }
        _ => {}
    }

    return BlockchainAccountIds::Regular;
}

/// Decodes and verifies the correctness of a keyless account.
///
/// # Arguments
///
/// * `account` - The keyless account to be decoded and verified.
///
/// # Returns
///
/// Returns a `Result` containing the decoded open part of the keyless account as a tuple of
/// `(AddressType, AppAgentId, Option<TransactionalId>, Option<AddressName>)`
/// if the decoding and verification are successful,
/// or a `String` if an error occurs during decoding or verification.
pub fn decode_account(account: AccountId32) -> BlockchainAddressInfo {
    match decode_account_ids(&account) {
        BlockchainAccountIds::Regular => BlockchainAddressInfo::from_regular_account(account),
        BlockchainAccountIds::AppAgent(app_agent_id) => {
            BlockchainAddressInfo::from_app_agent_account(account, app_agent_id)
        }
        BlockchainAccountIds::Transactional((app_agent_id, ta_id)) => {
            BlockchainAddressInfo::from_ta_account(account, app_agent_id, ta_id)
        }
        BlockchainAccountIds::Named((app_agent_id, address_name)) => {
            BlockchainAddressInfo::from_named_account(account, app_agent_id, address_name)
        }
    }
}

/// Decodes the provided keyless account and retrieves the corresponding `AppAgentId`.
///
/// # Arguments
///
/// * `account` - The keyless account to be decoded.
///
/// # Returns
///
/// Returns a `Result` containing the `AppAgentId` decoded from the keyless account if successful,
/// or a `String` if an error occurs during the decoding process.
pub fn decode_app_agent_account(account: &AccountId32) -> Result<AppAgentId, String> {
    match decode_account_ids(account) {
        BlockchainAccountIds::AppAgent(app_agent_id) => Ok(app_agent_id),
        _ => Err("Provided account is not an AppAgent keyless account".to_string()),
    }
}

/// Decodes the provided keyless account and retrieves the `AppAgentId` and
/// `TransactionalId`.
///
/// # Arguments
///
/// * `account` - The keyless account to be decoded.
///
/// # Returns
///
/// Returns a `Result` containing the decoded `(AppAgentId, TransactionalId)` if successful,
/// or a `String` if an error occurs during the decoding process.
pub fn decode_transactional_account(
    account: &AccountId32,
) -> Result<(AppAgentId, TransactionalId), String> {
    match decode_account_ids(account) {
        BlockchainAccountIds::Transactional((app_agent_id, ta_id)) => Ok((app_agent_id, ta_id)),
        _ => Err("Provided account is not a Transactional keyless account".to_string()),
    }
}

/// Decodes the provided keyless account and retrieves the `AppAgentId` and
/// `AddressName`.
///
/// # Arguments
///
/// * `account` - The keyless account to be decoded.
///
/// # Returns
///
/// Returns a `Result` containing the decoded `(AppAgentId, AddressName)` if successful,
/// or a `String` if an error occurs during the decoding process.
pub fn decode_named_account(account: &AccountId32) -> Result<(AppAgentId, AddressName), String> {
    match decode_account_ids(account) {
        BlockchainAccountIds::Named((app_agent_id, address_name)) => {
            Ok((app_agent_id, address_name))
        }
        _ => Err("Provided account is not a Named keyless account".to_string()),
    }
}

/// Checks if provided account is a keyless account.
///
/// # Arguments
///
/// * `account` - The keyless account to be checked.
///
/// # Returns
///
/// Returns a `true` if provided accouont is a keyless one, otherwise `false`.
pub fn is_keyless_account(account: &AccountId32) -> bool {
    match decode_account_ids(account) {
        BlockchainAccountIds::Regular => false,
        BlockchainAccountIds::AppAgent(_)
        | BlockchainAccountIds::Transactional(_)
        | BlockchainAccountIds::Named(_) => true,
    }
}
