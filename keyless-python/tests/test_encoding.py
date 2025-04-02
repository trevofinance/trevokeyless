import unittest

import trevokeyless


class TestKeylessAddresses(unittest.TestCase):
    def test_app_agent_address(self: "TestKeylessAddresses") -> None:
        ss58_format = 5335

        app_agent_id = 123
        account_id = "0x7b00000001293833058fc7db52fc03f6ce344bca98bd7825ff747743f1ff63e2"
        blockchain_address = "ttowKp8AmQuGfbBGikG2pbdYNnErhHRaLrdktJeZEfJVeVnTp"

        ###

        encoded_address = trevokeyless.encode_app_agent_address(app_agent_id, ss58_format)
        self.assertEqual(
            encoded_address,
            blockchain_address,
            "Couldn't encode AppAgent ID to AppAgent address.",
        )

        decoded_app_agent_id = trevokeyless.decode_app_agent_address(blockchain_address, ss58_format)
        self.assertEqual(
            decoded_app_agent_id,
            app_agent_id,
            "Couldn't decode AppAgent address to AppAgent ID.",
        )

        decoded_address_info = trevokeyless.decode_address(blockchain_address, ss58_format)
        expected_address_info = trevokeyless.BlockchainAddressInfo(
            address=blockchain_address,
            account_id=account_id,
            address_type=trevokeyless.AddressType.AppAgent,
            app_agent_id=app_agent_id,
            ta_id=None,
            address_name=None,
        )
        self.assertEqual(
            decoded_address_info,
            expected_address_info,
            "Couldn't decode AppAgent address to address info.",
        )

    def test_transactional_address(self: "TestKeylessAddresses") -> None:
        ss58_format = 5335

        app_agent_id = 123
        transactional_address_id = 456
        account_id = "0x7b00000002c801000033399aeb61b087f1a20a58c41ea5ff1b7bfb2fda27bfc0"
        blockchain_address = "ttowKp8AmjjQh4GoN7xMiQWwVyyrU1Pu7GRf5HxFmV5t43TXG"

        ###

        encoded_address = trevokeyless.encode_transactional_address(
            app_agent_id,
            transactional_address_id,
            ss58_format,
        )
        self.assertEqual(
            encoded_address,
            blockchain_address,
            "Couldn't encode Transactional address ID to Transactional address.",
        )

        transactional_address_data = trevokeyless.decode_transactional_address(encoded_address, ss58_format)
        self.assertEqual(
            transactional_address_data,
            (app_agent_id, transactional_address_id),
            "Couldn't decode Transactional address to AppAgent ID & Transactional address ID.",
        )

        decoded_address_info = trevokeyless.decode_address(encoded_address, ss58_format)
        expected_address_info = trevokeyless.BlockchainAddressInfo(
            address=blockchain_address,
            account_id=account_id,
            address_type=trevokeyless.AddressType.Transactional,
            app_agent_id=app_agent_id,
            ta_id=transactional_address_id,
            address_name=None,
        )
        self.assertEqual(
            decoded_address_info,
            expected_address_info,
            "Couldn't decode Transactional address to address info.",
        )

    def test_named_address(self: "TestKeylessAddresses") -> None:
        ss58_format = 5335

        app_agent_id = 123
        address_name = "example123"
        account_id = "0x7b000000036578616d706c653132330a72315aba532de9903c05a71e7d2d0409"
        blockchain_address = "ttowKp8Amrt2FQ2eNdsbMsW2EEeEsEmsT8KHLvPhVppp9zs8k"

        ###

        encoded_address = trevokeyless.encode_named_address(app_agent_id, address_name, ss58_format)
        self.assertEqual(
            encoded_address,
            blockchain_address,
            "Couldn't encode name of a Named address to Named address.",
        )

        named_address_data = trevokeyless.decode_named_address(encoded_address, ss58_format)
        self.assertEqual(
            named_address_data,
            (app_agent_id, address_name),
            "Couldn't decode Named address to AppAgent ID & address name.",
        )

        decoded_address_info = trevokeyless.decode_address(encoded_address, ss58_format)
        expected_address_info = trevokeyless.BlockchainAddressInfo(
            address=blockchain_address,
            account_id=account_id,
            address_type=trevokeyless.AddressType.Named,
            app_agent_id=app_agent_id,
            ta_id=None,
            address_name=address_name,
        )
        self.assertEqual(
            decoded_address_info,
            expected_address_info,
            "Couldn't decode Named address to address info.",
        )

    def test_mismatch_of_address_type(self: "TestKeylessAddresses") -> None:
        ss58_format = 5335

        # Alice
        account_id = "0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d"
        regular_address = "ttqxHzRJmmjFBcE7Lb5Xs4GNMq2gFSt28JyvTEqjhqzE9EGP4"

        ###

        with self.assertRaisesRegex(
            ValueError,
            "^Provided address is not an AppAgent address but is instance of AddressType.Regular.$",
            msg="Decoding of regular address as an AppAgent address failed.",
        ):
            trevokeyless.decode_app_agent_address(regular_address, ss58_format)

        with self.assertRaisesRegex(
            ValueError,
            "^Provided address is not a Transactional address but is instance of AddressType.Regular.$",
            msg="Decoding of regular address as an AppAgent address failed.",
        ):
            trevokeyless.decode_transactional_address(regular_address, ss58_format)

        with self.assertRaisesRegex(
            ValueError,
            "^Provided address is not a Named address but is instance of AddressType.Regular.$",
            msg="Decoding of regular address as an AppAgent address failed.",
        ):
            trevokeyless.decode_named_address(regular_address, ss58_format)

        address_data = trevokeyless.decode_address(regular_address, ss58_format)
        self.assertEqual(
            address_data,
            trevokeyless.BlockchainAddressInfo(
                address=regular_address,
                account_id=account_id,
                address_type=trevokeyless.AddressType.Regular,
                app_agent_id=None,
                ta_id=None,
                address_name=None,
            ),
            "Decoding of regular address to address info failed.",
        )


if __name__ == "__main__":
    unittest.main()
