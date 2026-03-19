#[cfg(test)]
mod tests {
    use crate::keyless::*;
    use crate::types::*;

    fn account_id_from_str(account_id_str: &str) -> AccountId32 {
        let account_id_vec = array_bytes::hex2bytes(account_id_str).unwrap();
        let account_id_array: [u8; 32] = account_id_vec[..].try_into().unwrap();

        AccountId32::from(account_id_array)
    }

    #[test]
    fn test_app_agent_account() {
        // let ss58_format = 5335;

        let app_agent_id: AppAgentId = 123;
        let account_id =
            account_id_from_str("7b00000001293833058fc7db52fc03f6ce344bca98bd7825ff747743f1ff63e2");
        // let blockchain_account = "ttowKp8AmQuGfbBGikG2pbdYNnErhHRaLrdktJeZEfJVeVnTp";

        // ******************************** //

        let encoded_account_id = encode_app_agent_account(app_agent_id);
        assert_eq!(
            encoded_account_id, account_id,
            "Couldn't encode AppAgent ID to AppAgent account."
        );

        let decoded_app_agent_id = decode_app_agent_account(&account_id);
        assert_eq!(
            decoded_app_agent_id.unwrap(),
            app_agent_id,
            "Couldn't decode AppAgent account to AppAgent ID."
        );

        let decoded_account_info = decode_account_info(&account_id);
        assert_eq!(
            decoded_account_info,
            BlockchainAccountInfo::AppAgent {
                account_id,
                app_agent_id
            },
            "Couldn't decode AppAgent account to account info."
        );

        // TODO enable feature = "serde" to make it work
        // assert(account_id.to_ss58check_with_version(ss58_format) == blockchain_account);
    }

    #[test]
    fn test_transactional_account() {
        // let ss58_format = 5335;

        let app_agent_id: AppAgentId = 123;
        let transactional_account_id: TransactionalId = 456;
        let account_id =
            account_id_from_str("7b00000002c801000033399aeb61b087f1a20a58c41ea5ff1b7bfb2fda27bfc0");
        // let blockchain_account = "ttowKp8AmjjQh4GoN7xMiQWwVyyrU1Pu7GRf5HxFmV5t43TXG";

        // ******************************** //

        let encoded_account_id =
            encode_transactional_account(app_agent_id, transactional_account_id);
        assert_eq!(
            encoded_account_id, account_id,
            "Couldn't encode Transactional account ID to Transactional account."
        );

        let decoded_ids = decode_transactional_account(&account_id);
        assert_eq!(
            decoded_ids.unwrap(),
            (app_agent_id, transactional_account_id),
            "Couldn't decode Transactional account to Transactional ID."
        );

        let decoded_account_info = decode_account_info(&account_id);
        assert_eq!(
            decoded_account_info,
            BlockchainAccountInfo::Transactional {
                account_id,
                app_agent_id,
                ta_id: transactional_account_id
            },
            "Couldn't decode Transactional account to account info."
        );

        // TODO enable feature = "serde" to make it work
        // assert(account_id.to_ss58check_with_version(ss58_format) == blockchain_account);
    }

    #[test]
    fn test_named_account() {
        // let ss58_format = 5335;

        let app_agent_id: AppAgentId = 123;
        let account_name = AccountName::new(b"example123").unwrap();
        let account_id =
            account_id_from_str("7b000000036578616d706c653132330a72315aba532de9903c05a71e7d2d0409");
        // let blockchain_account = "ttowKp8Amrt2FQ2eNdsbMsW2EEeEsEmsT8KHLvPhVppp9zs8k";

        // ******************************** //

        let encoded_account_id = encode_named_account(app_agent_id, &account_name);
        assert_eq!(
            encoded_account_id, account_id,
            "Couldn't encode Named account ID to Named account."
        );

        let decoded_ids = decode_named_account(&account_id);
        assert_eq!(
            decoded_ids.unwrap(),
            (app_agent_id, account_name),
            "Couldn't decode Named account to Named ID."
        );

        let decoded_account_info = decode_account_info(&account_id);
        assert_eq!(
            decoded_account_info,
            BlockchainAccountInfo::Named {
                account_id,
                app_agent_id,
                account_name
            },
            "Couldn't decode Named account to account info."
        );

        // TODO enable feature = "serde" to make it work
        // assert(account_id.to_ss58check_with_version(ss58_format) == blockchain_account);
    }

    #[test]
    fn test_boundary_values() {
        // AppAgentId = 0
        let account = encode_app_agent_account(0);
        assert_eq!(decode_app_agent_account(&account).unwrap(), 0);

        // AppAgentId = u32::MAX
        let account = encode_app_agent_account(u32::MAX);
        assert_eq!(decode_app_agent_account(&account).unwrap(), u32::MAX);

        // TransactionalId = 0
        let account = encode_transactional_account(0, 0);
        assert_eq!(decode_transactional_account(&account).unwrap(), (0, 0));

        // TransactionalId = u32::MAX, AppAgentId = u32::MAX
        let account = encode_transactional_account(u32::MAX, u32::MAX);
        assert_eq!(
            decode_transactional_account(&account).unwrap(),
            (u32::MAX, u32::MAX)
        );
    }

    #[test]
    fn test_boundary_values_named() {
        let name = AccountName::new(b"0000000000").unwrap();

        // AppAgentId = 0
        let account = encode_named_account(0, &name);
        assert_eq!(decode_named_account(&account).unwrap(), (0, name));

        // AppAgentId = u32::MAX
        let account = encode_named_account(u32::MAX, &name);
        assert_eq!(decode_named_account(&account).unwrap(), (u32::MAX, name));
    }

