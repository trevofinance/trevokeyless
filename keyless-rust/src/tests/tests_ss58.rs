use crate::keyless::*;
use crate::ss58::*;
use crate::types::*;

// Alice's well-known development account.
const ALICE_HEX: &str = "d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d";
const ALICE_TREVO: &str = "ttqxHzRJmmjFBcE7Lb5Xs4GNMq2gFSt28JyvTEqjhqzE9EGP4";
const ALICE_SUBSTRATE_GENERIC: &str = "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY";

fn alice() -> AccountId32 {
    let bytes: [u8; 32] = array_bytes::Dehexify::dehexify(ALICE_HEX).unwrap();
    AccountId32::from(bytes)
}

#[test]
fn test_to_ss58check_trevo_format() {
    // Vectors shared with the Python and TypeScript test suites.
    assert_eq!(
        encode_app_agent_account(123).to_ss58check(),
        "ttowKp8AmQuGfbBGikG2pbdYNnErhHRaLrdktJeZEfJVeVnTp"
    );
    assert_eq!(
        encode_transactional_account(123, 456).to_ss58check(),
        "ttowKp8AmjjQh4GoN7xMiQWwVyyrU1Pu7GRf5HxFmV5t43TXG"
    );
    assert_eq!(
        encode_named_account(123, AccountName::new(b"example123").unwrap()).to_ss58check(),
        "ttowKp8Amrt2FQ2eNdsbMsW2EEeEsEmsT8KHLvPhVppp9zs8k"
    );
    assert_eq!(alice().to_ss58check(), ALICE_TREVO);
}

#[test]
fn test_to_ss58check_with_version_single_byte_prefix() {
    // Format 42 (Substrate generic) uses a single-byte prefix.
    assert_eq!(
        alice().to_ss58check_with_version(42).unwrap(),
        ALICE_SUBSTRATE_GENERIC
    );
}

#[test]
fn test_from_ss58check() {
    assert_eq!(AccountId32::from_ss58check(ALICE_TREVO).unwrap(), alice());

    let account = encode_app_agent_account(123);
    assert_eq!(
        AccountId32::from_ss58check(&account.to_ss58check()).unwrap(),
        account
    );
}

#[test]
fn test_from_ss58check_with_version() {
    assert_eq!(
        AccountId32::from_ss58check_with_version(ALICE_TREVO).unwrap(),
        (alice(), SS58_FORMAT_TREVO_ASSET_HUB)
    );
    assert_eq!(
        AccountId32::from_ss58check_with_version(ALICE_SUBSTRATE_GENERIC).unwrap(),
        (alice(), 42)
    );
}

#[test]
fn test_round_trip_format_boundaries() {
    // Cover both prefix encodings: single-byte (0, 63) and two-byte (64, 16383).
    // Formats 46 and 47 are reserved and tested separately.
    let account = encode_app_agent_account(u32::MAX);
    for format in [0u16, 1, 42, 63, 64, 5335, 16383] {
        let address = account.to_ss58check_with_version(format).unwrap();
        assert_eq!(
            AccountId32::from_ss58check_with_version(&address).unwrap(),
            (account, format),
            "round trip must preserve account and format {format}"
        );
    }
}

#[test]
fn test_encode_invalid_format() {
    let account = alice();
    assert_eq!(
        account.to_ss58check_with_version(46).unwrap_err(),
        Ss58Error::InvalidFormat,
        "format 46 is reserved"
    );
    assert_eq!(
        account.to_ss58check_with_version(47).unwrap_err(),
        Ss58Error::InvalidFormat,
        "format 47 is reserved"
    );
    assert_eq!(
        account.to_ss58check_with_version(16384).unwrap_err(),
        Ss58Error::InvalidFormat,
        "formats above 16383 do not fit the 14-bit prefix"
    );
}

#[test]
fn test_decode_invalid_base58() {
    // '0', 'O', 'I', and 'l' are not in the base58 alphabet.
    assert_eq!(
        AccountId32::from_ss58check_with_version("0OIl").unwrap_err(),
        Ss58Error::InvalidBase58
    );
}

#[test]
fn test_decode_invalid_length() {
    assert_eq!(
        AccountId32::from_ss58check_with_version("").unwrap_err(),
        Ss58Error::InvalidLength
    );

    // A valid base58 payload that is too short to contain an account id.
    let short = bs58::encode([42u8; 10]).into_string();
    assert_eq!(
        AccountId32::from_ss58check_with_version(&short).unwrap_err(),
        Ss58Error::InvalidLength
    );

    // A valid base58 payload that is longer than prefix + account id + checksum.
    let long = bs58::encode([42u8; 40]).into_string();
    assert_eq!(
        AccountId32::from_ss58check_with_version(&long).unwrap_err(),
        Ss58Error::InvalidLength
    );
}

