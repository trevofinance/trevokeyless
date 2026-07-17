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
        // The wording is shared verbatim with the Python and TypeScript implementations
        // ("characters", the requirement statement — see the Display impl).
        assert_eq!(
            format!("{}", AccountNameError::InvalidLength),
            "Account name must be exactly 10 characters long"
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

    /// The inherent byte accessors are the unambiguous way to reach the bytes.
    ///
    /// `AccountId32` and `AccountName` each implement *both* `AsRef<[u8; N]>` and
    /// `AsRef<[u8]>`, so a bare `x.as_ref()` does not compile (`error[E0283]`: multiple
    /// applicable impls). Callers — and, before this, the crate's own code — had to write a
    /// type ascription at every site. These accessors need none.
    #[test]
    fn test_inherent_byte_accessors_need_no_type_ascription() {
        let account = AccountId32::new([7u8; 32]);
        let bytes = account.as_bytes();
        assert_eq!(bytes.len(), ACCOUNT_ID_LENGTH);
        assert_eq!(account.as_slice(), &bytes[..]);
        // The AsRef impls still work when the target type is known.
        let via_as_ref: &[u8; 32] = account.as_ref();
        assert_eq!(via_as_ref, bytes);

        let name = AccountName::new(b"hot-wallet").unwrap();
        let name_bytes = name.as_bytes();
        assert_eq!(name_bytes.len(), ACCOUNT_NAME_LENGTH);
        assert_eq!(name.as_slice(), &name_bytes[..]);
        assert_eq!(name.as_str(), "hot-wallet");
    }

    /// Compile-time check that every public type of the crate supports `core::hash::Hash`,
    /// independently of the `std` feature. The error types are listed too: all of them
    /// derive `Hash` (values a caller may deduplicate or key on), and an accidentally
    /// dropped derive would be a breaking API change.
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
        assert_impl::<TransactionalAccountIdentifiers>();
        assert_impl::<NamedAccountIdentifiers>();
        assert_impl::<ParseAccountIdError>();
        assert_impl::<InvalidAccountIdLength>();
        assert_impl::<InvalidAccountType>();
        #[cfg(feature = "ss58")]
        assert_impl::<crate::Ss58Error>();
        #[cfg(feature = "ss58")]
        assert_impl::<crate::InvalidSs58Format>();
        #[cfg(feature = "ss58")]
        assert_impl::<crate::Ss58ExpectingError>();
        #[cfg(feature = "test-accounts")]
        assert_impl::<crate::AccountId64>();
        #[cfg(feature = "test-accounts")]
        assert_impl::<crate::AccountId128>();
    }

    /// The checked inverse of `AccountType as u8`: every variant round-trips through its
    /// numeric value (the cross-language contract), everything else reports the rejected
    /// value back.
    #[test]
    fn test_account_type_try_from_u8() {
        for account_type in [
            AccountType::Regular,
            AccountType::AppAgent,
            AccountType::Transactional,
            AccountType::Named,
        ] {
            assert_eq!(
                AccountType::try_from(account_type as u8),
                Ok(account_type),
                "{account_type} must round-trip through its numeric value"
            );
        }
        for value in [4u8, 46, u8::MAX] {
            let error = AccountType::try_from(value).unwrap_err();
            assert_eq!(error.value(), value, "the rejected value must be reported");
            assert_eq!(
                error.to_string(),
                format!("Account type must be 0, 1, 2, or 3, got {value}")
            );
        }
    }

    /// Compile-time check of the ordered public surface (the types usable as ordered
    /// map keys); removing a derived `Ord` would be a breaking API change.
    #[test]
    fn test_ordered_types_implement_ord() {
        fn assert_impl<T: Ord>() {}
        assert_impl::<AppAgentId>();
        assert_impl::<TransactionalId>();
        assert_impl::<AccountId32>();
        assert_impl::<AccountName>();
        assert_impl::<AccountType>();
    }

    /// Compile-time check that every error type implements `core::error::Error`
    /// unconditionally (`std::error::Error` is the same trait, so `std` consumers see
    /// no difference and `no_std` builds keep the impls).
    #[test]
    fn test_error_types_implement_core_error() {
        fn assert_impl<T: core::error::Error>() {}
        assert_impl::<AccountNameError>();
        assert_impl::<KeylessDecodeError>();
        assert_impl::<ParseAccountIdError>();
        assert_impl::<InvalidAccountIdLength>();
        assert_impl::<InvalidAccountType>();
        #[cfg(feature = "ss58")]
        assert_impl::<crate::Ss58Error>();
        #[cfg(feature = "ss58")]
        assert_impl::<crate::InvalidSs58Format>();
        #[cfg(feature = "ss58")]
        assert_impl::<crate::Ss58ExpectingError>();
    }
}