    #[test]
    fn test_mismatch_of_account_type() {
        let account_aa = encode_app_agent_account(1);
        assert!(
            decode_transactional_account(&account_aa).is_err(),
            "AppAgent account can not be considered as a Transactional account"
        );
        assert!(
            decode_named_account(&account_aa).is_err(),
            "AppAgent account can not be considered as a Named account"
        );

        let account_ta = encode_transactional_account(1, 0);
        assert!(
            decode_app_agent_account(&account_ta).is_err(),
            "Transactional account can not be considered as an AppAgent account"
        );
        assert!(
            decode_named_account(&account_ta).is_err(),
            "Transactional account can not be considered as a Named account"
        );

        let account_named = encode_named_account(1, &AccountName::new(b"test123456").unwrap());
        assert!(
            decode_app_agent_account(&account_named).is_err(),
            "Named account can not be considered as an AppAgent account"
        );
        assert!(
            decode_transactional_account(&account_named).is_err(),
            "Named account can not be considered as a Transactional account"
        );
    }

    #[test]
    fn test_is_keyless_account() {
        assert!(
            is_keyless_account(&encode_app_agent_account(1)),
            "AppAgent account must be keyless"
        );
        assert!(
            is_keyless_account(&encode_transactional_account(1, 2)),
            "Transactional account must be keyless"
        );
        assert!(
            is_keyless_account(&encode_named_account(
                1,
                &AccountName::new(b"test123456").unwrap()
            )),
            "Named account must be keyless"
        );

        // A zeroed account has no valid keyless discriminant → Regular
        assert!(
            !is_keyless_account(&AccountId32::from([0u8; 32])),
            "Zeroed account must not be keyless"
        );

        // Tampered checksum → Regular.
        // Byte 10 is in the checksum region of an AppAgent account (open part is [0..5]).
        let mut bytes: [u8; 32] = encode_app_agent_account(1).into();
        bytes[10] ^= 0xFF;
        assert!(
            !is_keyless_account(&AccountId32::from(bytes)),
            "Tampered account must not be keyless"
        );
    }

    #[test]
    fn test_tampered_checksum_falls_back_to_regular() {
        // Flip one byte in the checksum region of each account type and verify that
        // decode_account_info returns Regular (i.e. the integrity check rejects the account).

        let mut bytes: [u8; 32] = encode_app_agent_account(42).into();
        bytes[10] ^= 0xFF; // corrupt a checksum byte (open part is [0..5], checksum starts at [5])
        assert_eq!(
            decode_account_info(&AccountId32::from(bytes)),
            BlockchainAccountInfo::Regular {
                account_id: AccountId32::from(bytes)
            },
            "Tampered AppAgent checksum must decode as Regular"
        );

        let mut bytes: [u8; 32] = encode_transactional_account(42, 7).into();
        bytes[15] ^= 0xFF; // corrupt a checksum byte (open part is [0..9], checksum starts at [9])
        assert_eq!(
            decode_account_info(&AccountId32::from(bytes)),
            BlockchainAccountInfo::Regular {
                account_id: AccountId32::from(bytes)
            },
            "Tampered Transactional checksum must decode as Regular"
        );

        let mut bytes: [u8; 32] =
            encode_named_account(42, &AccountName::new(b"test123456").unwrap()).into();
        bytes[20] ^= 0xFF; // corrupt a checksum byte (open part is [0..15], checksum starts at [15])
        assert_eq!(
            decode_account_info(&AccountId32::from(bytes)),
            BlockchainAccountInfo::Regular {
                account_id: AccountId32::from(bytes)
            },
            "Tampered Named checksum must decode as Regular"
        );
    }

    #[test]
    fn test_named_decode_invalid_charset_falls_back_to_regular() {
        // Construct a Named account with a valid checksum but name bytes that contain a character
        // outside VALID_CHARS (0x00 = null byte). This exercises the fall-through in
        // decode_account_info that returns Regular when AccountName::try_from fails.
        let mut open_part = [0u8; 15];
        open_part[0..4].copy_from_slice(&1u32.to_le_bytes()); // AppAgentId = 1
        open_part[4] = NAMED_ACCOUNT_IDENTIFIER;
        // Place a null byte at position 5 — not in the allowed charset.
        open_part[5..15].copy_from_slice(b"\x00nvalid123");

        let hash = blake2_256(&open_part);
        let mut account_bytes = [0u8; 32];
        account_bytes[0..15].copy_from_slice(&open_part);
        account_bytes[15..32].copy_from_slice(&hash[15..32]);

        let account = AccountId32::from(account_bytes);
        assert_eq!(
            decode_account_info(&account),
            BlockchainAccountInfo::Regular {
                account_id: account
            },
            "Named account with valid checksum but invalid charset must decode as Regular"
        );
        assert!(!is_keyless_account(&account));
    }

    #[test]
    fn test_decode_regular_account() {
        // Zeroed account: byte [4] = 0, which is not a known discriminant.
        let account = AccountId32::from([0u8; 32]);
        assert_eq!(
            decode_account_info(&account),
            BlockchainAccountInfo::Regular {
                account_id: account
            },
            "Zeroed account must decode as Regular"
        );

        // Account with an unknown discriminant (4) at byte [4].
        let mut bytes = [0u8; 32];
        bytes[4] = 4;
        let account = AccountId32::from(bytes);
        assert_eq!(
            decode_account_info(&account),
            BlockchainAccountInfo::Regular {
                account_id: account
            },
            "Unknown discriminant must decode as Regular"
        );
    }
}
