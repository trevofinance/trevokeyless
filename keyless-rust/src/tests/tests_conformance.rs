//! Verifies the implementation against the committed cross-language conformance fixture
//! (`conformance/vectors.json` at the repository root).
//!
//! The fixture is generated from this crate by `examples/gen_conformance.rs` — the Rust
//! implementation is the source of truth — and is also consumed by the Python and
//! TypeScript test suites. This test pins the Rust implementation to the committed
//! fixture, so an accidental semantic change to the encoding fails here first instead of
//! silently regenerating different vectors for the other languages.

use crate::keyless::*;
use crate::types::*;

/// Every group of the fixture with its exact vector count, pinned so an accidentally
/// dropped (or newly generated but unconsumed) group — and, just as importantly, a
/// generator edit that silently *shrinks* a group — fails here instead of passing
/// silently: a regenerated fixture is self-consistent by construction, so the counts are
/// the only thing standing between "the generator lost 396 classify vectors" and a green
/// test run.
///
/// Update these numbers deliberately when adding (or intentionally removing) vectors.
const FIXTURE_GROUPS: [(&str, usize); 18] = [
    ("app_agent", 63),
    ("transactional", 64),
    ("named", 67),
    ("classify", 401),
    ("ss58_formats", 101),
    ("ss58_decode_only", 5),
    ("ss58_invalid", 37),
    ("named_invalid_encode", 16),
    ("hex_invalid", 13),
    ("hex_valid", 12),
    ("hex_not_an_address", 4),
    ("address_not_an_account_id", 4),
    ("format_invalid_encode", 4),
    ("format_invalid_decode", 2),
    ("format_invalid_host", 5),
    ("u32_invalid_encode", 7),
    ("constants", 8),
    ("type_mismatch", 9),
];

/// The fixture lives at the repository root, outside the crate directory, so it is absent from
/// the published `.crate` tarball. This marker — a sibling of the fixture itself — tells the two
/// situations apart: present means we are testing inside the repository, where a missing or
/// unreadable fixture is a broken path that must fail loudly rather than turn the whole
/// cross-language pin into a green no-op; absent means we are testing a published or vendored
/// crate, where there is simply nothing to verify against and skipping is correct.
const FIXTURE_DIRECTORY: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../conformance");

fn fixture() -> Option<serde_json::Value> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../conformance/vectors.json");

    // Skipping is the default for a published crate (nothing to verify against) and an
    // explicit opt-in anywhere else; `cargo test` on the published crate must not hard-fail
    // on a file that was never shipped.
    if !std::path::Path::new(FIXTURE_DIRECTORY).is_dir()
        || std::env::var_os("TREVOKEYLESS_CONFORMANCE_SKIP").is_some()
    {
        // Rust has no first-class skip, so a skipped run shows as a *passing* test.
        // That is fine for a published crate, but in CI — where the repository fixture
        // is always checked out — a skip can only mean a broken path, and it must not
        // read as green. check-rust.yml exports TREVOKEYLESS_CONFORMANCE_REQUIRE to
        // close that hole.
        assert!(
            std::env::var_os("TREVOKEYLESS_CONFORMANCE_REQUIRE").is_none(),
            "conformance fixture not found, but TREVOKEYLESS_CONFORMANCE_REQUIRE is set: \
             refusing to skip the cross-language pin"
        );
        return None;
    }

    let data = match std::fs::read_to_string(path) {
        Ok(data) => data,
        Err(error) => panic!(
            "conformance fixture not readable at {path}: {error} \
             (the repository's conformance/ directory is present, so this is a broken path, \
             not a published-crate checkout)"
        ),
    };
    let vectors: serde_json::Value =
        serde_json::from_str(&data).expect("fixture must be valid JSON");

    // Pin the exact group set and each group's exact size: an accidentally emptied — or
    // merely thinned — group would otherwise pass every consuming suite silently.
    let mut groups: Vec<&str> = vectors
        .as_object()
        .expect("fixture must be a JSON object")
        .keys()
        .map(String::as_str)
        .collect();
    groups.sort_unstable();
    let mut expected: Vec<&str> = FIXTURE_GROUPS.iter().map(|(name, _)| *name).collect();
    expected.sort_unstable();
    assert_eq!(groups, expected, "fixture groups must match the pinned set");
    for (group, count) in FIXTURE_GROUPS {
        assert_eq!(
            vectors[group].as_array().unwrap().len(),
            count,
            "fixture group {group:?} must hold exactly {count} vectors \
             (update FIXTURE_GROUPS deliberately when adding vectors)"
        );
    }
    Some(vectors)
}

