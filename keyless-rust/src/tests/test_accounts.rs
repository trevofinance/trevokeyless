//! Tests for the `test-accounts` feature: the padded `AccountId64` / `AccountId128` wrappers
//! and their pad/unpad conversions to `AccountId32`.

use crate::*;

#[test]
fn pad_then_unpad_round_trips() {
    let core = encode_app_agent_account(123);

    let wide64 = AccountId64::from(core);
    assert_eq!(wide64.to_account_id32(), Some(core));

    let wide128 = AccountId128::from(core);
    assert_eq!(wide128.to_account_id32(), Some(core));
}

#[test]
fn pad_writes_the_core_then_zeros() {
    let core = encode_transactional_account(7, 9);
    let core_bytes: [u8; 32] = core.into();

    let bytes: [u8; 64] = AccountId64::from(core).into();
    assert_eq!(&bytes[..32], &core_bytes[..]);
    assert!(bytes[32..].iter().all(|&b| b == 0));
}

#[test]
fn non_canonical_padding_does_not_unpad() {
    let core = encode_app_agent_account(42);
    let mut bytes: [u8; 64] = AccountId64::from(core).into();
    bytes[63] = 1; // corrupt the trailing padding
    assert_eq!(AccountId64::from(bytes).to_account_id32(), None);
}

#[test]
fn wide_keyless_account_classifies_through_its_core() {
    let core = encode_named_account(1, AccountName::new(b"hot-wallet").unwrap());
    let wide = AccountId128::from(core);

    let recovered = wide.to_account_id32().expect("canonical padding");
    assert!(is_keyless_account(&recovered));
    match decode_account_info(&recovered) {
        BlockchainAccountInfo::Named {
            app_agent_id,
            account_name,
            ..
        } => {
            assert_eq!(app_agent_id, 1);
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
    assert!(h.parse::<AccountId64>().is_err());
    assert!("0x".parse::<AccountId64>().is_err());
    assert!("zz".parse::<AccountId64>().is_err());
}

#[cfg(feature = "scale")]
mod scale_codec {
    use crate::*;
    use parity_scale_codec::{Decode, Encode, MaxEncodedLen};

    #[test]
    fn scale_round_trip_64() {
        let wide = AccountId64::from(encode_app_agent_account(5));
        let encoded = wide.encode();
        assert_eq!(encoded.len(), 64);
        assert_eq!(AccountId64::decode(&mut &encoded[..]).unwrap(), wide);
        assert_eq!(AccountId64::max_encoded_len(), 64);
    }

    #[test]
    fn scale_round_trip_128() {
        let wide = AccountId128::from(encode_app_agent_account(5));
        let encoded = wide.encode();
        assert_eq!(encoded.len(), 128);
        assert_eq!(AccountId128::decode(&mut &encoded[..]).unwrap(), wide);
        assert_eq!(AccountId128::max_encoded_len(), 128);
    }
}

#[cfg(feature = "serde")]
mod serde_codec {
    use crate::*;

    #[test]
    fn serde_json_round_trip() {
        let wide = AccountId64::from(encode_app_agent_account(9));
        let json = serde_json::to_string(&wide).unwrap();
        // Serializes as the "0x"-prefixed hex string.
        assert_eq!(json, format!("\"{wide}\""));
        let back: AccountId64 = serde_json::from_str(&json).unwrap();
        assert_eq!(back, wide);
    }
}
