"""Adversarial and boundary-value tests, mirroring the Rust test suite
(`tests_ss58.rs`, `tests_properties.rs`, `tests_encoding.rs`).

The crafted inputs are built with the reference construction rules so that each test
documents exactly what is being fed to the decoder.
"""

import unittest
from hashlib import blake2b

import base58

import trevokeyless
from trevokeyless import AddressType, AddressTypeMismatchError
from trevokeyless.ss58 import ss58_encode

SS58_FORMAT = 5335
U32_MAX = 2**32 - 1


def _ss58_checksum(payload: bytes) -> bytes:
    """First bytes of blake2b-512(b"SS58PRE" + payload) are the SS58 checksum."""
    return blake2b(b"SS58PRE" + payload, digest_size=64).digest()


def _raw_ss58(payload: bytes, checksum_length: int = 2) -> str:
    """Wraps a raw prefix+data payload in an SS58 envelope with a valid checksum."""
    return base58.b58encode(payload + _ss58_checksum(payload)[:checksum_length]).decode()


def _format_prefix(ss58_format: int) -> bytes:
    """The 1- or 2-byte SS58 prefix encoding of a format."""
    if ss58_format < 64:  # noqa: PLR2004 - boundary defined by the SS58 specification
        return bytes([ss58_format])
    return bytes(
        [
            ((ss58_format & 0b0000_0000_1111_1100) >> 2) | 0b0100_0000,
            (ss58_format >> 8) | ((ss58_format & 0b0000_0000_0000_0011) << 6),
        ]
    )


def _keyless_account(open_part: bytes) -> bytes:
    """32-byte keyless account id: open part + trailing bytes of its blake2b-256 hash."""
    checksum = blake2b(open_part, digest_size=32).digest()
    return open_part + checksum[len(open_part) :]


# The AppAgent(123) account used by the shared golden vectors.
APP_AGENT_123 = _keyless_account((123).to_bytes(4, byteorder="little") + b"\x01")


class TestBoundaryValues(unittest.TestCase):
    """Mirrors the Rust `test_boundary_values*` tests."""

    def test_app_agent_id_boundaries(self: "TestBoundaryValues") -> None:
        for app_agent_id in [0, 1, U32_MAX]:
            with self.subTest(app_agent_id=app_agent_id):
                address = trevokeyless.encode_app_agent_address(app_agent_id, SS58_FORMAT)
                decoded = trevokeyless.decode_app_agent_address(address, SS58_FORMAT)
                self.assertEqual(decoded, app_agent_id)

    def test_transactional_id_boundaries(self: "TestBoundaryValues") -> None:
        for app_agent_id, ta_id in [(0, 0), (0, U32_MAX), (U32_MAX, 0), (U32_MAX, U32_MAX)]:
            with self.subTest(app_agent_id=app_agent_id, ta_id=ta_id):
                address = trevokeyless.encode_transactional_address(app_agent_id, ta_id, SS58_FORMAT)
                decoded = trevokeyless.decode_transactional_address(address, SS58_FORMAT)
                self.assertEqual(decoded, (app_agent_id, ta_id))

    def test_named_boundaries(self: "TestBoundaryValues") -> None:
        for name in ["0000000000", "##########", "----------", "ZZZZZZZZZZ"]:
            for app_agent_id in [0, U32_MAX]:
                with self.subTest(app_agent_id=app_agent_id, name=name):
                    address = trevokeyless.encode_named_address(app_agent_id, name, SS58_FORMAT)
                    decoded = trevokeyless.decode_named_address(address, SS58_FORMAT)
                    self.assertEqual(decoded, (app_agent_id, name))


class TestAccountIdLength(unittest.TestCase):
    """Account ids must be exactly 32 bytes; the Rust AccountId32 makes this structural."""

    def test_rejects_short_ss58_payloads(self: "TestAccountIdLength") -> None:
        # Valid SS58 envelopes (correct checksum, format 5335) around 1-, 4- and 8-byte
        # payloads: legal "account index" lengths in the generic SS58 scheme, but never a
        # 32-byte account id. Previously these raised IndexError or decoded as Regular.
        for payload in [b"\x07", b"\xde\xad\xbe\xef", bytes(8)]:
            with self.subTest(payload=payload):
                address = ss58_encode(payload, SS58_FORMAT)
                with self.assertRaises(ValueError):
                    trevokeyless.decode_address(address, SS58_FORMAT)

    def test_rejects_33_byte_payload(self: "TestAccountIdLength") -> None:
        # A 33-byte payload (ECDSA-style public key) is legal generic SS58 but never a
        # 32-byte account id; its address is 51 chars, so the length pre-bound fires.
        address = ss58_encode(APP_AGENT_123 + b"\x00", SS58_FORMAT)
        with self.assertRaises(ValueError):
            trevokeyless.decode_address(address, SS58_FORMAT)

    def test_rejects_31_byte_payload(self: "TestAccountIdLength") -> None:
        # Not a legal SS58 length at all; built manually since ss58_encode refuses it.
        address = _raw_ss58(_format_prefix(SS58_FORMAT) + APP_AGENT_123[:31])
        with self.assertRaises(ValueError):
            trevokeyless.decode_address(address, SS58_FORMAT)


