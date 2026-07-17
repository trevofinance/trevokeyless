import unittest

import trevokeyless


class TestKeylessAddresses(unittest.TestCase):
    def test_valid_name(self: "TestKeylessAddresses") -> None:
        valid_names = ["example123", "abcdEFGHIJ", "1234567890", "name-#test"]
        for name in valid_names:
            with self.subTest(name=name):
                try:
                    trevokeyless.encode_named_address(
                        app_agent_id=trevokeyless.AppAgentId(123),
                        account_name=trevokeyless.AccountName(name),
                        ss58_format=42,
                    )
                except ValueError:
                    self.fail(f"Valid name '{name}' was incorrectly rejected.")

    def test_invalid_name(self: "TestKeylessAddresses") -> None:
        invalid_names = {
            "example!23": "InvalidCharacter",
            "name@12345": "InvalidCharacter",
            "abcd*EFGHI": "InvalidCharacter",
            # Exactly 10 characters, so the space/underscore actually reaches the
            # charset check (a longer spelling would fail on length first).
            "with space": "InvalidCharacter",
            "with_under": "InvalidCharacter",
            "short": "InvalidLength",
            "veeeerrrrryyyyloooongg": "InvalidLength",
        }
        # These names cannot be wrapped in AccountName() — the constructor is exactly what
        # rejects them. The raw `str` is passed to the encoder on purpose, with a type: ignore
        # simulating an untyped caller, to prove the encoder re-validates the name at runtime.
        for name, expected_kind in invalid_names.items():
            with self.subTest(name=name):
                with self.assertRaises(trevokeyless.AccountNameError) as ctx:
                    trevokeyless.encode_named_address(
                        app_agent_id=trevokeyless.AppAgentId(123),
                        account_name=name,  # type: ignore[arg-type]
                        ss58_format=42,
                    )
                self.assertEqual(ctx.exception.kind, expected_kind)
                self.assertIsInstance(ctx.exception, ValueError)


class TestAccountNameType(unittest.TestCase):
    """The validating AccountName type — the counterpart of AppAgentId, of the Rust
    AccountName newtype, and of the TypeScript accountName() factory. A bad name must fail
    where it is written, not later inside an encoder."""

    def test_accepts_a_valid_name(self) -> None:
        name = trevokeyless.AccountName("Treasury-1")
        self.assertEqual(name, "Treasury-1")
        self.assertIsInstance(name, str)

    def test_accepts_every_allowed_character(self) -> None:
        self.assertEqual(trevokeyless.AccountName("aZ09-#aaaa"), "aZ09-#aaaa")

    def test_rejects_a_wrong_length_name(self) -> None:
        for value in ("short", "far-too-long-a-name", ""):
            with self.subTest(value=value), self.assertRaises(trevokeyless.AccountNameError) as ctx:
                trevokeyless.AccountName(value)
            self.assertEqual(ctx.exception.kind, "InvalidLength")
            self.assertEqual(str(ctx.exception), "Account name must be exactly 10 characters long")

    def test_rejects_an_out_of_charset_name(self) -> None:
        for value in ("bad name!!", "hot_wallet"):
            with self.subTest(value=value), self.assertRaises(trevokeyless.AccountNameError) as ctx:
                trevokeyless.AccountName(value)
            self.assertEqual(ctx.exception.kind, "InvalidCharacter")
            self.assertEqual(str(ctx.exception), "Account name contains invalid characters")

    def test_rejects_a_non_string(self) -> None:
        with self.assertRaises(TypeError):
            trevokeyless.AccountName(123)  # type: ignore[arg-type]

    def test_validates_by_utf8_byte_length_like_rust(self) -> None:
        # "héllo12345" is 10 code points but 11 UTF-8 bytes -> InvalidLength.
        with self.assertRaises(trevokeyless.AccountNameError) as ctx:
            trevokeyless.AccountName("héllo12345")
        self.assertEqual(ctx.exception.kind, "InvalidLength")
        # "café12345" is 9 code points but 10 UTF-8 bytes -> reaches the charset check.
        with self.assertRaises(trevokeyless.AccountNameError) as ctx_charset:
            trevokeyless.AccountName("café12345")
        self.assertEqual(ctx_charset.exception.kind, "InvalidCharacter")

    def test_a_decoded_name_is_an_account_name(self) -> None:
        address = trevokeyless.encode_named_address(
            app_agent_id=trevokeyless.AppAgentId(7),
            account_name=trevokeyless.AccountName("Treasury-1"),
        )
        decoded = trevokeyless.decode_named_address(address)
        self.assertIsInstance(decoded.account_name, trevokeyless.AccountName)


class TestNonThrowingGuards(unittest.TestCase):
    """The is_* guards, mirroring the TypeScript isAccountName / isAppAgentId /
    isTransactionalId type guards case for case: True exactly for the values the
    validating constructors accept, False (never an exception) for everything else."""

    def test_is_account_name_matches_exactly_what_the_constructor_accepts(self) -> None:
        for valid in ["Treasury-1", "example123", "name-#test", "1234567890"]:
            with self.subTest(value=valid):
                self.assertTrue(trevokeyless.is_account_name(valid))
        # Wrong byte length, out-of-charset, and non-strings are all rejected
        # without raising.
        invalid_values: list[object] = [
            "short",
            "toolongname11",
            "example!23",
            "café12345",
            123,
            None,
            ["Treasury-1"],
            b"Treasury-1",
        ]
        for invalid in invalid_values:
            with self.subTest(value=invalid):
                self.assertFalse(trevokeyless.is_account_name(invalid))

    def test_is_app_agent_id_and_is_transactional_id_match_a_valid_u32(self) -> None:
        for guard in (trevokeyless.is_app_agent_id, trevokeyless.is_transactional_id):
            with self.subTest(guard=guard.__name__):
                self.assertTrue(guard(0))
                self.assertTrue(guard(123))
                self.assertTrue(guard(trevokeyless.U32_MAX))
                # Out of range, non-integer, and non-int are all rejected without
                # raising; bool is an int subclass and is rejected like the
                # constructors reject it.
                self.assertFalse(guard(-1))
                self.assertFalse(guard(trevokeyless.U32_MAX + 1))
                self.assertFalse(guard(1.5))
                self.assertFalse(guard(float("nan")))
                self.assertFalse(guard(True))
                self.assertFalse(guard("123"))
                self.assertFalse(guard(None))
                # The float -0.0 signals a float-typed source and is rejected (the
                # int 0 is accepted above), mirroring the TypeScript guard rejecting
                # the double -0 — the documented host-number split.
                self.assertFalse(guard(-0.0))


if __name__ == "__main__":
    unittest.main()
