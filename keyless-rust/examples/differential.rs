//! JSON driver used by the repository's valid-and-hostile differential check.

use serde_json::{json, Value};
use std::{env, fs, process};
use trevokeyless::{
    decode_account_info, encode_app_agent_account, encode_named_account,
    encode_transactional_account, AccountId32, AccountName, AccountNameError, AccountType,
    AppAgentId, NamedAccountIdentifiers, Ss58Error, TransactionalAccountIdentifiers,
    TransactionalId,
};

fn account_type_name(account_type: AccountType) -> &'static str {
    match account_type {
        AccountType::Regular => "Regular",
        AccountType::AppAgent => "AppAgent",
        AccountType::Transactional => "Transactional",
        AccountType::Named => "Named",
    }
}

fn required_u32(case: &Value, field: &str) -> u32 {
    case[field]
        .as_u64()
        .and_then(|value| value.try_into().ok())
        .unwrap_or_else(|| panic!("{field} must be a u32"))
}

fn success_result(account: &AccountId32, format: u16) -> Value {
    let info = decode_account_info(account);
    json!({
        "status": "ok",
        "format": format,
        "account_id": account.to_string(),
        "account_type": account_type_name(info.account_type()),
        "app_agent_id": info.app_agent_id().map(AppAgentId::get),
        "ta_id": info.ta_id().map(TransactionalId::get),
        "account_name": info.account_name().map(|name| name.to_string()),
    })
}

fn error_result(error: Ss58Error) -> Value {
    match error {
        Ss58Error::InvalidBase58 => json!({"status": "error", "kind": "InvalidBase58"}),
        Ss58Error::InvalidLength => json!({"status": "error", "kind": "InvalidLength"}),
        Ss58Error::InvalidChecksum => {
            json!({"status": "error", "kind": "InvalidChecksum"})
        }
        Ss58Error::InvalidFormat => json!({"status": "error", "kind": "InvalidFormat"}),
        Ss58Error::UnexpectedFormat { expected, found } => json!({
            "status": "error",
            "kind": "UnexpectedFormat",
            "expected": expected,
            "found": found,
        }),
    }
}

fn encode_results(cases: &[Value]) -> Vec<Value> {
    let mut results = Vec::with_capacity(cases.len());
    for case in cases {
        let kind = case["kind"].as_str().expect("kind must be a string");
        let format = case["format"]
            .as_u64()
            .and_then(|value| value.try_into().ok())
            .expect("format must be a u16");
        let app_agent_id = AppAgentId::new(required_u32(case, "app_agent_id"));
        let account = match kind {
            "app_agent" => encode_app_agent_account(app_agent_id),
            "transactional" => encode_transactional_account(TransactionalAccountIdentifiers::new(
                app_agent_id,
                TransactionalId::new(required_u32(case, "ta_id")),
            )),
            "named" => encode_named_account(NamedAccountIdentifiers::new(
                app_agent_id,
                AccountName::new(
                    case["account_name"]
                        .as_str()
                        .expect("account_name must be a string")
                        .as_bytes(),
                )
                .expect("generated name must be valid"),
            )),
            other => panic!("unknown case kind {other}"),
        };
        let info = decode_account_info(&account);
        results.push(json!({
            "kind": kind,
            "account_id": account.to_string(),
            "address": account.to_ss58check_with_version(format).unwrap(),
            "account_type": account_type_name(info.account_type()),
            "app_agent_id": info.app_agent_id().map(AppAgentId::get),
            "ta_id": info.ta_id().map(TransactionalId::get),
            "account_name": info.account_name().map(|name| name.to_string()),
        }));
    }
    results
}

fn decode_results(cases: &[Value]) -> Vec<Value> {
    let mut results = Vec::with_capacity(cases.len());
    for case in cases {
        let label = case["label"].as_str().expect("label must be a string");
        let address = case["address"].as_str().expect("address must be a string");
        let format = case["format"]
            .as_u64()
            .and_then(|value| value.try_into().ok())
            .expect("format must be a u16");

        let expecting = match AccountId32::from_ss58check_expecting(address, format) {
            Ok(account) => success_result(&account, format),
            // Fold the parameter branch into the address-level taxonomy: the harness
            // compares the five shared kinds, and its generated formats are all valid
            // parameters anyway (out-of-range expected formats are host-boundary cases
            // the fixture's format_invalid_decode group pins per language instead).
            Err(error) => error_result(Ss58Error::from(error)),
        };
        let any_format = match AccountId32::from_ss58check_with_version(address) {
            Ok((account, found)) => success_result(&account, found),
            Err(error) => error_result(error),
        };
        results.push(json!({
            "label": label,
            "expecting": expecting,
            "any_format": any_format,
        }));
    }
    results
}

