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

fn app_agent_account(app_agent_id: u32) -> AccountId32 {
    encode_app_agent_account(AppAgentId::new(app_agent_id))
}

fn transactional_account(app_agent_id: u32, ta_id: u32) -> AccountId32 {
    encode_transactional_account(TransactionalAccountIdentifiers::new(
        AppAgentId::new(app_agent_id),
        TransactionalId::new(ta_id),
    ))
}

fn named_account(app_agent_id: u32, account_name: AccountName) -> AccountId32 {
    encode_named_account(NamedAccountIdentifiers::new(
        AppAgentId::new(app_agent_id),
        account_name,
    ))
}

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
        // `% 64` takes xorshift64*'s statistically weakest low bits — deliberately
        // kept: the same expression runs in the Python and TypeScript property suites,
        // and changing it here would regenerate the committed fixture vectors.
        *byte = CHARSET[(rng.next_u64() % 64) as usize];
    }
    AccountName::new(&bytes).expect("generated from the allowed charset")
}

/// Lowercase hex of the raw account bytes (no `0x` prefix). Formats the bytes
/// explicitly rather than via `Display`, so the committed fixture is independent of the
/// crate's rendering choices.
fn hex(account: &AccountId32) -> String {
    let bytes = account.as_bytes();
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
/// The rendered message of the Python and TypeScript `AccountTypeMismatchError` — which each
/// document themselves as sharing it with the other.
///
/// This crate's `KeylessDecodeError` renders a different string on purpose (Rust says
/// *account*, the two address-level libraries say *address*; see
/// docs/cross-language-contracts.md), so the
/// wording is generated here from the same `(expected, found)` pair that produces the vector,
/// rather than read back from a `Display` impl. It is the one field of the fixture that is not
/// this crate's own output — pinning it is what stops the Python and TypeScript wordings from
/// drifting apart while each claims to match the other.
fn address_mismatch_message(expected: AccountType, found: AccountType) -> String {
    fn article(account_type: AccountType) -> &'static str {
        if account_type == AccountType::AppAgent {
            "an"
        } else {
            "a"
        }
    }
    format!(
        "Provided address is not {} {expected} address but is {} {found} address",
        article(expected),
        article(found)
    )
}

fn ss58_error_kind(error: &Ss58Error) -> &'static str {
    match error {
        Ss58Error::InvalidBase58 => "InvalidBase58",
        Ss58Error::InvalidLength => "InvalidLength",
        Ss58Error::InvalidChecksum => "InvalidChecksum",
        Ss58Error::InvalidFormat => "InvalidFormat",
        Ss58Error::UnexpectedFormat { .. } => "UnexpectedFormat",
    }
}

/// The fixture name of an `AccountNameError`: the Rust variant name, which every
/// implementation's account-name validation must reproduce (the length is the UTF-8 byte
/// length, checked before the charset).
fn account_name_error_kind(error: &AccountNameError) -> &'static str {
    match error {
        AccountNameError::InvalidLength => "InvalidLength",
        AccountNameError::InvalidCharacter => "InvalidCharacter",
    }
}

