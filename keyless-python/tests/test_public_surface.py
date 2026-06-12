"""Pins the public surface of the package.

The vendored generic SS58 codec lives in the private module ``trevokeyless._ss58``;
it must not be reachable under a public name, and ``__all__`` must stay the complete
list of supported names.
"""

import unittest

import trevokeyless


class TestPublicSurface(unittest.TestCase):
    def test_all_is_the_complete_public_surface(self: "TestPublicSurface") -> None:
        expected = {
            "ACCOUNT_ID_LENGTH",
            "NAMED_ADDRESS_LENGTH",
            "SS58_FORMAT__TREVO_ASSET_HUB",
            "SS58_MAX_ADDRESS_LENGTH",
            "U32_MAX",
            "AddressName",
            "AddressType",
            "AddressTypeMismatchError",
            "AppAgentId",
            "BlockchainAccountId",
            "BlockchainAddress",
            "BlockchainAddressInfo",
            "SS58Format",
            "TransactionalAddressId",
            "decode_account_id",
            "decode_address",
            "decode_app_agent_address",
            "decode_named_address",
            "decode_transactional_address",
            "encode_app_agent_address",
            "encode_named_address",
            "encode_transactional_address",
            "is_keyless_address",
        }
        self.assertEqual(set(trevokeyless.__all__), expected)
        for name in trevokeyless.__all__:
            self.assertTrue(hasattr(trevokeyless, name), f"__all__ contains unresolvable name {name!r}")

    def test_vendored_ss58_module_is_private(self: "TestPublicSurface") -> None:
        # The generic Substrate SS58 helpers intentionally have no public name: their
        # semantics (hex passthrough, non-32-byte payloads) are broader than the
        # contract of this library.
        self.assertFalse(hasattr(trevokeyless, "ss58"))
        self.assertNotIn("ss58", trevokeyless.__all__)
        self.assertNotIn("_ss58", trevokeyless.__all__)


if __name__ == "__main__":
    unittest.main()
