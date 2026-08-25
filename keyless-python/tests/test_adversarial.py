"""Adversarial and boundary-value tests, mirroring the Rust test suite
(`tests_ss58.rs`, `tests_properties.rs`, `tests_encoding.rs`).

The crafted inputs are built with the reference construction rules so that each test
documents exactly what is being fed to the decoder.
"""

import copy
import dataclasses
import json
import pickle
import string
import time
import unittest
from hashlib import blake2b

import trevokeyless
from trevokeyless import (
    SS58_FORMAT_TREVO_ASSET_HUB,
    AccountNameError,
    AccountType,
    AccountTypeMismatchError,
    AppAgentId,
    NotAnAddressError,
    ParseAccountIdError,
    Ss58Error,
    Ss58FormatMismatchError,
    TransactionalId,
    encode_named_address,
)
from trevokeyless._ss58 import _b58decode, _b58encode, ss58_decode, ss58_encode
from trevokeyless.keyless import _ALLOWED_NAME_CHARS

SS58_FORMAT = 5335
U32_MAX = 2**32 - 1


def _ss58_checksum(payload: bytes) -> bytes:
    """First bytes of blake2b-512(b"SS58PRE" + payload) are the SS58 checksum."""
    return blake2b(b"SS58PRE" + payload, digest_size=64).digest()


def _raw_ss58(payload: bytes, checksum_length: int = 2) -> str:
    """Wraps a raw prefix+data payload in an SS58 envelope with a valid checksum."""
    return _b58encode(payload + _ss58_checksum(payload)[:checksum_length])


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
                address = trevokeyless.encode_app_agent_address(
                    trevokeyless.AppAgentId(app_agent_id), ss58_format=SS58_FORMAT
                )
                decoded = trevokeyless.decode_app_agent_address(address, ss58_format=SS58_FORMAT)
                self.assertEqual(decoded, app_agent_id)

    def test_transactional_id_boundaries(self: "TestBoundaryValues") -> None:
        for app_agent_id, ta_id in [(0, 0), (0, U32_MAX), (U32_MAX, 0), (U32_MAX, U32_MAX)]:
            with self.subTest(app_agent_id=app_agent_id, ta_id=ta_id):
                address = trevokeyless.encode_transactional_address(
                    app_agent_id=trevokeyless.AppAgentId(app_agent_id),
                    ta_id=trevokeyless.TransactionalId(ta_id),
                    ss58_format=SS58_FORMAT,
                )
                decoded = trevokeyless.decode_transactional_address(address, ss58_format=SS58_FORMAT)
                self.assertEqual(decoded, (app_agent_id, ta_id))

    def test_named_boundaries(self: "TestBoundaryValues") -> None:
        for name in ["0000000000", "##########", "----------", "ZZZZZZZZZZ"]:
            for app_agent_id in [0, U32_MAX]:
                with self.subTest(app_agent_id=app_agent_id, name=name):
                    address = trevokeyless.encode_named_address(
                        app_agent_id=trevokeyless.AppAgentId(app_agent_id),
                        account_name=trevokeyless.AccountName(name),
                        ss58_format=SS58_FORMAT,
                    )
                    decoded = trevokeyless.decode_named_address(address, ss58_format=SS58_FORMAT)
                    self.assertEqual(decoded, (app_agent_id, name))


class TestEncodeIdValidation(unittest.TestCase):
    """Mirrors the TS `rejects out-of-range ids at encoding time` test. The Rust
    implementation enforces the range through the u32 parameter types."""

    def test_multi_identifier_encoders_are_keyword_only(self: "TestEncodeIdValidation") -> None:
        # The arguments are valid; passing them positionally is the violation, so the ignore
        # is [call-arg] (too many positional arguments) - the runtime must reject it too.
        with self.assertRaises(TypeError):
            trevokeyless.encode_transactional_address(  # type: ignore[call-arg]
                trevokeyless.AppAgentId(1), trevokeyless.TransactionalId(2)
            )
        with self.assertRaises(TypeError):
            trevokeyless.encode_named_address(  # type: ignore[call-arg]
                trevokeyless.AppAgentId(1), trevokeyless.AccountName("test123456")
            )

    def test_rejects_out_of_range_ids(self: "TestEncodeIdValidation") -> None:
        # An out-of-range id cannot be wrapped in AppAgentId/TransactionalId - the constructor
        # is what rejects it. The raw int is passed on purpose, the type: ignore standing in for
        # an untyped caller, to prove the encoder re-validates the range at runtime.
        for bad_id in [-1, U32_MAX + 1, 2**40]:
            with self.subTest(bad_id=bad_id):
                with self.assertRaisesRegex(ValueError, r"^app_agent_id must be an integer in \[0, 4294967295\]"):
                    trevokeyless.encode_app_agent_address(bad_id, ss58_format=SS58_FORMAT)  # type: ignore[arg-type]
                with self.assertRaisesRegex(ValueError, r"^ta_id must be an integer in \[0, 4294967295\]"):
                    trevokeyless.encode_transactional_address(
                        app_agent_id=trevokeyless.AppAgentId(1),
                        ta_id=bad_id,  # type: ignore[arg-type]
                        ss58_format=SS58_FORMAT,
                    )
                with self.assertRaisesRegex(ValueError, r"^app_agent_id must be an integer in \[0, 4294967295\]"):
                    trevokeyless.encode_named_address(
                        app_agent_id=bad_id,  # type: ignore[arg-type]
                        account_name=trevokeyless.AccountName("example123"),
                        ss58_format=SS58_FORMAT,
                    )

    def test_rejects_non_integer_ids(self: "TestEncodeIdValidation") -> None:
        # Wrong types raise TypeError (Python convention, matching the TypeError for
        # non-string address/name parameters); out-of-range ints raise ValueError.
        for bad_id in [1.5, "123", None]:
            with self.subTest(bad_id=bad_id), self.assertRaisesRegex(TypeError, "^app_agent_id must be an int"):
                trevokeyless.encode_app_agent_address(bad_id, ss58_format=SS58_FORMAT)  # type: ignore[arg-type]

    def test_rejects_bool_ids(self: "TestEncodeIdValidation") -> None:
        # bool is an int subclass, but silently encoding True as 1 would hide a caller
        # bug that the TypeScript (Number.isInteger) and Rust (u32) implementations
        # reject; it is classified as a wrong type, not an out-of-range value.
        # A bool cannot be wrapped in AppAgentId/TransactionalId either; the raw bool is passed
        # on purpose (the type: ignore simulates an untyped caller) so the encoder's own
        # runtime rejection is what gets tested.
        for bad_id in [True, False]:
            with self.subTest(bad_id=bad_id):
                with self.assertRaisesRegex(TypeError, "^app_agent_id must be an int"):
                    trevokeyless.encode_app_agent_address(bad_id, ss58_format=SS58_FORMAT)  # type: ignore[arg-type]
                with self.assertRaisesRegex(TypeError, "^ta_id must be an int"):
                    trevokeyless.encode_transactional_address(
                        app_agent_id=trevokeyless.AppAgentId(1),
                        ta_id=bad_id,  # type: ignore[arg-type]
                        ss58_format=SS58_FORMAT,
                    )

    def test_accepts_u32_boundaries(self: "TestEncodeIdValidation") -> None:
        for good_id in [0, U32_MAX]:
            with self.subTest(good_id=good_id):
                trevokeyless.encode_app_agent_address(trevokeyless.AppAgentId(good_id), ss58_format=SS58_FORMAT)
                trevokeyless.encode_transactional_address(
                    app_agent_id=trevokeyless.AppAgentId(good_id),
                    ta_id=trevokeyless.TransactionalId(good_id),
                    ss58_format=SS58_FORMAT,
                )
                trevokeyless.encode_named_address(
                    app_agent_id=trevokeyless.AppAgentId(good_id),
                    account_name=trevokeyless.AccountName("example123"),
                    ss58_format=SS58_FORMAT,
                )

    def test_named_validates_id_before_name(self: "TestEncodeIdValidation") -> None:
        # When both the id and the name are invalid, the id is reported first (argument
        # order), matching the TypeScript implementation. "bad@name!!" is 10 characters
        # but carries out-of-charset bytes, so the order decides which error wins.
        # Neither argument can be constructed as its nominal type - both are invalid, which is
        # the point - so both are passed raw, the type: ignore simulating an untyped caller.
        with self.assertRaisesRegex(ValueError, r"^app_agent_id must be an integer in \[0, 4294967295\]"):
            trevokeyless.encode_named_address(
                app_agent_id=-1,  # type: ignore[arg-type]
                account_name="bad@name!!",  # type: ignore[arg-type]
                ss58_format=SS58_FORMAT,
            )

    # The raw-account-id encoders share the identifier and name validation of the address
    # encoders; the cases below mirror the address ones one for one, because an encoder
    # that skipped a check would silently mint an account id no other implementation can
    # produce.

    def test_account_id_encoders_reject_out_of_range_ids(self: "TestEncodeIdValidation") -> None:
        # Out-of-range ids again: unconstructable as the nominal types, so they are passed raw
        # (the ignores below stand in for an untyped caller) to test the encoder's validation.
        for bad_id in [-1, U32_MAX + 1, 2**40]:
            with self.subTest(bad_id=bad_id):
                with self.assertRaisesRegex(ValueError, r"^app_agent_id must be an integer in \[0, 4294967295\]"):
                    trevokeyless.encode_app_agent_account_id(bad_id)  # type: ignore[arg-type]
                with self.assertRaisesRegex(ValueError, r"^ta_id must be an integer in \[0, 4294967295\]"):
                    trevokeyless.encode_transactional_account_id(
                        app_agent_id=trevokeyless.AppAgentId(1),
                        ta_id=bad_id,  # type: ignore[arg-type]
                    )
                with self.assertRaisesRegex(ValueError, r"^app_agent_id must be an integer in \[0, 4294967295\]"):
                    trevokeyless.encode_named_account_id(
                        app_agent_id=bad_id,  # type: ignore[arg-type]
                        account_name=trevokeyless.AccountName("example123"),
                    )

    def test_account_id_encoders_reject_non_integer_ids(self: "TestEncodeIdValidation") -> None:
        for bad_id in [1.5, "123", None]:
            with self.subTest(bad_id=bad_id):
                with self.assertRaisesRegex(TypeError, "^app_agent_id must be an int"):
                    trevokeyless.encode_app_agent_account_id(bad_id)  # type: ignore[arg-type]
                with self.assertRaisesRegex(TypeError, "^ta_id must be an int"):
                    trevokeyless.encode_transactional_account_id(
                        app_agent_id=trevokeyless.AppAgentId(1),
                        ta_id=bad_id,  # type: ignore[arg-type]
                    )

    def test_account_id_encoders_reject_bool_ids(self: "TestEncodeIdValidation") -> None:
        # bools, likewise unconstructable as AppAgentId/TransactionalId: passed raw on purpose
        # (type: ignore = untyped caller) so the encoder's runtime bool rejection is tested.
        for bad_id in [True, False]:
            with self.subTest(bad_id=bad_id):
                with self.assertRaisesRegex(TypeError, "^app_agent_id must be an int"):
                    trevokeyless.encode_app_agent_account_id(bad_id)  # type: ignore[arg-type]
                with self.assertRaisesRegex(TypeError, "^ta_id must be an int"):
                    trevokeyless.encode_transactional_account_id(
                        app_agent_id=trevokeyless.AppAgentId(1),
                        ta_id=bad_id,  # type: ignore[arg-type]
                    )
                with self.assertRaisesRegex(TypeError, "^app_agent_id must be an int"):
                    trevokeyless.encode_named_account_id(
                        app_agent_id=bad_id,  # type: ignore[arg-type]
                        account_name=trevokeyless.AccountName("example123"),
                    )

    def test_account_id_encoders_reject_invalid_names(self: "TestEncodeIdValidation") -> None:
        # Same taxonomy as the address encoder: the UTF-8 byte length decides
        # InvalidLength, everything else of the right length is InvalidCharacter.
        # None of these names can be wrapped in AccountName() - the constructor is what rejects
        # them - so the raw str goes to the encoder, the type: ignore simulating an untyped
        # caller, proving the encoder re-validates the name itself.
        for name, kind in [
            ("short", "InvalidLength"),
            ("héllo12345", "InvalidLength"),
            ("invalid@12", "InvalidCharacter"),
            ("café12345", "InvalidCharacter"),
        ]:
            with self.subTest(name=name):
                with self.assertRaises(trevokeyless.AccountNameError) as ctx:
                    trevokeyless.encode_named_account_id(
                        app_agent_id=trevokeyless.AppAgentId(1),
                        account_name=name,  # type: ignore[arg-type]
                    )
                self.assertEqual(ctx.exception.kind, kind)
        with self.assertRaisesRegex(TypeError, "^name must be a str"):
            trevokeyless.encode_named_account_id(
                app_agent_id=trevokeyless.AppAgentId(1),
                account_name=None,  # type: ignore[arg-type]
            )

    def test_account_id_encoders_are_keyword_only(self: "TestEncodeIdValidation") -> None:
        # As above: valid arguments, passed positionally on purpose ([call-arg]).
        with self.assertRaises(TypeError):
            trevokeyless.encode_transactional_account_id(  # type: ignore[call-arg]
                trevokeyless.AppAgentId(1), trevokeyless.TransactionalId(2)
            )
        with self.assertRaises(TypeError):
            trevokeyless.encode_named_account_id(  # type: ignore[call-arg]
                trevokeyless.AppAgentId(1), trevokeyless.AccountName("test123456")
            )

    def test_account_id_encoders_accept_u32_boundaries(self: "TestEncodeIdValidation") -> None:
        for good_id in [0, U32_MAX]:
            with self.subTest(good_id=good_id):
                trevokeyless.encode_app_agent_account_id(trevokeyless.AppAgentId(good_id))
                trevokeyless.encode_transactional_account_id(
                    app_agent_id=trevokeyless.AppAgentId(good_id),
                    ta_id=trevokeyless.TransactionalId(good_id),
                )
                trevokeyless.encode_named_account_id(
                    app_agent_id=trevokeyless.AppAgentId(good_id),
                    account_name=trevokeyless.AccountName("example123"),
                )


