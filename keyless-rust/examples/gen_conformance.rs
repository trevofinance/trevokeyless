//! Generates the cross-language conformance fixture `conformance/vectors.json`.
//!
//! The Rust implementation is the source of truth for the keyless encoding; this example
//! derives a deterministic set of encoding, classification, and SS58 vectors from it —
//! including `ss58_invalid`, address strings every implementation must *reject*. The
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

/// The 64-byte SS58 hash (`blake2b_512(b"SS58PRE" ++ data)`); its first bytes are the
/// SS58 checksum.
fn ss58hash(data: &[u8]) -> [u8; 64] {
    blake2b_simd::Params::new()
        .hash_length(64)
        .to_state()
        .update(b"SS58PRE")
        .update(data)
        .finalize()
        .as_bytes()
        .try_into()
        .expect("slice is always the necessary length")
}

/// The two-byte SS58 prefix encoding of a format. Canonical for formats 64..=16383;
/// non-canonical (but decodable) for smaller formats.
fn two_byte_prefix(format: u16) -> Vec<u8> {
    vec![
        (((format & 0b0000_0000_1111_1100) >> 2) | 0b0100_0000) as u8,
        ((format >> 8) | ((format & 0b0000_0000_0000_0011) << 6)) as u8,
    ]
}

/// The canonical 1- or 2-byte SS58 prefix encoding of a format.
fn format_prefix(format: u16) -> Vec<u8> {
    if format < 64 {
        vec![format as u8]
    } else {
        two_byte_prefix(format)
    }
}

/// Wraps a raw prefix+data payload in an SS58 envelope with a valid checksum of
/// `checksum_len` bytes (2 is canonical for 32-byte account ids).
fn raw_ss58(payload: &[u8], checksum_len: usize) -> String {
    let mut data = payload.to_vec();
    data.extend_from_slice(&ss58hash(payload)[..checksum_len]);
    bs58::encode(data).into_string()
}

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

    // 6. SS58 address strings that every implementation must reject when decoding with
    // the accompanying expected format. These pin the *rejection* side of the SS58
    // contract — non-canonical shapes that permissive decoders historically accepted
    // (e.g. one-byte checksums) must fail in every language, exactly like in this crate.
    let account_id = encode_app_agent_account(123);
    let canonical_address = account_id.to_ss58check();
    let account: [u8; 32] = account_id.into();
    let trevo_prefix = format_prefix(SS58_FORMAT_TREVO_ASSET_HUB);
    let payload = |prefix: &[u8], data: &[u8]| [prefix, data].concat();
    let tampered = {
        let canonical = payload(&trevo_prefix, &account);
        let mut data = canonical.clone();
        data.extend_from_slice(&ss58hash(&canonical)[..2]);
        let last = data.len() - 1;
        data[last] ^= 0xFF;
        bs58::encode(data).into_string()
    };
    let trevo = SS58_FORMAT_TREVO_ASSET_HUB;
    let ss58_invalid: Vec<(String, u16, &str)> = vec![
        (
            raw_ss58(&payload(&trevo_prefix, &account), 1),
            trevo,
            "canonical account with a truncated one-byte checksum (decoded length 35)",
        ),
        (
            raw_ss58(&payload(&[42u8], &account), 1),
            42,
            "single-byte-prefix account with a truncated one-byte checksum (decoded length 34)",
        ),
        (
            raw_ss58(&payload(&[42u8], &[account.as_slice(), &[0u8]].concat()), 2),
            42,
            "33-byte payload under a single-byte prefix (decoded length 36, within the address length bound)",
        ),
        (
            raw_ss58(&payload(&trevo_prefix, &account[..31]), 2),
            trevo,
            "31-byte account id with a full checksum (decoded length 35)",
        ),
        (
            raw_ss58(
                &payload(&trevo_prefix, &[account.as_slice(), &[0u8]].concat()),
                2,
            ),
            trevo,
            "33-byte payload (ECDSA-style public key, decoded length 37)",
        ),
        (
            raw_ss58(&payload(&trevo_prefix, &[7u8; 8]), 1),
            trevo,
            "8-byte account-index payload (decoded length 11)",
        ),
        (
            raw_ss58(&payload(&[46u8], &account), 2),
            46,
            "reserved format 46, canonical single-byte prefix",
        ),
        (
            raw_ss58(&payload(&two_byte_prefix(46), &account), 2),
            46,
            "reserved format 46, non-canonical two-byte prefix",
        ),
        (
            raw_ss58(&payload(&two_byte_prefix(47), &account), 2),
            47,
            "reserved format 47, non-canonical two-byte prefix",
        ),
        (
            raw_ss58(&payload(&[0x80u8], &account), 2),
            trevo,
            "first byte 0x80: not a valid SS58 prefix encoding",
        ),
        (
            tampered,
            trevo,
            "canonical address with the last checksum byte flipped",
        ),
        (
            "1".repeat(51),
            trevo,
            "longer than any valid address of a 32-byte account id",
        ),
        (String::new(), trevo, "empty string"),
        (
            "0OIl".to_string(),
            trevo,
            "characters outside the base58 alphabet",
        ),
        // Whitespace around an otherwise canonical address: permissive base58 decoders
        // (e.g. python base58, which right-strips its input) would otherwise give every
        // account additional accepted spellings.
        (
            format!("{canonical_address} "),
            trevo,
            "canonical address with a trailing space",
        ),
        (
            format!(" {canonical_address}"),
            trevo,
            "canonical address with a leading space",
        ),
    ];
    let ss58_invalid: Vec<String> = ss58_invalid
        .into_iter()
        .map(|(address, format, reason)| {
            // The generator must never emit a vector this crate itself accepts.
            assert!(
                AccountId32::from_ss58check_with_version(&address).is_err(),
                "self check: {reason:?} must not decode"
            );
            format!(r#"{{"ss58":"{address}","format":{format},"reason":"{reason}"}}"#)
        })
        .collect();

    println!("{{");
    print_group("app_agent", &app_agent, false);
    print_group("transactional", &transactional, false);
    print_group("named", &named, false);
    print_group("classify", &classify, false);
    print_group("ss58_formats", &ss58_formats, false);
    print_group("ss58_invalid", &ss58_invalid, true);
    println!("}}");
}
