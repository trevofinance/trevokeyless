use crate::keyless::*;
use crate::types::*;

#[test]
fn test_account_id32_serde() {
    let account = encode_app_agent_account(AppAgentId::new(123));

    let json = serde_json::to_string(&account).unwrap();
    assert_eq!(
        json, "\"ttowKp8AmQuGfbBGikG2pbdYNnErhHRaLrdktJeZEfJVeVnTp\"",
        "AccountId32 must serialize as its Trevo SS58 address"
    );

    let decoded: AccountId32 = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded, account);
}

#[test]
fn test_account_id32_deserialize_rejects_invalid_input() {
    // Not base58.
    assert!(serde_json::from_str::<AccountId32>("\"not an address\"").is_err());
    // Valid SS58 but not the Trevo format (Substrate generic, 42).
    assert!(serde_json::from_str::<AccountId32>(
        "\"5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY\""
    )
    .is_err());
    // Not a string.
    assert!(serde_json::from_str::<AccountId32>("42").is_err());
}

#[test]
fn test_account_name_serde() {
    let name = AccountName::new(b"example123").unwrap();

    let json = serde_json::to_string(&name).unwrap();
    assert_eq!(json, "\"example123\"");

    let decoded: AccountName = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded, name);
}

#[test]
fn test_account_name_deserialize_rejects_invalid_input() {
    // Wrong length.
    assert!(serde_json::from_str::<AccountName>("\"too-short\"").is_err());
    // Invalid character.
    assert!(serde_json::from_str::<AccountName>("\"invalid@12\"").is_err());
}

#[test]
fn test_blockchain_account_info_serde() {
    let app_agent_id = AppAgentId::new(123);
    let infos = [
        decode_account_info(&AccountId32::from([0u8; 32])),
        decode_account_info(&encode_app_agent_account(app_agent_id)),
        decode_account_info(&encode_transactional_account(
            TransactionalAccountIdentifiers::new(app_agent_id, TransactionalId::new(456)),
        )),
        decode_account_info(&encode_named_account(NamedAccountIdentifiers::new(
            app_agent_id,
            AccountName::new(b"example123").unwrap(),
        ))),
    ];

    for info in infos {
        let json = serde_json::to_string(&info).unwrap();
        let decoded: BlockchainAccountInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, info, "serde round trip must preserve {info:?}");
    }
}

#[test]
fn test_blockchain_account_info_serde_shape() {
    // Pin the documented externally tagged enum representation.
    let info = decode_account_info(&encode_named_account(NamedAccountIdentifiers::new(
        AppAgentId::new(123),
        AccountName::new(b"example123").unwrap(),
    )));
    assert_eq!(
        serde_json::to_string(&info).unwrap(),
        "{\"Named\":{\"account_id\":\"ttowKp8Amrt2FQ2eNdsbMsW2EEeEsEmsT8KHLvPhVppp9zs8k\",\
         \"app_agent_id\":123,\"account_name\":\"example123\"}}"
    );
}

