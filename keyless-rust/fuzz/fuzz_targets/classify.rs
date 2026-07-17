//! Fuzzes the classification path (`decode_account_info`), the entry point every consumer
//! of an untrusted 32-byte account id goes through. It must never panic on arbitrary
//! bytes, and its result must be internally consistent with the rest of the API:
//!
//! * the reported `account_id` is always the input;
//! * `is_keyless_account` agrees with the classification;
//! * a keyless classification is *reversible*: re-encoding from the decoded identifiers
//!   reproduces the input bytes exactly, and the matching typed decoder returns those same
//!   identifiers (so the classifier can never invent identifiers the encoder would not
//!   derive the same account from);
//! * the two typed decoders for the *other* keyless types — and all three on a `Regular`
//!   account — fail with `WrongAccountType { found: <the classification> }`;
//! * a keyless account is checksum-fragile: flipping any single bit of it demotes it to
//!   `Regular` (there is no second byte string with the same classification and
//!   identifiers reachable by a one-byte edit).
//!
//! Inputs are consumed as raw bytes, so the committed seeds are literal 32-byte account
//! ids (see `fuzz/seeds/README.md`); anything shorter is skipped, and the leading 32 bytes
//! of a longer input are used.
#![no_main]

use libfuzzer_sys::fuzz_target;
use trevokeyless::*;

/// Every typed decoder, keyed by the keyless type it decodes.
fn typed_decode_type(
    account: &AccountId32,
    decode_as: AccountType,
) -> Result<AccountType, AccountType> {
    let error = match decode_as {
        AccountType::AppAgent => decode_app_agent_account(account).err(),
        AccountType::Transactional => decode_transactional_account(account).err(),
        AccountType::Named => decode_named_account(account).err(),
        AccountType::Regular => unreachable!("there is no Regular decoder"),
    };
    match error {
        None => Ok(decode_as),
        Some(KeylessDecodeError::WrongAccountType { expected, found }) => {
            assert_eq!(
                expected, decode_as,
                "the error must name the called decoder"
            );
            Err(found)
        }
    }
}

/// Asserts that the typed decoders of every keyless type other than `actual` reject
/// `account` with `found == actual`.
fn assert_other_decoders_reject(account: &AccountId32, actual: AccountType) {
    for decode_as in [
        AccountType::AppAgent,
        AccountType::Transactional,
        AccountType::Named,
    ] {
        if decode_as == actual {
            continue;
        }
        assert_eq!(
            typed_decode_type(account, decode_as),
            Err(actual),
            "the {decode_as} decoder must report the actual classification"
        );
    }
}

fuzz_target!(|data: &[u8]| {
    if data.len() < ACCOUNT_ID_LENGTH {
        return;
    }
    let mut bytes = [0u8; ACCOUNT_ID_LENGTH];
    bytes.copy_from_slice(&data[..ACCOUNT_ID_LENGTH]);
    let account = AccountId32::from(bytes);

    let info = decode_account_info(&account);
    assert_eq!(
        info.account_id(),
        account,
        "the classification must carry the input bytes"
    );
    assert_eq!(
        is_keyless_account(&account),
        info.is_keyless_account(),
        "is_keyless_account must agree with decode_account_info"
    );
    assert_eq!(
        info.is_keyless_account(),
        info.account_type() != AccountType::Regular,
        "only Regular is the non-keyless classification"
    );

    // Re-encoding from the decoded identifiers must reproduce the input bytes exactly, and
    // the typed decoder for the account's own type must return the same identifiers.
    let actual = info.account_type();
    match info {
        BlockchainAccountInfo::Regular { .. } => {
            // No decoder claims a regular account.
            assert_other_decoders_reject(&account, AccountType::Regular);
            return;
        }
        BlockchainAccountInfo::AppAgent { app_agent_id, .. } => {
            assert_eq!(encode_app_agent_account(app_agent_id), account);
            assert_eq!(decode_app_agent_account(&account), Ok(app_agent_id));
        }
        BlockchainAccountInfo::Transactional {
            app_agent_id,
            ta_id,
            ..
        } => {
            let identifiers = TransactionalAccountIdentifiers::new(app_agent_id, ta_id);
            assert_eq!(encode_transactional_account(identifiers), account);
            assert_eq!(decode_transactional_account(&account), Ok(identifiers));
        }
        BlockchainAccountInfo::Named {
            app_agent_id,
            account_name,
            ..
        } => {
            let identifiers = NamedAccountIdentifiers::new(app_agent_id, account_name);
            assert_eq!(encode_named_account(identifiers), account);
            assert_eq!(decode_named_account(&account), Ok(identifiers));
        }
    }

    // A keyless account: the other typed decoders must report it as such, ...
    assert_other_decoders_reject(&account, actual);

    // ... and its checksum must cover every byte — flipping any single bit demotes it to
    // Regular, so no one-byte edit yields another account of the same type (let alone one
    // carrying the same identifiers).
    for position in 0..ACCOUNT_ID_LENGTH {
        for shift in 0..8 {
            let bit = 1u8 << shift;
            let mut corrupted = bytes;
            corrupted[position] ^= bit;
            let corrupted = decode_account_info(&AccountId32::from(corrupted));
            assert_eq!(
                corrupted.account_type(),
                AccountType::Regular,
                "flipping a bit of byte {position} must break the keyless checksum"
            );
        }
    }
});
