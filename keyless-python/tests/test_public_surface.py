"""Pins the public surface of the package.

The SS58 codec lives in the private module ``trevokeyless._ss58``; its helpers must not
be reachable under a public name (only its error types are re-exported), and
``__all__`` must stay the complete list of supported names.
"""

import unittest

import trevokeyless


class TestPublicSurface(unittest.TestCase):
    def test_all_is_the_complete_public_surface(self: "TestPublicSurface") -> None:
        expected = {
            "ACCOUNT_ID_LENGTH",
            "ACCOUNT_NAME_LENGTH",
            "APP_AGENT_ACCOUNT_IDENTIFIER",
            "TRANSACTIONAL_ACCOUNT_IDENTIFIER",
            "NAMED_ACCOUNT_IDENTIFIER",
            "SS58_FORMAT_TREVO_ASSET_HUB",
            "SS58_MAX_ADDRESS_LENGTH",
            "U32_MAX",
            "__version__",
            "AccountName",
            "AccountNameError",
            "AccountNameErrorKind",
            "AccountType",
            "AccountTypeMismatchError",
            "AppAgentId",
            "AppAgentAccountInfo",
            "BlockchainAccountId",
            "BlockchainAddress",
            "BlockchainAccountInfo",
            "DecodedAddressWithFormat",
            "DecodedTransactionalAddress",
            "DecodedNamedAddress",
            "NamedAccountInfo",
            "NotAnAddressError",
            "ParseAccountIdError",
            "RegularAccountInfo",
            "SS58Format",
            "Ss58Error",
            "Ss58ErrorKind",
            "Ss58FormatMismatchError",
            "TransactionalId",
            "TransactionalAccountInfo",
            "decode_account_id",
            "decode_address",
            "decode_address_with_format",
            "decode_app_agent_account_id",
            "decode_app_agent_address",
            "decode_named_account_id",
            "decode_named_address",
            "decode_transactional_account_id",
            "decode_transactional_address",
            "encode_address",
            "encode_app_agent_account_id",
            "encode_app_agent_address",
            "encode_named_account_id",
            "encode_named_address",
            "encode_transactional_account_id",
            "encode_transactional_address",
            "is_account_name",
            "is_app_agent_id",
            "is_keyless_account_id",
            "is_keyless_address",
            "is_transactional_id",
        }
        self.assertEqual(set(trevokeyless.__all__), expected)
        for name in trevokeyless.__all__:
            self.assertTrue(hasattr(trevokeyless, name), f"__all__ contains unresolvable name {name!r}")

    def test_exported_constant_values(self: "TestPublicSurface") -> None:
        # The values are part of the public API (callers size buffers and validate
        # inputs against them); pinning them here keeps a drift in the source from
        # passing while every other test re-declares its own literals.
        self.assertEqual(trevokeyless.ACCOUNT_ID_LENGTH, 32)
        self.assertEqual(trevokeyless.ACCOUNT_NAME_LENGTH, 10)
        self.assertEqual(trevokeyless.SS58_FORMAT_TREVO_ASSET_HUB, 5335)
        self.assertEqual(trevokeyless.SS58_MAX_ADDRESS_LENGTH, 50)
        self.assertEqual(trevokeyless.U32_MAX, 4294967295)
        # The byte-[4] account type discriminants, under the same names as the Rust
        # constants; they equal the corresponding AccountType values.
        self.assertEqual(trevokeyless.APP_AGENT_ACCOUNT_IDENTIFIER, trevokeyless.AccountType.AppAgent.value)
        self.assertEqual(trevokeyless.TRANSACTIONAL_ACCOUNT_IDENTIFIER, trevokeyless.AccountType.Transactional.value)
        self.assertEqual(trevokeyless.NAMED_ACCOUNT_IDENTIFIER, trevokeyless.AccountType.Named.value)

    def test_identifier_types_validate(self: "TestPublicSurface") -> None:
        # AppAgentId / TransactionalId are validating int subclasses, not identity
        # functions: constructing one out of range fails at the construction site, like
        # the TypeScript appAgentId() / transactionalId() factories.
        for identifier in (trevokeyless.AppAgentId, trevokeyless.TransactionalId):
            with self.subTest(identifier=identifier.__name__):
                self.assertEqual(identifier(7), 7)
                self.assertIsInstance(identifier(7), int)
                self.assertEqual(identifier(trevokeyless.U32_MAX), trevokeyless.U32_MAX)
                with self.assertRaises(ValueError):
                    identifier(-1)
                with self.assertRaises(ValueError):
                    identifier(trevokeyless.U32_MAX + 1)
                with self.assertRaises(TypeError):
                    identifier(True)
                with self.assertRaises(TypeError):
                    identifier("1")  # type: ignore[arg-type]

    def test_ss58_codec_module_is_private(self: "TestPublicSurface") -> None:
        # The SS58 codec helpers intentionally have no public name; only the error
        # types (Ss58Error, Ss58ErrorKind) defined alongside them are public.
        self.assertFalse(hasattr(trevokeyless, "ss58"))
        self.assertNotIn("ss58", trevokeyless.__all__)
        self.assertNotIn("_ss58", trevokeyless.__all__)
        self.assertNotIn("ss58_decode", trevokeyless.__all__)
        self.assertNotIn("ss58_encode", trevokeyless.__all__)


if __name__ == "__main__":
    unittest.main()