#[test]
fn test_blockchain_account_info_deserialize_rejects_inconsistent_input() {
    // AppAgent claim over an account id that is not keyless at all.
    let regular = AccountId32::from([0u8; 32]).to_ss58check();
    let crafted = format!("{{\"AppAgent\":{{\"account_id\":\"{regular}\",\"app_agent_id\":999}}}}");
    assert!(
        serde_json::from_str::<BlockchainAccountInfo>(&crafted).is_err(),
        "AppAgent claim over a regular account must be rejected"
    );

    // AppAgent claim with the wrong app_agent_id for a genuine AppAgent account.
    let aa_address = encode_app_agent_account(AppAgentId::new(123)).to_ss58check();
    let crafted =
        format!("{{\"AppAgent\":{{\"account_id\":\"{aa_address}\",\"app_agent_id\":124}}}}");
    assert!(
        serde_json::from_str::<BlockchainAccountInfo>(&crafted).is_err(),
        "AppAgent claim with a mismatching app_agent_id must be rejected"
    );

    // Regular claim over an account that is actually keyless.
    let crafted = format!("{{\"Regular\":{{\"account_id\":\"{aa_address}\"}}}}");
    assert!(
        serde_json::from_str::<BlockchainAccountInfo>(&crafted).is_err(),
        "Regular claim over a keyless account must be rejected"
    );

    // Named claim with a valid name that differs from the embedded one.
    let named_address = encode_named_account(NamedAccountIdentifiers::new(
        AppAgentId::new(123),
        AccountName::new(b"example123").unwrap(),
    ))
    .to_ss58check();
    let crafted = format!(
        "{{\"Named\":{{\"account_id\":\"{named_address}\",\"app_agent_id\":123,\
         \"account_name\":\"example124\"}}}}"
    );
    assert!(
        serde_json::from_str::<BlockchainAccountInfo>(&crafted).is_err(),
        "Named claim with a mismatching name must be rejected"
    );

    // Transactional claim with swapped ids.
    let ta_address = encode_transactional_account(TransactionalAccountIdentifiers::new(
        AppAgentId::new(123),
        TransactionalId::new(456),
    ))
    .to_ss58check();
    let crafted = format!(
        "{{\"Transactional\":{{\"account_id\":\"{ta_address}\",\"app_agent_id\":456,\
         \"ta_id\":123}}}}"
    );
    assert!(
        serde_json::from_str::<BlockchainAccountInfo>(&crafted).is_err(),
        "Transactional claim with swapped ids must be rejected"
    );
}

#[test]
fn test_account_name_error_serde() {
    // The wire shape is pinned (not just round-tripped): a variant rename would be a
    // serde wire break even though a round trip alone would still pass.
    for (error, json) in [
        (AccountNameError::InvalidLength, "\"InvalidLength\""),
        (AccountNameError::InvalidCharacter, "\"InvalidCharacter\""),
    ] {
        assert_eq!(serde_json::to_string(&error).unwrap(), json);
        let decoded: AccountNameError = serde_json::from_str(json).unwrap();
        assert_eq!(decoded, error);
    }
}

#[test]
fn test_keyless_decode_error_serde() {
    // The wire shape (derived externally tagged enum with unit-variant AccountType
    // fields) is pinned, not just round-tripped.
    for (error, json) in [
        (
            KeylessDecodeError::WrongAccountType {
                expected: AccountType::AppAgent,
                found: AccountType::Regular,
            },
            "{\"WrongAccountType\":{\"expected\":\"AppAgent\",\"found\":\"Regular\"}}",
        ),
        (
            KeylessDecodeError::WrongAccountType {
                expected: AccountType::Transactional,
                found: AccountType::AppAgent,
            },
            "{\"WrongAccountType\":{\"expected\":\"Transactional\",\"found\":\"AppAgent\"}}",
        ),
        (
            KeylessDecodeError::WrongAccountType {
                expected: AccountType::Named,
                found: AccountType::Transactional,
            },
            "{\"WrongAccountType\":{\"expected\":\"Named\",\"found\":\"Transactional\"}}",
        ),
    ] {
        assert_eq!(serde_json::to_string(&error).unwrap(), json);
        let decoded: KeylessDecodeError = serde_json::from_str(json).unwrap();
        assert_eq!(decoded, error);
    }
}

#[test]
fn test_account_type_serde() {
    // The wire shape (the variant name string) is pinned, not just round-tripped.
    for (account_type, json) in [
        (AccountType::Regular, "\"Regular\""),
        (AccountType::AppAgent, "\"AppAgent\""),
        (AccountType::Transactional, "\"Transactional\""),
        (AccountType::Named, "\"Named\""),
    ] {
        assert_eq!(serde_json::to_string(&account_type).unwrap(), json);
        let decoded: AccountType = serde_json::from_str(json).unwrap();
        assert_eq!(decoded, account_type);
    }
}