class TestIdentifierTypes(unittest.TestCase):
    """AppAgentId / TransactionalId are validating int subclasses, so an out-of-range
    identifier fails where it is constructed - like the TypeScript appAgentId() /
    transactionalId() factories - instead of only at the encoder."""

    def test_constructors_reject_out_of_range(self: "TestIdentifierTypes") -> None:
        for bad_id in [-1, U32_MAX + 1, 2**40]:
            with self.subTest(bad_id=bad_id):
                with self.assertRaisesRegex(ValueError, r"^app_agent_id must be an integer in \[0, 4294967295\]"):
                    trevokeyless.AppAgentId(bad_id)
                with self.assertRaisesRegex(ValueError, r"^ta_id must be an integer in \[0, 4294967295\]"):
                    trevokeyless.TransactionalId(bad_id)

    def test_constructors_reject_wrong_types(self: "TestIdentifierTypes") -> None:
        for bad_id in [1.5, "123", None, True, False]:
            with self.subTest(bad_id=bad_id):
                with self.assertRaisesRegex(TypeError, "^app_agent_id must be an int"):
                    trevokeyless.AppAgentId(bad_id)  # type: ignore[arg-type]
                with self.assertRaisesRegex(TypeError, "^ta_id must be an int"):
                    trevokeyless.TransactionalId(bad_id)  # type: ignore[arg-type]

    def test_identifiers_behave_like_ints(self: "TestIdentifierTypes") -> None:
        app_agent_id = trevokeyless.AppAgentId(123)
        self.assertEqual(app_agent_id, 123)
        self.assertEqual(app_agent_id + 1, 124)
        self.assertEqual(app_agent_id.to_bytes(4, byteorder="little"), (123).to_bytes(4, byteorder="little"))
        self.assertEqual(hash(app_agent_id), hash(123))
        self.assertIsInstance(app_agent_id, int)
        # Equal identifiers of different kinds compare equal as ints; the distinction is
        # nominal (enforced by the type checker), not structural.
        self.assertEqual(trevokeyless.TransactionalId(123), app_agent_id)

    def test_decoders_return_validating_identifiers(self: "TestIdentifierTypes") -> None:
        info = trevokeyless.decode_account_id("0x" + APP_AGENT_123.hex())
        self.assertIsInstance(info, trevokeyless.AppAgentAccountInfo)
        self.assertIsInstance(getattr(info, "app_agent_id", None), trevokeyless.AppAgentId)


class TestDecodeAccountIdTypedDecoders(unittest.TestCase):
    """The typed account-id decoders (the account-id counterparts of the typed address
    decoders, mirroring the Rust decode_*_account functions)."""

    def test_round_trips(self: "TestDecodeAccountIdTypedDecoders") -> None:
        app_agent_account = trevokeyless.encode_app_agent_account_id(trevokeyless.AppAgentId(123))
        self.assertEqual(trevokeyless.decode_app_agent_account_id(app_agent_account), 123)

        transactional_account = trevokeyless.encode_transactional_account_id(
            app_agent_id=trevokeyless.AppAgentId(123), ta_id=trevokeyless.TransactionalId(456)
        )
        self.assertEqual(trevokeyless.decode_transactional_account_id(transactional_account), (123, 456))

        named_account = trevokeyless.encode_named_account_id(
            app_agent_id=trevokeyless.AppAgentId(123), account_name=trevokeyless.AccountName("hot-wallet")
        )
        self.assertEqual(trevokeyless.decode_named_account_id(named_account), (123, "hot-wallet"))

    def test_type_mismatch_carries_expected_and_found(self: "TestDecodeAccountIdTypedDecoders") -> None:
        app_agent_account = trevokeyless.encode_app_agent_account_id(trevokeyless.AppAgentId(123))
        for decode, expected in (
            (trevokeyless.decode_transactional_account_id, AccountType.Transactional),
            (trevokeyless.decode_named_account_id, AccountType.Named),
        ):
            with self.subTest(expected=expected):
                with self.assertRaises(AccountTypeMismatchError) as ctx:
                    decode(app_agent_account)
                self.assertIs(ctx.exception.expected, expected)
                self.assertIs(ctx.exception.found, AccountType.AppAgent)

        # A regular (key-derived) account: Alice.
        alice = "0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d"
        with self.assertRaises(AccountTypeMismatchError) as ctx_regular:
            trevokeyless.decode_app_agent_account_id(alice)
        self.assertIs(ctx_regular.exception.expected, AccountType.AppAgent)
        self.assertIs(ctx_regular.exception.found, AccountType.Regular)

    def test_reject_malformed_hex(self: "TestDecodeAccountIdTypedDecoders") -> None:
        decoders = [
            trevokeyless.decode_app_agent_account_id,
            trevokeyless.decode_transactional_account_id,
            trevokeyless.decode_named_account_id,
        ]
        bad_inputs = ["", "0x", "0xdeadbeef", "0x" + "zz" * 32, "0x" + "00" * 33, APP_AGENT_123.hex()]
        for decode in decoders:
            for bad in bad_inputs:
                with (
                    self.subTest(decoder=decode.__name__, input=bad),
                    self.assertRaises(trevokeyless.ParseAccountIdError),
                ):
                    decode(bad)

    def test_reject_non_str(self: "TestDecodeAccountIdTypedDecoders") -> None:
        decoders = [
            trevokeyless.decode_app_agent_account_id,
            trevokeyless.decode_transactional_account_id,
            trevokeyless.decode_named_account_id,
        ]
        for decode in decoders:
            for bad in [None, 123, b"0x" + APP_AGENT_123.hex().encode()]:
                with (
                    self.subTest(decoder=decode.__name__, input=bad),
                    self.assertRaisesRegex(TypeError, "^account_id must be a str"),
                ):
                    decode(bad)  # type: ignore[arg-type]

    def test_reject_ss58_addresses(self: "TestDecodeAccountIdTypedDecoders") -> None:
        # The account-id decoders take the raw form only; an SS58 address is not a
        # 0x-prefixed 64-hex string, so it fails the account-id contract.
        address = trevokeyless.encode_app_agent_address(trevokeyless.AppAgentId(123), ss58_format=SS58_FORMAT)
        with self.assertRaises(trevokeyless.ParseAccountIdError):
            trevokeyless.decode_app_agent_account_id(address)