#[test]
fn test_decode_lone_two_byte_prefix_byte() {
    // A single byte in 64..=127 announces a two-byte prefix but the second byte is
    // missing; the decoder must report InvalidLength rather than read past the payload.
    let address = bs58::encode([64u8]).into_string();
    assert_eq!(
        AccountId32::from_ss58check_with_version(&address).unwrap_err(),
        Ss58Error::InvalidLength
    );
}

#[test]
fn test_max_address_length_bound_is_50() {
    // The reference value of the bound, exported as-is by the Python and TypeScript
    // implementations (whose suites pin their own copy against this same literal).
    // Keeping the three in lockstep guarantees the oversized-input rejection tests
    // exercise the same boundary in every language.
    assert_eq!(SS58_MAX_ADDRESS_LENGTH, 50);
}

#[test]
fn test_decode_rejects_oversized_strings_before_base58_decoding() {
    // Strings longer than SS58_MAX_ADDRESS_LENGTH cannot be valid addresses and are
    // rejected up front, keeping the base58 decoding cost bounded on untrusted input.
    let oversized = "1".repeat(SS58_MAX_ADDRESS_LENGTH + 1);
    assert_eq!(
        AccountId32::from_ss58check_with_version(&oversized).unwrap_err(),
        Ss58Error::InvalidLength
    );

    // The longest legitimate addresses (two-byte prefix, high account id bytes) stay
    // within the bound and still decode.
    let account = AccountId32::new([0xFF; 32]);
    let address = account.to_ss58check_with_version(16383).unwrap();
    assert!(
        address.len() <= SS58_MAX_ADDRESS_LENGTH,
        "a legitimate address must never exceed the bound"
    );
    assert_eq!(
        AccountId32::from_ss58check_with_version(&address).unwrap(),
        (account, 16383)
    );
}

#[test]
fn test_decode_rejects_surrounding_whitespace() {
    // An address with leading or trailing whitespace is not the canonical spelling of
    // the account: any whitespace is outside the base58 alphabet and must be rejected.
    // (Pinned cross-language by the `ss58_invalid` conformance vectors; permissive
    // decoders such as python base58 historically right-stripped their input.)
    let address = encode_app_agent_account(123).to_ss58check();
    for spelling in [
        format!("{address} "),
        format!(" {address}"),
        format!("{address}\n"),
        format!("{address}\t"),
    ] {
        assert_eq!(
            AccountId32::from_ss58check_with_version(&spelling).unwrap_err(),
            Ss58Error::InvalidBase58,
            "{spelling:?} must be rejected"
        );
    }
}

#[test]
fn test_decode_invalid_checksum() {
    let mut payload = bs58::decode(ALICE_TREVO).into_vec().unwrap();
    let last = payload.len() - 1;
    payload[last] ^= 0xFF;
    let tampered = bs58::encode(payload).into_string();
    assert_eq!(
        AccountId32::from_ss58check_with_version(&tampered).unwrap_err(),
        Ss58Error::InvalidChecksum
    );
}

#[test]
fn test_decode_invalid_prefix() {
    // Prefix bytes 128..=255 are not a valid SS58 prefix encoding.
    let mut payload = vec![0x80u8];
    payload.extend_from_slice(alice().as_ref() as &[u8; 32]);
    payload.extend_from_slice(&[0, 0]);
    let address = bs58::encode(payload).into_string();
    assert_eq!(
        AccountId32::from_ss58check_with_version(&address).unwrap_err(),
        Ss58Error::InvalidFormat
    );
}

#[test]
fn test_decode_reserved_format() {
    // Build an address with reserved format 46 and a valid checksum, bypassing the
    // encoder's validation.
    let mut payload = vec![46u8];
    payload.extend_from_slice(alice().as_ref() as &[u8; 32]);
    let checksum = blake2b_simd::Params::new()
        .hash_length(64)
        .to_state()
        .update(b"SS58PRE")
        .update(&payload)
        .finalize();
    payload.extend_from_slice(&checksum.as_bytes()[0..2]);
    let address = bs58::encode(payload).into_string();
    assert_eq!(
        AccountId32::from_ss58check_with_version(&address).unwrap_err(),
        Ss58Error::InvalidFormat
    );
}