#[test]
fn test_decoded_account_structs_serde() {
    // The derived flat-struct wire shapes of the two typed-decoder result types.
    let transactional = TransactionalAccountIdentifiers {
        app_agent_id: AppAgentId::new(123),
        ta_id: TransactionalId::new(456),
    };
    let json = serde_json::to_string(&transactional).unwrap();
    assert_eq!(json, "{\"app_agent_id\":123,\"ta_id\":456}");
    let decoded: TransactionalAccountIdentifiers = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded, transactional);

    let named = NamedAccountIdentifiers {
        app_agent_id: AppAgentId::new(123),
        account_name: AccountName::new(b"example123").unwrap(),
    };
    let json = serde_json::to_string(&named).unwrap();
    assert_eq!(
        json,
        "{\"app_agent_id\":123,\"account_name\":\"example123\"}"
    );
    let decoded: NamedAccountIdentifiers = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded, named);

    // The name inside NamedAccountIdentifiers deserializes through the validating
    // AccountName deserializer.
    assert!(serde_json::from_str::<NamedAccountIdentifiers>(
        "{\"app_agent_id\":123,\"account_name\":\"invalid@12\"}"
    )
    .is_err());
}

/// The serde representation depends on the format: a human-readable format (JSON, YAML...)
/// gets the SS58 address string a person expects to read; a binary format (bincode,
/// postcard, MessagePack...) gets the raw account-id bytes.
///
/// Before this split, `AccountId32` serialized as a 49-character base58 string into *every*
/// format - four times the size of the bytes, plus a base58 encode and a Blake2b checksum
/// per value, in formats no human ever reads. `serde_test`'s `Configure` gives both modes:
/// `.readable()` sets `is_human_readable()` to true, `.compact()` sets it to false.
mod tests_human_readable {
    use crate::keyless::*;
    use crate::types::*;
    use alloc::boxed::Box;
    use serde_test::{
        assert_de_tokens, assert_de_tokens_error, assert_tokens, Compact, Configure, Token,
    };