class TestDecodeValidationOrder(unittest.TestCase):
    """The decode entry points validate the ss58_format parameter before the address."""

    def test_format_is_validated_before_address_type(self: "TestDecodeValidationOrder") -> None:
        # When both the address (wrong type) and the ss58_format (out of range) are
        # invalid, the format error wins - matching the Rust from_ss58check_expecting and
        # the TypeScript decodeSs58AccountId, which both check the format first. The address
        # is deliberately not a str (the ignore stands in for an untyped caller).
        with self.assertRaisesRegex(ValueError, r"^ss58_format must be an integer in \[0, 16383\]"):
            trevokeyless.decode_address(123, ss58_format=99999)  # type: ignore[arg-type]

    def test_format_is_validated_before_raw_hex_rejection(self: "TestDecodeValidationOrder") -> None:
        # A raw 0x account id with an out-of-range format reports the format error first.
        with self.assertRaisesRegex(ValueError, r"^ss58_format must be an integer in \[0, 16383\]"):
            trevokeyless.decode_address("0x" + "00" * 32, ss58_format=99999)


class TestAccountIdLength(unittest.TestCase):
    """Account ids must be exactly 32 bytes; the Rust AccountId32 makes this structural."""

    def test_rejects_short_ss58_payloads(self: "TestAccountIdLength") -> None:
        # SS58 envelopes (correct checksum, format 5335) around 1-, 4- and 8-byte
        # payloads with the one-byte checksum of the generic "account index" scheme:
        # never a 32-byte account id. Built manually since ss58_encode refuses them.
        for payload in [b"\x07", b"\xde\xad\xbe\xef", bytes(8)]:
            with self.subTest(payload=payload):
                address = _raw_ss58(_format_prefix(SS58_FORMAT) + payload, checksum_length=1)
                with self.assertRaises(Ss58Error) as ctx:
                    trevokeyless.decode_address(address, ss58_format=SS58_FORMAT)
                self.assertEqual(ctx.exception.kind, "InvalidLength")

    def test_rejects_one_byte_checksum_on_full_account(self: "TestAccountIdLength") -> None:
        # A full 32-byte account under the Trevo prefix but with only a one-byte checksum
        # (decoded length 35, not the canonical 36). Permissive decoders that derive the
        # checksum length from the payload would accept this; it must be InvalidLength.
        # Mirrors the TypeScript one-byte-checksum test and the first ss58_invalid vector.
        address = _raw_ss58(_format_prefix(SS58_FORMAT) + APP_AGENT_123, checksum_length=1)
        with self.assertRaises(Ss58Error) as ctx:
            trevokeyless.decode_address(address, ss58_format=SS58_FORMAT)
        self.assertEqual(ctx.exception.kind, "InvalidLength")

    def test_rejects_33_byte_payload(self: "TestAccountIdLength") -> None:
        # A 33-byte payload (ECDSA-style public key) is legal generic SS58 but never a
        # 32-byte account id; its address is 51 chars, so the length pre-bound fires.
        address = _raw_ss58(_format_prefix(SS58_FORMAT) + APP_AGENT_123 + b"\x00")
        with self.assertRaises(Ss58Error) as ctx:
            trevokeyless.decode_address(address, ss58_format=SS58_FORMAT)
        self.assertEqual(ctx.exception.kind, "InvalidLength")

    def test_rejects_31_byte_payload(self: "TestAccountIdLength") -> None:
        # Not a legal SS58 length at all.
        address = _raw_ss58(_format_prefix(SS58_FORMAT) + APP_AGENT_123[:31])
        with self.assertRaises(Ss58Error) as ctx:
            trevokeyless.decode_address(address, ss58_format=SS58_FORMAT)
        self.assertEqual(ctx.exception.kind, "InvalidLength")


class TestMalformedSs58(unittest.TestCase):
    """Mirrors the Rust `tests_ss58.rs` rejection tests."""

    def test_rejects_tampered_ss58_checksum(self: "TestMalformedSs58") -> None:
        address = trevokeyless.encode_app_agent_address(trevokeyless.AppAgentId(123), ss58_format=SS58_FORMAT)
        payload = bytearray(_b58decode(address))
        payload[-1] ^= 0xFF
        tampered = _b58encode(bytes(payload))
        with self.assertRaisesRegex(Ss58Error, "^Address has an invalid checksum$") as ctx:
            trevokeyless.decode_address(tampered, ss58_format=SS58_FORMAT)
        self.assertEqual(ctx.exception.kind, "InvalidChecksum")

    def test_tampered_keyless_checksum_decodes_as_regular(self: "TestMalformedSs58") -> None:
        # Corrupt a byte in the keyless checksum region (open part of an AppAgent account
        # is bytes [0..5]) and rewrap in a valid SS58 envelope: the SS58 layer accepts it,
        # the keyless layer must demote it to Regular.
        account = bytearray(APP_AGENT_123)
        account[10] ^= 0xFF
        address = ss58_encode(bytes(account), SS58_FORMAT)
        info = trevokeyless.decode_address(address, ss58_format=SS58_FORMAT)
        self.assertIsInstance(info, trevokeyless.RegularAccountInfo)
        self.assertFalse(hasattr(info, "app_agent_id"))

    def test_rejects_reserved_format(self: "TestMalformedSs58") -> None:
        # Reserved format 46, crafted with a valid checksum (the encoder refuses to
        # produce it).
        reserved_format = 46
        address = _raw_ss58(_format_prefix(reserved_format) + APP_AGENT_123)
        with self.assertRaisesRegex(Ss58Error, "^SS58 format is invalid or reserved$") as ctx:
            trevokeyless.decode_address(address, ss58_format=reserved_format)
        self.assertEqual(ctx.exception.kind, "InvalidFormat")
        with self.assertRaises(Ss58Error) as encode_ctx:
            ss58_encode(APP_AGENT_123, reserved_format)
        self.assertEqual(encode_ctx.exception.kind, "InvalidFormat")

    def test_rejects_prefix_byte_in_reserved_range(self: "TestMalformedSs58") -> None:
        # Per the SS58 spec the first byte must be 0..=63 or 64..=127. A crafted address
        # with first byte 0x80 and a valid checksum must be rejected, exactly like the
        # Rust and TypeScript decoders.
        address = _raw_ss58(bytes([0x80]) + APP_AGENT_123)
        with self.assertRaisesRegex(Ss58Error, "^SS58 format is invalid or reserved$") as ctx:
            trevokeyless.decode_address(address, ss58_format=128)
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
        info = trevokeyless.decode_address(address, ss58_format=small_format)
        # `assert isinstance` rather than assertIsInstance: it asserts the same thing and also
        # narrows the BlockchainAccountInfo union for the type checker.
        assert isinstance(info, trevokeyless.AppAgentAccountInfo)  # noqa: S101 - isinstance narrowing for mypy
        self.assertEqual(info.app_agent_id, 123)

    def test_rejects_garbage_input(self: "TestMalformedSs58") -> None:
        # All of these must raise ValueError (never IndexError or any other type).
        for garbage in ["", " ", "   ", "0OIl", "x", "1"]:
            with self.subTest(garbage=garbage), self.assertRaises(ValueError):
                trevokeyless.decode_address(garbage, ss58_format=SS58_FORMAT)

    def test_rejects_oversized_input_before_base58_decoding(self: "TestMalformedSs58") -> None:
        oversized = "1" * (trevokeyless.SS58_MAX_ADDRESS_LENGTH + 1)
        with self.assertRaisesRegex(Ss58Error, "^Address has an invalid length$") as ctx:
            trevokeyless.decode_address(oversized, ss58_format=SS58_FORMAT)
        self.assertEqual(ctx.exception.kind, "InvalidLength")

        # The codec applies the same bound before charset validation: oversized input
        # full of out-of-alphabet characters is still classified as a length failure,
        # exactly like the Rust decoder.
        with self.assertRaises(Ss58Error) as codec_ctx:
            ss58_decode("$" * (trevokeyless.SS58_MAX_ADDRESS_LENGTH + 1))
        self.assertEqual(codec_ctx.exception.kind, "InvalidLength")

    def test_max_address_length_bound_is_50(self: "TestMalformedSs58") -> None:
        # The oversized-input tests use SS58_MAX_ADDRESS_LENGTH as the boundary; pin its
        # exported value so it cannot silently drift from the Rust constant (50), which
        # would make those tests exercise a different boundary than the reference decoder.
        self.assertEqual(trevokeyless.SS58_MAX_ADDRESS_LENGTH, 50)

    def test_length_bound_is_utf8_bytes(self: "TestMalformedSs58") -> None:
        # The 50-character bound is defined in UTF-8 bytes, like the Rust reference
        # decoder: input can stay within 50 code points (or 50 UTF-16 units) and still
        # exceed the bound. Also pinned cross-language by the ss58_invalid vectors.
        for over_bound in ["é" * 26, "𝄞" * 26]:  # 52 and 104 UTF-8 bytes
            with self.subTest(over_bound=over_bound):
                with self.assertRaises(Ss58Error) as ctx:
                    ss58_decode(over_bound)
                self.assertEqual(ctx.exception.kind, "InvalidLength")
        # A single non-ASCII character is within the bound and fails the charset check.
        with self.assertRaises(Ss58Error) as charset_ctx:
            ss58_decode("é")
        self.assertEqual(charset_ctx.exception.kind, "InvalidBase58")

    def test_longest_legitimate_address_still_decodes(self: "TestMalformedSs58") -> None:
        # The longest legitimate addresses (two-byte prefix, high account id bytes) stay
        # within the bound and must still decode.
        max_format = 16383
        address = ss58_encode(b"\xff" * 32, max_format)
        self.assertLessEqual(len(address), trevokeyless.SS58_MAX_ADDRESS_LENGTH)
        info = trevokeyless.decode_address(address, ss58_format=max_format)
        self.assertIs(info.account_type, AccountType.Regular)


