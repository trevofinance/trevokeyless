#[cfg(test)]
mod tests_encoding {
    use crate::keyless::*;
    use crate::types::*;

    fn account_id_from_str(account_id_str: &str) -> AccountId32 {
        let account_id_vec = array_bytes::hex2bytes(account_id_str).unwrap();
        let account_id_array: [u8; 32] = account_id_vec[..].try_into().unwrap();
        let account_id = AccountId32::from(account_id_array);
        return account_id;
    }

    #[test]
    fn test_app_agent_account() {
        // let ss58_format = 5335;

        let app_agent_id: AppAgentId = 123;
        let account_id =
            account_id_from_str("7b00000001293833058fc7db52fc03f6ce344bca98bd7825ff747743f1ff63e2");
        // let blockchain_address = "ttowKp8AmQuGfbBGikG2pbdYNnErhHRaLrdktJeZEfJVeVnTp";

        // ******************************** //

        let encoded_account_id = encode_app_agent_account(&app_agent_id);
        assert!(
            encoded_account_id == account_id,
            "Couldn't encode AppAgent ID to AppAgent account."
        );

        let decoded_app_agent_id = decode_app_agent_account(&account_id);
        assert!(
            decoded_app_agent_id.unwrap() == app_agent_id,
            "Couldn't decode AppAgent account to AppAgent ID."
        );

        let decoded_account_info = decode_account(account_id.clone());
        let expected_account_info = BlockchainAddressInfo {
            account_id: account_id.clone(),
            address_type: AddressType::AppAgent,
            app_agent_id: Some(app_agent_id.clone()),
            ta_id: None,
            address_name: None,
        };
        assert!(
            decoded_account_info == expected_account_info,
            "Couldn't decode AppAgent account to account info."
        )

        // TODO enable feature = "serde" to make it work
        // assert(account_id.to_ss58check_with_version(ss58_format) == blockchain_address);
    }

    #[test]
    fn test_transactional_account() {
        // let ss58_format = 5335;

        let app_agent_id: AppAgentId = 123;
        let transactional_address_id: TransactionalId = 456;
        let account_id =
            account_id_from_str("7b00000002c801000033399aeb61b087f1a20a58c41ea5ff1b7bfb2fda27bfc0");
        // let blockchain_address = "ttowKp8AmjjQh4GoN7xMiQWwVyyrU1Pu7GRf5HxFmV5t43TXG";

        // ******************************** //

        let encoded_account_id =
            encode_transactional_account(&app_agent_id, &transactional_address_id);
        assert!(
            encoded_account_id == account_id,
            "Couldn't encode Transactional account ID to Transactional account."
        );

        let decoded_ids = decode_transactional_account(&account_id);
        assert!(
            decoded_ids.unwrap() == (app_agent_id, transactional_address_id),
            "Couldn't decode Transactional account to Transactional ID."
        );

        let decoded_account_info = decode_account(account_id.clone());
        let expected_account_info = BlockchainAddressInfo {
            account_id: account_id.clone(),
            address_type: AddressType::Transactional,
            app_agent_id: Some(app_agent_id.clone()),
            ta_id: Some(transactional_address_id),
            address_name: None,
        };
        assert!(
            decoded_account_info == expected_account_info,
            "Couldn't decode Transactional account to account info."
        )

        // TODO enable feature = "serde" to make it work
        // assert(account_id.to_ss58check_with_version(ss58_format) == blockchain_address);
    }

    #[test]
    fn test_named_account() {
        // let ss58_format = 5335;

        let app_agent_id: AppAgentId = 123;
        let address_name = AddressName::new(b"example123").unwrap();
        let account_id =
            account_id_from_str("7b000000036578616d706c653132330a72315aba532de9903c05a71e7d2d0409");
        // let blockchain_address = "ttowKp8Amrt2FQ2eNdsbMsW2EEeEsEmsT8KHLvPhVppp9zs8k";

        // ******************************** //

        let encoded_account_id = encode_named_account(&app_agent_id, &address_name);
        assert!(
            encoded_account_id == account_id,
            "Couldn't encode Named account ID to Named account."
        );

        let decoded_ids = decode_named_account(&account_id);
        assert!(
            decoded_ids.unwrap() == (app_agent_id, address_name),
            "Couldn't decode Named account to Named ID."
        );

        let decoded_account_info = decode_account(account_id.clone());
        let expected_account_info = BlockchainAddressInfo {
            account_id: account_id.clone(),
            address_type: AddressType::Named,
            app_agent_id: Some(app_agent_id.clone()),
            ta_id: None,
            address_name: Some(address_name),
        };
        assert!(
            decoded_account_info == expected_account_info,
            "Couldn't decode Named account to account info."
        );

        // TODO enable feature = "serde" to make it work
        // assert(account_id.to_ss58check_with_version(ss58_format) == blockchain_address);
    }

    #[test]
    fn test_mismatch_of_address_type() {
        let address_aa = encode_app_agent_account(&1);
        assert!(
            decode_transactional_account(&address_aa).is_err(),
            "AppAgent address can not be considered as a Transactional address"
        );
        assert!(
            decode_named_account(&address_aa).is_err(),
            "AppAgent address can not be considered as a Named address"
        );

        let address_ta = encode_transactional_account(&1, &0);
        assert!(
            decode_app_agent_account(&address_ta).is_err(),
            "Transactional address can not be considered as an AppAgent address"
        );
        assert!(
            decode_named_account(&address_ta).is_err(),
            "Transactional address can not be considered as a Named address"
        );

        let address_named = encode_named_account(&1, &AddressName::new(b"test123456").unwrap());
        assert!(
            decode_app_agent_account(&address_named).is_err(),
            "Named address can not be considered as an AppAgent address"
        );
        assert!(
            decode_transactional_account(&address_named).is_err(),
            "Named address can not be considered as a Transactional address"
        );
    }
}
