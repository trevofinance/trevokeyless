//! Tests for the `types` module: `AccountName`, `AccountId32` display and parsing,
//! and the SCALE codec implementations (the `scale`-gated module below).

mod tests_account_name {
    use crate::types::*;

    #[test]
    fn test_valid_account_name() {
        let valid_account = b"abcdEF1234";
        assert!(AccountName::new(valid_account).is_ok());
    }

    #[test]
    fn test_invalid_length() {
        let short_account = b"abcdEF123";
        let long_account = b"abcdEF12345";
        assert_eq!(
            AccountName::new(short_account).unwrap_err(),
            AccountNameError::InvalidLength
        );
        assert_eq!(
            AccountName::new(long_account).unwrap_err(),
            AccountNameError::InvalidLength
        );
    }

    #[test]
    fn test_invalid_characters() {
        let invalid_account1 = b"abcdEF@123";
        let invalid_account2 = b"abcdEF*123";
        let invalid_account3 = b"abcdEF/123";
        assert_eq!(
            AccountName::new(invalid_account1).unwrap_err(),
            AccountNameError::InvalidCharacter
        );
        assert_eq!(
            AccountName::new(invalid_account2).unwrap_err(),
            AccountNameError::InvalidCharacter
        );
        assert_eq!(
            AccountName::new(invalid_account3).unwrap_err(),
            AccountNameError::InvalidCharacter
        );
    }

    #[test]
    fn test_edge_cases() {
        let valid_account1 = b"0000000000";
        let valid_account2 = b"##########";
        let valid_account3 = b"abcdEFghij";
        let valid_account4 = b"1234567890";

        assert!(AccountName::new(valid_account1).is_ok());
        assert!(AccountName::new(valid_account2).is_ok());
        assert!(AccountName::new(valid_account3).is_ok());
        assert!(AccountName::new(valid_account4).is_ok());
    }

    #[test]
    fn test_try_from_owned_array() {
        let arr: [u8; 10] = *b"hot-wallet";
        let name = AccountName::try_from(arr).unwrap();
        assert_eq!(name.as_bytes(), b"hot-wallet");

        let invalid: [u8; 10] = *b"invalid\x00xx";
        assert!(AccountName::try_from(invalid).is_err());
    }

    #[test]
    fn test_from_str() {
        let name: AccountName = "hot-wallet".parse().unwrap();
        assert_eq!(name.as_bytes(), b"hot-wallet");

        assert!("too-short".parse::<AccountName>().is_err());
        assert!("invalid@xxx".parse::<AccountName>().is_err());
    }

    #[test]
    fn test_into_array() {
        let name = AccountName::new(b"hot-wallet").unwrap();
        let arr: [u8; 10] = name.into();
        assert_eq!(&arr, b"hot-wallet");
    }
}

mod tests_display {
    use crate::types::*;

    #[test]
    fn test_account_id32_display() {
        // The Display form is the cross-language raw interchange form: "0x" + 64
        // lowercase hex characters, accepted verbatim by the Python decode_account_id
        // and TypeScript decodeAccountId entry points.
        let account = AccountId32::new([0u8; 32]);
        assert_eq!(format!("{}", account), format!("0x{}", "0".repeat(64)));

        let mut bytes = [0u8; 32];
        bytes[0] = 0xAB;
        bytes[31] = 0xCD;
        let s = format!("{}", AccountId32::new(bytes));
        assert!(s.starts_with("0xab"));
        assert!(s.ends_with("cd"));
        assert_eq!(s.len(), 66);
    }

    #[test]
    fn test_account_name_display() {
        let name = AccountName::new(b"hot-wallet").unwrap();
        assert_eq!(format!("{}", name), "hot-wallet");
    }

    #[test]
    fn test_account_name_as_str() {
        let name = AccountName::new(b"hot-wallet").unwrap();
        assert_eq!(name.as_str(), "hot-wallet");
    }

    #[test]
    fn test_account_type_display() {
        assert_eq!(format!("{}", AccountType::Regular), "Regular");
        assert_eq!(format!("{}", AccountType::AppAgent), "AppAgent");
        assert_eq!(format!("{}", AccountType::Transactional), "Transactional");
        assert_eq!(format!("{}", AccountType::Named), "Named");
    }

    #[test]
    fn test_account_name_error_display() {
        assert_eq!(
            format!("{}", AccountNameError::InvalidLength),
            "Account name must be exactly 10 bytes long"
        );
        assert_eq!(
            format!("{}", AccountNameError::InvalidCharacter),
            "Account name contains invalid characters"
        );
    }

