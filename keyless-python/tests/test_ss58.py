import unittest

import trevokeyless


class TestKeylessAddresses(unittest.TestCase):
    # Tests check that SS58 is properly processed

    def test_app_agent_address_ss58_format(self: "TestKeylessAddresses") -> None:
        app_agent_id = 123
        account_id = "0x7b00000001293833058fc7db52fc03f6ce344bca98bd7825ff747743f1ff63e2"
        blockchain_address = "ttowKp8AmQuGfbBGikG2pbdYNnErhHRaLrdktJeZEfJVeVnTp"
        blockchain_address_ss58_42 = "5EqykH6EjAZ513RNK37NMagT2KPL4xr2vJXxGCRucbXqBSA7"

        ###

        encoded_address = trevokeyless.encode_app_agent_address(app_agent_id)
        self.assertEqual(
            encoded_address,
            blockchain_address,
            "Default SS58 format for encoding of AppAgent address is set to wrong value.",
        )

        encoded_address_ss58_42 = trevokeyless.encode_app_agent_address(app_agent_id, 42)
        self.assertEqual(
            encoded_address_ss58_42,
            blockchain_address_ss58_42,
            "Encoding of AppAgent ID to AppAgent address doesn't take into account SS58 format.",
        )

        ###

        decoded_app_agent_id = trevokeyless.decode_app_agent_address(blockchain_address)
        self.assertEqual(
            decoded_app_agent_id,
            app_agent_id,
            "Default SS58 format for decoding of AppAgent address is set to wrong value.",
        )

        with self.assertRaisesRegex(
            ValueError,
            "^Expected SS58 format 42 but found 5335$",
            msg="Decoding of AppAgent address to AppAgent ID doesn't take into account SS58 format.",
        ):
            trevokeyless.decode_app_agent_address(blockchain_address, 42)

        ###

        decoded_address_info = trevokeyless.decode_address(blockchain_address)
        expected_address_info = trevokeyless.BlockchainAccountInfo(
            address=blockchain_address,
            account_id=account_id,
            account_type=trevokeyless.AccountType.AppAgent,
            app_agent_id=app_agent_id,
            ta_id=None,
            account_name=None,
        )
        self.assertEqual(
            decoded_address_info,
            expected_address_info,
            "Default SS58 format for decoding of AppAgent address is set to wrong value.",
        )

        with self.assertRaisesRegex(
            ValueError,
            "^Expected SS58 format 42 but found 5335$",
            msg="Decoding of AppAgent address to address info doesn't take into account SS58 format.",
        ):
            trevokeyless.decode_address(blockchain_address, 42)

    def test_transactional_address_ss58_format(self: "TestKeylessAddresses") -> None:
        app_agent_id = 123
        transactional_address_id = 456
        account_id = "0x7b00000002c801000033399aeb61b087f1a20a58c41ea5ff1b7bfb2fda27bfc0"
        blockchain_address = "ttowKp8AmjjQh4GoN7xMiQWwVyyrU1Pu7GRf5HxFmV5t43TXG"
        blockchain_address_ss58_42 = "5EqykH6F3zh6U8x1gjSGAU5aE4P6nwAoL6S9FW8SSNvEj5KD"

        ###

        encoded_address = trevokeyless.encode_transactional_address(app_agent_id, transactional_address_id)
        self.assertEqual(
            encoded_address,
            blockchain_address,
            "Default SS58 format for encoding of Transactional address is set to wrong value.",
        )

        encoded_address_ss58_42 = trevokeyless.encode_transactional_address(app_agent_id, transactional_address_id, 42)
        self.assertEqual(
            encoded_address_ss58_42,
            blockchain_address_ss58_42,
            "Encoding of Transactional address ID to Transactional address doesn't take into account SS58 format.",
        )

        ###

        address_data_1 = trevokeyless.decode_transactional_address(blockchain_address)
        self.assertEqual(
            address_data_1,
            (app_agent_id, transactional_address_id),
            "Default SS58 format for decoding of Transactional address is set to wrong value.",
        )

        with self.assertRaisesRegex(
            ValueError,
            "^Expected SS58 format 42 but found 5335$",
            msg="Decoding of Transactional address to Transactional address ID doesn't take into account SS58 format.",
        ):
            trevokeyless.decode_transactional_address(blockchain_address, 42)

        ###

        decoded_address_info = trevokeyless.decode_address(blockchain_address)
        expected_address_info = trevokeyless.BlockchainAccountInfo(
            address=blockchain_address,
            account_id=account_id,
            account_type=trevokeyless.AccountType.Transactional,
            app_agent_id=app_agent_id,
            ta_id=transactional_address_id,
            account_name=None,
        )
        self.assertEqual(
            decoded_address_info,
            expected_address_info,
            "Default SS58 format for decoding of Transactional address is set to wrong value.",
        )

        with self.assertRaisesRegex(
            ValueError,
            "^Expected SS58 format 42 but found 5335$",
            msg="Decoding of Transactional address to address info doesn't take into account SS58 format.",
        ):
            trevokeyless.decode_address(blockchain_address, 42)

    def test_named_address_ss58_format(self: "TestKeylessAddresses") -> None:
        app_agent_id = 123
        account_name = "example123"
        account_id = "0x7b000000036578616d706c653132330a72315aba532de9903c05a71e7d2d0409"
        blockchain_address = "ttowKp8Amrt2FQ2eNdsbMsW2EEeEsEmsT8KHLvPhVppp9zs8k"
        blockchain_address_ss58_42 = "5EqykH6FB9Jeoto2CefudTAJUimW2K99Bz4QswaAn7rLge2y"

        ###

        encoded_address = trevokeyless.encode_named_address(app_agent_id, account_name)
        self.assertEqual(
            encoded_address,
            blockchain_address,
            "Default SS58 format for encoding of Named address is set to wrong value.",
        )

        encoded_address_ss58_42 = trevokeyless.encode_named_address(app_agent_id, account_name, 42)
        self.assertEqual(
            encoded_address_ss58_42,
            blockchain_address_ss58_42,
            "Encoding of address name to Named address doesn't take into account SS58 format.",
        )

        ###

        address_data_1 = trevokeyless.decode_named_address(blockchain_address)
        self.assertEqual(
            address_data_1,
            (app_agent_id, account_name),
            "Default SS58 format for decoding of Transactional address is set to wrong value.",
        )

        with self.assertRaisesRegex(
            ValueError,
            "^Expected SS58 format 42 but found 5335$",
            msg="Decoding of Transactional address to Transactional address ID doesn't take into account SS58 format.",
        ):
            trevokeyless.decode_transactional_address(blockchain_address, 42)

        ###

        decoded_address_info = trevokeyless.decode_address(blockchain_address)
        expected_address_info = trevokeyless.BlockchainAccountInfo(
            address=blockchain_address,
            account_id=account_id,
            account_type=trevokeyless.AccountType.Named,
            app_agent_id=app_agent_id,
            ta_id=None,
            account_name=account_name,
        )
        self.assertEqual(
            decoded_address_info,
            expected_address_info,
            "Default SS58 format for decoding of Named address is set to wrong value.",
        )

        with self.assertRaisesRegex(
            ValueError,
            "^Expected SS58 format 42 but found 5335$",
            msg="Decoding of Named address to address info doesn't take into account SS58 format.",
        ):
            trevokeyless.decode_address(blockchain_address, 42)


if __name__ == "__main__":
    unittest.main()
