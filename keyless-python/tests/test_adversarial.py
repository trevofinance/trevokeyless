"""Adversarial and boundary-value tests, mirroring the Rust test suite
(`tests_ss58.rs`, `tests_properties.rs`, `tests_encoding.rs`).

The crafted inputs are built with the reference construction rules so that each test
documents exactly what is being fed to the decoder.
"""

import dataclasses
import unittest
from hashlib import blake2b

import base58

import trevokeyless
from trevokeyless import AddressType, AddressTypeMismatchError, Ss58DecodeError
from trevokeyless._ss58 import ss58_decode, ss58_encode

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


class TestEncodeIdValidation(unittest.TestCase):
    """Mirrors the TS `rejects out-of-range ids at encoding time` test. The Rust
    implementation enforces the range through the u32 parameter types."""

    def test_rejects_out_of_range_ids(self: "TestEncodeIdValidation") -> None:
        for bad_id in [-1, U32_MAX + 1, 2**40]:
            with self.subTest(bad_id=bad_id):
                with self.assertRaisesRegex(ValueError, r"^app_agent_id must be an integer in \[0, 4294967295\]"):
                    trevokeyless.encode_app_agent_address(bad_id, SS58_FORMAT)
                with self.assertRaisesRegex(ValueError, r"^ta_id must be an integer in \[0, 4294967295\]"):
                    trevokeyless.encode_transactional_address(1, bad_id, SS58_FORMAT)
                with self.assertRaisesRegex(ValueError, r"^app_agent_id must be an integer in \[0, 4294967295\]"):
                    trevokeyless.encode_named_address(bad_id, "example123", SS58_FORMAT)

    def test_rejects_non_integer_ids(self: "TestEncodeIdValidation") -> None:
        for bad_id in [1.5, "123", None]:
            with self.subTest(bad_id=bad_id), self.assertRaisesRegex(ValueError, "must be an integer"):
                trevokeyless.encode_app_agent_address(bad_id, SS58_FORMAT)  # type: ignore[arg-type]

    def test_rejects_bool_ids(self: "TestEncodeIdValidation") -> None:
        # bool is an int subclass, but silently encoding True as 1 would hide a caller
        # bug that the TypeScript (Number.isInteger) and Rust (u32) implementations reject.
        for bad_id in [True, False]:
            with self.subTest(bad_id=bad_id):
                with self.assertRaisesRegex(ValueError, "must be an integer"):
                    trevokeyless.encode_app_agent_address(bad_id, SS58_FORMAT)
                with self.assertRaisesRegex(ValueError, "must be an integer"):
                    trevokeyless.encode_transactional_address(1, bad_id, SS58_FORMAT)

    def test_accepts_u32_boundaries(self: "TestEncodeIdValidation") -> None:
        for good_id in [0, U32_MAX]:
            with self.subTest(good_id=good_id):
                trevokeyless.encode_app_agent_address(good_id, SS58_FORMAT)
                trevokeyless.encode_transactional_address(good_id, good_id, SS58_FORMAT)
                trevokeyless.encode_named_address(good_id, "example123", SS58_FORMAT)