class TestMalformedSs58(unittest.TestCase):
    """Mirrors the Rust `tests_ss58.rs` rejection tests."""

    def test_rejects_tampered_ss58_checksum(self: "TestMalformedSs58") -> None:
        address = trevokeyless.encode_app_agent_address(123, SS58_FORMAT)
        payload = bytearray(base58.b58decode(address))
        payload[-1] ^= 0xFF
        tampered = base58.b58encode(bytes(payload)).decode()
        with self.assertRaisesRegex(ValueError, "^Invalid checksum$"):
            trevokeyless.decode_address(tampered, SS58_FORMAT)

    def test_tampered_keyless_checksum_decodes_as_regular(self: "TestMalformedSs58") -> None:
        # Corrupt a byte in the keyless checksum region (open part of an AppAgent account
        # is bytes [0..5]) and rewrap in a valid SS58 envelope: the SS58 layer accepts it,
        # the keyless layer must demote it to Regular.
        account = bytearray(APP_AGENT_123)
        account[10] ^= 0xFF
        address = ss58_encode(bytes(account), SS58_FORMAT)
        info = trevokeyless.decode_address(address, SS58_FORMAT)
        self.assertIs(info.address_type, AddressType.Regular)
        self.assertIsNone(info.app_agent_id)

    def test_rejects_reserved_format(self: "TestMalformedSs58") -> None:
        # Reserved format 46, crafted with a valid checksum (the encoder refuses to
        # produce it).
        reserved_format = 46
        address = _raw_ss58(_format_prefix(reserved_format) + APP_AGENT_123)
        with self.assertRaisesRegex(ValueError, "reserved SS58 format"):
            trevokeyless.decode_address(address, reserved_format)
        with self.assertRaises(ValueError):
            ss58_encode(APP_AGENT_123, reserved_format)

    def test_rejects_prefix_byte_in_reserved_range(self: "TestMalformedSs58") -> None:
        # Per the SS58 spec the first byte must be 0..=63 or 64..=127. A crafted address
        # with first byte 0x80 and a valid checksum must be rejected (the Rust and
        # polkadot-js decoders reject it; this pins the fix of the vendored decoder).
        address = _raw_ss58(bytes([0x80]) + APP_AGENT_123)
        with self.assertRaisesRegex(ValueError, "^Invalid SS58 format prefix$"):
            trevokeyless.decode_address(address, 128)

    def test_accepts_noncanonical_two_byte_prefix(self: "TestMalformedSs58") -> None:
        # The two-byte SS58 prefix can also (non-canonically) encode formats below 64.
        # Like the Rust and upstream Substrate codecs, decoding accepts such addresses.
        small_format = 42
        noncanonical_prefix = bytes(
            [
                ((small_format & 0b0000_0000_1111_1100) >> 2) | 0b0100_0000,
                (small_format >> 8) | ((small_format & 0b0000_0000_0000_0011) << 6),
            ]
        )
        address = _raw_ss58(noncanonical_prefix + APP_AGENT_123)
        info = trevokeyless.decode_address(address, small_format)
        self.assertIs(info.address_type, AddressType.AppAgent)
        self.assertEqual(info.app_agent_id, 123)

    def test_rejects_garbage_input(self: "TestMalformedSs58") -> None:
        # All of these must raise ValueError (never IndexError or any other type).
        for garbage in ["", " ", "   ", "0OIl", "x", "1"]:
            with self.subTest(garbage=garbage), self.assertRaises(ValueError):
                trevokeyless.decode_address(garbage, SS58_FORMAT)

    def test_rejects_oversized_input_before_base58_decoding(self: "TestMalformedSs58") -> None:
        oversized = "1" * (trevokeyless.SS58_MAX_ADDRESS_LENGTH + 1)
        with self.assertRaisesRegex(ValueError, "^Invalid address length$"):
            trevokeyless.decode_address(oversized, SS58_FORMAT)

    def test_longest_legitimate_address_still_decodes(self: "TestMalformedSs58") -> None:
        # The longest legitimate addresses (two-byte prefix, high account id bytes) stay
        # within the bound and must still decode.
        max_format = 16383
        address = ss58_encode(b"\xff" * 32, max_format)
        self.assertLessEqual(len(address), trevokeyless.SS58_MAX_ADDRESS_LENGTH)
        info = trevokeyless.decode_address(address, max_format)
        self.assertIs(info.address_type, AddressType.Regular)