    #[test]
    fn test_keyless_decode_error_display() {
        assert_eq!(
            format!(
                "{}",
                KeylessDecodeError::WrongAccountType {
                    expected: AccountType::AppAgent,
                    found: AccountType::Regular
                }
            ),
            "Account is not an App Agent root account (found: Regular)"
        );
        assert_eq!(
            format!(
                "{}",
                KeylessDecodeError::WrongAccountType {
                    expected: AccountType::Transactional,
                    found: AccountType::Named
                }
            ),
            "Account is not a Transactional account (found: Named)"
        );
        assert_eq!(
            format!(
                "{}",
                KeylessDecodeError::WrongAccountType {
                    expected: AccountType::Named,
                    found: AccountType::AppAgent
                }
            ),
            "Account is not a Named account (found: AppAgent)"
        );
    }

    /// Compile-time check that every public type of the crate supports `core::hash::Hash`,
    /// independently of the `std` feature.
    #[test]
    fn test_all_types_implement_hash() {
        fn assert_impl<T: core::hash::Hash>() {}
        assert_impl::<AppAgentId>();
        assert_impl::<TransactionalId>();
        assert_impl::<AccountId32>();
        assert_impl::<AccountName>();
        assert_impl::<AccountNameError>();
        assert_impl::<AccountType>();
        assert_impl::<KeylessDecodeError>();
        assert_impl::<BlockchainAccountInfo>();
        assert_impl::<DecodedTransactionalAccount>();
        assert_impl::<DecodedNamedAccount>();
        assert_impl::<ParseAccountIdError>();
    }
}

mod tests_account_id_from_str {
    use crate::types::*;

    #[test]
    fn test_round_trips_with_display() {
        let bytes: [u8; 32] = core::array::from_fn(|i| (7 * i) as u8);
        let account = AccountId32::new(bytes);
        let rendered = account.to_string();
        assert_eq!(rendered.parse::<AccountId32>().unwrap(), account);
        assert_eq!(AccountId32::try_from(rendered.as_str()).unwrap(), account);
    }

    #[test]
    fn test_accepts_both_hex_digit_cases() {
        let lower = format!("0x{}", "ab".repeat(32));
        let upper = format!("0x{}", "AB".repeat(32));
        let mixed = format!("0x{}", "aB".repeat(32));
        let expected = AccountId32::new([0xAB; 32]);
        for spelling in [lower, upper, mixed] {
            assert_eq!(
                spelling.parse::<AccountId32>().unwrap(),
                expected,
                "{spelling:?} must parse to the same account"
            );
        }
    }

    #[test]
    fn test_rejects_deviations_from_the_contract() {
        let h = "ab".repeat(32);
        let rejected = [
            String::new(),
            "0x".to_string(),
            format!("0x{}", &h[..62]),
            format!("0x{}", &h[..63]),
            format!("0x{h}0"),
            format!("0X{h}"),
            h.clone(),
            format!("0x{} {}", &h[..8], &h[9..]),
            format!("0x{h} "),
            format!(" 0x{h}"),
            format!("0x{h}\n"),
            format!("0x{}zz", &h[..62]),
            // 64 bytes after the prefix, but two of them form one multi-byte char.
            format!("0x{}é", &h[..62]),
        ];
        for input in rejected {
            assert_eq!(
                input.parse::<AccountId32>().unwrap_err(),
                ParseAccountIdError,
                "{input:?} must be rejected"
            );
        }
    }

    #[test]
    fn test_parse_account_id_error_display() {
        assert_eq!(
            format!("{}", ParseAccountIdError),
            "Invalid hex account id: expected 0x followed by 64 hex characters"
        );
    }
}

