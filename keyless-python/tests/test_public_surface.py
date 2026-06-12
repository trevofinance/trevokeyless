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
            "NAMED_ADDRESS_LENGTH",
            "SS58_FORMAT_TREVO_ASSET_HUB",
            "SS58_MAX_ADDRESS_LENGTH",
            "U32_MAX",
            "__version__",
            "AddressName",
            "AddressType",
            "AddressTypeMismatchError",
            "AppAgentId",
            "BlockchainAccountId",
            "BlockchainAddress",
            "BlockchainAddressInfo",
            "SS58Format",
            "Ss58DecodeError",
            "Ss58ErrorKind",
            "Ss58FormatMismatchError",
            "TransactionalAddressId",
            "decode_account_id",
            "decode_address",
            "decode_app_agent_address",
            "decode_named_address",
            "decode_transactional_address",
            "encode_address",
            "encode_app_agent_address",
            "encode_named_address",
            "encode_transactional_address",
            "is_keyless_address",
        }
        self.assertEqual(set(trevokeyless.__all__), expected)
        for name in trevokeyless.__all__:
            self.assertTrue(hasattr(trevokeyless, name), f"__all__ contains unresolvable name {name!r}")

    def test_ss58_codec_module_is_private(self: "TestPublicSurface") -> None:
        # The SS58 codec helpers intentionally have no public name; only the error
        # types (Ss58DecodeError, Ss58ErrorKind) defined alongside them are public.
        self.assertFalse(hasattr(trevokeyless, "ss58"))
        self.assertNotIn("ss58", trevokeyless.__all__)
        self.assertNotIn("_ss58", trevokeyless.__all__)
        self.assertNotIn("ss58_decode", trevokeyless.__all__)
        self.assertNotIn("ss58_encode", trevokeyless.__all__)


if __name__ == "__main__":
    unittest.main()