class TestSurroundingWhitespace(unittest.TestCase):
    """An address with surrounding whitespace is not the canonical spelling of the
    account: whitespace is outside the base58 alphabet and must be rejected, as in the
    Rust and TypeScript implementations (pinned by the ss58_invalid conformance
    vectors). The historically used base58 package right-stripped its input; the
    vendored decoder rejects out-of-alphabet characters outright, and the charset is
    still checked explicitly before decoding."""

    def test_decode_rejects_surrounding_whitespace(self: "TestSurroundingWhitespace") -> None:
        address = trevokeyless.encode_app_agent_address(trevokeyless.AppAgentId(123), ss58_format=SS58_FORMAT)
        spellings = [
            address + " ",
            " " + address,
            address + "\n",
            address + "\t",
        ]
        for spelling in spellings:
            with self.subTest(spelling=spelling):
                with self.assertRaisesRegex(Ss58Error, "^Address is not valid base58$") as ctx:
                    trevokeyless.decode_address(spelling, ss58_format=SS58_FORMAT)
                self.assertEqual(ctx.exception.kind, "InvalidBase58")
                # The codec itself applies the same charset check before base58.b58decode.
                with self.assertRaises(Ss58Error) as codec_ctx:
                    ss58_decode(spelling)
                self.assertEqual(codec_ctx.exception.kind, "InvalidBase58")

    def test_decode_rejects_appended_non_breaking_space(self: "TestSurroundingWhitespace") -> None:
        # A non-breaking space (also stripped by str.rstrip) is a *two-byte* UTF-8
        # character: appended to a 49-character Trevo address it brings the input to 50
        # code points but 51 UTF-8 bytes, so the byte-length bound fires first - exactly
        # like the Rust decoder, which measures bytes. (The previous expectation of
        # InvalidBase58 pinned a Python-only divergence from the reference.)
        address = trevokeyless.encode_app_agent_address(trevokeyless.AppAgentId(123), ss58_format=SS58_FORMAT)
        with self.assertRaises(Ss58Error) as ctx:
            trevokeyless.decode_address(address + "\u00a0", ss58_format=SS58_FORMAT)
        self.assertEqual(ctx.exception.kind, "InvalidLength")


class TestUnpairedSurrogates(unittest.TestCase):
    """Strings containing unpaired surrogates - a Python str can hold them, most
    realistically from ``json.loads`` of a ``"\\ud800"`` escape - cannot be encoded by
    the default UTF-8 codec. The byte-length bound applies JavaScript TextEncoder's
    replacement/combination rules so hostile host strings classify identically in Python
    and TypeScript instead of leaking ``UnicodeEncodeError``. Rust cannot represent lone
    surrogates (``&str`` is always valid UTF-8), so these cases are host-language tests
    rather than conformance JSON vectors."""

    def test_in_bound_surrogate_is_invalid_base58(self: "TestUnpairedSurrogates") -> None:
        # One lone surrogate: 1 code point, 3 surrogatepass bytes - within the length
        # bound, rejected by the base58 charset check, exactly like the TypeScript
        # implementation classifies it.
        with self.assertRaises(Ss58Error) as ctx:
            trevokeyless.decode_address("\ud800", ss58_format=SS58_FORMAT)
        self.assertEqual(ctx.exception.kind, "InvalidBase58")

        with self.assertRaises(Ss58Error) as codec_ctx:
            ss58_decode("\ud800")
        self.assertEqual(codec_ctx.exception.kind, "InvalidBase58")

    def test_surrogates_over_the_byte_bound_are_invalid_length(self: "TestUnpairedSurrogates") -> None:
        # 50 lone surrogates: 50 code points but 150 surrogatepass bytes - over the
        # 50-byte bound, exactly like the TypeScript implementation classifies it.
        with self.assertRaises(Ss58Error) as ctx:
            trevokeyless.decode_address("\ud800" * 50, ss58_format=SS58_FORMAT)
        self.assertEqual(ctx.exception.kind, "InvalidLength")

        # A canonical 49-character address with one appended surrogate: 50 code points
        # but 52 bytes - the byte bound fires first, like for any non-ASCII suffix.
        address = trevokeyless.encode_app_agent_address(trevokeyless.AppAgentId(123), ss58_format=SS58_FORMAT)
        with self.assertRaises(Ss58Error) as suffixed_ctx:
            trevokeyless.decode_address(address + "\ud800", ss58_format=SS58_FORMAT)
        self.assertEqual(suffixed_ctx.exception.kind, "InvalidLength")

    def test_json_sourced_surrogate_classifies_via_the_taxonomy(self: "TestUnpairedSurrogates") -> None:
        # json.loads turns a "\ud800" escape into a real lone surrogate; decoding such
        # a string must classify through the SS58 taxonomy, never UnicodeEncodeError.
        surrogate = json.loads('"\\ud800"')
        self.assertEqual(len(surrogate), 1)
        with self.assertRaises(Ss58Error) as ctx:
            trevokeyless.decode_address(surrogate, ss58_format=SS58_FORMAT)
        self.assertEqual(ctx.exception.kind, "InvalidBase58")

    def test_programmatic_surrogate_pairs_match_text_encoder(self: "TestUnpairedSurrogates") -> None:
        # Nine explicit UTF-16 pairs are 18 Python code points but nine four-byte Unicode
        # scalars to TextEncoder (36 bytes). They are within the byte bound and fail the
        # base58 alphabet check, matching TypeScript. Standard JSON combines each pair
        # before it reaches this function.
        paired_surrogates = "\ud83d\ude00" * 9
        self.assertEqual(len(paired_surrogates), 18)
        with self.assertRaises(Ss58Error) as ctx:
            trevokeyless.decode_address(paired_surrogates, ss58_format=SS58_FORMAT)
        self.assertEqual(ctx.exception.kind, "InvalidBase58")

    def test_lone_surrogate_names_classify_via_the_name_taxonomy(self: "TestUnpairedSurrogates") -> None:
        # Account names measure the same TextEncoder UTF-8 bytes as addresses (the
        # surrogate contract covers "addresses and account names alike", per the
        # conformance README), so a hostile host string classifies through the
        # AccountNameError taxonomy \u2014 never UnicodeEncodeError \u2014 identically to the
        # TypeScript twin. Nine ASCII + one lone surrogate: 10 code points but
        # 9 + 3 = 12 bytes, so the byte rule reports the length.
        with self.assertRaises(trevokeyless.AccountNameError) as ctx:
            trevokeyless.AccountName("abcdefghi\ud800")
        self.assertEqual(ctx.exception.kind, "InvalidLength")

        # One lone surrogate + seven ASCII: 3 + 7 = exactly 10 bytes, so the length
        # gate passes and the charset check reports the surrogate itself.
        with self.assertRaises(trevokeyless.AccountNameError) as charset_ctx:
            trevokeyless.AccountName("\ud800abcdefg")
        self.assertEqual(charset_ctx.exception.kind, "InvalidCharacter")

        # Ten lone surrogates: exactly 10 code points but 30 bytes. An implementation
        # measuring code points would reach the charset check; the byte rule must
        # report the length.
        with self.assertRaises(trevokeyless.AccountNameError) as bound_ctx:
            trevokeyless.AccountName("\udc00" * 10)
        self.assertEqual(bound_ctx.exception.kind, "InvalidLength")

    def test_programmatic_surrogate_pair_names_match_text_encoder(self: "TestUnpairedSurrogates") -> None:
        # An explicit UTF-16 pair combines into one four-byte scalar (TextEncoder), so
        # pair + six ASCII is 8 code points but exactly 10 bytes: the length gate
        # passes and the charset check fires \u2014 not InvalidLength, which a
        # replacement-per-unit measurement (3 + 3 + 6 = 12 bytes) would report.
        # Standard JSON combines the pair before it reaches this constructor.
        paired_name = "\ud83d\ude00aaaaaa"
        self.assertEqual(len(paired_name), 8)
        with self.assertRaises(trevokeyless.AccountNameError) as ctx:
            trevokeyless.AccountName(paired_name)
        self.assertEqual(ctx.exception.kind, "InvalidCharacter")


