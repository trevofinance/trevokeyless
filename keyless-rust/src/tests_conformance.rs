//! Verifies the implementation against the committed cross-language conformance fixture
//! (`conformance/vectors.json` at the repository root).
//!
//! The fixture is generated from this crate by `examples/gen_conformance.rs` — the Rust
//! implementation is the source of truth — and is also consumed by the Python and
//! TypeScript test suites. This test pins the Rust implementation to the committed
//! fixture, so an accidental semantic change to the encoding fails here first instead of
//! silently regenerating different vectors for the other languages.

#[cfg(test)]
mod tests {
    use crate::keyless::*;
    use crate::types::*;

    fn fixture() -> Option<serde_json::Value> {
        // The fixture lives outside the crate directory and is therefore absent from
        // published crate tarballs; skip (rather than fail) when it is missing.
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../conformance/vectors.json");
        let data = std::fs::read_to_string(path).ok()?;
        Some(serde_json::from_str(&data).expect("fixture must be valid JSON"))
    }

    fn account_from_hex(hex: &str) -> AccountId32 {
        let bytes: [u8; 32] = array_bytes::hex2bytes(hex).unwrap()[..].try_into().unwrap();
        AccountId32::from(bytes)
    }

    /// Lowercase hex of the raw account bytes, matching the fixture's `hex` fields
    /// (which are formatted from the bytes directly, independent of the crate's
    /// `Display` rendering).
    fn account_hex(account: &AccountId32) -> String {
        let bytes: &[u8; 32] = account.as_ref();
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }

    /// Decodes with an expected format, mirroring the decode entry points of the Python
    /// and TypeScript implementations (and `from_ss58check`, which is fixed to the
    /// Trevo format): full validation first, then the format comparison. The same
    /// construction lives in `examples/gen_conformance.rs`.
    #[cfg(feature = "ss58")]
    fn decode_expecting(address: &str, format: u16) -> Result<AccountId32, crate::Ss58Error> {
        let (account, found) = AccountId32::from_ss58check_with_version(address)?;
        if found != format {
            return Err(crate::Ss58Error::UnexpectedFormat {
                expected: format,
                found,
            });
        }
        Ok(account)
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
            expected["app_agent_id"].as_u64().map(|v| v as u32),
            "app_agent_id of {account} must match the fixture"
        );
        assert_eq!(
            info.ta_id(),
            expected["ta_id"].as_u64().map(|v| v as u32),
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
            let id = v["id"].as_u64().unwrap() as u32;
            let account = encode_app_agent_account(id);
            assert_eq!(account_hex(&account), v["hex"].as_str().unwrap());
            assert_eq!(decode_app_agent_account(&account), Ok(id));
            #[cfg(feature = "ss58")]
            assert_eq!(account.to_ss58check(), v["ss58"].as_str().unwrap());
        }

        for v in vectors["transactional"].as_array().unwrap() {
            let aa = v["aa"].as_u64().unwrap() as u32;
            let ta = v["ta"].as_u64().unwrap() as u32;
            let account = encode_transactional_account(aa, ta);
            assert_eq!(account_hex(&account), v["hex"].as_str().unwrap());
            assert_eq!(decode_transactional_account(&account), Ok((aa, ta)));
            #[cfg(feature = "ss58")]
            assert_eq!(account.to_ss58check(), v["ss58"].as_str().unwrap());
        }

        for v in vectors["named"].as_array().unwrap() {
            let aa = v["aa"].as_u64().unwrap() as u32;
            let name = AccountName::try_from(v["name"].as_str().unwrap()).unwrap();
            let account = encode_named_account(aa, name);
            assert_eq!(account_hex(&account), v["hex"].as_str().unwrap());
            assert_eq!(decode_named_account(&account), Ok((aa, name)));
            #[cfg(feature = "ss58")]
            assert_eq!(account.to_ss58check(), v["ss58"].as_str().unwrap());
        }

        for v in vectors["classify"].as_array().unwrap() {
            let account = account_from_hex(v["hex"].as_str().unwrap());
            assert_classification(&account, &v["expected"]);
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
                decode_expecting(address, format),
                Ok(account),
                "{address:?} must decode: {reason}"
            );
            assert_ne!(
                account.to_ss58check_with_version(format).unwrap(),
                address,
                "re-encoding {address:?} must stay canonical: {reason}"
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
            let error = decode_expecting(address, format)
                .expect_err(&format!("{address:?} must be rejected: {reason}"));
            assert_eq!(
                ss58_error_kind(&error),
                error_kind,
                "error kind of {address:?} must match the fixture: {reason}"
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
            }
        }

        // Names every implementation must reject at encode time. In this crate the
        // rejection happens at AccountName construction; encode_named_account only
        // accepts an already-validated AccountName.
        for v in vectors["named_invalid_encode"].as_array().unwrap() {
            let name = v["name"].as_str().unwrap();
            let reason = v["reason"].as_str().unwrap();
            assert!(
                AccountName::try_from(name).is_err(),
                "{name:?} must be rejected at encode time: {reason}"
            );
        }

        // The `hex_invalid` and `hex_valid` groups pin the "0x" + 64 hex contract of
        // the Python decode_account_id and TypeScript decodeAccountId entry points
        // (rejection and mixed-case acceptance respectively). This crate has no string
        // form of an account id (AccountId32 is constructed from bytes), so there is
        // nothing to verify here beyond the groups being present.
        assert!(
            !vectors["hex_invalid"].as_array().unwrap().is_empty(),
            "hex_invalid group must be present for the Python/TypeScript suites"
        );
        assert!(
            !vectors["hex_valid"].as_array().unwrap().is_empty(),
            "hex_valid group must be present for the Python/TypeScript suites"
        );
    }
}