class TestAccountIdLength(unittest.TestCase):
    """Account ids must be exactly 32 bytes; the Rust AccountId32 makes this structural."""

    def test_rejects_short_ss58_payloads(self: "TestAccountIdLength") -> None:
        # SS58 envelopes (correct checksum, format 5335) around 1-, 4- and 8-byte
        # payloads with the one-byte checksum of the generic "account index" scheme:
        # never a 32-byte account id. Built manually since ss58_encode refuses them.
        for payload in [b"\x07", b"\xde\xad\xbe\xef", bytes(8)]:
            with self.subTest(payload=payload):
                address = _raw_ss58(_format_prefix(SS58_FORMAT) + payload, checksum_length=1)
                with self.assertRaises(Ss58DecodeError) as ctx:
                    trevokeyless.decode_address(address, SS58_FORMAT)
                self.assertEqual(ctx.exception.kind, "InvalidLength")

    def test_rejects_33_byte_payload(self: "TestAccountIdLength") -> None:
        # A 33-byte payload (ECDSA-style public key) is legal generic SS58 but never a
        # 32-byte account id; its address is 51 chars, so the length pre-bound fires.
        address = _raw_ss58(_format_prefix(SS58_FORMAT) + APP_AGENT_123 + b"\x00")
        with self.assertRaises(Ss58DecodeError) as ctx:
            trevokeyless.decode_address(address, SS58_FORMAT)
        self.assertEqual(ctx.exception.kind, "InvalidLength")

    def test_rejects_31_byte_payload(self: "TestAccountIdLength") -> None:
        # Not a legal SS58 length at all.
        address = _raw_ss58(_format_prefix(SS58_FORMAT) + APP_AGENT_123[:31])
        with self.assertRaises(Ss58DecodeError) as ctx:
            trevokeyless.decode_address(address, SS58_FORMAT)
        self.assertEqual(ctx.exception.kind, "InvalidLength")


class TestMalformedSs58(unittest.TestCase):
    """Mirrors the Rust `tests_ss58.rs` rejection tests."""

    def test_rejects_tampered_ss58_checksum(self: "TestMalformedSs58") -> None:
        address = trevokeyless.encode_app_agent_address(123, SS58_FORMAT)
        payload = bytearray(base58.b58decode(address))
        payload[-1] ^= 0xFF
        tampered = base58.b58encode(bytes(payload)).decode()
        with self.assertRaisesRegex(Ss58DecodeError, "^Address has an invalid checksum$") as ctx:
            trevokeyless.decode_address(tampered, SS58_FORMAT)
        self.assertEqual(ctx.exception.kind, "InvalidChecksum")

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
        with self.assertRaisesRegex(Ss58DecodeError, "^SS58 format is invalid or reserved$") as ctx:
            trevokeyless.decode_address(address, reserved_format)
        self.assertEqual(ctx.exception.kind, "InvalidFormat")
        with self.assertRaises(Ss58DecodeError) as encode_ctx:
            ss58_encode(APP_AGENT_123, reserved_format)
        self.assertEqual(encode_ctx.exception.kind, "InvalidFormat")

    def test_rejects_prefix_byte_in_reserved_range(self: "TestMalformedSs58") -> None:
        # Per the SS58 spec the first byte must be 0..=63 or 64..=127. A crafted address
        # with first byte 0x80 and a valid checksum must be rejected, exactly like the
        # Rust and TypeScript decoders.
        address = _raw_ss58(bytes([0x80]) + APP_AGENT_123)
        with self.assertRaisesRegex(Ss58DecodeError, "^SS58 format is invalid or reserved$") as ctx:
            trevokeyless.decode_address(address, 128)
        self.assertEqual(ctx.exception.kind, "InvalidFormat")

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
        with self.assertRaisesRegex(Ss58DecodeError, "^Address has an invalid length$") as ctx:
            trevokeyless.decode_address(oversized, SS58_FORMAT)
        self.assertEqual(ctx.exception.kind, "InvalidLength")

        # The codec applies the same bound before charset validation: oversized input
        # full of out-of-alphabet characters is still classified as a length failure,
        # exactly like the Rust decoder.
        with self.assertRaises(Ss58DecodeError) as codec_ctx:
            ss58_decode("$" * (trevokeyless.SS58_MAX_ADDRESS_LENGTH + 1))
        self.assertEqual(codec_ctx.exception.kind, "InvalidLength")

    def test_longest_legitimate_address_still_decodes(self: "TestMalformedSs58") -> None:
        # The longest legitimate addresses (two-byte prefix, high account id bytes) stay
        # within the bound and must still decode.
        max_format = 16383
        address = ss58_encode(b"\xff" * 32, max_format)
        self.assertLessEqual(len(address), trevokeyless.SS58_MAX_ADDRESS_LENGTH)
        info = trevokeyless.decode_address(address, max_format)
        self.assertIs(info.address_type, AddressType.Regular)


