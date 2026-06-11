//! Generates the cross-language conformance fixture `conformance/vectors.json`.
//!
//! The Rust implementation is the source of truth for the keyless encoding; this example
//! derives a deterministic set of encoding, classification, and SS58 vectors from it. The
//! Rust, Python, and TypeScript test suites all consume the committed fixture, so any
//! semantic drift between the implementations fails the test suite of the drifting port.
//!
//! Regenerate (and review the diff!) with:
//!
//! ```text
//! cargo run --example gen_conformance --features ss58 > ../conformance/vectors.json
//! ```
//!
//! CI verifies that the committed fixture matches this generator's output.

use trevo_keyless::*;

/// Deterministic xorshift64* PRNG (same construction as `tests_properties.rs`).
struct Rng(u64);

impl Rng {
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    fn fill(&mut self, buf: &mut [u8]) {
        for chunk in buf.chunks_mut(8) {
            let bytes = self.next_u64().to_le_bytes();
            chunk.copy_from_slice(&bytes[..chunk.len()]);
        }
    }
}

/// The 64 characters allowed in an [`AccountName`].
const CHARSET: &[u8; 64] = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ-#";

fn random_name(rng: &mut Rng) -> AccountName {
    let mut bytes = [0u8; 10];
    for byte in &mut bytes {
        *byte = CHARSET[(rng.next_u64() % 64) as usize];
    }
    AccountName::new(&bytes).expect("generated from the allowed charset")
}

/// Lowercase hex of the raw account bytes (no `0x` prefix).
fn hex(account: &AccountId32) -> String {
    format!("{account}")
}

