import unittest

import trevokeyless


class TestKeylessAddresses(unittest.TestCase):
    # Tests check that SS58 is properly processed

    def test_app_agent_address_ss58_format(self: "TestKeylessAddresses") -> None:
        app_agent_id = trevokeyless.AppAgentId(123)
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

        encoded_address_ss58_42 = trevokeyless.encode_app_agent_address(app_agent_id, ss58_format=42)
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
            trevokeyless.decode_app_agent_address(blockchain_address, ss58_format=42)

        ###

        decoded_address_info = trevokeyless.decode_address(blockchain_address)
        expected_address_info = trevokeyless.AppAgentAccountInfo(
            address=blockchain_address,
            account_id=account_id,
            app_agent_id=app_agent_id,
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
            trevokeyless.decode_address(blockchain_address, ss58_format=42)

    def test_transactional_address_ss58_format(self: "TestKeylessAddresses") -> None:
        app_agent_id = trevokeyless.AppAgentId(123)
        transactional_address_id = trevokeyless.TransactionalId(456)
        account_id = "0x7b00000002c801000033399aeb61b087f1a20a58c41ea5ff1b7bfb2fda27bfc0"
        blockchain_address = "ttowKp8AmjjQh4GoN7xMiQWwVyyrU1Pu7GRf5HxFmV5t43TXG"
        blockchain_address_ss58_42 = "5EqykH6F3zh6U8x1gjSGAU5aE4P6nwAoL6S9FW8SSNvEj5KD"

        ###

        encoded_address = trevokeyless.encode_transactional_address(
            app_agent_id=app_agent_id,
            ta_id=transactional_address_id,
        )
        self.assertEqual(
            encoded_address,
            blockchain_address,
            "Default SS58 format for encoding of Transactional address is set to wrong value.",
        )

        encoded_address_ss58_42 = trevokeyless.encode_transactional_address(
            app_agent_id=app_agent_id,
            ta_id=transactional_address_id,
            ss58_format=42,
        )
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
            trevokeyless.decode_transactional_address(blockchain_address, ss58_format=42)

        ###

        decoded_address_info = trevokeyless.decode_address(blockchain_address)
        expected_address_info = trevokeyless.TransactionalAccountInfo(
            address=blockchain_address,
            account_id=account_id,
            app_agent_id=app_agent_id,
            ta_id=transactional_address_id,
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
            trevokeyless.decode_address(blockchain_address, ss58_format=42)

    def test_named_address_ss58_format(self: "TestKeylessAddresses") -> None:
        app_agent_id = trevokeyless.AppAgentId(123)
        account_name = trevokeyless.AccountName("example123")
        account_id = "0x7b000000036578616d706c653132330a72315aba532de9903c05a71e7d2d0409"
        blockchain_address = "ttowKp8Amrt2FQ2eNdsbMsW2EEeEsEmsT8KHLvPhVppp9zs8k"
        blockchain_address_ss58_42 = "5EqykH6FB9Jeoto2CefudTAJUimW2K99Bz4QswaAn7rLge2y"

        ###

        encoded_address = trevokeyless.encode_named_address(
            app_agent_id=app_agent_id,
            account_name=account_name,
        )
        self.assertEqual(
            encoded_address,
            blockchain_address,
            "Default SS58 format for encoding of Named address is set to wrong value.",
        )

        encoded_address_ss58_42 = trevokeyless.encode_named_address(
            app_agent_id=app_agent_id,
            account_name=account_name,
            ss58_format=42,
        )
        self.assertEqual(
            encoded_address_ss58_42,
            blockchain_address_ss58_42,
            "Encoding of an account name to a Named address doesn't take into account SS58 format.",
        )

        ###

        address_data_1 = trevokeyless.decode_named_address(blockchain_address)
        self.assertEqual(
            address_data_1,
            (app_agent_id, account_name),
            "Default SS58 format for decoding of Named address is set to wrong value.",
        )

        with self.assertRaisesRegex(
            ValueError,
            "^Expected SS58 format 42 but found 5335$",
            msg="Decoding of Named address to Named address data doesn't take into account SS58 format.",
        ):
            trevokeyless.decode_named_address(blockchain_address, ss58_format=42)

        ###

        decoded_address_info = trevokeyless.decode_address(blockchain_address)
        expected_address_info = trevokeyless.NamedAccountInfo(
            address=blockchain_address,
            account_id=account_id,
            app_agent_id=app_agent_id,
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
            trevokeyless.decode_address(blockchain_address, ss58_format=42)

    def test_decode_address_with_format(self: "TestKeylessAddresses") -> None:
        # decode_address_with_format decodes an address of any (non-reserved) format,
        # returning the classification and the format found in the address - the
        # counterpart of the Rust from_ss58check_with_version for addresses whose
        # format is not known up front.
        app_agent_id = 123
        account_id = "0x7b00000001293833058fc7db52fc03f6ce344bca98bd7825ff747743f1ff63e2"
        blockchain_address = "ttowKp8AmQuGfbBGikG2pbdYNnErhHRaLrdktJeZEfJVeVnTp"
        blockchain_address_ss58_42 = "5EqykH6EjAZ513RNK37NMagT2KPL4xr2vJXxGCRucbXqBSA7"

        # The Trevo-format spelling decodes to the same info as decode_address, plus
        # the format found in the address; the pair unpacks as a named tuple.
        info, ss58_format = trevokeyless.decode_address_with_format(blockchain_address)
        self.assertEqual(ss58_format, trevokeyless.SS58_FORMAT_TREVO_ASSET_HUB)
        self.assertEqual(info, trevokeyless.decode_address(blockchain_address))

        # The format-42 spelling of the same account id decodes without an expected
        # format (decode_address would raise Ss58FormatMismatchError).
        result = trevokeyless.decode_address_with_format(blockchain_address_ss58_42)
        self.assertIsInstance(result, trevokeyless.DecodedAddressWithFormat)
        self.assertEqual(result.ss58_format, 42)
        self.assertEqual(result.info.account_id, account_id)
        # `assert isinstance` rather than assertIsInstance: it asserts the same thing and
        # also narrows the BlockchainAccountInfo union for the type checker.
        assert isinstance(result.info, trevokeyless.AppAgentAccountInfo)  # noqa: S101 - isinstance narrowing for mypy
        self.assertEqual(result.info.app_agent_id, app_agent_id)
        # The address field is the input spelling verbatim.
        self.assertEqual(result.info.address, blockchain_address_ss58_42)

        # An invalid address raises its validation kind, exactly like decode_address;
        # only the expected-format comparison is skipped.
        with self.assertRaises(trevokeyless.Ss58Error) as ctx:
            trevokeyless.decode_address_with_format(blockchain_address[:-1])
        self.assertNotEqual(ctx.exception.kind, "UnexpectedFormat")

        # Raw account ids carry no SS58 format to report and are rejected outright.
        with self.assertRaisesRegex(
            ValueError,
            "^Raw account id hex is not an SS58 address; use decode_account_id$",
        ):
            trevokeyless.decode_address_with_format(account_id)

        with self.assertRaisesRegex(TypeError, "^blockchain_address must be a str, got int$"):
            trevokeyless.decode_address_with_format(123)  # type: ignore[arg-type]


if __name__ == "__main__":
    unittest.main()
