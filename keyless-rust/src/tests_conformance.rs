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
            assert_eq!(format!("{account}"), v["hex"].as_str().unwrap());
            assert_eq!(decode_app_agent_account(&account), Ok(id));
            #[cfg(feature = "ss58")]
            assert_eq!(account.to_ss58check(), v["ss58"].as_str().unwrap());
        }

        for v in vectors["transactional"].as_array().unwrap() {
            let aa = v["aa"].as_u64().unwrap() as u32;
            let ta = v["ta"].as_u64().unwrap() as u32;
            let account = encode_transactional_account(aa, ta);
            assert_eq!(format!("{account}"), v["hex"].as_str().unwrap());
            assert_eq!(decode_transactional_account(&account), Ok((aa, ta)));
            #[cfg(feature = "ss58")]
            assert_eq!(account.to_ss58check(), v["ss58"].as_str().unwrap());
        }

        for v in vectors["named"].as_array().unwrap() {
            let aa = v["aa"].as_u64().unwrap() as u32;
            let name = AccountName::try_from(v["name"].as_str().unwrap()).unwrap();
            let account = encode_named_account(aa, name);
            assert_eq!(format!("{account}"), v["hex"].as_str().unwrap());
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
    }
}
