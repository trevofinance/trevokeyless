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

/// Do a Blake2b 256-bit hash and return result.
// pub(crate) so sibling test modules can verify hardcoded checksums.
pub(crate) fn blake2_256(data: &[u8]) -> [u8; 32] {
    blake2(data)
}

/// Builds a keyless `AccountId32` from its `N`-byte open part.
///
/// The 32-byte account is constructed as follows:
/// 1. Compute `account_bytes = blake2_256(open_part)` (32 bytes).
/// 2. Overwrite `account_bytes[0..N]` with `open_part`.
///
/// The result is: `open_part || account_bytes[N..]`
/// where `account_bytes[N..]` is the trailing portion of the hash, used as the checksum.
///
/// `N <= 32` is enforced at compile time.
fn encode_account<const N: usize>(open_part: &[u8; N]) -> AccountId32 {
    const {
        assert!(N <= 32, "open part cannot exceed the 32-byte AccountId32");
    }
    let mut account_bytes = blake2_256(open_part);
    account_bytes[0..N].copy_from_slice(open_part);
    AccountId32::from(account_bytes)
}

/// Derives the root keyless account for the given App Agent.
///
/// Byte layout of the 32-byte result:
/// ```text
/// [0..=3]  AppAgentId (little-endian u32)
/// [4]      Account type identifier = APP_AGENT_ACCOUNT_IDENTIFIER (1)
/// [5..=31] blake2_256(open_part)[5..=31]  — checksum
/// ```
#[must_use]
pub fn encode_app_agent_account(app_agent_id: AppAgentId) -> AccountId32 {
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
#[must_use]
pub fn encode_transactional_account(
    app_agent_id: AppAgentId,
    ta_id: TransactionalId,
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
#[must_use]
pub fn encode_named_account(app_agent_id: AppAgentId, name: AccountName) -> AccountId32 {
    let mut open_part: [u8; 15] = [0u8; 15];
    open_part[0..4].copy_from_slice(&app_agent_id.to_le_bytes());
    open_part[4] = NAMED_ACCOUNT_IDENTIFIER;
    open_part[5..15].copy_from_slice(name.as_bytes());

    encode_account(&open_part)
}

/// Returns `true` iff `account_bytes` carries a valid keyless checksum for an open part of
/// `N` bytes, i.e. the trailing `32 - N` bytes equal the trailing bytes of `blake2_256` over
/// the leading `N` bytes.
fn open_part_valid<const N: usize>(account_bytes: &[u8; 32]) -> bool {
    blake2_256(&account_bytes[..N])[N..] == account_bytes[N..]
}

/// Reads a little-endian `u32` from `bytes[OFFSET..OFFSET + 4]`.
///
/// `OFFSET + 4 <= 32` is enforced at compile time.
fn le_u32_at<const OFFSET: usize>(bytes: &[u8; 32]) -> u32 {
    const {
        assert!(
            OFFSET + 4 <= 32,
            "u32 read must lie within the account bytes"
        );
    }
    let mut le_bytes = [0u8; 4];
    le_bytes.copy_from_slice(&bytes[OFFSET..OFFSET + 4]);
    u32::from_le_bytes(le_bytes)
}

/// Classifies an `AccountId32` and extracts any embedded keyless identifiers.
///
/// Reads the type discriminant at byte `[4]`, re-derives the expected checksum for the
/// corresponding open-part length, and compares it to the stored bytes. If the checksum
/// matches, the account is decoded into the appropriate [`BlockchainAccountInfo`] variant.
/// Any account whose byte `[4]` is not a known discriminant, or whose checksum does not
/// match, is returned as [`BlockchainAccountInfo::Regular`].
///
/// For Named accounts the name bytes must additionally pass charset validation
/// (`0-9 a-z A-Z - #`). Anyone can construct an account carrying a valid checksum over
/// out-of-charset name bytes (no private key is needed); such an account is classified as
/// [`BlockchainAccountInfo::Regular`].
#[must_use]
pub fn decode_account_info(account: &AccountId32) -> BlockchainAccountInfo {
    let account_bytes: &[u8; 32] = account.as_ref();

    let type_identifier: AccountIdentifierType = account_bytes[4];
    match type_identifier {
        APP_AGENT_ACCOUNT_IDENTIFIER if open_part_valid::<5>(account_bytes) => {
            BlockchainAccountInfo::AppAgent {
                account_id: *account,
                app_agent_id: le_u32_at::<0>(account_bytes),
            }
        }
        TRANSACTIONAL_ACCOUNT_IDENTIFIER if open_part_valid::<9>(account_bytes) => {
            BlockchainAccountInfo::Transactional {
                account_id: *account,
                app_agent_id: le_u32_at::<0>(account_bytes),
                ta_id: le_u32_at::<5>(account_bytes),
            }
        }
        NAMED_ACCOUNT_IDENTIFIER if open_part_valid::<15>(account_bytes) => {
            // The name bytes must pass charset validation. If they don't (e.g. a crafted
            // account with a valid checksum over out-of-charset name bytes), fall back to
            // Regular rather than panic.
            match AccountName::try_from(&account_bytes[5..15]) {
                Ok(account_name) => BlockchainAccountInfo::Named {
                    account_id: *account,
                    app_agent_id: le_u32_at::<0>(account_bytes),
                    account_name,
                },
                Err(_) => BlockchainAccountInfo::Regular {
                    account_id: *account,
                },
            }
        }
        _ => BlockchainAccountInfo::Regular {
            account_id: *account,
        },
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
/// `Err(KeylessDecodeError::NotAnAppAgentAccount { found })` carrying the account's actual
/// classification otherwise.
pub fn decode_app_agent_account(account: &AccountId32) -> Result<AppAgentId, KeylessDecodeError> {
    match decode_account_info(account) {
        BlockchainAccountInfo::AppAgent { app_agent_id, .. } => Ok(app_agent_id),
        other => Err(KeylessDecodeError::NotAnAppAgentAccount {
            found: other.account_type(),
        }),
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
/// verification, or `Err(KeylessDecodeError::NotATransactionalAccount { found })` carrying the
/// account's actual classification otherwise.
pub fn decode_transactional_account(
    account: &AccountId32,
) -> Result<(AppAgentId, TransactionalId), KeylessDecodeError> {
    match decode_account_info(account) {
        BlockchainAccountInfo::Transactional {
            app_agent_id,
            ta_id,
            ..
        } => Ok((app_agent_id, ta_id)),
        other => Err(KeylessDecodeError::NotATransactionalAccount {
            found: other.account_type(),
        }),
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
/// or `Err(KeylessDecodeError::NotANamedAccount { found })` carrying the account's actual
/// classification otherwise.
pub fn decode_named_account(
    account: &AccountId32,
) -> Result<(AppAgentId, AccountName), KeylessDecodeError> {
    match decode_account_info(account) {
        BlockchainAccountInfo::Named {
            app_agent_id,
            account_name,
            ..
        } => Ok((app_agent_id, account_name)),
        other => Err(KeylessDecodeError::NotANamedAccount {
            found: other.account_type(),
        }),
    }
}

/// Returns `true` if the account passes the keyless checksum verification (App Agent,
/// Transactional, or Named). Returns `false` for accounts whose checksum does not match or whose
/// type discriminant is unknown. Internally performs a full Blake2b-256 hash computation.
#[must_use]
pub fn is_keyless_account(account: &AccountId32) -> bool {
    decode_account_info(account).is_keyless_account()
}
