use crate::types::*;

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

/// Builds a keyless `AccountId32` from its open part.
///
/// The 32-byte account is constructed as follows:
/// 1. Compute `hash = blake2_256(open_part)` (32 bytes).
/// 2. Overwrite `hash[0..open_part.len()]` with `open_part`.
///
/// The result is: `open_part || hash[open_part.len()..]`
/// where `hash[open_part.len()..]` is the trailing portion of the hash, used as the checksum.
fn encode_account(open_part: &[u8]) -> AccountId32 {
    let mut checksum = blake2_256(open_part);
    checksum[0..open_part.len()].copy_from_slice(open_part);
    AccountId32::from(checksum)
}

/// Derives the root keyless account for the given App Agent.
///
/// Byte layout of the 32-byte result:
/// ```text
/// [0..=3]  AppAgentId (little-endian u32)
/// [4]      Account type identifier = APP_AGENT_ACCOUNT_IDENTIFIER (1)
/// [5..=31] blake2_256(open_part)[5..=31]  — checksum
/// ```
pub fn encode_app_agent_account(app_agent_id: &AppAgentId) -> AccountId32 {
    let mut open_part: [u8; 5] = [0u8; 5];
    open_part[0..4].copy_from_slice(&app_agent_id.to_le_bytes());
    open_part[4] = APP_AGENT_ACCOUNT_IDENTIFIER;

    encode_account(&open_part)
}

/// Derives the transactional keyless account for the given App Agent and transactional ID.
///
/// Byte layout of the 32-byte result:
/// ```text
/// [0..=3]  AppAgentId (little-endian u32)
/// [4]      Account type identifier = TRANSACTIONAL_ACCOUNT_IDENTIFIER (2)
/// [5..=8]  TransactionalId (little-endian u32)
/// [9..=31] blake2_256(open_part)[9..=31]  — checksum
/// ```
pub fn encode_transactional_account(
    app_agent_id: &AppAgentId,
    ta_id: &TransactionalId,
) -> AccountId32 {
    let mut open_part: [u8; 9] = [0u8; 9];
    open_part[0..4].copy_from_slice(&app_agent_id.to_le_bytes());
    open_part[4] = TRANSACTIONAL_ACCOUNT_IDENTIFIER;
    open_part[5..9].copy_from_slice(&ta_id.to_le_bytes());

    encode_account(&open_part)
}

/// Derives the named keyless account for the given App Agent and account name.
///
/// Byte layout of the 32-byte result:
/// ```text
/// [0..=3]   AppAgentId (little-endian u32)
/// [4]       Account type identifier = NAMED_ACCOUNT_IDENTIFIER (3)
/// [5..=14]  AccountName (10 bytes, restricted charset: 0-9 a-z A-Z - #)
/// [15..=31] blake2_256(open_part)[15..=31]  — checksum
/// ```
pub fn encode_named_account(app_agent_id: &AppAgentId, name: &AccountName) -> AccountId32 {
    let mut open_part: [u8; 15] = [0u8; 15];
    open_part[0..4].copy_from_slice(&app_agent_id.to_le_bytes());
    open_part[4] = NAMED_ACCOUNT_IDENTIFIER;
    open_part[5..15].copy_from_slice(name.as_bytes());

    encode_account(&open_part)
}