/// The expected classification of `account` as a JSON object, with the same field values
/// the Python and TypeScript info objects expose.
fn classification_json(account: &AccountId32) -> String {
    match decode_account_info(account) {
        BlockchainAccountInfo::Regular { .. } => r#"{"type":"Regular"}"#.to_string(),
        BlockchainAccountInfo::AppAgent { app_agent_id, .. } => {
            format!(r#"{{"type":"AppAgent","app_agent_id":{app_agent_id}}}"#)
        }
        BlockchainAccountInfo::Transactional {
            app_agent_id,
            ta_id,
            ..
        } => format!(r#"{{"type":"Transactional","app_agent_id":{app_agent_id},"ta_id":{ta_id}}}"#),
        BlockchainAccountInfo::Named {
            app_agent_id,
            account_name,
            ..
        } => format!(
            r#"{{"type":"Named","app_agent_id":{app_agent_id},"name":"{}"}}"#,
            account_name.as_str()
        ),
    }
}

fn print_group(name: &str, items: &[String], last: bool) {
    println!("\"{name}\": [");
    for (i, item) in items.iter().enumerate() {
        let comma = if i + 1 == items.len() { "" } else { "," };
        println!("{item}{comma}");
    }
    println!("]{}", if last { "" } else { "," });
}

fn main() {
    // 1. AppAgent encoding vectors: random ids plus the u32 boundaries.
    let mut rng = Rng(0xC0DE_0001);
    let mut app_agent: Vec<String> = Vec::new();
    for id in (0..60).map(|_| rng.next_u32()).chain([0, 1, u32::MAX]) {
        let account = encode_app_agent_account(id);
        app_agent.push(format!(
            r#"{{"id":{id},"hex":"{}","ss58":"{}"}}"#,
            hex(&account),
            account.to_ss58check()
        ));
    }

    // 2. Transactional encoding vectors: random id pairs plus the boundary combinations.
    let mut rng = Rng(0xC0DE_0002);
    let mut transactional: Vec<String> = Vec::new();
    for (aa, ta) in (0..60).map(|_| (rng.next_u32(), rng.next_u32())).chain([
        (0, 0),
        (0, u32::MAX),
        (u32::MAX, 0),
        (u32::MAX, u32::MAX),
    ]) {
        let account = encode_transactional_account(aa, ta);
        transactional.push(format!(
            r#"{{"aa":{aa},"ta":{ta},"hex":"{}","ss58":"{}"}}"#,
            hex(&account),
            account.to_ss58check()
        ));
    }

    // 3. Named encoding vectors: random names plus charset boundary names.
    let mut rng = Rng(0xC0DE_0003);
    let mut named: Vec<String> = Vec::new();
    let boundary_names = [
        "0000000000",
        "##########",
        "----------",
        "ZZZZZZZZZZ",
        "aA0-#zZ9-#",
        "9zZ#-aA0--",
    ];
    let named_cases: Vec<(u32, AccountName)> = (0..60)
        .map(|_| (rng.next_u32(), random_name(&mut rng)))
        .chain(
            boundary_names
                .iter()
                .map(|name| (u32::MAX, AccountName::try_from(*name).unwrap())),
        )
        .collect();
    for (aa, name) in named_cases {
        let account = encode_named_account(aa, name);
        named.push(format!(
            r#"{{"aa":{aa},"name":"{}","hex":"{}","ss58":"{}"}}"#,
            name.as_str(),
            hex(&account),
            account.to_ss58check()
        ));
    }

    // 4. Classification vectors.
    let mut classify: Vec<String> = Vec::new();

    // 4a. Arbitrary 32-byte accounts with the discriminant byte cycled through 0..=7, so
    // every decode branch (unknown discriminant and all three checksum checks) is hit.
    let mut rng = Rng(0xC0DE_0004);
    for i in 0..200u64 {
        let mut bytes = [0u8; 32];
        rng.fill(&mut bytes);
        bytes[4] = (i % 8) as u8;
        let account = AccountId32::from(bytes);
        classify.push(format!(
            r#"{{"hex":"{}","expected":{}}}"#,
            hex(&account),
            classification_json(&account)
        ));
    }

    // 4b. Every single-byte corruption of one account per keyless type (must all demote
    // to Regular: either the open part or the stored checksum no longer matches).
    let mut rng = Rng(0xC0DE_0005);
    let accounts = [
        encode_app_agent_account(rng.next_u32()),
        encode_transactional_account(rng.next_u32(), rng.next_u32()),
        encode_named_account(rng.next_u32(), random_name(&mut rng)),
    ];
    for account in accounts {
        let original: [u8; 32] = account.into();
        for (position, xor) in (0..32).zip([0x01u8, 0x80, 0xFF].into_iter().cycle()) {
            let mut corrupted = original;
            corrupted[position] ^= xor;
            let corrupted = AccountId32::from(corrupted);
            classify.push(format!(
                r#"{{"hex":"{}","expected":{}}}"#,
                hex(&corrupted),
                classification_json(&corrupted)
            ));
        }
    }

    // 4c. Accounts carrying a *valid* checksum over name bytes at the charset boundaries
    // (the bytes adjacent to the allowed ranges, whitespace, high bytes, and a few valid
    // characters as controls). Anyone can construct these without a private key; decoders
    // must agree byte-for-byte on which of them classify as Named.
    let boundary_bytes: &[u8] = &[
        0x00, 0x09, 0x0A, 0x20, b'!', b'"', b'$', b'%', b'/', b':', b'@', b'[', b'`', b'{', 0x7F,
        0x80, 0xC3, 0xFF, b'_', b'.', b'~', b'*', b'a', b'Z', b'0', b'-', b'#',
    ];
    let mut rng = Rng(0xC0DE_0006);
    for &boundary_byte in boundary_bytes {
        for position in [0usize, 4, 9] {
            let mut name_bytes = *b"valid12345";
            name_bytes[position] = boundary_byte;
            let mut open_part = [0u8; 15];
            open_part[0..4].copy_from_slice(&rng.next_u32().to_le_bytes());
            open_part[4] = NAMED_ACCOUNT_IDENTIFIER;
            open_part[5..15].copy_from_slice(&name_bytes);
            let hash = blake2b_simd::Params::new().hash_length(32).hash(&open_part);
            let mut account_bytes = [0u8; 32];
            account_bytes[0..15].copy_from_slice(&open_part);
            account_bytes[15..32].copy_from_slice(&hash.as_bytes()[15..32]);
            let account = AccountId32::from(account_bytes);
            classify.push(format!(
                r#"{{"hex":"{}","expected":{}}}"#,
                hex(&account),
                classification_json(&account)
            ));
        }
    }

    // 5. SS58 encoding of fixed accounts across single-byte and two-byte prefix formats
    // (45 and 48 are the neighbors of the reserved formats 46 and 47).
    let mut rng = Rng(0xC0DE_0007);
    let mut ss58_formats: Vec<String> = Vec::new();
    for _ in 0..10 {
        let mut bytes = [0u8; 32];
        rng.fill(&mut bytes);
        let account = AccountId32::from(bytes);
        for format in [0u16, 1, 42, 45, 48, 63, 64, 255, 5335, 16383] {
            let address = account.to_ss58check_with_version(format).unwrap();
            ss58_formats.push(format!(
                r#"{{"hex":"{}","format":{format},"ss58":"{}"}}"#,
                hex(&account),
                address
            ));
        }
    }

    println!("{{");
    print_group("app_agent", &app_agent, false);
    print_group("transactional", &transactional, false);
    print_group("named", &named, false);
    print_group("classify", &classify, false);
    print_group("ss58_formats", &ss58_formats, true);
    println!("}}");
}