fn account_from_hex(hex: &str) -> AccountId32 {
    let bytes: [u8; 32] = array_bytes::Dehexify::dehexify(hex).unwrap();
    AccountId32::from(bytes)
}

/// Lowercase hex of the raw account bytes, matching the fixture's `hex` fields
/// (which are formatted from the bytes directly, independent of the crate's
/// `Display` rendering).
fn account_hex(account: &AccountId32) -> String {
    let bytes = account.as_bytes();
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// The fixture name of an `AccountNameError`: the Rust variant name, which the Python and
/// TypeScript name validation must reproduce (UTF-8 byte length first, then the charset).
fn account_name_error_kind(error: &AccountNameError) -> &'static str {
    match error {
        AccountNameError::InvalidLength => "InvalidLength",
        AccountNameError::InvalidCharacter => "InvalidCharacter",
    }
}

/// The fixture name of an `Ss58Error`: the Rust variant name, matched by the
/// `Ss58Error.kind` of the TypeScript and Python implementations.
#[cfg(feature = "ss58")]
fn ss58_error_kind(error: &crate::Ss58Error) -> &'static str {
    match error {
        crate::Ss58Error::InvalidBase58 => "InvalidBase58",
        crate::Ss58Error::InvalidLength => "InvalidLength",
        crate::Ss58Error::InvalidChecksum => "InvalidChecksum",
        crate::Ss58Error::InvalidFormat => "InvalidFormat",
        crate::Ss58Error::UnexpectedFormat { .. } => "UnexpectedFormat",
    }
}

fn assert_classification(account: &AccountId32, expected: &serde_json::Value) {
    let info = decode_account_info(account);
    let expected_type = expected["type"].as_str().unwrap();
    assert_eq!(
        format!("{}", info.account_type()),
        expected_type,
        "classification of {account} must match the fixture"
    );
    assert_eq!(
        info.app_agent_id(),
        expected["app_agent_id"]
            .as_u64()
            .map(|v| AppAgentId::new(v as u32)),
        "app_agent_id of {account} must match the fixture"
    );
    assert_eq!(
        info.ta_id(),
        expected["ta_id"]
            .as_u64()
            .map(|v| TransactionalId::new(v as u32)),
        "ta_id of {account} must match the fixture"
    );
    assert_eq!(
        info.account_name().map(|name| name.as_str().to_string()),
        expected["name"].as_str().map(str::to_string),
        "account_name of {account} must match the fixture"
    );
}