class TestSurroundingWhitespace(unittest.TestCase):
    """An address with surrounding whitespace is not the canonical spelling of the
    account: whitespace is outside the base58 alphabet and must be rejected, as in the
    Rust and TypeScript implementations (pinned by the ss58_invalid conformance
    vectors). base58.b58decode right-strips its input, so the charset is checked
    explicitly before decoding."""

    def test_decode_rejects_surrounding_whitespace(self: "TestSurroundingWhitespace") -> None:
        address = trevokeyless.encode_app_agent_address(123, SS58_FORMAT)
        spellings = [
            address + " ",
            " " + address,
            address + "\n",
            address + "\t",
            address + "\u00a0",  # non-breaking space, also stripped by str.rstrip
        ]
        for spelling in spellings:
            with self.subTest(spelling=spelling):
                with self.assertRaisesRegex(Ss58DecodeError, "^Address is not valid base58$") as ctx:
                    trevokeyless.decode_address(spelling, SS58_FORMAT)
                self.assertEqual(ctx.exception.kind, "InvalidBase58")
                # The codec itself applies the same charset check before base58.b58decode.
                with self.assertRaises(Ss58DecodeError) as codec_ctx:
                    ss58_decode(spelling)
                self.assertEqual(codec_ctx.exception.kind, "InvalidBase58")


class TestStrictCodecContract(unittest.TestCase):
    """The private codec (trevokeyless._ss58) is a strict port of the Rust reference
    decoder: bytes in, bytes out, 32-byte account ids only, no hex passthrough. The
    historically vendored generic codec accepted "0x" strings and other payload sizes;
    these tests pin their removal."""

    def test_ss58_decode_has_no_hex_passthrough(self: "TestStrictCodecContract") -> None:
        # "0x..." is not an SS58 address. A full 66-character account id hex string
        # exceeds the 50-character bound (checked first, like in the Rust decoder);
        # shorter "0x" strings fail the base58 charset ('0' and 'x' are excluded).
        with self.assertRaises(Ss58DecodeError) as ctx:
            ss58_decode("0x" + APP_AGENT_123.hex())
        self.assertEqual(ctx.exception.kind, "InvalidLength")

        with self.assertRaises(Ss58DecodeError) as short_ctx:
            ss58_decode("0x" + APP_AGENT_123.hex()[:40])
        self.assertEqual(short_ctx.exception.kind, "InvalidBase58")

    def test_ss58_decode_returns_account_bytes_and_format(self: "TestStrictCodecContract") -> None:
        # The expected format is deliberately not checked by the codec: it returns the
        # found format, and the caller compares only after full validation (the Rust
        # from_ss58check_with_version contract).
        address = ss58_encode(APP_AGENT_123, SS58_FORMAT)
        self.assertEqual(ss58_decode(address), (APP_AGENT_123, SS58_FORMAT))

    def test_ss58_encode_accepts_bytes_only(self: "TestStrictCodecContract") -> None:
        # 32-byte payloads only; account-index and public-key sizes are rejected.
        for bad_payload in [b"", b"\x07", bytes(8), APP_AGENT_123[:31], APP_AGENT_123 + b"\x00"]:
            with self.subTest(bad_payload=bad_payload):
                with self.assertRaises(Ss58DecodeError) as ctx:
                    ss58_encode(bad_payload, SS58_FORMAT)
                self.assertEqual(ctx.exception.kind, "InvalidLength")