/// Account names, valid and hostile. The `AccountNameError` kind is a cross-language
/// contract, so it is reported rather than a bare rejection.
fn name_results(cases: &[Value]) -> Vec<Value> {
    let mut results = Vec::with_capacity(cases.len());
    for case in cases {
        let name = case["name"].as_str().expect("name must be a string");
        let app_agent_id = AppAgentId::new(required_u32(case, "app_agent_id"));
        let result = match AccountName::new(name.as_bytes()) {
            Ok(account_name) => {
                let account =
                    encode_named_account(NamedAccountIdentifiers::new(app_agent_id, account_name));
                json!({"status": "ok", "account_id": account.to_string()})
            }
            Err(AccountNameError::InvalidLength) => {
                json!({"status": "error", "kind": "InvalidLength"})
            }
            Err(AccountNameError::InvalidCharacter) => {
                json!({"status": "error", "kind": "InvalidCharacter"})
            }
        };
        results.push(json!({"label": case["label"], "result": result}));
    }
    results
}

/// SS58 formats an encoder must accept or reject. Only the rejection is compared: the
/// classification of a rejected *argument* is per-language (`InvalidSs58Format` here, a
/// `ValueError` in Python, a `RangeError` in TypeScript). What this pins is that the two
/// ways a format can be unusable — reserved, and out of the 14-bit range — are not told
/// apart by any implementation.
fn format_results(cases: &[Value]) -> Vec<Value> {
    let mut results = Vec::with_capacity(cases.len());
    for case in cases {
        let account: AccountId32 = case["account_id"]
            .as_str()
            .expect("account_id must be a string")
            .parse()
            .expect("account_id must be a valid raw account id");
        let format: u16 = case["format"]
            .as_u64()
            .and_then(|value| value.try_into().ok())
            .expect("format must be a u16");
        let result = match account.to_ss58check_with_version(format) {
            Ok(address) => json!({"status": "ok", "address": address}),
            Err(_) => json!({"status": "rejected"}),
        };
        results.push(json!({"label": case["label"], "result": result}));
    }
    results
}

fn typed_result(expected: AccountType, found: AccountType) -> Value {
    if expected == found {
        json!({"status": "ok"})
    } else {
        json!({
            "status": "mismatch",
            "expected": account_type_name(expected),
            "found": account_type_name(found),
        })
    }
}

/// Raw 32-byte account ids through the classifier and all three typed decoders. Arbitrary
/// bytes must classify as Regular without panicking; a corrupted checksum must demote a
/// keyless account to Regular; and the typed decoders' (expected, found) mismatch pair is
/// itself a cross-language contract.
fn classify_results(cases: &[Value]) -> Vec<Value> {
    let mut results = Vec::with_capacity(cases.len());
    for case in cases {
        let account: AccountId32 = case["hex"]
            .as_str()
            .expect("hex must be a string")
            .parse()
            .expect("hex must be a valid raw account id");
        let info = decode_account_info(&account);
        let found = info.account_type();
        results.push(json!({
            "label": case["label"],
            "info": {
                "account_id": account.to_string(),
                "account_type": account_type_name(found),
                "app_agent_id": info.app_agent_id().map(AppAgentId::get),
                "ta_id": info.ta_id().map(TransactionalId::get),
                "account_name": info.account_name().map(|name| name.to_string()),
            },
            "typed": {
                "app_agent": typed_result(AccountType::AppAgent, found),
                "transactional": typed_result(AccountType::Transactional, found),
                "named": typed_result(AccountType::Named, found),
            },
        }));
    }
    results
}

/// The raw account-id string contract: "0x" followed by exactly 64 hex characters, both
/// digit cases accepted. Shared verbatim with the Python `decode_account_id` and the
/// TypeScript `decodeAccountId`.
fn hex_results(cases: &[Value]) -> Vec<Value> {
    let mut results = Vec::with_capacity(cases.len());
    for case in cases {
        let input = case["input"].as_str().expect("input must be a string");
        let result = match input.parse::<AccountId32>() {
            Ok(account) => {
                let info = decode_account_info(&account);
                json!({
                    "status": "ok",
                    "account_id": account.to_string(),
                    "account_type": account_type_name(info.account_type()),
                })
            }
            Err(_) => json!({"status": "error"}),
        };
        results.push(json!({"label": case["label"], "result": result}));
    }
    results
}

fn main() {
    let Some(path) = env::args().nth(1) else {
        eprintln!("usage: cargo run --example differential --features ss58,serde -- <cases.json>");
        process::exit(2);
    };
    let payload: Value = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
    let section = |name: &str| -> Vec<Value> {
        payload[name]
            .as_array()
            .unwrap_or_else(|| panic!("{name} must be an array"))
            .clone()
    };

    println!(
        "{}",
        serde_json::to_string(&json!({
            "encode": encode_results(&section("encode_cases")),
            "decode": decode_results(&section("decode_cases")),
            "name": name_results(&section("name_cases")),
            "format": format_results(&section("format_cases")),
            "classify": classify_results(&section("classify_cases")),
            "hex": hex_results(&section("hex_cases")),
        }))
        .unwrap()
    );
}