    /// `serde_test::Token` borrows for `'static`, so the derived values are leaked. This is
    /// a test; the process is about to end.
    fn leak_str(value: alloc::string::String) -> &'static str {
        Box::leak(value.into_boxed_str())
    }

    fn leak_bytes(value: [u8; ACCOUNT_ID_LENGTH]) -> &'static [u8] {
        Box::leak(Box::new(value))
    }

    #[test]
    fn test_account_id32_is_ss58_when_readable_and_raw_bytes_when_not() {
        for id in [0u32, 1, 123, u32::MAX] {
            let account = encode_app_agent_account(AppAgentId::new(id));
            let address = Token::Str(leak_str(account.to_ss58check()));
            let raw = Token::Bytes(leak_bytes(*account.as_bytes()));

            // Human-readable: the SS58 address, exactly as the Python and TypeScript
            // libraries render it.
            assert_tokens(&account.readable(), &[address]);

            // Binary: the 32 account-id bytes verbatim - no address envelope, no base58,
            // no SS58 checksum.
            assert_tokens(&account.compact(), &[raw]);
        }
    }

    /// The JSON round trip goes through the readable path and is unchanged by the split.
    #[test]
    fn test_json_still_round_trips_through_the_address() {
        let account = encode_app_agent_account(AppAgentId::new(123));
        let json = serde_json::to_string(&account).unwrap();
        assert_eq!(json, alloc::format!("\"{}\"", account.to_ss58check()));
        assert_eq!(serde_json::from_str::<AccountId32>(&json).unwrap(), account);
    }

    /// A binary payload of the wrong length is rejected, not silently truncated or padded.
    #[test]
    fn test_compact_rejects_a_wrong_length_payload() {
        // `Compact<T>` is how serde_test drives the non-human-readable path.
        assert_de_tokens_error::<Compact<AccountId32>>(
            &[Token::Bytes(&[0u8; 31])],
            "Account id must be exactly 32 bytes long, got 31",
        );
        assert_de_tokens_error::<Compact<AccountId32>>(
            &[Token::Bytes(&[0u8; 33])],
            "Account id must be exactly 32 bytes long, got 33",
        );
    }

    /// Several binary formats (bincode among them) hand a byte payload to the visitor
    /// element by element rather than as one borrowed slice — the `visit_seq` path, which
    /// `Token::Bytes` never exercises. Drive it directly with sequence tokens.
    #[test]
    fn test_compact_accepts_an_element_by_element_byte_sequence() {
        let account = encode_app_agent_account(AppAgentId::new(123));
        let mut tokens = alloc::vec![Token::Seq {
            len: Some(ACCOUNT_ID_LENGTH),
        }];
        tokens.extend(account.as_bytes().iter().map(|&byte| Token::U8(byte)));
        tokens.push(Token::SeqEnd);
        assert_de_tokens(&account.compact(), &tokens);
    }

    /// The element-by-element path rejects wrong lengths too: one element short reports
    /// the index the sequence ended at, one element long reports 33 (the visitor checks
    /// for a trailing element after reading 32).
    #[test]
    fn test_compact_rejects_a_wrong_length_byte_sequence() {
        let mut short = alloc::vec![Token::Seq { len: Some(31) }];
        short.extend(core::iter::repeat_n(Token::U8(0), 31));
        short.push(Token::SeqEnd);
        assert_de_tokens_error::<Compact<AccountId32>>(
            &short,
            "invalid length 31, expected 32 raw account id bytes",
        );

        let mut long = alloc::vec![Token::Seq { len: Some(33) }];
        long.extend(core::iter::repeat_n(Token::U8(0), 33));
        long.push(Token::SeqEnd);
        assert_de_tokens_error::<Compact<AccountId32>>(
            &long,
            "invalid length 33, expected 32 raw account id bytes",
        );
    }

    /// `BlockchainAccountInfo` carries an `AccountId32`, so it inherits the split - and its
    /// validating deserializer (which re-classifies the bytes) must keep working in both.
    #[test]
    fn test_blockchain_account_info_round_trips_in_both_formats() {
        let account = encode_named_account(NamedAccountIdentifiers::new(
            AppAgentId::new(7),
            AccountName::new(b"hot-wallet").unwrap(),
        ));
        let info = decode_account_info(&account);

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains(&account.to_ss58check()));
        assert_eq!(
            serde_json::from_str::<BlockchainAccountInfo>(&json).unwrap(),
            info
        );

        // The binary path: the same externally tagged enum, but over the raw account-id
        // bytes (no address envelope), through the derived representation in both
        // directions — including the validating deserializer.
        assert_tokens(
            &info.compact(),
            &[
                Token::StructVariant {
                    name: "BlockchainAccountInfo",
                    variant: "Named",
                    len: 3,
                },
                Token::Str("account_id"),
                Token::Bytes(leak_bytes(*account.as_bytes())),
                Token::Str("app_agent_id"),
                Token::U32(7),
                Token::Str("account_name"),
                Token::Str("hot-wallet"),
                Token::StructVariantEnd,
            ],
        );
    }

    /// The validating deserializer holds on the binary path too: identifiers that do not
    /// match a re-classification of `account_id` are rejected, not silently accepted.
    #[test]
    fn test_compact_blockchain_account_info_rejects_inconsistent_identifiers() {
        let account = encode_named_account(NamedAccountIdentifiers::new(
            AppAgentId::new(7),
            AccountName::new(b"hot-wallet").unwrap(),
        ));
        assert_de_tokens_error::<Compact<BlockchainAccountInfo>>(
            &[
                Token::StructVariant {
                    name: "BlockchainAccountInfo",
                    variant: "Named",
                    len: 3,
                },
                Token::Str("account_id"),
                Token::Bytes(leak_bytes(*account.as_bytes())),
                // Claims App Agent 8 over an account whose bytes embed 7.
                Token::Str("app_agent_id"),
                Token::U32(8),
                Token::Str("account_name"),
                Token::Str("hot-wallet"),
                Token::StructVariantEnd,
            ],
            "inconsistent BlockchainAccountInfo: the identifiers do not match account_id",
        );
    }
}