class TestSs58FormatValidation(unittest.TestCase):
    """The ss58_format parameter must be an integer in [0, 16383] (bool excluded),
    mirroring the u16-typed format of the Rust implementation and the runtime check of
    the TypeScript implementation. In particular a float must not surface as TypeError,
    and True must not be silently encoded as format 1."""

    def test_encode_rejects_invalid_formats(self: "TestSs58FormatValidation") -> None:
        for bad_format in [-1, 16384, True, False, 5335.0, "5335", None]:
            with self.subTest(bad_format=bad_format):
                with self.assertRaisesRegex(ValueError, r"^ss58_format must be an integer in \[0, 16383\]"):
                    trevokeyless.encode_app_agent_address(123, bad_format)  # type: ignore[arg-type]
                with self.assertRaisesRegex(ValueError, r"^ss58_format must be an integer in \[0, 16383\]"):
                    trevokeyless.encode_transactional_address(123, 456, bad_format)  # type: ignore[arg-type]
                with self.assertRaisesRegex(ValueError, r"^ss58_format must be an integer in \[0, 16383\]"):
                    trevokeyless.encode_named_address(123, "example123", bad_format)  # type: ignore[arg-type]

    def test_decode_rejects_invalid_formats(self: "TestSs58FormatValidation") -> None:
        address = trevokeyless.encode_app_agent_address(123, SS58_FORMAT)
        for bad_format in [-1, 16384, True, 5335.0, None]:
            with (
                self.subTest(bad_format=bad_format),
                self.assertRaisesRegex(ValueError, r"^ss58_format must be an integer in \[0, 16383\]"),
            ):
                trevokeyless.decode_address(address, bad_format)  # type: ignore[arg-type]

    def test_decode_account_id_takes_no_format(self: "TestSs58FormatValidation") -> None:
        # Raw account ids carry no SS58 format; decode_account_id therefore has no
        # ss58_format parameter at all.
        with self.assertRaises(TypeError):
            trevokeyless.decode_account_id("0x" + APP_AGENT_123.hex(), 5335)  # type: ignore[call-arg]

    def test_format_boundaries_accepted(self: "TestSs58FormatValidation") -> None:
        for good_format in [0, 16383]:
            with self.subTest(good_format=good_format):
                address = trevokeyless.encode_app_agent_address(123, good_format)
                self.assertEqual(trevokeyless.decode_app_agent_address(address, good_format), 123)

    def test_reserved_formats_still_rejected_by_the_ss58_layer(self: "TestSs58FormatValidation") -> None:
        # 46 and 47 pass the integer range validation; the SS58 layer rejects them.
        for reserved in [46, 47]:
            with self.subTest(reserved=reserved):
                with self.assertRaisesRegex(Ss58DecodeError, "^SS58 format is invalid or reserved$") as ctx:
                    trevokeyless.encode_app_agent_address(123, reserved)
                self.assertEqual(ctx.exception.kind, "InvalidFormat")


