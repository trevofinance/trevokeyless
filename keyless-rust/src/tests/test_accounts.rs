//! Tests for the `test-accounts` feature: the padded `AccountId64` / `AccountId128` wrappers
//! and their pad/unpad conversions to `AccountId32`.

use crate::*;

#[test]
fn pad_then_unpad_round_trips() {
    let core = encode_app_agent_account(AppAgentId::new(123));

    let wide64 = AccountId64::from(core);
    assert_eq!(wide64.to_account_id32(), Some(core));

    let wide128 = AccountId128::from(core);
    assert_eq!(wide128.to_account_id32(), Some(core));
}

#[test]
fn pad_writes_the_core_then_zeros() {
    let core = encode_transactional_account(TransactionalAccountIdentifiers::new(
        AppAgentId::new(7),
        TransactionalId::new(9),
    ));
    let core_bytes: [u8; 32] = core.into();

    let bytes: [u8; 64] = AccountId64::from(core).into();
    assert_eq!(&bytes[..32], &core_bytes[..]);
    assert!(bytes[32..].iter().all(|&b| b == 0));
}

#[test]
fn non_canonical_padding_does_not_unpad() {
    let core = encode_app_agent_account(AppAgentId::new(42));
    let mut bytes: [u8; 64] = AccountId64::from(core).into();
    bytes[63] = 1; // corrupt the trailing padding
    assert_eq!(AccountId64::from(bytes).to_account_id32(), None);
}

#[test]
fn wide_keyless_account_classifies_through_its_core() {
    let core = encode_named_account(NamedAccountIdentifiers::new(
        AppAgentId::new(1),
        AccountName::new(b"hot-wallet").unwrap(),
    ));
    let wide = AccountId128::from(core);

    let recovered = wide.to_account_id32().expect("canonical padding");
    assert!(is_keyless_account(&recovered));
    match decode_account_info(&recovered) {
        BlockchainAccountInfo::Named {
            app_agent_id,
            account_name,
            ..
        } => {
            assert_eq!(app_agent_id, AppAgentId::new(1));
            assert_eq!(account_name.as_str(), "hot-wallet");
        }
        other => panic!("expected Named, got {other:?}"),
    }
}

#[test]
fn arbitrary_wide_account_is_not_keyless() {
    // A general (non-keyless) substrate test account: arbitrary bytes, non-zero padding.
    let wide = AccountId64::from([7u8; 64]);
    assert_eq!(wide.to_account_id32(), None);
}

#[test]
fn default_is_all_zero_and_decodes_regular() {
    let wide = AccountId64::default();
    // All-zero trailing bytes are canonical padding, so the core is recoverable.
    let core = wide.to_account_id32().expect("zero padding is canonical");
    assert_eq!(core, AccountId32::new([0u8; 32]));
    assert!(!is_keyless_account(&core));
}

#[test]
fn display_and_from_str_round_trip() {
    let wide64 = AccountId64::from([0xABu8; 64]);
    let rendered = wide64.to_string();
    assert_eq!(rendered.len(), 2 + 128);
    assert!(rendered.starts_with("0xab"));
    assert_eq!(rendered.parse::<AccountId64>().unwrap(), wide64);

    let wide128 = AccountId128::from([0x01u8; 128]);
    assert_eq!(
        wide128.to_string().parse::<AccountId128>().unwrap(),
        wide128
    );
}

#[test]
fn from_str_enforces_the_full_width() {
    // The 64-byte type needs 128 hex chars; AccountId32's 64-char form must be rejected.
    let h = format!("0x{}", "ab".repeat(32));
    for invalid in [h.as_str(), "0x", "zz"] {
        let error = invalid.parse::<AccountId64>().unwrap_err();
        assert_eq!(error.expected_hex_characters(), 128);
        assert_eq!(
            error.to_string(),
            "Invalid hex account id: expected 0x followed by 128 hex characters"
        );
    }

    let error = "0x".parse::<AccountId128>().unwrap_err();
    assert_eq!(error.expected_hex_characters(), 256);
    assert_eq!(
        error.to_string(),
        "Invalid hex account id: expected 0x followed by 256 hex characters"
    );
}

#[cfg(feature = "scale")]
mod scale_codec {
    use crate::*;
    use parity_scale_codec::{Decode, Encode, MaxEncodedLen};

    #[test]
    fn scale_round_trip_64() {
        let wide = AccountId64::from(encode_app_agent_account(AppAgentId::new(5)));
        let encoded = wide.encode();
        assert_eq!(encoded.len(), 64);
        assert_eq!(AccountId64::decode(&mut &encoded[..]).unwrap(), wide);
        assert_eq!(AccountId64::max_encoded_len(), 64);
    }