/// The `KeylessDecodeError` from decoding `account` with the typed decoder for
/// `decode_as` (which must not be the account's own type, so a mismatch is guaranteed).
fn typed_decode_error(account: &AccountId32, decode_as: AccountType) -> KeylessDecodeError {
    let error = match decode_as {
        AccountType::AppAgent => decode_app_agent_account(account).err(),
        AccountType::Transactional => decode_transactional_account(account).err(),
        AccountType::Named => decode_named_account(account).err(),
        AccountType::Regular => unreachable!("there is no Regular decoder"),
    };
    error.expect("a typed decoder on a different type must error")
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
        let account = app_agent_account(id);
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
        let account = transactional_account(aa, ta);
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
        // The AppAgent-id boundary on the Named layout: the boundary names above are all
        // paired with u32::MAX, and 60 random draws essentially never produce 0.
        .chain(std::iter::once((
            0,
            AccountName::try_from("aa-zero#00").unwrap(),
        )))
        .collect();
    for (aa, name) in named_cases {
        let account = named_account(aa, name);
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
        app_agent_account(rng.next_u32()),
        transactional_account(rng.next_u32(), rng.next_u32()),
        named_account(rng.next_u32(), random_name(&mut rng)),
    ];
    for account in accounts {
        let original: [u8; 32] = account.into();
        for (position, xor) in (0..32).zip([0x01u8, 0x80, 0xFF].into_iter().cycle()) {
            let mut corrupted = original;
            corrupted[position] ^= xor;
            let corrupted = AccountId32::from(corrupted);
            classify.push(format!(
                r#"{{"hex":{},"expected":{}}}"#,
                js(&hex(&corrupted)),
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
                r#"{{"hex":{},"expected":{}}}"#,
                js(&hex(&account)),
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
        app_agent_account(rng.next_u32()),
        transactional_account(rng.next_u32(), rng.next_u32()),
        named_account(rng.next_u32(), random_name(&mut rng)),
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
                r#"{{"hex":{},"expected":{}}}"#,
                js(&hex(&rewritten)),
                classification_json(&rewritten)
            ));
        }
    }

    // 4e. Valid accounts of each keyless type straight into the classify group, so the
    // classification path is pinned on positives independently of the encoding groups
    // (a port whose raw-account-id classify path diverged from its address-decode path
    // would otherwise see AppAgent/Transactional positives only via `hex_valid`).
    let mut rng = Rng(0xC0DE_000B);
    for _ in 0..5 {
        let accounts = [
            app_agent_account(rng.next_u32()),
            transactional_account(rng.next_u32(), rng.next_u32()),
            named_account(rng.next_u32(), random_name(&mut rng)),
        ];
        for account in accounts {
            classify.push(format!(
                r#"{{"hex":{},"expected":{}}}"#,
                js(&hex(&account)),
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
        let canonical = account
            .to_ss58check_with_version(format)
            .expect("valid format");
        assert_ne!(
            canonical, noncanonical,
            "self check: encoding must stay canonical (one-byte prefix)"
        );
        // `canonical_ss58` pins the canonical spelling itself, so an encoder that
        // produced a third spelling (neither the canonical nor the decode-only one)
        // could not pass with the mere "re-encode differs from the input" check.
        ss58_decode_only.push(format!(
            r#"{{"ss58":{},"hex":{},"format":{format},"canonical_ss58":{},"reason":{}}}"#,
            js(&noncanonical),
            js(&hex(&account)),
            js(&canonical),
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
    let account_id = app_agent_account(123);
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
            // A *reserved* format is still a well-formed thing to expect: no address can
            // carry it, so the answer is simply that this address does not match. This is
            // the decode-side counterpart of `format_invalid_encode`, where the same value
            // is a rejected argument — a decoder must not borrow that rejection and report
            // an invalid-format error about a perfectly valid address.
            account_id.to_ss58check(),
            46,
            "UnexpectedFormat",
            "valid Trevo address, decoded expecting the reserved format 46",
        ),
        (
            account_id.to_ss58check(),
            47,
            "UnexpectedFormat",
            "valid Trevo address, decoded expecting the reserved format 47",
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
        (
            // Exactly at the 50-character pre-decode bound: the input passes the bound
            // (no valid address is this long, but the bound is a deliberate
            // over-approximation) and must then fail the structural length check — it
            // decodes to 50 zero bytes.
            "1".repeat(50),
            trevo,
            "InvalidLength",
            "exactly at the 50-character pre-decode bound; structurally invalid (decodes to 50 zero bytes)",
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
            // Control characters: the space vectors above pin ordinary whitespace, but a
            // trimming decoder may treat tabs differently, and NUL is where C-string APIs
            // truncate. Both must classify exactly like any other out-of-alphabet byte.
            format!("{canonical_address}\t"),
            trevo,
            "InvalidBase58",
            "canonical address with a trailing tab",
        ),
        (
            format!(
                "{}\0{}",
                &canonical_address[..20],
                &canonical_address[21..]
            ),
            trevo,
            "InvalidBase58",
            "canonical address with one character replaced by NUL",
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
        // Doubly-invalid payloads: a reserved or unparsable prefix *and* a wrong payload
        // length. Every reserved/invalid-prefix vector above carries a full-length
        // payload, so nothing else pins which error kind wins when both conditions hold.
        // The contract is the decoder's check order: prefix and format validity are
        // established before the exact-length check (a length class only means anything
        // for a parsed prefix), so the format failure wins — except for a payload too
        // short to finish parsing its two-byte prefix, which has no format to judge yet
        // and stays a length failure.
        (
            raw_ss58(&payload(&[46u8], &account[..8]), 2),
            46,
            "InvalidFormat",
            "reserved format 46 over an 8-byte truncated account: the format failure wins over the length",
        ),
        (
            raw_ss58(&payload(&two_byte_prefix(47), &account[..8]), 2),
            47,
            "InvalidFormat",
            "reserved format 47 (two-byte prefix) over an 8-byte truncated account: the format failure wins",
        ),
        (
            raw_ss58(&payload(&[0x80u8], &account[..8]), 2),
            trevo,
            "InvalidFormat",
            "first byte 0x80 over an 8-byte truncated account: the unparsable prefix wins over the length",
        ),
        (
            raw_ss58(&[0b0100_0000u8], 0),
            trevo,
            "InvalidLength",
            "lone two-byte-prefix first byte (decoded length 1): too short to parse a format at all",
        ),
    ];
    let ss58_invalid: Vec<String> = ss58_invalid
        .into_iter()
        .map(|(address, format, error_kind, reason)| {
            // The generator must never emit a vector whose declared kind differs from
            // the actual classification of this crate (decoding with the expected
            // format, exactly like the consuming suites). Every vector's format is a
            // valid parameter, so the rejection always blames the address.
            let error = match AccountId32::from_ss58check_expecting(&address, format)
                .expect_err(&format!("self check: {reason:?} must not decode"))
            {
                Ss58ExpectingError::Address(error) => error,
                Ss58ExpectingError::InvalidExpectedFormat(error) => {
                    panic!("self check: {reason:?} must carry a valid expected format ({error})")
                }
            };
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
                r#"{{"ss58":{},"format":{format},"error_kind":{},"message":{}{details},"reason":{}}}"#,
                js(&address),
                js(error_kind),
                // The message is read back from this crate's Display, never hand-written:
                // it is shared verbatim with the Python and TypeScript implementations
                // (docs/cross-language-contracts.md), and nothing enforced that before this field existed.
                js(&error.to_string()),
                js(reason)
            )
        })
        .collect();

    // 8. Names every implementation must reject at *encode* time, with the failure
    // classification pinned by `error_kind` (the Rust `AccountNameError` variant name).
    // The validation rule is: the UTF-8 *byte* length must be exactly 10
    // (`InvalidLength`), and only then are the bytes checked against the charset
    // (`InvalidCharacter`) — so a non-ASCII name is judged by its byte length, never by
    // its code-point or UTF-16 length. The non-ASCII vectors below separate those three
    // counts pairwise, so an implementation that measures the wrong unit is caught by the
    // kind even though every vector is rejected either way. The tab, NUL, and non-ASCII
    // entries also exercise the JSON escaping of the generator and the parsers of the
    // consuming suites. `error_kind` is not hand-written: it is read back from this
    // crate's own validation below.
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
            "11 UTF-8 bytes (10 code points, 10 UTF-16 units): a length failure, not a charset one",
        ),
        (
            "café12345",
            "10 UTF-8 bytes but 9 code points: the length is right, 'é' is out of charset",
        ),
        (
            "cafe\u{0301}12345",
            "NFD spelling of \"café12345\": 11 UTF-8 bytes (10 code points) — a length failure",
        ),
        (
            "🙂aaaaaaaa",
            "12 UTF-8 bytes (9 code points, 10 UTF-16 units): a length failure",
        ),
        (
            "🙂aaaaaa",
            "10 UTF-8 bytes (7 code points, 8 UTF-16 units): the length is right, the emoji is out of charset",
        ),
        (
            "ααααα",
            "10 UTF-8 bytes (5 code points): the length is right, 'α' is out of charset",
        ),
    ];
    let named_invalid_encode: Vec<String> = named_invalid_encode
        .into_iter()
        .map(|(name, reason)| {
            // The generator must never emit a name this crate itself accepts, and the
            // pinned kind is read straight back from the real validation, so the fixture
            // cannot drift from the implementation.
            let error = AccountName::try_from(name).expect_err(&format!(
                "self check: {reason:?} must not construct an AccountName"
            ));
            format!(
                r#"{{"name":{},"error_kind":{},"message":{},"reason":{}}}"#,
                js(name),
                js(account_name_error_kind(&error)),
                js(&error.to_string()),
                js(reason)
            )
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
            // The ParseAccountIdError message is shared verbatim with the Python and
            // TypeScript implementations; read it back from the real parse failure.
            let error = input
                .parse::<AccountId32>()
                .expect_err("self check: the input must not parse");
            format!(
                r#"{{"input":{},"message":{},"reason":{}}}"#,
                js(&input),
                js(&error.to_string()),
                js(reason)
            )
        })
        .collect();

    // 10. Raw account id strings that must be *accepted*: the "0x" + 64 hex contract
    // allows both hex digit cases ([0-9a-fA-F]), and the normalized `account_id` of the
    // result is always "0x" + lowercase hex (this crate's `Display`). Pinning the
    // acceptance — including the plain-lowercase spelling, which doubles as the
    // identity case — keeps a future "strictness fix" in one implementation from
    // silently diverging.
    let mut hex_valid: Vec<String> = Vec::new();
    let regular_account = AccountId32::from([0x11u8; 32]);
    assert_eq!(
        decode_account_info(&regular_account).account_type(),
        AccountType::Regular,
        "self check: the hex_valid Regular vector must classify as Regular"
    );
    for account in [
        app_agent_account(123),
        transactional_account(123, 456),
        named_account(123, AccountName::try_from("example123").unwrap()),
        // A Regular account too: mixed-case acceptance is a property of the hex
        // contract, not of the keyless classification, so it must hold for accounts
        // that classify as nothing.
        regular_account,
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
        for spelling in [lower.clone(), upper, alternating] {
            let input = format!("0x{spelling}");
            // The generator must never emit a string outside the contract.
            assert!(
                is_valid_hex_account_id(&input),
                "self check: {input:?} must be a valid hex account id"
            );
            hex_valid.push(format!(
                r#"{{"input":{},"account_id":{},"expected":{}}}"#,
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

    // 12. SS58 addresses fed to the *raw-account-id* entry points: the mirror image of
    // `hex_not_an_address`. An address is never "0x" + 64 hex characters, so
    // decode_account_id / decodeAccountId (and this crate's `FromStr`) must reject it
    // under the hex contract rather than silently base58-decoding it. Rejection only;
    // each implementation reports its own ParseAccountIdError.
    let address_not_an_account_id: Vec<(String, &str)> = vec![
        (
            account_id.to_ss58check(),
            "the canonical Trevo address of AppAgent(123)",
        ),
        (
            account_id.to_ss58check_with_version(42).unwrap(),
            "a valid format-42 address of the same account",
        ),
        (
            boundary_address.clone(),
            "the longest achievable valid address (49 characters)",
        ),
        (
            raw_ss58(&[two_byte_prefix(0), account.to_vec()].concat(), 2),
            "the non-canonical two-byte-prefix spelling of a format-0 address",
        ),
    ];
    let address_not_an_account_id: Vec<String> = address_not_an_account_id
        .into_iter()
        .map(|(input, reason)| {
            // The generator must never emit a string the raw-account-id contract accepts.
            assert!(
                !is_valid_hex_account_id(&input),
                "self check: {reason:?} must not satisfy the hex contract"
            );
            input
                .parse::<AccountId32>()
                .expect_err(&format!("self check: {reason:?} must not parse"));
            format!(r#"{{"input":{},"reason":{}}}"#, js(&input), js(reason))
        })
        .collect();

    // 13. SS58 formats every implementation must reject at *encode* time — the reserved
    // formats 46/47 (no address may carry them) and the formats outside the 14-bit range.
    // The rejection is the contract; the classification is deliberately *not* pinned,
    // because an unusable format is a rejected *argument*, and each language reports a bad
    // argument in its own way: `InvalidSs58Format` in this crate, a `ValueError` in Python,
    // a `RangeError` in TypeScript. (Reserved formats used to reach the SS58 layer in
    // Python and TypeScript and surface as an `Ss58Error` — the error type that reports on
    // an *address* — so the same mistake raised two different types depending on how the
    // format was unusable. All three now reject both kinds the same way.)
    //
    // The `hex` field gives consumers a valid account id for their raw-account-id encoders;
    // the keyless encode entry points must reject the same formats.
    let format_invalid_encode: Vec<(u16, &str)> = vec![
        (46, "reserved format 46"),
        (47, "reserved format 47"),
        (16384, "one above the 14-bit format range"),
        (
            u16::MAX,
            "largest u16 value, outside the 14-bit format range",
        ),
    ];
    let format_invalid_encode: Vec<String> = format_invalid_encode
        .into_iter()
        .map(|(format, reason)| {
            // The generator must never emit a format this crate's encoder accepts.
            account_id
                .to_ss58check_with_version(format)
                .expect_err(&format!("self check: format {format} must not encode"));
            format!(
                r#"{{"hex":{},"format":{format},"reason":{}}}"#,
                js(&h),
                js(reason)
            )
        })
        .collect();

    // 14. SS58 formats that every implementation must reject as the *expected* format of a
    // decode call, against an otherwise valid address. `format_invalid_encode` above pins the
    // encode side only; nothing pinned the decode side, so the three could silently diverge on
    // whether 16384 is rejected at all (this crate returns `Ss58Error::InvalidFormat`, Python
    // raises `ValueError`, TypeScript raises `RangeError` — the rejection is the contract, the
    // classification is per-language, so no `error_kind`).
    //
    // The reserved formats 46 and 47 are deliberately absent: they are *valid* parameters. No
    // address can carry them, so all three report the address's own mismatch instead, which the
    // `ss58_invalid` group already covers.
    let valid_address = account_id.to_ss58check();
    let format_invalid_decode: Vec<(u16, &str)> = vec![
        (16384, "one above the 14-bit format range"),
        (
            u16::MAX,
            "largest u16 value, outside the 14-bit format range",
        ),
    ];
    let format_invalid_decode: Vec<String> = format_invalid_decode
        .into_iter()
        .map(|(format, reason)| {
            // The generator must never emit an expected format this crate's decoder accepts —
            // and the rejection must blame the parameter, never the (valid) address.
            match AccountId32::from_ss58check_expecting(&valid_address, format) {
                Err(Ss58ExpectingError::InvalidExpectedFormat(error)) => {
                    assert_eq!(error.format(), format, "self check: rejected argument")
                }
                other => panic!(
                    "self check: expected format {format} must be rejected as a parameter, \
                     got {other:?}"
                ),
            }
            // The address itself is valid, so the rejection can only come from the parameter.
            assert!(
                AccountId32::from_ss58check(&valid_address).is_ok(),
                "self check: the address must otherwise decode"
            );
            format!(
                r#"{{"ss58":{},"format":{format},"reason":{}}}"#,
                js(&valid_address),
                js(reason)
            )
        })
        .collect();

    // 15. Format values that exist only at the host-language boundary: JSON-representable
    // values that are not u16-typed (boolean, negative, fractional, beyond u16), fed both to
    // the encode entry points (as `ss58_format`) and to the decode entry points (as the
    // expected format). `format_invalid_encode`/`format_invalid_decode` above pin only u16
    // values, which this crate's type system already excludes; this group exists for the
    // Python and TypeScript implementations, whose runtime format validation must reject
    // these before any payload work — exactly as `u32_invalid_encode` below does for the
    // identifiers. Rejection is the contract; the classification is per-language (Python
    // TypeError/ValueError, TypeScript RangeError), so no `error_kind`.
    //
    // One host-value class is deliberately absent: negative zero. JSON `-0` parses to an
    // accepted int 0 in Python but to a rejected -0 double in JavaScript, so no shared
    // vector can pin it; the per-language adversarial suites cover it (see
    // docs/conformance-schema.md).
    let format_invalid_host: Vec<(&str, &str)> = vec![
        ("-1", "negative"),
        ("1.5", "not an integer"),
        (
            "true",
            "boolean true: not an integer format, even where bool subclasses int",
        ),
        (
            "4294967296",
            "2^32: far above the 14-bit format range yet exactly representable in IEEE-754",
        ),
        ("-4294967296", "-2^32: a large negative"),
    ];
    let format_invalid_host: Vec<String> = format_invalid_host
        .into_iter()
        .map(|(value, reason)| {
            // Self check: none of these is u16-representable — which is exactly why the
            // group exists for the host languages and there is nothing to call here.
            assert!(
                value.parse::<u16>().is_err(),
                "self check: {value} must not be a valid u16"
            );
            format!(
                r#"{{"hex":{},"ss58":{},"value":{value},"reason":{}}}"#,
                js(&h),
                js(&valid_address),
                js(reason)
            )
        })
        .collect();

    // 16. Identifier values every implementation must reject at *encode* time. This crate
    // enforces the u32 range through the type system, so these are unrepresentable here and the
    // group exists for the Python and TypeScript implementations, whose runtime `AppAgentId` /
    // `TransactionalId` constructors must reject them. Rejection is the contract; the
    // classification is per-language (Python splits TypeError/ValueError along the wrong-type
    // vs wrong-value line, TypeScript raises RangeError for both), so no `error_kind`.
    let u32_invalid_encode: Vec<(&str, &str)> = vec![
        ("-1", "negative"),
        ("4294967296", "one above u32::MAX"),
        ("1.5", "not an integer"),
        ("9007199254740993", "beyond IEEE-754 exact integer range"),
        (
            "true",
            "boolean true: not an integer identifier, even where bool subclasses int",
        ),
        (
            "1099511627776",
            "2^40: far above u32::MAX yet exactly representable in IEEE-754",
        ),
        ("-4294967296", "-2^32: a large negative"),
    ];
    let u32_invalid_encode: Vec<String> = u32_invalid_encode
        .into_iter()
        .map(|(value, reason)| {
            // Self check: none of these is a u32. Parsing as u32 must fail for every one.
            assert!(
                value.parse::<u32>().is_err(),
                "self check: {value} must not be a valid u32"
            );
            format!(r#"{{"value":{value},"reason":{}}}"#, js(reason))
        })
        .collect();

    // 17. The exported constant *values*, read from this crate's own constants. The
    // behavior at the bounds is pinned by other groups (the exact-50-byte address, the
    // 2^32-1/2^32 identifiers, format 16383); this pins the constants themselves, which
    // the READMEs tell users to rely on and which every implementation exports under the
    // same name — except `U32_MAX`, which Python and TypeScript export while Rust callers
    // use the native `u32::MAX`.
    let constants: Vec<String> = [
        ("ACCOUNT_ID_LENGTH", ACCOUNT_ID_LENGTH as u64),
        ("ACCOUNT_NAME_LENGTH", ACCOUNT_NAME_LENGTH as u64),
        (
            "APP_AGENT_ACCOUNT_IDENTIFIER",
            u64::from(APP_AGENT_ACCOUNT_IDENTIFIER),
        ),
        (
            "TRANSACTIONAL_ACCOUNT_IDENTIFIER",
            u64::from(TRANSACTIONAL_ACCOUNT_IDENTIFIER),
        ),
        (
            "NAMED_ACCOUNT_IDENTIFIER",
            u64::from(NAMED_ACCOUNT_IDENTIFIER),
        ),
        (
            "SS58_FORMAT_TREVO_ASSET_HUB",
            u64::from(SS58_FORMAT_TREVO_ASSET_HUB),
        ),
        ("SS58_MAX_ADDRESS_LENGTH", SS58_MAX_ADDRESS_LENGTH as u64),
        ("U32_MAX", u64::from(u32::MAX)),
    ]
    .into_iter()
    .map(|(name, value)| format!(r#"{{"name":{},"value":{value}}}"#, js(name)))
    .collect();

    // 18. Typed-decoder mismatch vectors: decoding an account with the wrong typed
    // decoder yields KeylessDecodeError::WrongAccountType { expected, found }. The
    // (expected, found) pair is the cross-language contract (the rendered *message* is
    // per-language); pin it for every (decoder, actual-type) mismatch, including the
    // Regular fall-through.
    let mut rng = Rng(0xC0DE_000A);
    let mismatch_accounts = [
        (app_agent_account(rng.next_u32()), AccountType::AppAgent),
        (
            transactional_account(rng.next_u32(), rng.next_u32()),
            AccountType::Transactional,
        ),
        (
            named_account(rng.next_u32(), random_name(&mut rng)),
            AccountType::Named,
        ),
        (AccountId32::from([0x11u8; 32]), AccountType::Regular),
    ];
    let mut type_mismatch: Vec<String> = Vec::new();
    for (account, actual_type) in mismatch_accounts {
        assert_eq!(
            decode_account_info(&account).account_type(),
            actual_type,
            "self check: account must classify as its declared type"
        );
        for decode_as in [
            AccountType::AppAgent,
            AccountType::Transactional,
            AccountType::Named,
        ] {
            if decode_as == actual_type {
                continue;
            }
            let KeylessDecodeError::WrongAccountType { expected, found } =
                typed_decode_error(&account, decode_as);
            assert_eq!(
                expected, decode_as,
                "self check: expected matches the decoder"
            );
            assert_eq!(found, actual_type, "self check: found matches the account");
            type_mismatch.push(format!(
                r#"{{"hex":{},"ss58":{},"decode_as":{},"found":{},"address_message":{}}}"#,
                js(&hex(&account)),
                js(&account.to_ss58check()),
                js(&format!("{decode_as}")),
                js(&format!("{actual_type}")),
                js(&address_mismatch_message(decode_as, actual_type))
            ));
        }
    }

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
    print_group("hex_not_an_address", &hex_not_an_address, false);
    print_group(
        "address_not_an_account_id",
        &address_not_an_account_id,
        false,
    );
    print_group("format_invalid_encode", &format_invalid_encode, false);
    print_group("format_invalid_decode", &format_invalid_decode, false);
    print_group("format_invalid_host", &format_invalid_host, false);
    print_group("u32_invalid_encode", &u32_invalid_encode, false);
    print_group("constants", &constants, false);
    print_group("type_mismatch", &type_mismatch, true);
    println!("}}");
}