class TestHexAccountIdContract(unittest.TestCase):
    """decode_account_id accepts a raw account id ("0x" + exactly 64 hex chars);
    anything else is rejected. decode_address accepts SS58 addresses only."""

    def test_valid_hex_account_id_is_classified(self: "TestHexAccountIdContract") -> None:
        info = trevokeyless.decode_account_id("0x" + APP_AGENT_123.hex())
        self.assertIs(info.address_type, AddressType.AppAgent)
        self.assertEqual(info.app_agent_id, 123)
        self.assertEqual(info.account_id, "0x" + APP_AGENT_123.hex())
        # Raw account bytes carry no address; encode_address renders one explicitly.
        self.assertIsNone(info.address)

    def test_uppercase_hex_is_normalized(self: "TestHexAccountIdContract") -> None:
        info = trevokeyless.decode_account_id("0x" + APP_AGENT_123.hex().upper())
        self.assertIs(info.address_type, AddressType.AppAgent)
        self.assertEqual(info.account_id, "0x" + APP_AGENT_123.hex())
        self.assertIsNone(info.address)

    def test_rejects_hex_of_wrong_length(self: "TestHexAccountIdContract") -> None:
        for bad_hex in ["0x", "0xdeadbeef", "0x" + APP_AGENT_123.hex() + "00", "0xabc"]:
            with self.subTest(bad_hex=bad_hex), self.assertRaisesRegex(ValueError, "^Invalid hex account id"):
                trevokeyless.decode_account_id(bad_hex)

    def test_rejects_invalid_hex(self: "TestHexAccountIdContract") -> None:
        # Wrong charset, an uppercase "0X" prefix, and a missing prefix are all rejected.
        for bad_hex in ["0xzz" + "00" * 31, "0X" + APP_AGENT_123.hex(), APP_AGENT_123.hex()]:
            with self.subTest(bad_hex=bad_hex), self.assertRaisesRegex(ValueError, "^Invalid hex account id"):
                trevokeyless.decode_account_id(bad_hex)

    def test_rejects_hex_with_whitespace(self: "TestHexAccountIdContract") -> None:
        # bytes.fromhex tolerates ASCII whitespace, but the contract is exactly 64 hex
        # characters; whitespace anywhere in the hex digits must be rejected (the Rust
        # AccountId32 has no string form at all, the TS implementation uses the same
        # strict regular expression).
        clean = APP_AGENT_123.hex()
        spaced_inputs = [
            "0x" + clean[:8] + " " + clean[8:],  # space inside
            "0x" + clean + " ",  # trailing space
            "0x " + clean,  # space after the prefix
            "0x" + clean[:32] + "\t" + clean[32:],  # tab inside
        ]
        for spaced in spaced_inputs:
            with self.subTest(spaced=spaced), self.assertRaisesRegex(ValueError, "^Invalid hex account id"):
                trevokeyless.decode_account_id(spaced)

    def test_decode_address_rejects_raw_hex(self: "TestHexAccountIdContract") -> None:
        # SS58 addresses and raw account ids are separate input classes: a raw account
        # id carries no SS58 format, so accepting it here would silently bypass the
        # expected-format check.
        for hex_input in ["0x" + APP_AGENT_123.hex(), "0x" + APP_AGENT_123.hex()[:46]]:
            with (
                self.subTest(hex_input=hex_input),
                self.assertRaisesRegex(
                    ValueError, "^Raw account id hex is not an SS58 address; use decode_account_id$"
                ),
            ):
                trevokeyless.decode_address(hex_input, SS58_FORMAT)
        with self.assertRaisesRegex(ValueError, "use decode_account_id"):
            trevokeyless.is_keyless_address("0x" + APP_AGENT_123.hex(), SS58_FORMAT)


class TestEncodeAddress(unittest.TestCase):
    """encode_address renders a raw account id ("0x" + exactly 64 hex chars) as its
    canonical SS58 address, mirroring AccountId32::to_ss58check_with_version."""

    def test_round_trips_with_decode_address(self: "TestEncodeAddress") -> None:
        hex_account = "0x" + APP_AGENT_123.hex()
        address = trevokeyless.encode_address(hex_account, SS58_FORMAT)
        self.assertEqual(address, trevokeyless.encode_app_agent_address(123, SS58_FORMAT))
        self.assertEqual(trevokeyless.decode_address(address, SS58_FORMAT).account_id, hex_account)

    def test_default_format_is_trevo(self: "TestEncodeAddress") -> None:
        hex_account = "0x" + APP_AGENT_123.hex()
        self.assertEqual(trevokeyless.encode_address(hex_account), trevokeyless.encode_app_agent_address(123))

    def test_rejects_invalid_hex(self: "TestEncodeAddress") -> None:
        clean = APP_AGENT_123.hex()
        for bad_hex in [
            "",
            "0x",
            "0x" + clean[:62],
            "0x" + clean + "00",
            "0X" + clean,
            clean,
            "0x" + clean[:62] + "zz",
        ]:
            with self.subTest(bad_hex=bad_hex), self.assertRaisesRegex(ValueError, "^Invalid hex account id"):
                trevokeyless.encode_address(bad_hex)

    def test_rejects_invalid_formats(self: "TestEncodeAddress") -> None:
        hex_account = "0x" + APP_AGENT_123.hex()
        for bad_format in [-1, 16384, True, 5335.0, None]:
            with (
                self.subTest(bad_format=bad_format),
                self.assertRaisesRegex(ValueError, r"^ss58_format must be an integer in \[0, 16383\]"),
            ):
                trevokeyless.encode_address(hex_account, bad_format)  # type: ignore[arg-type]
        for reserved in [46, 47]:
            with self.subTest(reserved=reserved):
                with self.assertRaises(Ss58DecodeError) as ctx:
                    trevokeyless.encode_address(hex_account, reserved)
                self.assertEqual(ctx.exception.kind, "InvalidFormat")