class TestVendoredBase58(unittest.TestCase):
    """The vendored strict base58 codec (Bitcoin alphabet, no whitespace tolerance,
    leading "1" characters encode leading zero bytes). The SS58 conformance vectors
    exercise it end to end - including format 0, whose single 0x00 prefix byte covers
    the leading-zero path; these tests pin the primitive directly."""

    def test_round_trips(self: "TestVendoredBase58") -> None:
        payloads = [b"", b"\x00", b"\x00\x00\x00", b"\x00abc", b"\xff" * 36, bytes(range(36))]
        for data in payloads:
            with self.subTest(data=data):
                self.assertEqual(_b58decode(_b58encode(data)), data)

    def test_known_vectors(self: "TestVendoredBase58") -> None:
        # Reference vectors of the Bitcoin base58 alphabet.
        self.assertEqual(_b58encode(b"Hello World!"), "2NEpo7TZRRrLZSi2U")
        self.assertEqual(_b58decode("2NEpo7TZRRrLZSi2U"), b"Hello World!")
        self.assertEqual(_b58encode(b"\x00\x00abc"), "11ZiCa")
        self.assertEqual(_b58decode("11ZiCa"), b"\x00\x00abc")
        self.assertEqual(_b58encode(bytes(36)), "1" * 36)

    def test_decode_rejects_out_of_alphabet(self: "TestVendoredBase58") -> None:
        # '0', 'O', 'I', and 'l' are excluded from the alphabet; whitespace anywhere
        # (the leniency of the historically used base58 package) must also be rejected.
        for bad in ["0", "O", "I", "l", "1 1", "abc\n", " abc", "abc "]:
            with self.subTest(bad=bad):
                with self.assertRaises(Ss58Error) as ctx:
                    _b58decode(bad)
                self.assertEqual(ctx.exception.kind, "InvalidBase58")


class TestStrictCodecContract(unittest.TestCase):
    """The private codec (trevokeyless._ss58) is a strict port of the Rust reference
    decoder: bytes in, bytes out, 32-byte account ids only, no hex passthrough. The
    historically vendored generic codec accepted "0x" strings and other payload sizes;
    these tests pin their removal."""

    def test_ss58_decode_has_no_hex_passthrough(self: "TestStrictCodecContract") -> None:
        # "0x..." is not an SS58 address. A full 66-character account id hex string
        # exceeds the 50-character bound (checked first, like in the Rust decoder);
        # shorter "0x" strings fail the base58 charset ('0' and 'x' are excluded).
        with self.assertRaises(Ss58Error) as ctx:
            ss58_decode("0x" + APP_AGENT_123.hex())
        self.assertEqual(ctx.exception.kind, "InvalidLength")

        with self.assertRaises(Ss58Error) as short_ctx:
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
                with self.assertRaises(Ss58Error) as ctx:
                    ss58_encode(bad_payload, SS58_FORMAT)
                self.assertEqual(ctx.exception.kind, "InvalidLength")


class TestSs58FormatValidation(unittest.TestCase):
    """The ss58_format parameter must be an integer in [0, 16383] (bool excluded),
    mirroring the u16-typed format of the Rust implementation and the runtime check of
    the TypeScript implementation. A wrong type (float, str, None, bool) raises a
    deliberate TypeError - not one leaked from the prefix byte arithmetic - and True
    must not be silently encoded as format 1; an out-of-range int raises ValueError."""

    def test_encode_rejects_wrong_type_formats(self: "TestSs58FormatValidation") -> None:
        # 5335.5 and NaN are genuinely fractional floats: the SS58 layer's bitwise prefix
        # arithmetic would silently truncate them, so they must be rejected as a wrong
        # type (mirrors the TypeScript "rejects a fractional ss58Format" test).
        # The formats are wrong-typed on purpose (the ignores stand in for an untyped caller);
        # the identifiers and the name are valid, so only the format is under test.
        for bad_format in [True, False, 5335.0, 5335.5, float("nan"), "5335", None]:
            with self.subTest(bad_format=bad_format):
                with self.assertRaisesRegex(TypeError, "^ss58_format must be an int"):
                    trevokeyless.encode_app_agent_address(
                        trevokeyless.AppAgentId(123),
                        ss58_format=bad_format,  # type: ignore[arg-type]
                    )
                with self.assertRaisesRegex(TypeError, "^ss58_format must be an int"):
                    trevokeyless.encode_transactional_address(
                        app_agent_id=trevokeyless.AppAgentId(123),
                        ta_id=trevokeyless.TransactionalId(456),
                        ss58_format=bad_format,  # type: ignore[arg-type]
                    )
                with self.assertRaisesRegex(TypeError, "^ss58_format must be an int"):
                    trevokeyless.encode_named_address(
                        app_agent_id=trevokeyless.AppAgentId(123),
                        account_name=trevokeyless.AccountName("example123"),
                        ss58_format=bad_format,  # type: ignore[arg-type]
                    )

    def test_encode_rejects_out_of_range_formats(self: "TestSs58FormatValidation") -> None:
        for bad_format in [-1, 16384]:
            with self.subTest(bad_format=bad_format):
                with self.assertRaisesRegex(ValueError, r"^ss58_format must be an integer in \[0, 16383\]"):
                    trevokeyless.encode_app_agent_address(trevokeyless.AppAgentId(123), ss58_format=bad_format)
                with self.assertRaisesRegex(ValueError, r"^ss58_format must be an integer in \[0, 16383\]"):
                    trevokeyless.encode_transactional_address(
                        app_agent_id=trevokeyless.AppAgentId(123),
                        ta_id=trevokeyless.TransactionalId(456),
                        ss58_format=bad_format,
                    )
                with self.assertRaisesRegex(ValueError, r"^ss58_format must be an integer in \[0, 16383\]"):
                    trevokeyless.encode_named_address(
                        app_agent_id=trevokeyless.AppAgentId(123),
                        account_name=trevokeyless.AccountName("example123"),
                        ss58_format=bad_format,
                    )

    def test_decode_rejects_wrong_type_formats(self: "TestSs58FormatValidation") -> None:
        address = trevokeyless.encode_app_agent_address(trevokeyless.AppAgentId(123), ss58_format=SS58_FORMAT)
        for bad_format in [True, 5335.0, 5335.5, float("nan"), None]:
            with (
                self.subTest(bad_format=bad_format),
                self.assertRaisesRegex(TypeError, "^ss58_format must be an int"),
            ):
                trevokeyless.decode_address(address, ss58_format=bad_format)  # type: ignore[arg-type]

    def test_decode_rejects_out_of_range_formats(self: "TestSs58FormatValidation") -> None:
        address = trevokeyless.encode_app_agent_address(trevokeyless.AppAgentId(123), ss58_format=SS58_FORMAT)
        for bad_format in [-1, 16384]:
            with (
                self.subTest(bad_format=bad_format),
                self.assertRaisesRegex(ValueError, r"^ss58_format must be an integer in \[0, 16383\]"),
            ):
                trevokeyless.decode_address(address, ss58_format=bad_format)

    def test_decode_account_id_takes_no_format(self: "TestSs58FormatValidation") -> None:
        # Raw account ids carry no SS58 format; decode_account_id therefore has no
        # ss58_format parameter at all.
        with self.assertRaises(TypeError):
            trevokeyless.decode_account_id("0x" + APP_AGENT_123.hex(), 5335)  # type: ignore[call-arg]

    def test_format_boundaries_accepted(self: "TestSs58FormatValidation") -> None:
        for good_format in [0, 16383]:
            with self.subTest(good_format=good_format):
                address = trevokeyless.encode_app_agent_address(trevokeyless.AppAgentId(123), ss58_format=good_format)
                self.assertEqual(trevokeyless.decode_app_agent_address(address, ss58_format=good_format), 123)

    def test_reserved_formats_are_valid_expected_decode_inputs(self: "TestSs58FormatValidation") -> None:
        address = trevokeyless.encode_app_agent_address(trevokeyless.AppAgentId(123), ss58_format=SS58_FORMAT)
        for reserved in [46, 47]:
            with self.subTest(reserved=reserved), self.assertRaises(Ss58Error) as ctx:
                trevokeyless.decode_address(address, ss58_format=reserved)
            self.assertEqual(ctx.exception.kind, "UnexpectedFormat")
            # The "UnexpectedFormat" kind is carried by the Ss58FormatMismatchError subclass,
            # which is what holds .expected/.found; narrow to it before reading them.
            assert isinstance(ctx.exception, trevokeyless.Ss58FormatMismatchError)  # noqa: S101
            self.assertEqual(ctx.exception.expected, reserved)
            self.assertEqual(ctx.exception.found, SS58_FORMAT)

    def test_reserved_formats_are_rejected_by_the_encoders(self: "TestSs58FormatValidation") -> None:
        # Reserved values are valid decoder expectations (above), but cannot be put on the
        # wire. An unusable format is a rejected *argument* however it is unusable, so the
        # encoders raise the same ValueError as for an out-of-range one - never an Ss58Error,
        # which reports on an *address*.
        for reserved in [46, 47]:
            with (
                self.subTest(reserved=reserved),
                self.assertRaisesRegex(ValueError, "^ss58_format must not be a reserved format"),
            ):
                trevokeyless.encode_app_agent_address(trevokeyless.AppAgentId(123), ss58_format=reserved)

    def test_reserved_format_rejection_is_not_an_ss58_error(self: "TestSs58FormatValidation") -> None:
        # The regression this guards: 46/47 used to fall through to the SS58 layer and raise
        # Ss58Error("InvalidFormat"), so `except RangeError`-style handling (TypeScript) or a
        # bare `except ValueError` that re-raised Ss58Error saw two different types for the
        # same mistake depending only on *how* the format was unusable.
        with self.assertRaises(ValueError) as ctx:
            trevokeyless.encode_app_agent_address(trevokeyless.AppAgentId(123), ss58_format=46)
        self.assertNotIsInstance(ctx.exception, Ss58Error)