#[test]
fn test_conformance_vectors() {
    let Some(vectors) = fixture() else {
        eprintln!("conformance fixture not found; skipping (running from a published crate?)");
        return;
    };

    for v in vectors["app_agent"].as_array().unwrap() {
        let id = AppAgentId::new(v["id"].as_u64().unwrap() as u32);
        let account = encode_app_agent_account(id);
        assert_eq!(account_hex(&account), v["hex"].as_str().unwrap());
        assert_eq!(decode_app_agent_account(&account), Ok(id));
        #[cfg(feature = "ss58")]
        {
            let address = v["ss58"].as_str().unwrap();
            assert_eq!(account.to_ss58check(), address);
            assert_eq!(AccountId32::from_ss58check(address), Ok(account));
            // The unknown-format entry point reports the same account plus the format
            // found in the address — the same pair the Python
            // decode_address_with_format and TypeScript decodeAddressWithFormat pin.
            assert_eq!(
                AccountId32::from_ss58check_with_version(address),
                Ok((account, crate::SS58_FORMAT_TREVO_ASSET_HUB))
            );
        }
    }

    for v in vectors["transactional"].as_array().unwrap() {
        let aa = AppAgentId::new(v["aa"].as_u64().unwrap() as u32);
        let ta = TransactionalId::new(v["ta"].as_u64().unwrap() as u32);
        let identifiers = TransactionalAccountIdentifiers::new(aa, ta);
        let account = encode_transactional_account(identifiers);
        assert_eq!(account_hex(&account), v["hex"].as_str().unwrap());
        assert_eq!(decode_transactional_account(&account), Ok(identifiers));
        #[cfg(feature = "ss58")]
        {
            let address = v["ss58"].as_str().unwrap();
            assert_eq!(account.to_ss58check(), address);
            assert_eq!(AccountId32::from_ss58check(address), Ok(account));
            assert_eq!(
                AccountId32::from_ss58check_with_version(address),
                Ok((account, crate::SS58_FORMAT_TREVO_ASSET_HUB))
            );
        }
    }

    for v in vectors["named"].as_array().unwrap() {
        let aa = AppAgentId::new(v["aa"].as_u64().unwrap() as u32);
        let name = AccountName::try_from(v["name"].as_str().unwrap()).unwrap();
        let identifiers = NamedAccountIdentifiers::new(aa, name);
        let account = encode_named_account(identifiers);
        assert_eq!(account_hex(&account), v["hex"].as_str().unwrap());
        assert_eq!(decode_named_account(&account), Ok(identifiers));
        #[cfg(feature = "ss58")]
        {
            let address = v["ss58"].as_str().unwrap();
            assert_eq!(account.to_ss58check(), address);
            assert_eq!(AccountId32::from_ss58check(address), Ok(account));
            assert_eq!(
                AccountId32::from_ss58check_with_version(address),
                Ok((account, crate::SS58_FORMAT_TREVO_ASSET_HUB))
            );
        }
    }

    for v in vectors["classify"].as_array().unwrap() {
        let account = account_from_hex(v["hex"].as_str().unwrap());
        assert_classification(&account, &v["expected"]);
        assert_eq!(
            is_keyless_account(&account),
            v["expected"]["type"].as_str().unwrap() != "Regular",
            "is_keyless_account of {account} must match the classification"
        );
    }

    #[cfg(feature = "ss58")]
    for v in vectors["ss58_formats"].as_array().unwrap() {
        let account = account_from_hex(v["hex"].as_str().unwrap());
        let format = v["format"].as_u64().unwrap() as u16;
        let address = v["ss58"].as_str().unwrap();
        assert_eq!(
            account.to_ss58check_with_version(format).unwrap(),
            address,
            "SS58 encoding with format {format} must match the fixture"
        );
        assert_eq!(
            AccountId32::from_ss58check_with_version(address).unwrap(),
            (account, format),
            "SS58 decoding of {address} must match the fixture"
        );
        assert_eq!(
            AccountId32::from_ss58check_expecting(address, format),
            Ok(account),
            "SS58 decoding of {address} with the expected format must match the fixture"
        );
        // The classification must be reached identically through the address and through the
        // raw bytes — the same cross-check the TypeScript and Python suites now make on this
        // group.
        assert_eq!(
            is_keyless_account(&AccountId32::from_ss58check_expecting(address, format).unwrap()),
            is_keyless_account(&account),
            "the keyless predicate of {address} must agree with that of its account bytes"
        );
    }

    // Decode-only vectors: the non-canonical two-byte-prefix spelling of formats
    // below 64 — the one documented malleability exception. Every implementation
    // must decode the spelling (to the pinned account id and format) while its
    // encoder keeps producing the canonical form, so re-encoding must NOT
    // reproduce the input spelling.
    #[cfg(feature = "ss58")]
    for v in vectors["ss58_decode_only"].as_array().unwrap() {
        let address = v["ss58"].as_str().unwrap();
        let format = v["format"].as_u64().unwrap() as u16;
        let account = account_from_hex(v["hex"].as_str().unwrap());
        let reason = v["reason"].as_str().unwrap();
        assert_eq!(
            AccountId32::from_ss58check_expecting(address, format),
            Ok(account),
            "{address:?} must decode: {reason}"
        );
        // The non-canonical spelling also decodes without an expected format,
        // reporting the format pinned by the vector.
        assert_eq!(
            AccountId32::from_ss58check_with_version(address),
            Ok((account, format)),
            "{address:?} must decode without an expected format: {reason}"
        );
        // The encoder must produce exactly the pinned canonical spelling — not the
        // decode-only one, and not some third spelling.
        let canonical = v["canonical_ss58"].as_str().unwrap();
        assert_eq!(
            account.to_ss58check_with_version(format).unwrap(),
            canonical,
            "re-encoding {address:?} must produce the pinned canonical spelling: {reason}"
        );
        assert_ne!(
            canonical, address,
            "the canonical spelling must differ from the decode-only one: {reason}"
        );
    }

    // Rejection vectors: address strings that every implementation must reject when
    // decoding with the accompanying expected format, with the failure
    // classification pinned by `error_kind` (the `Ss58Error` variant name). All
    // three suites decode with the expected format and assert the kind, so the
    // error taxonomy cannot drift apart unnoticed either.
    #[cfg(feature = "ss58")]
    for v in vectors["ss58_invalid"].as_array().unwrap() {
        let address = v["ss58"].as_str().unwrap();
        let format = v["format"].as_u64().unwrap() as u16;
        let error_kind = v["error_kind"].as_str().unwrap();
        let reason = v["reason"].as_str().unwrap();
        let error = match AccountId32::from_ss58check_expecting(address, format)
            .expect_err(&format!("{address:?} must be rejected: {reason}"))
        {
            crate::Ss58ExpectingError::Address(error) => error,
            crate::Ss58ExpectingError::InvalidExpectedFormat(error) => panic!(
                "every ss58_invalid vector carries a valid expected format, \
                 so the rejection must blame the address: {reason} ({error})"
            ),
        };
        assert_eq!(
            ss58_error_kind(&error),
            error_kind,
            "error kind of {address:?} must match the fixture: {reason}"
        );
        // The rendered message is shared verbatim with the Python and TypeScript
        // implementations (docs/cross-language-contracts.md); pinning it in the fixture is what stops one
        // language's wording from drifting while every suite stays green on its own literal.
        assert_eq!(
            error.to_string(),
            v["message"].as_str().unwrap(),
            "message of {address:?} must match the fixture: {reason}"
        );
        // UnexpectedFormat vectors additionally pin the expected/found pair.
        if let crate::Ss58Error::UnexpectedFormat { expected, found } = error {
            assert_eq!(
                (u64::from(expected), u64::from(found)),
                (
                    v["expected"].as_u64().unwrap(),
                    v["found"].as_u64().unwrap()
                ),
                "expected/found of {address:?} must match the fixture: {reason}"
            );
            // Without an expected format the same address is simply a valid address
            // of another network: the unknown-format entry point accepts it and
            // reports the embedded format (the Python and TypeScript suites pin the
            // same behavior on their decode-with-format entry points).
            let (_, found_any) =
                AccountId32::from_ss58check_with_version(address).unwrap_or_else(|e| {
                    panic!("{address:?} must decode without an expected format: {e} ({reason})")
                });
            assert_eq!(
                u64::from(found_any),
                v["found"].as_u64().unwrap(),
                "format found in {address:?} must match the fixture: {reason}"
            );
        } else {
            // Structural failures are independent of the expected format: the
            // unknown-format entry point rejects with the same classification.
            let error_any = AccountId32::from_ss58check_with_version(address).expect_err(&format!(
                "{address:?} must be rejected without an expected format: {reason}"
            ));
            assert_eq!(
                ss58_error_kind(&error_any),
                error_kind,
                "unknown-format error kind of {address:?} must match the fixture: {reason}"
            );
        }
    }

    // Names every implementation must reject at encode time, with the failure
    // classification pinned by `error_kind`. In this crate the rejection happens at
    // AccountName construction (encode_named_account only accepts an already-validated
    // AccountName); the kind follows from the shared rule "UTF-8 byte length first, then
    // the charset", which is exactly what the non-ASCII vectors pin.
    for v in vectors["named_invalid_encode"].as_array().unwrap() {
        let name = v["name"].as_str().unwrap();
        let reason = v["reason"].as_str().unwrap();
        let error = AccountName::try_from(name).expect_err(&format!(
            "{name:?} must be rejected at encode time: {reason}"
        ));
        assert_eq!(
            account_name_error_kind(&error),
            v["error_kind"].as_str().unwrap(),
            "error kind of {name:?} must match the fixture: {reason}"
        );
        assert_eq!(
            error.to_string(),
            v["message"].as_str().unwrap(),
            "message of {name:?} must match the fixture: {reason}"
        );
    }

    // The `hex_invalid` and `hex_valid` groups pin the "0x" + 64 hex contract
    // shared by this crate's `FromStr` implementation for `AccountId32` and the
    // Python decode_account_id / TypeScript decodeAccountId entry points
    // (rejection and mixed-case acceptance respectively).
    for v in vectors["hex_invalid"].as_array().unwrap() {
        let input = v["input"].as_str().unwrap();
        let reason = v["reason"].as_str().unwrap();
        let error = input
            .parse::<AccountId32>()
            .expect_err(&format!("{input:?} must be rejected: {reason}"));
        assert_eq!(
            error.to_string(),
            v["message"].as_str().unwrap(),
            "message of {input:?} must match the fixture: {reason}"
        );
    }
    for v in vectors["hex_valid"].as_array().unwrap() {
        let input = v["input"].as_str().unwrap();
        let account: AccountId32 = input
            .parse()
            .unwrap_or_else(|_| panic!("{input:?} must parse as an account id"));
        // Display renders the normalized lowercase form pinned by the fixture.
        assert_eq!(
            account.to_string(),
            v["account_id"].as_str().unwrap(),
            "Display of {input:?} must be the normalized lowercase form"
        );
        assert_classification(&account, &v["expected"]);
    }

    // Raw account id hex strings must be rejected by the SS58 address entry
    // points: the Python decode_address and TypeScript decodeAddress reject
    // "0x"-prefixed input outright; this crate's SS58 decode rejects the same
    // strings structurally. The group pins the rejection only - the error
    // classification is per-language.
    #[cfg(feature = "ss58")]
    for v in vectors["hex_not_an_address"].as_array().unwrap() {
        let input = v["input"].as_str().unwrap();
        let reason = v["reason"].as_str().unwrap();
        assert!(
            AccountId32::from_ss58check_expecting(input, crate::SS58_FORMAT_TREVO_ASSET_HUB)
                .is_err(),
            "{input:?} must not decode as an SS58 address: {reason}"
        );
        assert!(
            AccountId32::from_ss58check_with_version(input).is_err(),
            "{input:?} must not decode as an SS58 address of any format: {reason}"
        );
    }

    // The mirror image of hex_not_an_address: SS58 addresses fed to the
    // raw-account-id entry points. An address is never "0x" + 64 hex, so the hex
    // contract must reject it rather than silently base58-decoding it. Rejection only;
    // each implementation reports its own ParseAccountIdError.
    for v in vectors["address_not_an_account_id"].as_array().unwrap() {
        let input = v["input"].as_str().unwrap();
        let reason = v["reason"].as_str().unwrap();
        assert!(
            input.parse::<AccountId32>().is_err(),
            "{input:?} must not parse as a raw account id: {reason}"
        );
    }

    // SS58 formats every implementation must reject at encode time. The group pins the
    // rejection only: an unusable format is a rejected *argument*, reported through each
    // language's own parameter-error type (here `InvalidSs58Format`, see its docs), so
    // no vector carries an `error_kind` or `message` to assert.
    #[cfg(feature = "ss58")]
    for v in vectors["format_invalid_encode"].as_array().unwrap() {
        let account = account_from_hex(v["hex"].as_str().unwrap());
        let format = v["format"].as_u64().unwrap() as u16;
        let reason = v["reason"].as_str().unwrap();
        let error = account
            .to_ss58check_with_version(format)
            .expect_err(&format!("format {format} must not encode: {reason}"));
        assert_eq!(
            error.format(),
            format,
            "the rejected argument must be reported back: {reason}"
        );
    }

    // The decode side of the same contract: an expected format outside the 14-bit range must
    // be rejected against an otherwise valid address. Only the rejection is cross-language —
    // this crate reports Ss58ExpectingError::InvalidExpectedFormat, Python a ValueError and
    // TypeScript a RangeError — so the fixture pins no kind here.
    #[cfg(feature = "ss58")]
    for v in vectors["format_invalid_decode"].as_array().unwrap() {
        let address = v["ss58"].as_str().unwrap();
        let format = v["format"].as_u64().unwrap() as u16;
        let reason = v["reason"].as_str().unwrap();
        match AccountId32::from_ss58check_expecting(address, format) {
            Err(crate::Ss58ExpectingError::InvalidExpectedFormat(error)) => assert_eq!(
                error.format(),
                format,
                "the rejected argument must be reported back: {reason}"
            ),
            other => panic!(
                "expected format {format} must be rejected as a parameter, got {other:?}: {reason}"
            ),
        }
        // The address itself is valid, so the rejection can only come from the parameter.
        assert!(
            AccountId32::from_ss58check(address).is_ok(),
            "the address of the {reason:?} vector must otherwise decode"
        );
    }

    // Format values that exist only at the host-language boundary (boolean, negative,
    // fractional, beyond u16). This crate's u16 makes them unrepresentable, so — as with
    // `u32_invalid_encode` below — there is nothing to call here; assert the vectors really
    // are outside u16 (exactly why Python and TypeScript need a runtime check) and that the
    // carrier hex/ss58 fields are otherwise valid, so a host-language rejection can only
    // come from the format parameter.
    for v in vectors["format_invalid_host"].as_array().unwrap() {
        let reason = v["reason"].as_str().unwrap();
        let value = &v["value"];
        let representable = value
            .as_u64()
            .is_some_and(|value| u16::try_from(value).is_ok());
        assert!(
            !representable,
            "{value} must not be a valid u16, or the vector is pointless: {reason}"
        );
        let _account = account_from_hex(v["hex"].as_str().unwrap());
        #[cfg(feature = "ss58")]
        assert_eq!(
            AccountId32::from_ss58check(v["ss58"].as_str().unwrap()),
            Ok(_account),
            "the carrier address of the {reason:?} vector must otherwise decode"
        );
    }

    // Identifier values the Python and TypeScript encoders must reject. This crate enforces
    // the u32 range through the type system, so there is nothing to call here; assert instead
    // that no vector is representable as a u32 — which is exactly why the other two need a
    // runtime check.
    for v in vectors["u32_invalid_encode"].as_array().unwrap() {
        let reason = v["reason"].as_str().unwrap();
        let value = &v["value"];
        let representable = value
            .as_u64()
            .is_some_and(|value| u32::try_from(value).is_ok());
        assert!(
            !representable,
            "{value} must not be a valid u32, or the vector is pointless: {reason}"
        );
    }

    // The exported constant values, pinned by the fixture so all three implementations
    // agree on the numbers users are told to rely on (the behavior at the bounds is
    // pinned by other groups). An unknown name is a hard failure: a constant added to
    // the generator must be consumed here, not silently skipped.
    for v in vectors["constants"].as_array().unwrap() {
        let name = v["name"].as_str().unwrap();
        let value = v["value"].as_u64().unwrap();
        let actual: Option<u64> = match name {
            "ACCOUNT_ID_LENGTH" => Some(ACCOUNT_ID_LENGTH as u64),
            "ACCOUNT_NAME_LENGTH" => Some(ACCOUNT_NAME_LENGTH as u64),
            "APP_AGENT_ACCOUNT_IDENTIFIER" => Some(u64::from(APP_AGENT_ACCOUNT_IDENTIFIER)),
            "TRANSACTIONAL_ACCOUNT_IDENTIFIER" => Some(u64::from(TRANSACTIONAL_ACCOUNT_IDENTIFIER)),
            "NAMED_ACCOUNT_IDENTIFIER" => Some(u64::from(NAMED_ACCOUNT_IDENTIFIER)),
            // Python and TypeScript export U32_MAX; Rust callers use the native u32::MAX.
            "U32_MAX" => Some(u64::from(u32::MAX)),
            #[cfg(feature = "ss58")]
            "SS58_FORMAT_TREVO_ASSET_HUB" => Some(u64::from(crate::SS58_FORMAT_TREVO_ASSET_HUB)),
            #[cfg(feature = "ss58")]
            "SS58_MAX_ADDRESS_LENGTH" => Some(crate::SS58_MAX_ADDRESS_LENGTH as u64),
            // Without the ss58 feature the two SS58 constants do not exist to compare.
            #[cfg(not(feature = "ss58"))]
            "SS58_FORMAT_TREVO_ASSET_HUB" | "SS58_MAX_ADDRESS_LENGTH" => None,
            other => panic!("unknown constant {other:?} in the fixture"),
        };
        if let Some(actual) = actual {
            assert_eq!(actual, value, "constant {name} must match the fixture");
        }
    }

    // The typed decoders carry a `(expected, found)` pair when the account is of a
    // different type. The pair is the cross-language contract (the rendered message
    // is per-language); this pins it for every (decoder, actual-type) mismatch.
    for v in vectors["type_mismatch"].as_array().unwrap() {
        let account = account_from_hex(v["hex"].as_str().unwrap());
        let decode_as = v["decode_as"].as_str().unwrap();
        let expected_found = v["found"].as_str().unwrap();
        // The vector's two spellings must agree from this side too: Python/TS drive both
        // the hex and the SS58 form, so the address must resolve to the same account
        // here (`address_message` stays Python/TS-only by design).
        #[cfg(feature = "ss58")]
        assert_eq!(
            AccountId32::from_ss58check(v["ss58"].as_str().unwrap()),
            Ok(account),
            "the ss58 spelling of the {decode_as} vector must decode to the same account"
        );
        let error = match decode_as {
            "AppAgent" => decode_app_agent_account(&account).err(),
            "Transactional" => decode_transactional_account(&account).err(),
            "Named" => decode_named_account(&account).err(),
            other => panic!("unexpected decode_as {other:?}"),
        }
        .expect("a typed decoder on a different type must error");
        let KeylessDecodeError::WrongAccountType { expected, found } = error;
        assert_eq!(
            format!("{expected}"),
            decode_as,
            "expected of {decode_as} mismatch must match the fixture"
        );
        assert_eq!(
            format!("{found}"),
            expected_found,
            "found of {decode_as} mismatch must match the fixture"
        );
    }
}