class TestStringParameterTypes(unittest.TestCase):
    """Public entry points raise TypeError - not an AttributeError leaked from a string
    method lookup - when a string parameter has the wrong type. (The TypeScript
    implementation surfaces a native TypeError the same way; the Rust implementation
    enforces the types at compile time.)"""

    def test_decode_address_rejects_non_str(self: "TestStringParameterTypes") -> None:
        for bad in [None, 123, b"ttowKp8Am", ["x"]]:
            with self.subTest(bad=bad), self.assertRaisesRegex(TypeError, "^blockchain_address must be a str"):
                trevokeyless.decode_address(bad)  # type: ignore[arg-type]

    def test_delegating_decoders_reject_non_str(self: "TestStringParameterTypes") -> None:
        delegating = [
            trevokeyless.is_keyless_address,
            trevokeyless.decode_app_agent_address,
            trevokeyless.decode_transactional_address,
            trevokeyless.decode_named_address,
        ]
        for fn in delegating:
            with self.subTest(fn=fn.__name__), self.assertRaisesRegex(TypeError, "^blockchain_address must be a str"):
                fn(None)  # type: ignore[arg-type]

    def test_decode_account_id_rejects_non_str(self: "TestStringParameterTypes") -> None:
        for bad in [None, 123, b"0x" + b"00" * 32]:
            with self.subTest(bad=bad), self.assertRaisesRegex(TypeError, "^account_id must be a str"):
                trevokeyless.decode_account_id(bad)  # type: ignore[arg-type]

    def test_encode_address_rejects_non_str(self: "TestStringParameterTypes") -> None:
        with self.assertRaisesRegex(TypeError, "^account_id must be a str"):
            trevokeyless.encode_address(None)  # type: ignore[arg-type]

    def test_encode_named_address_rejects_non_str_name(self: "TestStringParameterTypes") -> None:
        # bytes previously fell through to the charset check and raised ValueError;
        # a non-str name is a type error, not a charset error.
        for bad in [None, b"hot-wallet", 123]:
            with self.subTest(bad=bad), self.assertRaisesRegex(TypeError, "^name must be a str"):
                trevokeyless.encode_named_address(1, bad)  # type: ignore[arg-type]


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
        # A zeroed account has no valid keyless discriminant; raw account ids are
        # classified via decode_account_id.
        info = trevokeyless.decode_account_id("0x" + "00" * 32)
        self.assertIs(info.address_type, AddressType.Regular)


class TestBlockchainAddressInfoValueSemantics(unittest.TestCase):
    """BlockchainAddressInfo is a frozen dataclass: immutable and hashable, mirroring
    the Rust BlockchainAccountInfo (Copy + Eq + Hash)."""

    def test_is_immutable(self: "TestBlockchainAddressInfoValueSemantics") -> None:
        info = trevokeyless.decode_address(trevokeyless.encode_app_agent_address(123, SS58_FORMAT), SS58_FORMAT)
        with self.assertRaises(dataclasses.FrozenInstanceError):
            info.app_agent_id = 124  # type: ignore[misc]

    def test_is_hashable(self: "TestBlockchainAddressInfoValueSemantics") -> None:
        address = trevokeyless.encode_app_agent_address(123, SS58_FORMAT)
        info_a = trevokeyless.decode_address(address, SS58_FORMAT)
        info_b = trevokeyless.decode_address(address, SS58_FORMAT)
        self.assertEqual(info_a, info_b)
        self.assertEqual(len({info_a, info_b}), 1)