class TestHexAccountIdContract(unittest.TestCase):
    """decode_account_id accepts a raw account id ("0x" + exactly 64 hex chars);
    anything else is rejected. decode_address accepts SS58 addresses only."""

    def test_valid_hex_account_id_is_classified(self: "TestHexAccountIdContract") -> None:
        info = trevokeyless.decode_account_id("0x" + APP_AGENT_123.hex())
        # `assert isinstance` rather than assertIsInstance: it asserts the same thing and also
        # narrows the BlockchainAccountInfo union for the type checker.
        assert isinstance(info, trevokeyless.AppAgentAccountInfo)  # noqa: S101 - isinstance narrowing for mypy
        self.assertEqual(info.app_agent_id, 123)
        self.assertEqual(info.account_id, "0x" + APP_AGENT_123.hex())
        # Raw account bytes carry no address; encode_address renders one explicitly.
        self.assertIsNone(info.address)

    def test_uppercase_hex_is_normalized(self: "TestHexAccountIdContract") -> None:
        info = trevokeyless.decode_account_id("0x" + APP_AGENT_123.hex().upper())
        self.assertIs(info.account_type, AccountType.AppAgent)
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
                trevokeyless.decode_address(hex_input, ss58_format=SS58_FORMAT)
        with self.assertRaisesRegex(ValueError, "use decode_account_id"):
            trevokeyless.is_keyless_address("0x" + APP_AGENT_123.hex(), ss58_format=SS58_FORMAT)


class TestEncodeAddress(unittest.TestCase):
    """encode_address renders a raw account id ("0x" + exactly 64 hex chars) as its
    canonical SS58 address, mirroring AccountId32::to_ss58check_with_version."""

    def test_round_trips_with_decode_address(self: "TestEncodeAddress") -> None:
        hex_account = "0x" + APP_AGENT_123.hex()
        address = trevokeyless.encode_address(hex_account, ss58_format=SS58_FORMAT)
        self.assertEqual(
            address,
            trevokeyless.encode_app_agent_address(trevokeyless.AppAgentId(123), ss58_format=SS58_FORMAT),
        )
        self.assertEqual(trevokeyless.decode_address(address, ss58_format=SS58_FORMAT).account_id, hex_account)

    def test_default_format_is_trevo(self: "TestEncodeAddress") -> None:
        hex_account = "0x" + APP_AGENT_123.hex()
        self.assertEqual(
            trevokeyless.encode_address(hex_account),
            trevokeyless.encode_app_agent_address(trevokeyless.AppAgentId(123)),
        )

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
        for wrong_type_format in [True, 5335.0, None]:
            with (
                self.subTest(wrong_type_format=wrong_type_format),
                self.assertRaisesRegex(TypeError, "^ss58_format must be an int"),
            ):
                trevokeyless.encode_address(hex_account, ss58_format=wrong_type_format)  # type: ignore[arg-type]
        for out_of_range_format in [-1, 16384]:
            with (
                self.subTest(out_of_range_format=out_of_range_format),
                self.assertRaisesRegex(ValueError, r"^ss58_format must be an integer in \[0, 16383\]"),
            ):
                trevokeyless.encode_address(hex_account, ss58_format=out_of_range_format)
        for reserved in [46, 47]:
            with (
                self.subTest(reserved=reserved),
                self.assertRaisesRegex(ValueError, "^ss58_format must not be a reserved format"),
            ):
                trevokeyless.encode_address(hex_account, ss58_format=reserved)

    def test_validates_format_before_the_account_id(self: "TestEncodeAddress") -> None:
        # The validation order is contractual and mirrored across languages: a caller
        # *parameter* is checked before the *data*, so a caller who got both wrong hears
        # about their own bug first. encode_address used to check the payload first, so it
        # disagreed with decode_address about which of two bad arguments to report.
        with self.assertRaisesRegex(ValueError, r"^ss58_format must be an integer in \[0, 16383\]"):
            trevokeyless.encode_address("not-an-account-id", ss58_format=99999)
        with self.assertRaisesRegex(ValueError, r"^ss58_format must be an integer in \[0, 16383\]"):
            trevokeyless.decode_address("not-an-address", ss58_format=99999)
        # ...and with only the payload wrong, the payload error still surfaces.
        with self.assertRaises(trevokeyless.ParseAccountIdError):
            trevokeyless.encode_address("not-an-account-id")


class TestKeylessEncoderValidationOrder(unittest.TestCase):
    """The keyless encoders check their identifier/name arguments *before* ss58_format,
    in declaration order (the open part is built before the format is examined) —
    unlike encode_address and the decoders, which check the format parameter first.
    The TypeScript implementation orders identically; pinning the both-invalid cases
    keeps a well-meaning reordering in one language from silently diverging from the
    other with every suite green."""

    def test_identifier_and_name_errors_win_over_the_format(self) -> None:
        with self.assertRaisesRegex(ValueError, r"^app_agent_id must be an integer"):
            trevokeyless.encode_app_agent_address(-1, ss58_format=99999)  # type: ignore[arg-type]
        with self.assertRaisesRegex(ValueError, r"^app_agent_id must be an integer"):
            trevokeyless.encode_transactional_address(
                app_agent_id=-1,  # type: ignore[arg-type]
                ta_id=-1,  # type: ignore[arg-type]
                ss58_format=99999,
            )
        with self.assertRaisesRegex(ValueError, r"^ta_id must be an integer"):
            trevokeyless.encode_transactional_address(
                app_agent_id=trevokeyless.AppAgentId(1),
                ta_id=-1,  # type: ignore[arg-type]
                ss58_format=99999,
            )
        with self.assertRaises(trevokeyless.AccountNameError):
            trevokeyless.encode_named_address(
                app_agent_id=trevokeyless.AppAgentId(1),
                account_name="bad",  # type: ignore[arg-type]
                ss58_format=99999,
            )
        # app_agent_id before the name: declaration order within the open part too.
        with self.assertRaisesRegex(ValueError, r"^app_agent_id must be an integer"):
            trevokeyless.encode_named_address(
                app_agent_id=-1,  # type: ignore[arg-type]
                account_name="bad",  # type: ignore[arg-type]
                ss58_format=99999,
            )
        # With only the format wrong, the format error still surfaces.
        with self.assertRaisesRegex(ValueError, r"^ss58_format must be an integer"):
            trevokeyless.encode_app_agent_address(trevokeyless.AppAgentId(123), ss58_format=99999)


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
                trevokeyless.encode_named_address(app_agent_id=trevokeyless.AppAgentId(1), account_name=bad)  # type: ignore[arg-type]


class TestIsKeylessAddress(unittest.TestCase):
    """Mirrors the Rust `test_is_keyless_account` test."""

    def test_keyless_addresses(self: "TestIsKeylessAddress") -> None:
        keyless_addresses = [
            trevokeyless.encode_app_agent_address(trevokeyless.AppAgentId(1), ss58_format=SS58_FORMAT),
            trevokeyless.encode_transactional_address(
                app_agent_id=trevokeyless.AppAgentId(1),
                ta_id=trevokeyless.TransactionalId(2),
                ss58_format=SS58_FORMAT,
            ),
            trevokeyless.encode_named_address(
                app_agent_id=trevokeyless.AppAgentId(1),
                account_name=trevokeyless.AccountName("test123456"),
                ss58_format=SS58_FORMAT,
            ),
        ]
        for address in keyless_addresses:
            with self.subTest(address=address):
                self.assertTrue(trevokeyless.is_keyless_address(address, ss58_format=SS58_FORMAT))

    def test_regular_addresses(self: "TestIsKeylessAddress") -> None:
        # Alice's well-known development account.
        alice = "ttqxHzRJmmjFBcE7Lb5Xs4GNMq2gFSt28JyvTEqjhqzE9EGP4"
        self.assertFalse(trevokeyless.is_keyless_address(alice, ss58_format=SS58_FORMAT))
        # A zeroed account has no valid keyless discriminant; raw account ids are
        # classified via decode_account_id.
        info = trevokeyless.decode_account_id("0x" + "00" * 32)
        self.assertIs(info.account_type, AccountType.Regular)


class TestBlockchainAccountInfoValueSemantics(unittest.TestCase):
    """BlockchainAccountInfo is a frozen dataclass: immutable and hashable, mirroring
    the Rust BlockchainAccountInfo (Copy + Eq + Hash)."""

    def test_is_immutable(self: "TestBlockchainAccountInfoValueSemantics") -> None:
        info = trevokeyless.decode_address(
            trevokeyless.encode_app_agent_address(trevokeyless.AppAgentId(123), ss58_format=SS58_FORMAT),
            ss58_format=SS58_FORMAT,
        )
        assert isinstance(info, trevokeyless.AppAgentAccountInfo)  # noqa: S101 - isinstance narrowing for mypy
        with self.assertRaises(dataclasses.FrozenInstanceError):
            # Assigning to a frozen dataclass field is the point of the test; mypy rejects it
            # statically ([misc]), the runtime must reject it too.
            info.app_agent_id = trevokeyless.AppAgentId(124)  # type: ignore[misc]

    def test_is_hashable(self: "TestBlockchainAccountInfoValueSemantics") -> None:
        address = trevokeyless.encode_app_agent_address(trevokeyless.AppAgentId(123), ss58_format=SS58_FORMAT)
        info_a = trevokeyless.decode_address(address, ss58_format=SS58_FORMAT)
        info_b = trevokeyless.decode_address(address, ss58_format=SS58_FORMAT)
        self.assertEqual(info_a, info_b)
        self.assertEqual(len({info_a, info_b}), 1)

    def test_each_variant_exposes_only_valid_identifiers(
        self: "TestBlockchainAccountInfoValueSemantics",
    ) -> None:
        regular = trevokeyless.decode_account_id("0x" + "00" * 32)
        app_agent = trevokeyless.decode_account_id("0x" + APP_AGENT_123.hex())
        transactional = trevokeyless.decode_account_id(
            trevokeyless.encode_transactional_account_id(
                app_agent_id=trevokeyless.AppAgentId(1),
                ta_id=trevokeyless.TransactionalId(2),
            )
        )
        named = trevokeyless.decode_account_id(
            trevokeyless.encode_named_account_id(
                app_agent_id=trevokeyless.AppAgentId(1),
                account_name=trevokeyless.AccountName("test123456"),
            )
        )

        self.assertEqual(set(regular.__slots__), {"address", "account_id", "account_type"})
        self.assertEqual(
            set(app_agent.__slots__),
            {"address", "account_id", "account_type", "app_agent_id"},
        )
        self.assertEqual(
            set(transactional.__slots__),
            {"address", "account_id", "account_type", "app_agent_id", "ta_id"},
        )
        self.assertEqual(
            set(named.__slots__),
            {"address", "account_id", "account_type", "app_agent_id", "account_name"},
        )