mod tests_account_id_from_slice {
    use crate::types::*;

    #[test]
    fn test_try_from_slice_accepts_exactly_32_bytes() {
        let bytes = [7u8; ACCOUNT_ID_LENGTH];
        let account = AccountId32::try_from(&bytes[..]).unwrap();
        assert_eq!(account, AccountId32::new(bytes));
    }

    #[test]
    fn test_try_from_slice_rejects_other_lengths() {
        for length in [0usize, 31, 33, 64] {
            let bytes = vec![0u8; length];
            let error = AccountId32::try_from(&bytes[..]).unwrap_err();
            assert_eq!(error.length(), length);
            assert_eq!(
                error.to_string(),
                format!("Account id must be exactly 32 bytes long, got {length}")
            );
        }
    }

    #[test]
    fn test_default_is_all_zero() {
        assert_eq!(AccountId32::default(), AccountId32::new([0u8; 32]));
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
                ParseAccountIdError::new(64),
                "{input:?} must be rejected"
            );
        }
    }

    #[test]
    fn test_parse_account_id_error_display() {
        assert_eq!(
            ParseAccountIdError::new(64).to_string(),
            "Invalid hex account id: expected 0x followed by 64 hex characters"
        );
    }
}

#[cfg(feature = "scale")]
mod tests_scale_codec {
    use crate::types::*;
    use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode, EncodeLike};

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
    fn test_identifier_scale_abi_is_u32_little_endian() {
        let app_agent_id = AppAgentId::new(0x1234_5678);
        let transactional_id = TransactionalId::new(0x90ab_cdef);
        assert_eq!(app_agent_id.encode(), [0x78, 0x56, 0x34, 0x12]);
        assert_eq!(transactional_id.encode(), [0xef, 0xcd, 0xab, 0x90]);
        assert_eq!(
            AppAgentId::decode(&mut &app_agent_id.encode()[..]).unwrap(),
            app_agent_id
        );
        assert_eq!(
            TransactionalId::decode(&mut &transactional_id.encode()[..]).unwrap(),
            transactional_id
        );
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
    fn test_public_struct_and_error_scale_abi_golden_bytes() {
        assert_eq!(AccountNameError::InvalidLength.encode(), [0]);
        assert_eq!(AccountNameError::InvalidCharacter.encode(), [1]);

        assert_eq!(
            KeylessDecodeError::WrongAccountType {
                expected: AccountType::Named,
                found: AccountType::Transactional,
            }
            .encode(),
            [0, 3, 2]
        );

        assert_eq!(
            TransactionalAccountIdentifiers::new(
                AppAgentId::new(0x1234_5678),
                TransactionalId::new(0x90ab_cdef),
            )
            .encode(),
            [0x78, 0x56, 0x34, 0x12, 0xef, 0xcd, 0xab, 0x90]
        );

        assert_eq!(
            NamedAccountIdentifiers::new(
                AppAgentId::new(0x1234_5678),
                AccountName::new(b"hot-wallet").unwrap(),
            )
            .encode(),
            *b"xV4\x12hot-wallet"
        );
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
        for info in [
            crate::decode_account_info(&AccountId32::new([0u8; 32])),
            crate::decode_account_info(&crate::encode_app_agent_account(AppAgentId::new(7))),
            crate::decode_account_info(&crate::encode_transactional_account(
                TransactionalAccountIdentifiers::new(AppAgentId::new(7), TransactionalId::new(9)),
            )),
            crate::decode_account_info(&crate::encode_named_account(NamedAccountIdentifiers::new(
                AppAgentId::new(7),
                AccountName::new(b"hot-wallet").unwrap(),
            ))),
        ] {
            let encoded = info.encode();
            assert_eq!(encoded.len(), 32);
            assert_eq!(encoded, info.account_id().encode());
            let decoded = BlockchainAccountInfo::decode(&mut &encoded[..]).unwrap();
            assert_eq!(decoded, info);
        }
    }

    /// SCALE encodes only the account id, so a manually constructed inconsistent value is
    /// normalized to the classification derived from that account id while decoding.
    #[test]
    fn test_blockchain_account_info_scale_decode_derives_identifiers() {
        let inconsistent = BlockchainAccountInfo::AppAgent {
            account_id: AccountId32::new([0u8; 32]),
            app_agent_id: AppAgentId::new(999),
        };
        assert_ne!(
            crate::keyless::decode_account_info(&inconsistent.account_id()),
            inconsistent,
            "self check: the crafted value must actually be inconsistent"
        );

        let encoded = inconsistent.encode();
        assert_eq!(encoded, [0u8; 32]);
        let decoded = BlockchainAccountInfo::decode(&mut &encoded[..]).unwrap();
        assert_eq!(
            decoded,
            BlockchainAccountInfo::Regular {
                account_id: AccountId32::new([0u8; 32])
            }
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
        assert_impl::<TransactionalAccountIdentifiers>();
        assert_impl::<NamedAccountIdentifiers>();
    }

    /// Compile-time check that every public type supports `codec::EncodeLike`.
    ///
    /// The derive macro emits `EncodeLike` alongside `Encode`, so a derived type gets this for
    /// free — but `BlockchainAccountInfo` hand-writes `Encode` and once silently lacked it.
    /// `frame_support::Parameter` is `Codec + EncodeLike + Clone + Eq + Debug + TypeInfo` and
    /// storage items require `FullCodec = Encode + EncodeLike + Decode`, so a missing impl makes
    /// the type unusable as a pallet call argument, event field, or storage value.
    #[test]
    fn test_all_types_implement_encode_like() {
        fn assert_impl<T: EncodeLike>() {}
        assert_impl::<AppAgentId>();
        assert_impl::<TransactionalId>();
        assert_impl::<AccountId32>();
        assert_impl::<AccountName>();
        assert_impl::<AccountNameError>();
        assert_impl::<AccountType>();
        assert_impl::<KeylessDecodeError>();
        assert_impl::<BlockchainAccountInfo>();
        assert_impl::<TransactionalAccountIdentifiers>();
        assert_impl::<NamedAccountIdentifiers>();
    }

    /// Compile-time check that the public types satisfy the full `frame_support::Parameter`
    /// bound the consuming runtime imposes, without depending on frame_support here.
    #[test]
    fn test_public_types_satisfy_parameter_bound() {
        use core::fmt::Debug;
        use parity_scale_codec::Codec;
        use scale_info::TypeInfo;

        fn assert_parameter<T: Codec + EncodeLike + Clone + Eq + Debug + TypeInfo + 'static>() {}
        assert_parameter::<AppAgentId>();
        assert_parameter::<TransactionalId>();
        assert_parameter::<AccountId32>();
        assert_parameter::<AccountType>();
        assert_parameter::<BlockchainAccountInfo>();
        assert_parameter::<TransactionalAccountIdentifiers>();
        assert_parameter::<NamedAccountIdentifiers>();
    }

    /// Pins the `MaxEncodedLen` values of every public type. These feed Substrate
    /// storage bounds and weight accounting in the consuming runtime, so a change
    /// here changes on-chain resource limits — never accidental.
    #[test]
    fn test_max_encoded_len_values() {
        use parity_scale_codec::MaxEncodedLen;
        assert_eq!(AccountId32::max_encoded_len(), 32);
        assert_eq!(AccountName::max_encoded_len(), 10);
        assert_eq!(AccountType::max_encoded_len(), 1);
        assert_eq!(AccountNameError::max_encoded_len(), 1);
        // Variant byte + expected + found.
        assert_eq!(KeylessDecodeError::max_encoded_len(), 3);
        // The verified info type encodes only its account id; identifiers are derived.
        assert_eq!(BlockchainAccountInfo::max_encoded_len(), 32);
        assert_eq!(TransactionalAccountIdentifiers::max_encoded_len(), 8);
        assert_eq!(NamedAccountIdentifiers::max_encoded_len(), 14);
    }

    #[test]
    fn test_account_type_scale_decode_rejects_invalid_variant_index() {
        // 0..=3 are the only variants; 4 is the first unassigned index.
        assert!(AccountType::decode(&mut &[4u8][..]).is_err());
        assert!(AccountType::decode(&mut &[255u8][..]).is_err());
        // Empty input carries no variant byte at all.
        assert!(AccountType::decode(&mut &[][..]).is_err());
    }

    #[test]
    fn test_blockchain_account_info_scale_decode_classifies_arbitrary_account_id() {
        let bytes = [4u8; 32];
        assert_eq!(
            BlockchainAccountInfo::decode(&mut &bytes[..]).unwrap(),
            BlockchainAccountInfo::Regular {
                account_id: AccountId32::new(bytes)
            }
        );
    }

    #[test]
    fn test_scale_decode_rejects_truncated_input() {
        let info = BlockchainAccountInfo::Named {
            account_id: AccountId32::new(core::array::from_fn(|i| i as u8)),
            app_agent_id: AppAgentId::new(7),
            account_name: AccountName::new(b"hot-wallet").unwrap(),
        };
        let encoded = info.encode();
        for len in 0..encoded.len() {
            assert!(
                BlockchainAccountInfo::decode(&mut &encoded[..len]).is_err(),
                "truncation to {len} bytes must not decode"
            );
        }

        let error = KeylessDecodeError::WrongAccountType {
            expected: AccountType::Named,
            found: AccountType::Regular,
        };
        let encoded = error.encode();
        for len in 0..encoded.len() {
            assert!(
                KeylessDecodeError::decode(&mut &encoded[..len]).is_err(),
                "truncation to {len} bytes must not decode"
            );
        }
    }

    /// Pins the `TypeInfo` identities (the type names a consuming runtime's metadata
    /// exposes); a rename would change the runtime metadata.
    #[test]
    fn test_type_info_identities() {
        use scale_info::TypeInfo;
        fn ident<T: TypeInfo>() -> Option<&'static str> {
            T::type_info().path.ident()
        }
        assert_eq!(ident::<AccountId32>(), Some("AccountId32"));
        assert_eq!(ident::<AccountName>(), Some("AccountName"));
        assert_eq!(ident::<AccountNameError>(), Some("AccountNameError"));
        assert_eq!(ident::<AccountType>(), Some("AccountType"));
        assert_eq!(ident::<KeylessDecodeError>(), Some("KeylessDecodeError"));
        assert_eq!(
            ident::<BlockchainAccountInfo>(),
            Some("BlockchainAccountInfo")
        );
        assert_eq!(
            ident::<TransactionalAccountIdentifiers>(),
            Some("TransactionalAccountIdentifiers")
        );
        assert_eq!(
            ident::<NamedAccountIdentifiers>(),
            Some("NamedAccountIdentifiers")
        );
    }
}