#[cfg(feature = "scale")]
mod tests_scale_codec {
    use crate::types::*;
    use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode};

    #[test]
    fn test_account_name_scale_codec() {
        let name = AccountName::new(b"hot-wallet").unwrap();
        let encoded = name.encode();
        assert_eq!(encoded.len(), 10);
        let decoded = AccountName::decode(&mut &encoded[..]).unwrap();
        assert_eq!(decoded, name);
    }

    #[test]
    fn test_account_name_scale_decode_rejects_invalid_characters() {
        // Exactly 10 bytes, but the first one (0x00) is outside the allowed charset.
        let bytes = *b"\x00ot-wallet";
        assert!(AccountName::decode(&mut &bytes[..]).is_err());
    }

    #[test]
    fn test_account_name_scale_decode_rejects_truncated_input() {
        // Fewer than the 10 bytes required by the fixed-size encoding.
        assert!(AccountName::decode(&mut &b"short"[..]).is_err());
    }

    #[test]
    fn test_account_id32_scale_codec() {
        let bytes: [u8; 32] = core::array::from_fn(|i| i as u8);
        let account = AccountId32::new(bytes);
        let encoded = account.encode();
        assert_eq!(encoded.len(), 32);
        let decoded = AccountId32::decode(&mut &encoded[..]).unwrap();
        assert_eq!(decoded, account);
    }

    #[test]
    fn test_keyless_decode_error_scale_codec() {
        for error in [
            KeylessDecodeError::WrongAccountType {
                expected: AccountType::AppAgent,
                found: AccountType::Regular,
            },
            KeylessDecodeError::WrongAccountType {
                expected: AccountType::Transactional,
                found: AccountType::AppAgent,
            },
            KeylessDecodeError::WrongAccountType {
                expected: AccountType::Named,
                found: AccountType::Transactional,
            },
        ] {
            let encoded = error.encode();
            // One byte for the error variant, one for each embedded `AccountType`
            // (`expected` and `found`).
            assert_eq!(encoded.len(), 3);
            let decoded = KeylessDecodeError::decode(&mut &encoded[..]).unwrap();
            assert_eq!(decoded, error);
        }
    }

    #[test]
    fn test_account_type_scale_codec() {
        for (account_type, expected_index) in [
            (AccountType::Regular, 0u8),
            (AccountType::AppAgent, 1),
            (AccountType::Transactional, 2),
            (AccountType::Named, 3),
        ] {
            let encoded = account_type.encode();
            // The SCALE variant index matches the numeric values of the Python/TypeScript
            // `AccountType` enums (and, for keyless variants, the byte [4] discriminant).
            assert_eq!(encoded, [expected_index]);
            let decoded = AccountType::decode(&mut &encoded[..]).unwrap();
            assert_eq!(decoded, account_type);
        }
    }

    #[test]
    fn test_blockchain_account_info_scale_codec() {
        let account_id = AccountId32::new(core::array::from_fn(|i| i as u8));
        for info in [
            BlockchainAccountInfo::Regular { account_id },
            BlockchainAccountInfo::AppAgent {
                account_id,
                app_agent_id: 7,
            },
            BlockchainAccountInfo::Transactional {
                account_id,
                app_agent_id: 7,
                ta_id: 9,
            },
            BlockchainAccountInfo::Named {
                account_id,
                app_agent_id: 7,
                account_name: AccountName::new(b"hot-wallet").unwrap(),
            },
        ] {
            let encoded = info.encode();
            let decoded = BlockchainAccountInfo::decode(&mut &encoded[..]).unwrap();
            assert_eq!(decoded, info);
        }
    }

    /// Pins the documented contract that SCALE decoding performs **no** consistency
    /// validation (unlike serde deserialization, which re-validates): a value whose
    /// identifiers do not match its `account_id` round-trips through SCALE unchanged.
    #[test]
    fn test_blockchain_account_info_scale_decode_performs_no_validation() {
        let inconsistent = BlockchainAccountInfo::AppAgent {
            account_id: AccountId32::new([0u8; 32]),
            app_agent_id: 999,
        };
        assert_ne!(
            crate::keyless::decode_account_info(&inconsistent.account_id()),
            inconsistent,
            "self check: the crafted value must actually be inconsistent"
        );

        let encoded = inconsistent.encode();
        let decoded = BlockchainAccountInfo::decode(&mut &encoded[..]).unwrap();
        assert_eq!(
            decoded, inconsistent,
            "SCALE decode must accept inconsistent values; see the type documentation"
        );
    }

    /// Compile-time check that every public type of the crate supports
    /// `codec::DecodeWithMemTracking`.
    #[test]
    fn test_all_types_implement_decode_with_mem_tracking() {
        fn assert_impl<T: DecodeWithMemTracking>() {}
        assert_impl::<AppAgentId>();
        assert_impl::<TransactionalId>();
        assert_impl::<AccountId32>();
        assert_impl::<AccountName>();
        assert_impl::<AccountNameError>();
        assert_impl::<AccountType>();
        assert_impl::<KeylessDecodeError>();
        assert_impl::<BlockchainAccountInfo>();
        assert_impl::<DecodedTransactionalAccount>();
        assert_impl::<DecodedNamedAccount>();
    }
}