    #[test]
    fn scale_round_trip_128() {
        let wide = AccountId128::from(encode_app_agent_account(AppAgentId::new(5)));
        let encoded = wide.encode();
        assert_eq!(encoded.len(), 128);
        assert_eq!(AccountId128::decode(&mut &encoded[..]).unwrap(), wide);
        assert_eq!(AccountId128::max_encoded_len(), 128);
    }

    /// The macro-generated widths must carry distinct `TypeInfo` identities (the
    /// keyless-rust/AGENTS.md "distinct metadata identities per width" claim): a runtime using both
    /// must expose them as two different metadata types.
    #[test]
    fn type_info_identities_are_distinct_per_width() {
        use scale_info::TypeInfo;
        let ident64 = AccountId64::type_info().path.ident();
        let ident128 = AccountId128::type_info().path.ident();
        assert_eq!(ident64, Some("AccountId64"));
        assert_eq!(ident128, Some("AccountId128"));
        assert_ne!(ident64, ident128);
    }
}

#[cfg(feature = "serde")]
mod serde_codec {
    use crate::*;
    use serde_test::{
        assert_de_tokens, assert_de_tokens_error, assert_tokens, Compact, Configure, Token,
    };

    #[test]
    fn serde_json_round_trip() {
        let wide = AccountId64::from(encode_app_agent_account(AppAgentId::new(9)));
        let json = serde_json::to_string(&wide).unwrap();
        // Serializes as the "0x"-prefixed hex string.
        assert_eq!(json, format!("\"{wide}\""));
        let back: AccountId64 = serde_json::from_str(&json).unwrap();
        assert_eq!(back, wide);
    }

    /// Into a binary format the wide ids follow the same `is_human_readable()` split as
    /// `AccountId32`: the raw bytes verbatim, accepted back both as one slice
    /// (`visit_bytes`) and element by element (`visit_seq`, how bincode delivers them).
    #[test]
    fn serde_compact_round_trips_as_raw_bytes_for_both_widths() {
        let wide64 = AccountId64::from(encode_app_agent_account(AppAgentId::new(9)));
        let bytes64: &'static [u8; 64] = Box::leak(Box::new(wide64.into()));
        assert_tokens(&wide64.compact(), &[Token::Bytes(bytes64)]);
        let mut seq64 = vec![Token::Seq { len: Some(64) }];
        seq64.extend(bytes64.iter().map(|&byte| Token::U8(byte)));
        seq64.push(Token::SeqEnd);
        assert_de_tokens(&wide64.compact(), &seq64);

        let wide128 = AccountId128::from(encode_app_agent_account(AppAgentId::new(9)));
        let bytes128: &'static [u8; 128] = Box::leak(Box::new(wide128.into()));
        assert_tokens(&wide128.compact(), &[Token::Bytes(bytes128)]);
        let mut seq128 = vec![Token::Seq { len: Some(128) }];
        seq128.extend(bytes128.iter().map(|&byte| Token::U8(byte)));
        seq128.push(Token::SeqEnd);
        assert_de_tokens(&wide128.compact(), &seq128);
    }

    /// Wrong-length binary payloads are rejected with each width's own expected count,
    /// on the slice path and the element-by-element path alike.
    #[test]
    fn serde_compact_rejects_wrong_length_payloads() {
        assert_de_tokens_error::<Compact<AccountId64>>(
            &[Token::Bytes(&[0u8; 63])],
            "invalid length 63, expected 64 raw account id bytes",
        );
        assert_de_tokens_error::<Compact<AccountId128>>(
            &[Token::Bytes(&[0u8; 129])],
            "invalid length 129, expected 128 raw account id bytes",
        );

        let mut short = vec![Token::Seq { len: Some(63) }];
        short.extend(core::iter::repeat_n(Token::U8(0), 63));
        short.push(Token::SeqEnd);
        assert_de_tokens_error::<Compact<AccountId64>>(
            &short,
            "invalid length 63, expected 64 raw account id bytes",
        );

        let mut long = vec![Token::Seq { len: Some(65) }];
        long.extend(core::iter::repeat_n(Token::U8(0), 65));
        long.push(Token::SeqEnd);
        assert_de_tokens_error::<Compact<AccountId64>>(
            &long,
            "invalid length 65, expected 64 raw account id bytes",
        );
    }
}
