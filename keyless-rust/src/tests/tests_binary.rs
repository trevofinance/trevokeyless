use crate::keyless::*;
use crate::types::*;

/// BLAKE2b-256 known-answer test. The expected digests come from an independent
/// implementation (Python's `hashlib.blake2b(..., digest_size=32)`) and are the
/// canonical unkeyed BLAKE2b-256 vectors for the empty input and `"abc"`. This pins
/// the hash primitive the entire keyless encoding is built on, so a future
/// `blake2b_simd` change (or a wrong parameterization) cannot silently shift every
/// account id.
#[test]
fn test_blake2_256_known_answers() {
    assert_eq!(
        blake2_256(b""),
        <[u8; 32] as array_bytes::Dehexify>::dehexify(
            "0e5751c026e543b2e8ab2eb06099daa1d1e5df47778f7787faab45cdf12fe3a8"
        )
        .unwrap()
    );
    assert_eq!(
        blake2_256(b"abc"),
        <[u8; 32] as array_bytes::Dehexify>::dehexify(
            "bddd813c634239723171ef3fee98579b94964e3bb1cb3e427262c8c068d52319"
        )
        .unwrap()
    );
}

#[test]
fn test_app_agent_structure() {
    let account = encode_app_agent_account(AppAgentId::new(123456789));
    let account: [u8; 32] = account.into();

    // Decimal 123456789 == little-endian bytes int 21, 205, 91, 7 == little-endian bytes HEX 15 CD 5B 07
    assert_eq!(
        account[0..=3],
        [21, 205, 91, 7],
        "Bytes [0..=3] must contain AppAgent ID in little-endian format"
    );
    assert_eq!(
        account[4], APP_AGENT_ACCOUNT_IDENTIFIER,
        "5th byte must be account type"
    );

    let checksum_account: [u8; 27] = account[5..].try_into().unwrap();

    let checksum_calculated: [u8; 27] = blake2_256(&[21, 205, 91, 7, APP_AGENT_ACCOUNT_IDENTIFIER])
        [5..]
        .try_into()
        .unwrap();
    assert_eq!(
        checksum_account, checksum_calculated,
        "Bytes after open part must contain checksum"
    );

    let checksum_hardcoded: [u8; 27] = [
        175, 178, 115, 98, 21, 35, 133, 141, 82, 196, 31, 130, 211, 175, 169, 172, 211, 234, 155,
        159, 169, 105, 60, 177, 233, 238, 159,
    ];
    assert_eq!(
        checksum_account, checksum_hardcoded,
        "Bytes after open part must contain checksum"
    );
}

#[test]
fn test_transactional_structure() {
    let account = encode_transactional_account(TransactionalAccountIdentifiers::new(
        AppAgentId::new(123456789),
        TransactionalId::new(987654321),
    ));
    let account: [u8; 32] = account.into();

    // Decimal 123456789 == little-endian bytes int 21, 205, 91, 7 == little-endian bytes HEX 15 CD 5B 07
    assert_eq!(
        account[0..=3],
        [21, 205, 91, 7],
        "Bytes [0..=3] must contain AppAgent ID in little-endian format"
    );
    assert_eq!(
        account[4], TRANSACTIONAL_ACCOUNT_IDENTIFIER,
        "5th byte must be account type"
    );
    // Decimal 987654321 == little-endian bytes int 177, 104, 222, 58 == little-endian bytes HEX B1 68 DE 3A
    assert_eq!(
        account[5..=8],
        [177, 104, 222, 58],
        "Bytes [5..=8] must contain Transactional account ID in little-endian format"
    );

    let checksum_account: [u8; 23] = account[9..].try_into().unwrap();

    let checksum_calculated: [u8; 23] = blake2_256(&[
        21,
        205,
        91,
        7,
        TRANSACTIONAL_ACCOUNT_IDENTIFIER,
        177,
        104,
        222,
        58,
    ])[9..]
        .try_into()
        .unwrap();
    assert_eq!(
        checksum_account, checksum_calculated,
        "Bytes after open part must contain checksum"
    );

    let checksum_hardcoded: [u8; 23] = [
        88, 124, 158, 192, 150, 58, 55, 138, 138, 202, 89, 248, 2, 227, 69, 15, 236, 185, 227, 19,
        61, 115, 11,
    ];
    assert_eq!(
        checksum_account, checksum_hardcoded,
        "Bytes after open part must contain checksum"
    );
}

#[test]
fn test_named_structure() {
    let account = encode_named_account(NamedAccountIdentifiers::new(
        AppAgentId::new(123456789),
        AccountName::new(b"test123456").unwrap(),
    ));
    let account: [u8; 32] = account.into();

    // Decimal 123456789 == little-endian bytes int 21, 205, 91, 7 == little-endian bytes HEX 15 CD 5B 07
    assert_eq!(
        account[0..=3],
        [21, 205, 91, 7],
        "Bytes [0..=3] must contain AppAgent ID in little-endian format"
    );
    assert_eq!(
        account[4], NAMED_ACCOUNT_IDENTIFIER,
        "5th byte must be account type"
    );
    assert_eq!(
        account[5..=14],
        [116, 101, 115, 116, 49, 50, 51, 52, 53, 54],
        "Bytes [5..=14] must contain Account name"
    );
    assert_eq!(
        &account[5..=14],
        b"test123456",
        "Bytes [5..=14] must contain Account name"
    );

    let checksum_account: [u8; 17] = account[15..].try_into().unwrap();

    let checksum_calculated: [u8; 17] = blake2_256(&[
        21,
        205,
        91,
        7,
        NAMED_ACCOUNT_IDENTIFIER,
        116,
        101,
        115,
        116,
        49,
        50,
        51,
        52,
        53,
        54,
    ])[15..]
        .try_into()
        .unwrap();
    assert_eq!(
        checksum_account, checksum_calculated,
        "Bytes after open part must contain checksum"
    );

    let checksum_hardcoded: [u8; 17] = [
        196, 27, 218, 140, 168, 186, 21, 248, 133, 112, 151, 83, 20, 219, 42, 153, 62,
    ];
    assert_eq!(
        checksum_account, checksum_hardcoded,
        "Bytes after open part must contain checksum"
    );
}