class TestHexAccountIdContract(unittest.TestCase):
    """A raw account id ("0x" + 64 hex chars) is accepted and validated; anything else
    hex-like is rejected."""

    def test_valid_hex_account_id_is_classified(self: "TestHexAccountIdContract") -> None:
        info = trevokeyless.decode_address("0x" + APP_AGENT_123.hex(), SS58_FORMAT)
        self.assertIs(info.address_type, AddressType.AppAgent)
        self.assertEqual(info.app_agent_id, 123)
        self.assertEqual(info.account_id, "0x" + APP_AGENT_123.hex())

    def test_uppercase_hex_is_normalized(self: "TestHexAccountIdContract") -> None:
        info = trevokeyless.decode_address("0x" + APP_AGENT_123.hex().upper(), SS58_FORMAT)
        self.assertIs(info.address_type, AddressType.AppAgent)
        self.assertEqual(info.account_id, "0x" + APP_AGENT_123.hex())

    def test_rejects_hex_of_wrong_length(self: "TestHexAccountIdContract") -> None:
        for bad_hex in ["0x", "0xdeadbeef", "0x" + APP_AGENT_123.hex() + "00"]:
            with self.subTest(bad_hex=bad_hex), self.assertRaisesRegex(ValueError, "^Invalid account id length"):
                trevokeyless.decode_address(bad_hex, SS58_FORMAT)

    def test_rejects_invalid_hex(self: "TestHexAccountIdContract") -> None:
        for bad_hex in ["0xabc", "0xzz" + "00" * 31]:
            with self.subTest(bad_hex=bad_hex), self.assertRaisesRegex(ValueError, "^Invalid hex account id$"):
                trevokeyless.decode_address(bad_hex, SS58_FORMAT)


class TestIsKeylessAddress(unittest.TestCase):
    """Mirrors the Rust `test_is_keyless_account` test."""

    def test_keyless_addresses(self: "TestIsKeylessAddress") -> None:
        keyless_addresses = [
            trevokeyless.encode_app_agent_address(1, SS58_FORMAT),
            trevokeyless.encode_transactional_address(1, 2, SS58_FORMAT),
            trevokeyless.encode_named_address(1, "test123456", SS58_FORMAT),
        ]
        for address in keyless_addresses:
            with self.subTest(address=address):
                self.assertTrue(trevokeyless.is_keyless_address(address, SS58_FORMAT))

    def test_regular_addresses(self: "TestIsKeylessAddress") -> None:
        # Alice's well-known development account.
        alice = "ttqxHzRJmmjFBcE7Lb5Xs4GNMq2gFSt28JyvTEqjhqzE9EGP4"
        self.assertFalse(trevokeyless.is_keyless_address(alice, SS58_FORMAT))
        # A zeroed account has no valid keyless discriminant.
        self.assertFalse(trevokeyless.is_keyless_address("0x" + "00" * 32, SS58_FORMAT))


class TestAddressTypeMismatchError(unittest.TestCase):
    def test_carries_expected_and_found(self: "TestAddressTypeMismatchError") -> None:
        named_address = trevokeyless.encode_named_address(1, "test123456", SS58_FORMAT)
        with self.assertRaises(AddressTypeMismatchError) as ctx:
            trevokeyless.decode_app_agent_address(named_address, SS58_FORMAT)
        self.assertIs(ctx.exception.expected, AddressType.AppAgent)
        self.assertIs(ctx.exception.found, AddressType.Named)
        # Backwards compatibility: the error is still a ValueError with the same message.
        self.assertIsInstance(ctx.exception, ValueError)
        self.assertEqual(
            str(ctx.exception),
            "Provided address is not an AppAgent address but is instance of AddressType.Named.",
        )


if __name__ == "__main__":
    unittest.main()