/// Classifies an `AccountId32` and extracts any embedded keyless identifiers.
///
/// Reads the type discriminant at byte [4], re-derives the expected checksum for the
/// corresponding open-part length, and compares it to the stored bytes. If the checksum
/// matches, the account is decoded into the appropriate [`BlockchainAccountInfo`] variant.
/// Any account whose byte [4] is not a known discriminant, or whose checksum does not match,
/// is returned as [`BlockchainAccountInfo::Regular`].
pub fn decode_account_info(account: AccountId32) -> BlockchainAccountInfo {
    let account_bytes: &[u8; 32] = account.as_ref();

    let type_identifier: AccountIdentifierType = account_bytes[4];
    match type_identifier {
        APP_AGENT_ACCOUNT_IDENTIFIER => {
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

                return BlockchainAccountInfo::AppAgent {
                    account_id: account,
                    app_agent_id,
                };
            }
        }
        TRANSACTIONAL_ACCOUNT_IDENTIFIER => {
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
                // 5 = appagent_id_size + account_identifier_size
                // 9 = appagent_id_size + account_identifier_size + ta_id_size
                let ta_id_bytes = account_bytes[5..9].try_into().unwrap();
                let ta_id: TransactionalId = u32::from_le_bytes(ta_id_bytes);

                return BlockchainAccountInfo::Transactional {
                    account_id: account,
                    app_agent_id,
                    ta_id,
                };
            }
        }
        NAMED_ACCOUNT_IDENTIFIER => {
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
                let account_name = account_bytes[5..15].try_into().unwrap();

                return BlockchainAccountInfo::Named {
                    account_id: account,
                    app_agent_id,
                    account_name,
                };
            }
        }
        _ => {}
    }

    BlockchainAccountInfo::Regular {
        account_id: account,
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
/// Returns `Ok(AppAgentId)` if the account passes the App Agent checksum verification, or
/// `Err(KeylessDecodeError::NotAnAppAgentAccount)` otherwise.
pub fn decode_app_agent_account(account: AccountId32) -> Result<AppAgentId, KeylessDecodeError> {
    match decode_account_info(account) {
        BlockchainAccountInfo::AppAgent { app_agent_id, .. } => Ok(app_agent_id),
        _ => Err(KeylessDecodeError::NotAnAppAgentAccount),
    }
}

/// Decodes the provided keyless account and retrieves the `AppAgentId` and `TransactionalId`.
///
/// # Arguments
///
/// * `account` - The keyless account to be decoded.
///
/// # Returns
///
/// Returns `Ok((AppAgentId, TransactionalId))` if the account passes the Transactional checksum
/// verification, or `Err(KeylessDecodeError::NotATransactionalAccount)` otherwise.
pub fn decode_transactional_account(
    account: AccountId32,
) -> Result<(AppAgentId, TransactionalId), KeylessDecodeError> {
    match decode_account_info(account) {
        BlockchainAccountInfo::Transactional {
            app_agent_id,
            ta_id,
            ..
        } => Ok((app_agent_id, ta_id)),
        _ => Err(KeylessDecodeError::NotATransactionalAccount),
    }
}

/// Decodes the provided keyless account and retrieves the `AppAgentId` and `AccountName`.
///
/// # Arguments
///
/// * `account` - The keyless account to be decoded.
///
/// # Returns
///
/// Returns `Ok((AppAgentId, AccountName))` if the account passes the Named checksum verification,
/// or `Err(KeylessDecodeError::NotANamedAccount)` otherwise.
pub fn decode_named_account(
    account: AccountId32,
) -> Result<(AppAgentId, AccountName), KeylessDecodeError> {
    match decode_account_info(account) {
        BlockchainAccountInfo::Named {
            app_agent_id,
            account_name,
            ..
        } => Ok((app_agent_id, account_name)),
        _ => Err(KeylessDecodeError::NotANamedAccount),
    }
}

/// Returns `true` if the account passes the keyless checksum verification (App Agent,
/// Transactional, or Named). Returns `false` for accounts whose checksum does not match or whose
/// type discriminant is unknown. Internally performs a full Blake2-256 hash computation.
pub fn is_keyless_account(account: AccountId32) -> bool {
    match decode_account_info(account) {
        BlockchainAccountInfo::Regular { .. } => false,
        BlockchainAccountInfo::AppAgent { .. }
        | BlockchainAccountInfo::Transactional { .. }
        | BlockchainAccountInfo::Named { .. } => true,
    }
}