class TestSs58FormatMismatchError(unittest.TestCase):
    """The wrong-format error carries the expected and the found format, mirroring the
    Rust ``Ss58Error::UnexpectedFormat`` variant."""

    def test_carries_expected_and_found(self: "TestSs58FormatMismatchError") -> None:
        substrate_generic = trevokeyless.encode_app_agent_address(trevokeyless.AppAgentId(123), ss58_format=42)
        with self.assertRaises(trevokeyless.Ss58FormatMismatchError) as ctx:
            trevokeyless.decode_address(substrate_generic, ss58_format=SS58_FORMAT)
        self.assertEqual(ctx.exception.expected, SS58_FORMAT)
        self.assertEqual(ctx.exception.found, 42)
        # The "UnexpectedFormat" case of Ss58Error; remains a ValueError, like
        # every other decoding failure.
        self.assertIsInstance(ctx.exception, Ss58Error)
        self.assertEqual(ctx.exception.kind, "UnexpectedFormat")
        self.assertIsInstance(ctx.exception, ValueError)
        self.assertEqual(str(ctx.exception), "Expected SS58 format 5335 but found 42")

    def test_raised_for_two_byte_prefix_addresses(self: "TestSs58FormatMismatchError") -> None:
        trevo_address = trevokeyless.encode_app_agent_address(trevokeyless.AppAgentId(123), ss58_format=SS58_FORMAT)
        with self.assertRaises(trevokeyless.Ss58FormatMismatchError) as ctx:
            trevokeyless.decode_address(trevo_address, ss58_format=16383)
        self.assertEqual((ctx.exception.expected, ctx.exception.found), (16383, SS58_FORMAT))

    def test_direct_construction_validates_both_formats(self: "TestSs58FormatMismatchError") -> None:
        # The decode paths always pass pre-validated formats, so this pins direct
        # construction: both fields are validated like every sibling error
        # constructor's inputs. A huge adversarial int must produce the documented
        # ValueError — capped by _describe_rejected_int — not CPython's int->str
        # digit-limit error escaping from the message f-string; a wrong-typed value
        # must raise TypeError instead of minting a nonsense error.
        with self.assertRaises(ValueError) as huge_ctx:
            trevokeyless.Ss58FormatMismatchError(10**10000, 42)
        self.assertIn("must be an integer in [0, 16383]", str(huge_ctx.exception))
        with self.assertRaises(TypeError):
            trevokeyless.Ss58FormatMismatchError("a", "b")  # type: ignore[arg-type]
        with self.assertRaises(ValueError):
            trevokeyless.Ss58FormatMismatchError(SS58_FORMAT, 16384)
        with self.assertRaises(TypeError):
            # bool is an int subclass, so this needs no type-ignore; the constructor
            # still rejects it, like every other int-taking entry point.
            trevokeyless.Ss58FormatMismatchError(True, 42)
        # The reserved formats stay constructible: 46/47 are well-formed *expected*
        # values (no address carries them, which is exactly what this error reports).
        reserved = trevokeyless.Ss58FormatMismatchError(46, SS58_FORMAT)
        self.assertEqual((reserved.expected, reserved.found), (46, SS58_FORMAT))

    def test_checksum_failure_wins_over_format_mismatch(self: "TestSs58FormatMismatchError") -> None:
        # An invalid address must raise its validation error, never a format mismatch:
        # the expected format is compared only after the checksum has verified, exactly
        # like the Rust from_ss58check (the format mismatch error must not claim that a
        # corrupt string is "a valid address of another network").
        substrate_generic = trevokeyless.encode_app_agent_address(trevokeyless.AppAgentId(123), ss58_format=42)
        payload = bytearray(_b58decode(substrate_generic))
        payload[-1] ^= 0xFF
        corrupted = _b58encode(bytes(payload))
        with self.assertRaises(Ss58Error) as ctx:
            trevokeyless.decode_address(corrupted, ss58_format=SS58_FORMAT)
        self.assertEqual(ctx.exception.kind, "InvalidChecksum")
        self.assertNotIsInstance(ctx.exception, trevokeyless.Ss58FormatMismatchError)

    def test_length_failure_wins_over_format_mismatch(self: "TestSs58FormatMismatchError") -> None:
        # Same ordering for structurally invalid payloads of a non-expected format.
        truncated_payload = _format_prefix(42) + APP_AGENT_123[:31]
        address = _raw_ss58(truncated_payload)
        with self.assertRaises(Ss58Error) as ctx:
            trevokeyless.decode_address(address, ss58_format=SS58_FORMAT)
        self.assertEqual(ctx.exception.kind, "InvalidLength")
        self.assertNotIsInstance(ctx.exception, trevokeyless.Ss58FormatMismatchError)


class TestAccountTypeMismatchError(unittest.TestCase):
    def test_carries_expected_and_found(self: "TestAccountTypeMismatchError") -> None:
        named_address = trevokeyless.encode_named_address(
            app_agent_id=trevokeyless.AppAgentId(1),
            account_name=trevokeyless.AccountName("test123456"),
            ss58_format=SS58_FORMAT,
        )
        with self.assertRaises(AccountTypeMismatchError) as ctx:
            trevokeyless.decode_app_agent_address(named_address, ss58_format=SS58_FORMAT)
        self.assertIs(ctx.exception.expected, AccountType.AppAgent)
        self.assertIs(ctx.exception.found, AccountType.Named)
        # Backwards compatibility: the error is still a ValueError. The message renders
        # the plain type name ("Named"), like the TypeScript implementation.
        self.assertIsInstance(ctx.exception, ValueError)
        self.assertEqual(
            str(ctx.exception),
            "Provided address is not an AppAgent address but is a Named address",
        )

    def test_article_agrees_with_found_type(self: "TestAccountTypeMismatchError") -> None:
        # The article must agree with both type names, not only the expected one:
        # when the found type is AppAgent, the found side must read "an AppAgent".
        app_agent_address = trevokeyless.encode_app_agent_address(trevokeyless.AppAgentId(1), ss58_format=SS58_FORMAT)
        with self.assertRaises(AccountTypeMismatchError) as ctx:
            trevokeyless.decode_transactional_address(app_agent_address, ss58_format=SS58_FORMAT)
        self.assertIs(ctx.exception.expected, AccountType.Transactional)
        self.assertIs(ctx.exception.found, AccountType.AppAgent)
        self.assertEqual(
            str(ctx.exception),
            "Provided address is not a Transactional address but is an AppAgent address",
        )

    def test_direct_construction_rejects_bool(self: "TestAccountTypeMismatchError") -> None:
        # bool subclasses int, so True would otherwise coerce to AccountType.AppAgent
        # through the IntEnum call; the package-wide convention is that bool never
        # stands in for an int. The plain numeric values (0-3) keep coercing.
        with self.assertRaises(TypeError):
            AccountTypeMismatchError(True, AccountType.Named)  # type: ignore[arg-type]
        with self.assertRaises(TypeError):
            AccountTypeMismatchError(AccountType.AppAgent, False)  # type: ignore[arg-type]
        numeric = AccountTypeMismatchError(1, 3)  # type: ignore[arg-type]
        self.assertIs(numeric.expected, AccountType.AppAgent)
        self.assertIs(numeric.found, AccountType.Named)


class TestExceptionsSurviveSerialization(unittest.TestCase):
    """Every public exception must survive pickling and copying with its structured data.

    ``BaseException.__reduce__`` rebuilds an exception as ``cls(*self.args)``, and ``args``
    holds only the message — so an exception whose ``__init__`` signature differs from that
    (all of ours do) is either unreconstructable or, worse, silently rebuilt wrong. This is
    not academic: an off-chain consumer that propagates a decoding error across a process
    boundary (``multiprocessing``, ``concurrent.futures.ProcessPoolExecutor``, Celery) or
    deep-copies a result object carrying one hits exactly this path.
    """

    def _cases(self) -> list[BaseException]:
        return [
            Ss58Error("InvalidBase58"),
            Ss58Error("InvalidChecksum"),
            Ss58FormatMismatchError(SS58_FORMAT_TREVO_ASSET_HUB, 42),
            AccountNameError("InvalidLength"),
            AccountNameError("InvalidCharacter"),
            AccountTypeMismatchError(AccountType.Named, AccountType.Regular),
            ParseAccountIdError(),
            NotAnAddressError(),
        ]

    def test_pickle_round_trip_preserves_type_message_and_data(self) -> None:
        for error in self._cases():
            with self.subTest(error=type(error).__name__, message=str(error)):
                # S301: round-tripping our own exception objects, not untrusted data.
                restored = pickle.loads(pickle.dumps(error))  # noqa: S301
                self.assertIs(type(restored), type(error))
                # The message must not drift. AccountNameError in particular used to
                # unpickle with its message flipped to the InvalidCharacter text while
                # its kind still said InvalidLength.
                self.assertEqual(str(restored), str(error))
                self.assertEqual(getattr(restored, "kind", None), getattr(error, "kind", None))
                self.assertEqual(getattr(restored, "expected", None), getattr(error, "expected", None))
                self.assertEqual(getattr(restored, "found", None), getattr(error, "found", None))

    def test_copy_round_trip_preserves_type_message_and_data(self) -> None:
        for error in self._cases():
            with self.subTest(error=type(error).__name__):
                restored = copy.copy(error)
                self.assertIs(type(restored), type(error))
                self.assertEqual(str(restored), str(error))
                self.assertEqual(getattr(restored, "kind", None), getattr(error, "kind", None))

    def test_deepcopy_of_a_result_carrying_an_error(self) -> None:
        # The realistic shape: an error stored in a result object that gets deep-copied.
        errors = {"failure": AccountNameError("InvalidLength")}
        restored = copy.deepcopy(errors)["failure"]
        self.assertEqual(restored.kind, "InvalidLength")
        self.assertEqual(str(restored), "Account name must be exactly 10 characters long")