/// Cross-language contracts that no fixture vector can carry, because they are about the
/// *shape* of the API rather than about values crossing a language boundary. They are
/// cheap to pin here, next to the fixture they complement (see docs/cross-language-contracts.md,
/// "Cross-language contracts").
#[test]
fn test_cross_language_constants() {
    // The numeric values of `AccountType` are shared with the Python and TypeScript
    // `AccountType` enums, and the three keyless ones coincide with the byte-`[4]`
    // discriminant of the account they classify.
    assert_eq!(AccountType::Regular as u8, 0);
    assert_eq!(AccountType::AppAgent as u8, 1);
    assert_eq!(AccountType::Transactional as u8, 2);
    assert_eq!(AccountType::Named as u8, 3);
    assert_eq!(AccountType::AppAgent as u8, APP_AGENT_ACCOUNT_IDENTIFIER);
    assert_eq!(
        AccountType::Transactional as u8,
        TRANSACTIONAL_ACCOUNT_IDENTIFIER
    );
    assert_eq!(AccountType::Named as u8, NAMED_ACCOUNT_IDENTIFIER);

    // The layout constants exported under the same names by all three implementations.
    assert_eq!(ACCOUNT_ID_LENGTH, 32);
    assert_eq!(ACCOUNT_NAME_LENGTH, 10);
    // ... and they really are the widths of the types they describe.
    assert_eq!(
        AsRef::<[u8]>::as_ref(&AccountId32::new([0u8; 32])).len(),
        ACCOUNT_ID_LENGTH
    );
    assert_eq!(
        AccountName::try_from("example123")
            .unwrap()
            .as_bytes()
            .len(),
        ACCOUNT_NAME_LENGTH
    );
}
