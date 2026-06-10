#[cfg(test)]
mod tests {
    use crate::keyless::*;
    use crate::types::*;

    #[test]
    fn test_account_id32_serde() {
        let account = encode_app_agent_account(123);

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
        let app_agent_id: AppAgentId = 123;
        let infos = [
            decode_account_info(&AccountId32::from([0u8; 32])),
            decode_account_info(&encode_app_agent_account(app_agent_id)),
            decode_account_info(&encode_transactional_account(app_agent_id, 456)),
            decode_account_info(&encode_named_account(
                app_agent_id,
                AccountName::new(b"example123").unwrap(),
            )),
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
        let info = decode_account_info(&encode_named_account(
            123,
            AccountName::new(b"example123").unwrap(),
        ));
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
        let crafted =
            format!("{{\"AppAgent\":{{\"account_id\":\"{regular}\",\"app_agent_id\":999}}}}");
        assert!(
            serde_json::from_str::<BlockchainAccountInfo>(&crafted).is_err(),
            "AppAgent claim over a regular account must be rejected"
        );

        // AppAgent claim with the wrong app_agent_id for a genuine AppAgent account.
        let aa_address = encode_app_agent_account(123).to_ss58check();
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
        let named_address =
            encode_named_account(123, AccountName::new(b"example123").unwrap()).to_ss58check();
        let crafted = format!(
            "{{\"Named\":{{\"account_id\":\"{named_address}\",\"app_agent_id\":123,\
             \"account_name\":\"example124\"}}}}"
        );
        assert!(
            serde_json::from_str::<BlockchainAccountInfo>(&crafted).is_err(),
            "Named claim with a mismatching name must be rejected"
        );

        // Transactional claim with swapped ids.
        let ta_address = encode_transactional_account(123, 456).to_ss58check();
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
        for error in [
            AccountNameError::InvalidLength,
            AccountNameError::InvalidCharacter,
        ] {
            let json = serde_json::to_string(&error).unwrap();
            let decoded: AccountNameError = serde_json::from_str(&json).unwrap();
            assert_eq!(decoded, error);
        }
    }

    #[test]
    fn test_keyless_decode_error_serde() {
        for error in [
            KeylessDecodeError::NotAnAppAgentAccount {
                found: AccountType::Regular,
            },
            KeylessDecodeError::NotATransactionalAccount {
                found: AccountType::AppAgent,
            },
            KeylessDecodeError::NotANamedAccount {
                found: AccountType::Transactional,
            },
        ] {
            let json = serde_json::to_string(&error).unwrap();
            let decoded: KeylessDecodeError = serde_json::from_str(&json).unwrap();
            assert_eq!(decoded, error);
        }
    }

    #[test]
    fn test_account_type_serde() {
        for account_type in [
            AccountType::Regular,
            AccountType::AppAgent,
            AccountType::Transactional,
            AccountType::Named,
        ] {
            let json = serde_json::to_string(&account_type).unwrap();
            let decoded: AccountType = serde_json::from_str(&json).unwrap();
            assert_eq!(decoded, account_type);
        }
    }
}
