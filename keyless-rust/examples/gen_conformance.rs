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

use trevokeyless::*;

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

/// Lowercase hex of the raw account bytes (no `0x` prefix). Formats the bytes
/// explicitly rather than via `Display`, so the committed fixture is independent of the
/// crate's rendering choices.
fn hex(account: &AccountId32) -> String {
    let bytes: &[u8; 32] = account.as_ref();
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// JSON string literal for `s`. Routing every string field through serde_json keeps the
/// hand-rolled printer correct even when a vector contains characters that need
/// escaping (quotes, control characters).
fn js(s: &str) -> String {
    serde_json::to_string(s).expect("strings always serialize")
}

/// The raw account id contract of the Rust `AccountId32` `FromStr` and the Python
/// `decode_account_id` / TypeScript `decodeAccountId` entry points: exactly "0x"
/// followed by 64 hex characters.
fn is_valid_hex_account_id(s: &str) -> bool {
    let bytes = s.as_bytes();
    bytes.len() == 66 && bytes.starts_with(b"0x") && bytes[2..].iter().all(u8::is_ascii_hexdigit)
}

/// The fixture name of an `Ss58Error`: the Rust variant name, matched by the
/// `Ss58Error.kind` of the TypeScript and Python implementations.
fn ss58_error_kind(error: &Ss58Error) -> &'static str {
    match error {
        Ss58Error::InvalidBase58 => "InvalidBase58",
        Ss58Error::InvalidLength => "InvalidLength",
        Ss58Error::InvalidChecksum => "InvalidChecksum",
        Ss58Error::InvalidFormat => "InvalidFormat",
        Ss58Error::UnexpectedFormat { .. } => "UnexpectedFormat",
    }
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
            r#"{{"type":"Named","app_agent_id":{app_agent_id},"name":{}}}"#,
            js(account_name.as_str())
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
            r#"{{"id":{id},"hex":{},"ss58":{}}}"#,
            js(&hex(&account)),
            js(&account.to_ss58check())
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
            r#"{{"aa":{aa},"ta":{ta},"hex":{},"ss58":{}}}"#,
            js(&hex(&account)),
            js(&account.to_ss58check())
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
            r#"{{"aa":{aa},"name":{},"hex":{},"ss58":{}}}"#,
            js(name.as_str()),
            js(&hex(&account)),
            js(&account.to_ss58check())
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
            r#"{{"hex":{},"expected":{}}}"#,
            js(&hex(&account)),
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

    // 4d. Valid accounts of each keyless type with the discriminant byte rewritten to
    // every other known discriminant (and 0). The stored checksum was computed for the
    // original open-part length, so every rewrite must classify as Regular — pinning
    // that a valid account of one type can never be read as another type. (The 4b
    // corruption cycle XORs position 4 with 0x80 only, so it never produces a
    // different *valid* discriminant.)
    let mut rng = Rng(0xC0DE_0008);
    let accounts = [
        encode_app_agent_account(rng.next_u32()),
        encode_transactional_account(rng.next_u32(), rng.next_u32()),
        encode_named_account(rng.next_u32(), random_name(&mut rng)),
    ];
    for account in accounts {
        let original: [u8; 32] = account.into();
        for discriminant in [
            0u8,
            APP_AGENT_ACCOUNT_IDENTIFIER,
            TRANSACTIONAL_ACCOUNT_IDENTIFIER,
            NAMED_ACCOUNT_IDENTIFIER,
        ] {
            if discriminant == original[4] {
                continue;
            }
            let mut rewritten = original;
            rewritten[4] = discriminant;
            let rewritten = AccountId32::from(rewritten);
            classify.push(format!(
                r#"{{"hex":"{}","expected":{}}}"#,
                hex(&rewritten),
                classification_json(&rewritten)
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
                r#"{{"hex":{},"format":{format},"ss58":{}}}"#,
                js(&hex(&account)),
                js(&address)
            ));
        }
    }
    // Deterministic boundary entry: the all-0xFF account id under the largest format
    // (two-byte prefix 0x7F 0xFF) yields the longest legitimate address. The first
    // payload byte of a valid address is at most 0x7F, so the payload value stays below
    // 2^287 and no valid address reaches SS58_MAX_ADDRESS_LENGTH (50) characters: the
    // longest achievable address is 49, one under the bound. This vector pins that
    // maximum, so every implementation must still decode an address of the maximal
    // length (and the bound itself stays a safe over-approximation).
    let boundary_account = AccountId32::from([0xFF; 32]);
    let boundary_address = boundary_account
        .to_ss58check_with_version(16383)
        .expect("16383 is a valid format");
    assert_eq!(
        boundary_address.len(),
        SS58_MAX_ADDRESS_LENGTH - 1,
        "self check: the longest achievable address is one under the bound"
    );
    ss58_formats.push(format!(
        r#"{{"hex":{},"format":16383,"ss58":{}}}"#,
        js(&hex(&boundary_account)),
        js(&boundary_address)
    ));

    // 6. SS58 address spellings that decode but are *not* the canonical encoding: the
    // two-byte-prefix form of formats below 64 — the one documented,
    // Substrate-compatible malleability exception (see the Security model section of
    // the repository README). Every implementation must decode these to the same
    // account id and format, while its encoder keeps producing the canonical one-byte
    // form, so the vectors are decode-only. Formats >= 64 have no second spelling (the
    // two-byte prefix is their canonical form), so the Trevo format is unaffected.
    let mut rng = Rng(0xC0DE_0009);
    let mut ss58_decode_only: Vec<String> = Vec::new();
    for format in [0u16, 1, 42, 45, 63] {
        let mut bytes = [0u8; 32];
        rng.fill(&mut bytes);
        let account = AccountId32::from(bytes);
        let noncanonical = raw_ss58(&[two_byte_prefix(format), bytes.to_vec()].concat(), 2);
        // The generator must never emit a spelling this crate itself rejects, decodes
        // differently, or — worse — considers canonical.
        assert_eq!(
            AccountId32::from_ss58check_with_version(&noncanonical).expect("self check: decodes"),
            (account, format),
            "self check: the non-canonical spelling must decode to the same account and format"
        );
        assert_ne!(
            account
                .to_ss58check_with_version(format)
                .expect("valid format"),
            noncanonical,
            "self check: encoding must stay canonical (one-byte prefix)"
        );
        ss58_decode_only.push(format!(
            r#"{{"ss58":{},"hex":{},"format":{format},"reason":{}}}"#,
            js(&noncanonical),
            js(&hex(&account)),
            js(&format!(
                "non-canonical two-byte-prefix spelling of format {format}"
            ))
        ));
    }

    // 7. SS58 address strings that every implementation must reject when decoding with
    // the accompanying expected format. These pin the *rejection* side of the SS58
    // contract — non-canonical shapes that permissive decoders historically accepted
    // (e.g. one-byte checksums) must fail in every language, exactly like in this
    // crate. `error_kind` pins the failure classification: the Rust `Ss58Error` variant
    // name, mirrored by the `Ss58Error.kind` of the TypeScript and Python
    // implementations — so the error taxonomy cannot drift apart unnoticed either.
    let account_id = encode_app_agent_account(123);
    let canonical_address = account_id.to_ss58check();
    let account: [u8; 32] = account_id.into();
    let trevo_prefix = format_prefix(SS58_FORMAT_TREVO_ASSET_HUB);
    let payload = |prefix: &[u8], data: &[u8]| [prefix, data].concat();
    // A canonically checksummed address of `account` under `prefix`, with the last
    // checksum byte flipped.
    let tampered = |prefix: &[u8]| {
        let canonical = payload(prefix, &account);
        let mut data = canonical.clone();
        data.extend_from_slice(&ss58hash(&canonical)[..2]);
        let last = data.len() - 1;
        data[last] ^= 0xFF;
        bs58::encode(data).into_string()
    };
    let trevo = SS58_FORMAT_TREVO_ASSET_HUB;
    let ss58_invalid: Vec<(String, u16, &str, &str)> = vec![
        (
            raw_ss58(&payload(&trevo_prefix, &account), 1),
            trevo,
            "InvalidLength",
            "canonical account with a truncated one-byte checksum (decoded length 35)",
        ),
        (
            raw_ss58(&payload(&[42u8], &account), 1),
            42,
            "InvalidLength",
            "single-byte-prefix account with a truncated one-byte checksum (decoded length 34)",
        ),
        (
            raw_ss58(&payload(&[42u8], &[account.as_slice(), &[0u8]].concat()), 2),
            42,
            "InvalidLength",
            "33-byte payload under a single-byte prefix (decoded length 36, within the address length bound)",
        ),
        (
            raw_ss58(&payload(&trevo_prefix, &account[..31]), 2),
            trevo,
            "InvalidLength",
            "31-byte account id with a full checksum (decoded length 35)",
        ),
        (
            raw_ss58(
                &payload(&trevo_prefix, &[account.as_slice(), &[0u8]].concat()),
                2,
            ),
            trevo,
            "InvalidLength",
            "33-byte payload (ECDSA-style public key, decoded length 37)",
        ),
        (
            raw_ss58(&payload(&trevo_prefix, &[7u8; 8]), 1),
            trevo,
            "InvalidLength",
            "8-byte account-index payload (decoded length 11)",
        ),
        (
            raw_ss58(&payload(&trevo_prefix, &account), 0),
            trevo,
            "InvalidLength",
            "canonical account with no checksum at all (decoded length 34)",
        ),
        (
            // Decoded length 36 is exactly the accepted length class for a two-byte
            // prefix, but the bytes split as prefix + 31-byte account + 3-byte checksum.
            // A decoder that derives the structure from the total length reads the last
            // two bytes as the checksum of the first 34 and must find a mismatch.
            raw_ss58(&payload(&trevo_prefix, &account[..31]), 3),
            trevo,
            "InvalidChecksum",
            "31-byte account with a three-byte checksum (decoded length 36, the canonical length class)",
        ),
        (
            raw_ss58(&payload(&[46u8], &account), 2),
            46,
            "InvalidFormat",
            "reserved format 46, canonical single-byte prefix",
        ),
        (
            raw_ss58(&payload(&two_byte_prefix(46), &account), 2),
            46,
            "InvalidFormat",
            "reserved format 46, non-canonical two-byte prefix",
        ),
        (
            raw_ss58(&payload(&two_byte_prefix(47), &account), 2),
            47,
            "InvalidFormat",
            "reserved format 47, non-canonical two-byte prefix",
        ),
        (
            raw_ss58(&payload(&[0x80u8], &account), 2),
            trevo,
            "InvalidFormat",
            "first byte 0x80: not a valid SS58 prefix encoding",
        ),
        (
            // The canonical Trevo two-byte prefix with the high bit set on its first byte
            // (0x75 -> 0xF5). A decoder that masks the prefix tag bits instead of
            // validating them would read the same format 5335 — and the checksum is valid
            // over the crafted payload — giving every account a second spelling.
            {
                let mut prefix = trevo_prefix.clone();
                prefix[0] |= 0b1000_0000;
                raw_ss58(&payload(&prefix, &account), 2)
            },
            trevo,
            "InvalidFormat",
            "first byte 0xF5: two-byte Trevo prefix with the high bit set",
        ),
        (
            tampered(&trevo_prefix),
            trevo,
            "InvalidChecksum",
            "canonical address with the last checksum byte flipped",
        ),
        (
            // A fully valid address of a non-expected format: the one rejection whose
            // kind is the format mismatch itself.
            account_id.to_ss58check_with_version(42).unwrap(),
            trevo,
            "UnexpectedFormat",
            "valid format-42 address, decoded expecting the Trevo format",
        ),
        (
            // A corrupt address of a non-expected format: the checksum failure must win
            // over the format mismatch. The format carried by an address that fails
            // validation is not trustworthy information, so reporting UnexpectedFormat
            // here (as the Python implementation once did) would claim that a corrupt
            // string is a valid address of another network.
            tampered(&[42u8]),
            trevo,
            "InvalidChecksum",
            "format-42 address with the last checksum byte flipped, decoded expecting the Trevo format",
        ),
        (
            // Same ordering for structural failures of a non-expected format.
            raw_ss58(&payload(&[42u8], &account[..31]), 2),
            trevo,
            "InvalidLength",
            "format-42 31-byte payload with a full checksum, decoded expecting the Trevo format",
        ),
        (
            "1".repeat(51),
            trevo,
            "InvalidLength",
            "longer than any valid address of a 32-byte account id",
        ),
        (String::new(), trevo, "InvalidLength", "empty string"),
        (
            "0OIl".to_string(),
            trevo,
            "InvalidBase58",
            "characters outside the base58 alphabet",
        ),
        // Whitespace around an otherwise canonical address: permissive base58 decoders
        // (e.g. python base58, which right-strips its input) would otherwise give every
        // account additional accepted spellings.
        (
            format!("{canonical_address} "),
            trevo,
            "InvalidBase58",
            "canonical address with a trailing space",
        ),
        (
            format!(" {canonical_address}"),
            trevo,
            "InvalidBase58",
            "canonical address with a leading space",
        ),
        (
            // ASCII case swap of every letter. Base58 is case-sensitive, and the swap
            // is guaranteed to leave the alphabet ('i' <-> 'I' and 'L' <-> 'l' cross
            // into the excluded characters) or at minimum break the checksum.
            canonical_address
                .chars()
                .map(|c| {
                    if c.is_ascii_lowercase() {
                        c.to_ascii_uppercase()
                    } else {
                        c.to_ascii_lowercase()
                    }
                })
                .collect::<String>(),
            trevo,
            "InvalidBase58",
            "case-swapped spelling of a canonical address",
        ),
        (
            // In-place corruption with characters outside the base58 alphabet, distinct
            // from the all-garbage "0OIl" vector: the string is canonical except for one
            // character, so decoders that sanitize instead of reject would accept it.
            format!("{}0{}", &canonical_address[..20], &canonical_address[21..]),
            trevo,
            "InvalidBase58",
            "canonical address with one character replaced by '0' (outside the base58 alphabet)",
        ),
        (
            format!("{}I{}", &canonical_address[..20], &canonical_address[21..]),
            trevo,
            "InvalidBase58",
            "canonical address with one character replaced by 'I' (outside the base58 alphabet)",
        ),
        // Non-ASCII input interacting with the length pre-bound. The bound is defined in
        // UTF-8 bytes (the unit this crate's `str::len` measures), so a string can stay
        // within 50 characters - or 50 UTF-16 units - yet exceed 50 bytes. These vectors
        // pin the classification that historically drifted between the implementations
        // (code points in Python, UTF-16 units in TypeScript).
        (
            "é".repeat(26),
            trevo,
            "InvalidLength",
            "26 two-byte UTF-8 characters: 26 code points but 52 bytes, over the 50-byte bound",
        ),
        (
            "𝄞".repeat(26),
            trevo,
            "InvalidLength",
            "26 four-byte UTF-8 characters: 52 UTF-16 units and 104 bytes, over the 50-byte bound",
        ),
        (
            "é".to_string(),
            trevo,
            "InvalidBase58",
            "single non-ASCII character, within the length bound",
        ),
    ];
    let ss58_invalid: Vec<String> = ss58_invalid
        .into_iter()
        .map(|(address, format, error_kind, reason)| {
            // The generator must never emit a vector whose declared kind differs from
            // the actual classification of this crate (decoding with the expected
            // format, exactly like the consuming suites).
            let error = AccountId32::from_ss58check_expecting(&address, format)
                .expect_err(&format!("self check: {reason:?} must not decode"));
            assert_eq!(
                ss58_error_kind(&error),
                error_kind,
                "self check: declared error kind of {reason:?} must match the implementation"
            );
            // UnexpectedFormat vectors additionally pin the expected/found pair the
            // error must carry (Rust variant fields, Python Ss58FormatMismatchError
            // attributes, TypeScript Ss58Error fields).
            let details = match error {
                Ss58Error::UnexpectedFormat { expected, found } => {
                    format!(r#","expected":{expected},"found":{found}"#)
                }
                _ => String::new(),
            };
            format!(
                r#"{{"ss58":{},"format":{format},"error_kind":{}{details},"reason":{}}}"#,
                js(&address),
                js(error_kind),
                js(reason)
            )
        })
        .collect();

    // 8. Names every implementation must reject at *encode* time (wrong length or
    // characters outside the charset). The tab, NUL, and non-ASCII entries deliberately
    // exercise the JSON escaping of the generator and the parsers of the consuming
    // suites. Note "héllo12345": 10 code points but 11 UTF-8 bytes — the byte-level
    // Rust validation reports a length error while Python/TypeScript report a charset
    // error; all three must reject it.
    let named_invalid_encode: Vec<(&str, &str)> = vec![
        ("", "empty string"),
        ("short", "5 characters"),
        ("abcdefghi", "9 characters"),
        ("abcdefghijk", "11 characters"),
        ("name with spaces", "16 characters including spaces"),
        ("invalid@12", "10 characters, '@' outside the charset"),
        ("with space", "10 characters, ' ' outside the charset"),
        ("with_under", "10 characters, '_' outside the charset"),
        ("tab\there12", "10 characters, tab outside the charset"),
        ("nul\0byte12", "10 characters, NUL outside the charset"),
        (
            "héllo12345",
            "10 code points, 'é' outside the charset (11 UTF-8 bytes)",
        ),
    ];
    let named_invalid_encode: Vec<String> = named_invalid_encode
        .into_iter()
        .map(|(name, reason)| {
            // The generator must never emit a name this crate itself accepts.
            assert!(
                AccountName::try_from(name).is_err(),
                "self check: {reason:?} must not construct an AccountName"
            );
            format!(r#"{{"name":{},"reason":{}}}"#, js(name), js(reason))
        })
        .collect();

    // 9. Raw account id strings that must be rejected: the contract of this crate's
    // `FromStr` for `AccountId32` and of the Python `decode_account_id` / TypeScript
    // `decodeAccountId` entry points is exactly "0x" + 64 hex characters.
    let h = hex(&account_id);
    let hex_invalid: Vec<(String, &str)> = vec![
        (String::new(), "empty string"),
        ("0x".to_string(), "prefix only"),
        ("0xabc".to_string(), "odd number of hex digits"),
        (format!("0x{}", &h[..62]), "62 hex characters"),
        (format!("0x{}", &h[..63]), "63 hex characters"),
        (format!("0x{h}0"), "65 hex characters"),
        (
            format!("0x{}zz", &h[..62]),
            "characters outside the hex alphabet",
        ),
        (format!("0X{h}"), "uppercase 0X prefix"),
        (h.clone(), "missing 0x prefix"),
        (
            format!("0x{} {}", &h[..8], &h[9..]),
            "embedded space (bytes.fromhex would tolerate it)",
        ),
        (format!("0x{h} "), "trailing space"),
        (format!(" 0x{h}"), "leading space"),
        (format!("0x{h}\n"), "trailing newline"),
    ];
    let hex_invalid: Vec<String> = hex_invalid
        .into_iter()
        .map(|(input, reason)| {
            // The generator must never emit a string that satisfies the contract.
            assert!(
                !is_valid_hex_account_id(&input),
                "self check: {reason:?} must not be a valid hex account id"
            );
            format!(r#"{{"hex":{},"reason":{}}}"#, js(&input), js(reason))
        })
        .collect();

    // 10. Raw account id strings that must be *accepted*: the "0x" + 64 hex contract
    // allows both hex digit cases ([0-9a-fA-F]), and the normalized `account_id` of the
    // result is always "0x" + lowercase hex (this crate's `Display`). Pinning the
    // acceptance keeps a future "strictness fix" in one implementation from silently
    // diverging.
    let mut hex_valid: Vec<String> = Vec::new();
    for account in [
        encode_app_agent_account(123),
        encode_transactional_account(123, 456),
        encode_named_account(123, AccountName::try_from("example123").unwrap()),
    ] {
        let lower = hex(&account);
        let upper = lower.to_uppercase();
        // Alternating-case spelling, mixing both digit cases in one string.
        let alternating: String = lower
            .chars()
            .enumerate()
            .map(|(i, c)| {
                if i % 2 == 0 {
                    c.to_ascii_uppercase()
                } else {
                    c
                }
            })
            .collect();
        for spelling in [upper, alternating] {
            let input = format!("0x{spelling}");
            // The generator must never emit a string outside the contract.
            assert!(
                is_valid_hex_account_id(&input),
                "self check: {input:?} must be a valid hex account id"
            );
            hex_valid.push(format!(
                r#"{{"hex":{},"account_id":{},"expected":{}}}"#,
                js(&input),
                js(&format!("0x{lower}")),
                classification_json(&account)
            ));
        }
    }

    // 11. Raw account id hex strings fed to the *SS58 address* entry points: SS58
    // addresses and raw account ids are separate input classes (a raw account id
    // carries no SS58 format), so the Python decode_address and TypeScript
    // decodeAddress reject any "0x"-prefixed input outright, steering the caller to
    // decode_account_id / decodeAccountId; this crate's from_ss58check_expecting
    // rejects the same strings structurally (never valid base58 of an SS58 payload,
    // or over the length bound). The group pins the rejection only — the error
    // classification is per-language (a kind-less guidance error in Python and
    // TypeScript, an Ss58Error in this crate).
    let hex_not_an_address: Vec<(String, &str)> = vec![
        (
            format!("0x{h}"),
            "the canonical raw account id of AppAgent(123)",
        ),
        (
            format!("0x{}", h.to_uppercase()),
            "uppercase-digit spelling of the same account id",
        ),
        ("0xdeadbeef".to_string(), "0x-prefixed short hex"),
        ("0x".to_string(), "bare 0x prefix"),
    ];
    let hex_not_an_address: Vec<String> = hex_not_an_address
        .into_iter()
        .map(|(input, reason)| {
            // The generator must never emit a string this crate's SS58 decode entry
            // point accepts.
            assert!(
                AccountId32::from_ss58check_expecting(&input, SS58_FORMAT_TREVO_ASSET_HUB).is_err(),
                "self check: {reason:?} must not decode as an SS58 address"
            );
            format!(r#"{{"input":{},"reason":{}}}"#, js(&input), js(reason))
        })
        .collect();

    println!("{{");
    print_group("app_agent", &app_agent, false);
    print_group("transactional", &transactional, false);
    print_group("named", &named, false);
    print_group("classify", &classify, false);
    print_group("ss58_formats", &ss58_formats, false);
    print_group("ss58_decode_only", &ss58_decode_only, false);
    print_group("ss58_invalid", &ss58_invalid, false);
    print_group("named_invalid_encode", &named_invalid_encode, false);
    print_group("hex_invalid", &hex_invalid, false);
    print_group("hex_valid", &hex_valid, false);
    print_group("hex_not_an_address", &hex_not_an_address, true);
    println!("}}");
}