class TestSs58FormatMismatchError(unittest.TestCase):
    """The wrong-format error carries the expected and the found format, mirroring the
    Rust ``Ss58Error::UnexpectedFormat`` variant."""

    def test_carries_expected_and_found(self: "TestSs58FormatMismatchError") -> None:
        substrate_generic = trevokeyless.encode_app_agent_address(123, 42)
        with self.assertRaises(trevokeyless.Ss58FormatMismatchError) as ctx:
            trevokeyless.decode_address(substrate_generic, SS58_FORMAT)
        self.assertEqual(ctx.exception.expected, SS58_FORMAT)
        self.assertEqual(ctx.exception.found, 42)
        # The "UnexpectedFormat" case of Ss58DecodeError; remains a ValueError, like
        # every other decoding failure.
        self.assertIsInstance(ctx.exception, Ss58DecodeError)
        self.assertEqual(ctx.exception.kind, "UnexpectedFormat")
        self.assertIsInstance(ctx.exception, ValueError)
        self.assertEqual(str(ctx.exception), "Expected SS58 format 5335 but found 42")

    def test_raised_for_two_byte_prefix_addresses(self: "TestSs58FormatMismatchError") -> None:
        trevo_address = trevokeyless.encode_app_agent_address(123, SS58_FORMAT)
        with self.assertRaises(trevokeyless.Ss58FormatMismatchError) as ctx:
            trevokeyless.decode_address(trevo_address, 16383)
        self.assertEqual((ctx.exception.expected, ctx.exception.found), (16383, SS58_FORMAT))

    def test_checksum_failure_wins_over_format_mismatch(self: "TestSs58FormatMismatchError") -> None:
        # An invalid address must raise its validation error, never a format mismatch:
        # the expected format is compared only after the checksum has verified, exactly
        # like the Rust from_ss58check (the format mismatch error must not claim that a
        # corrupt string is "a valid address of another network").
        substrate_generic = trevokeyless.encode_app_agent_address(123, 42)
        payload = bytearray(base58.b58decode(substrate_generic))
        payload[-1] ^= 0xFF
        corrupted = base58.b58encode(bytes(payload)).decode()
        with self.assertRaises(Ss58DecodeError) as ctx:
            trevokeyless.decode_address(corrupted, SS58_FORMAT)
        self.assertEqual(ctx.exception.kind, "InvalidChecksum")
        self.assertNotIsInstance(ctx.exception, trevokeyless.Ss58FormatMismatchError)

    def test_length_failure_wins_over_format_mismatch(self: "TestSs58FormatMismatchError") -> None:
        # Same ordering for structurally invalid payloads of a non-expected format.
        truncated_payload = _format_prefix(42) + APP_AGENT_123[:31]
        address = _raw_ss58(truncated_payload)
        with self.assertRaises(Ss58DecodeError) as ctx:
            trevokeyless.decode_address(address, SS58_FORMAT)
        self.assertEqual(ctx.exception.kind, "InvalidLength")
        self.assertNotIsInstance(ctx.exception, trevokeyless.Ss58FormatMismatchError)


class TestAddressTypeMismatchError(unittest.TestCase):
    def test_carries_expected_and_found(self: "TestAddressTypeMismatchError") -> None:
        named_address = trevokeyless.encode_named_address(1, "test123456", SS58_FORMAT)
        with self.assertRaises(AddressTypeMismatchError) as ctx:
            trevokeyless.decode_app_agent_address(named_address, SS58_FORMAT)
        self.assertIs(ctx.exception.expected, AddressType.AppAgent)
        self.assertIs(ctx.exception.found, AddressType.Named)
        # Backwards compatibility: the error is still a ValueError. The message renders
        # the plain type name ("Named"), like the TypeScript implementation.
        self.assertIsInstance(ctx.exception, ValueError)
        self.assertEqual(
            str(ctx.exception),
            "Provided address is not an AppAgent address but is instance of Named",
        )


if __name__ == "__main__":
    unittest.main()