class TestIntegralFloatsAreRejected(unittest.TestCase):
    """The host-number twin of the TypeScript negative-zero rejection.

    JSON ``123.0`` parses to a float here (rejected: wrong type) but to the number 123 in
    JavaScript (accepted: 123.0 *is* 123 in IEEE-754, so there is nothing left to
    reject), and JSON ``-0`` parses to int 0 here (accepted) but to the double -0 there
    (rejected). Neither direction can be pinned by a shared fixture vector — see the
    host-number note in docs/conformance-schema.md — so the twin adversarial suites hold the
    parity: this is the Python half, mirroring the ``-0`` cases in
    keyless-ts/tests/adversarial.test.ts.
    """

    def test_integral_floats_are_wrong_types_not_values(self) -> None:
        for value in (123.0, 1.0, -0.0, 0.0, 4294967295.0):
            with self.subTest(value=value):
                with self.assertRaisesRegex(TypeError, "^app_agent_id must be an int, got float"):
                    AppAgentId(value)  # type: ignore[arg-type]
                with self.assertRaisesRegex(TypeError, "^ta_id must be an int, got float"):
                    TransactionalId(value)  # type: ignore[arg-type]
                with self.assertRaisesRegex(TypeError, "^ss58_format must be an int, got float"):
                    trevokeyless.encode_app_agent_address(AppAgentId(1), ss58_format=value)  # type: ignore[arg-type]

    def test_int_negative_zero_is_plain_zero(self) -> None:
        # Python's int -0 IS 0 — there is no distinct value to reject — so it is accepted,
        # the mirror image of JavaScript accepting 123.0. JSON `-0` therefore diverges by
        # host language: accepted here, rejected in TypeScript.
        self.assertEqual(AppAgentId(-0), 0)


class TestHugeIntegersAreNotEchoedBack(unittest.TestCase):
    """A rejected out-of-range int is echoed back verbatim only while it is small.

    CPython's int->str conversion limit (default 4300 digits) turns the message f-string
    itself into the raised error for a huge value — the caller would see CPython's
    "Exceeds the limit ... for integer string conversion" instead of the documented
    message — and Pythons without the limit (< 3.10.7) would echo megabytes of
    adversarial input back at the caller (and into logs). Beyond 64 bits the message
    reports the bit length instead.
    """

    def test_huge_identifier_reports_bit_length_not_digits(self) -> None:
        for ctor, label in ((AppAgentId, "app_agent_id"), (TransactionalId, "ta_id")):
            with self.subTest(label=label):
                pattern = rf"^{label} must be an integer in \[0, 4294967295\], got a 33220-bit int$"
                with self.assertRaisesRegex(ValueError, pattern):
                    ctor(10**10000)

    def test_huge_negative_identifier_reports_bit_length(self) -> None:
        with self.assertRaisesRegex(ValueError, r"got a negative 33220-bit int$"):
            AppAgentId(-(10**10000))

    def test_huge_ss58_format_reports_bit_length(self) -> None:
        with self.assertRaisesRegex(
            ValueError, r"^ss58_format must be an integer in \[0, 16383\], got a 33220-bit int$"
        ):
            trevokeyless.encode_app_agent_address(AppAgentId(1), ss58_format=10**10000)

    def test_the_64_bit_boundary_is_still_echoed_verbatim(self) -> None:
        # Every plausible wrong-width-type mistake (2**40, 2**53 + 1, -(2**32)) stays
        # echoed verbatim; the rendering only switches at 64 bits.
        with self.assertRaisesRegex(ValueError, r"got 18446744073709551615$"):
            AppAgentId(2**64 - 1)
        with self.assertRaisesRegex(ValueError, r"got a 65-bit int$"):
            AppAgentId(2**64)


class TestErrorKindTaxonomyIsClosed(unittest.TestCase):
    """The `kind` values are a pinned cross-language contract, so an out-of-taxonomy kind
    must not be constructible: it used to be silently mapped to the wrong message."""

    def test_ss58_error_rejects_an_unknown_kind(self) -> None:
        with self.assertRaises(ValueError):
            Ss58Error("Bogus")  # type: ignore[arg-type]

    def test_account_name_error_rejects_an_unknown_kind(self) -> None:
        with self.assertRaises(ValueError):
            AccountNameError("Bogus")  # type: ignore[arg-type]

    def test_the_five_ss58_kinds_are_all_constructible(self) -> None:
        for kind in ("InvalidBase58", "InvalidLength", "InvalidChecksum", "InvalidFormat"):
            self.assertEqual(Ss58Error(kind).kind, kind)
        # The fifth kind carries an expected/found pair and is its dedicated subclass.
        self.assertEqual(Ss58FormatMismatchError(5335, 42).kind, "UnexpectedFormat")

    def test_unexpected_format_without_a_message_points_to_the_subclass(self) -> None:
        # Bare construction of the one kind with no fixed standard message used to escape
        # as a KeyError from the message table — outside the package's own
        # TypeError/ValueError taxonomy for a documented public kind.
        with self.assertRaisesRegex(ValueError, "Ss58FormatMismatchError"):
            Ss58Error("UnexpectedFormat")

    def test_the_message_cannot_be_forged_to_contradict_the_kind(self) -> None:
        # The message is derived from the kind; there is no message parameter to abuse,
        # so a kind/message contradiction is unconstructible.
        with self.assertRaises(TypeError):
            Ss58Error("InvalidBase58", "Address has an invalid checksum")  # type: ignore[call-arg]
        self.assertEqual(str(Ss58Error("InvalidBase58")), "Address is not valid base58")

    def test_account_type_mismatch_error_coerces_and_validates_its_pair(self) -> None:
        # Sibling constructors validate their inputs; this one used to leak an
        # AttributeError from the message f-string for non-AccountType arguments.
        error = AccountTypeMismatchError(1, 2)  # type: ignore[arg-type]
        self.assertIs(error.expected, AccountType.AppAgent)
        self.assertIs(error.found, AccountType.Transactional)
        with self.assertRaises(ValueError):
            AccountTypeMismatchError(7, 2)  # type: ignore[arg-type]


class TestIdentifiersRequireAValue(unittest.TestCase):
    """A forgotten argument must not silently mean AppAgent 0 — the one case that would
    defeat the purpose of these validating types."""

    def test_app_agent_id_requires_an_argument(self) -> None:
        with self.assertRaises(TypeError):
            AppAgentId()  # type: ignore[call-arg]

    def test_transactional_id_requires_an_argument(self) -> None:
        with self.assertRaises(TypeError):
            TransactionalId()  # type: ignore[call-arg]


class TestAccountNameLengthIsBoundedEarly(unittest.TestCase):
    """A long name must be rejected without a full scan: encode_named_address is the natural
    consumer of caller-supplied names, and it was the only entry point that did not bound its
    input before doing per-character work."""

    def test_a_very_long_name_is_rejected_promptly(self) -> None:
        started = time.perf_counter()
        with self.assertRaises(AccountNameError) as caught:
            # A 5 MB name cannot be wrapped in AccountName() - the length check is what rejects
            # it - so the raw str goes straight to the encoder (the ignore stands in for an
            # untyped caller), which is the entry point whose bounding is under test.
            encode_named_address(
                app_agent_id=AppAgentId(1),
                account_name="a" * 5_000_000,  # type: ignore[arg-type]
            )
        elapsed = time.perf_counter() - started
        self.assertEqual(caught.exception.kind, "InvalidLength")
        # Generous bound: the point is O(1)-ish rejection, not a benchmark, so this stays
        # well clear of the wall-clock noise on shared CI runners. The unbounded scan took
        # ~0.3s for this input.
        self.assertLess(elapsed, 1.0)


class TestAccountNameCharsetIsImmutable(unittest.TestCase):
    """The account-name charset gates the wire format, so it must not be mutable.

    It was a plain module-global `set`: adding a character to it in-process made the
    encoder accept that character and emit an account id that neither Rust nor TypeScript
    can produce or classify.
    """

    def test_the_charset_is_a_frozenset(self: "TestAccountNameCharsetIsImmutable") -> None:
        charset = _ALLOWED_NAME_CHARS
        self.assertIsInstance(charset, frozenset)
        with self.assertRaises(AttributeError):
            charset.add("!")  # type: ignore[attr-defined] # the point of the test

    def test_the_charset_is_exactly_the_wire_contract(self: "TestAccountNameCharsetIsImmutable") -> None:
        expected = frozenset(string.digits + string.ascii_lowercase + string.ascii_uppercase + "-#")
        self.assertEqual(_ALLOWED_NAME_CHARS, expected)
        self.assertEqual(len(_ALLOWED_NAME_CHARS), 64)


if __name__ == "__main__":
    unittest.main()
