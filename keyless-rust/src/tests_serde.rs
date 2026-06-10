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
    fn test_keyless_decode_error_serde() {
        for error in [
            KeylessDecodeError::NotAnAppAgentAccount,
            KeylessDecodeError::NotATransactionalAccount,
            KeylessDecodeError::NotANamedAccount,
        ] {
            let json = serde_json::to_string(&error).unwrap();
            let decoded: KeylessDecodeError = serde_json::from_str(&json).unwrap();
            assert_eq!(decoded, error);
        }
    }
}