#[test]
fn test_decode_two_byte_prefix_of_small_format() {
    // The two-byte SS58 prefix can also (non-canonically) encode formats below 64. Like
    // the upstream Substrate codec, decoding accepts such addresses; this test pins that
    // behavior: the result is identical to decoding the canonical single-byte form, even
    // though the address strings differ.
    let format: u16 = 42;
    let mut payload = vec![
        (((format & 0b0000_0000_1111_1100) >> 2) | 0b0100_0000) as u8,
        ((format >> 8) | ((format & 0b0000_0000_0000_0011) << 6)) as u8,
    ];
    payload.extend_from_slice(alice().as_ref() as &[u8; 32]);
    let checksum = blake2b_simd::Params::new()
        .hash_length(64)
        .to_state()
        .update(b"SS58PRE")
        .update(&payload)
        .finalize();
    payload.extend_from_slice(&checksum.as_bytes()[0..2]);
    let address = bs58::encode(payload).into_string();

    assert_ne!(
        address, ALICE_SUBSTRATE_GENERIC,
        "the two-byte form is a distinct address string"
    );
    assert_eq!(
        AccountId32::from_ss58check_with_version(&address).unwrap(),
        (alice(), format)
    );
}

#[test]
fn test_from_ss58check_rejects_unexpected_format() {
    assert_eq!(
        AccountId32::from_ss58check(ALICE_SUBSTRATE_GENERIC).unwrap_err(),
        Ss58Error::UnexpectedFormat {
            expected: SS58_FORMAT_TREVO_ASSET_HUB,
            found: 42
        }
    );
}

#[test]
fn test_from_ss58check_expecting() {
    assert_eq!(
        AccountId32::from_ss58check_expecting(ALICE_TREVO, SS58_FORMAT_TREVO_ASSET_HUB).unwrap(),
        alice()
    );
    assert_eq!(
        AccountId32::from_ss58check_expecting(ALICE_SUBSTRATE_GENERIC, 42).unwrap(),
        alice()
    );
    // A fully valid address of another format: the one rejection that reports the
    // expected/found pair.
    assert_eq!(
        AccountId32::from_ss58check_expecting(ALICE_SUBSTRATE_GENERIC, SS58_FORMAT_TREVO_ASSET_HUB)
            .unwrap_err(),
        Ss58Error::UnexpectedFormat {
            expected: SS58_FORMAT_TREVO_ASSET_HUB,
            found: 42
        }
    );
    assert_eq!(
        AccountId32::from_ss58check_expecting(ALICE_TREVO, 42).unwrap_err(),
        Ss58Error::UnexpectedFormat {
            expected: 42,
            found: SS58_FORMAT_TREVO_ASSET_HUB
        }
    );
}

#[test]
fn test_from_ss58check_expecting_validation_wins_over_format_mismatch() {
    // A corrupt address of a non-expected format must report its validation error,
    // never UnexpectedFormat: the format carried by an address that fails
    // validation is not trustworthy information (also pinned by the conformance
    // fixture's ss58_invalid vectors).
    let mut payload = bs58::decode(ALICE_SUBSTRATE_GENERIC).into_vec().unwrap();
    let last = payload.len() - 1;
    payload[last] ^= 0xFF;
    let tampered = bs58::encode(payload).into_string();
    assert_eq!(
        AccountId32::from_ss58check_expecting(&tampered, SS58_FORMAT_TREVO_ASSET_HUB).unwrap_err(),
        Ss58Error::InvalidChecksum
    );
}

#[test]
fn test_from_ss58check_expecting_validates_the_expected_format_parameter() {
    // An expected format outside the 14-bit range is rejected up front; the
    // reserved formats 46/47 are accepted as a parameter (no valid address can
    // carry them, so a valid address of another format reports the mismatch).
    assert_eq!(
        AccountId32::from_ss58check_expecting(ALICE_TREVO, 16384).unwrap_err(),
        Ss58Error::InvalidFormat
    );
    assert_eq!(
        AccountId32::from_ss58check_expecting(ALICE_TREVO, 46).unwrap_err(),
        Ss58Error::UnexpectedFormat {
            expected: 46,
            found: SS58_FORMAT_TREVO_ASSET_HUB
        }
    );
}

#[test]
fn test_ss58_error_display() {
    assert_eq!(
        format!("{}", Ss58Error::InvalidBase58),
        "Address is not valid base58"
    );
    assert_eq!(
        format!("{}", Ss58Error::InvalidLength),
        "Address has an invalid length"
    );
    assert_eq!(
        format!("{}", Ss58Error::InvalidChecksum),
        "Address has an invalid checksum"
    );
    assert_eq!(
        format!("{}", Ss58Error::InvalidFormat),
        "SS58 format is invalid or reserved"
    );
    assert_eq!(
        format!(
            "{}",
            Ss58Error::UnexpectedFormat {
                expected: 5335,
                found: 42
            }
        ),
        "Expected SS58 format 5335 but found 42"
    );
}
