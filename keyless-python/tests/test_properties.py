"""Property-style randomized tests mirroring the Rust suite (`tests_properties.rs`).

The same deterministic xorshift64* PRNG construction and seeds are used in the Rust,
Python, and TypeScript suites, so a failure in one implementation reproduces
bit-for-bit in the others — the three suites together act as a cross-language
differential fuzzer (``test_prng_matches_the_reference_sequence`` pins the stream). Keep the
construction byte-for-byte identical across the languages when refactoring, or they
silently stop testing the same inputs. Iteration counts are reduced where the
interpreted languages would otherwise dominate the suite runtime; since the PRNG streams
are identical, the reduced runs cover a prefix of the Rust runs.
"""

import unittest

import trevokeyless
from trevokeyless import AccountType, AccountTypeMismatchError

SS58_FORMAT = 5335
U32_MAX = 2**32 - 1

_U64_MASK = 0xFFFFFFFFFFFFFFFF

# The 64 characters allowed in an account name.
_CHARSET = "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ-#"


class _Rng:
    """Deterministic xorshift64* PRNG (same construction and seeds as the Rust
    `tests_properties.rs` and the TypeScript `properties.test.ts`). Seeds must be
    non-zero."""

    def __init__(self, seed: int) -> None:
        self._state = seed

    def next_u64(self) -> int:
        x = self._state
        x ^= x >> 12
        x = (x ^ (x << 25)) & _U64_MASK
        x ^= x >> 27
        self._state = x
        return (x * 0x2545F4914F6CDD1D) & _U64_MASK

    def next_u32(self) -> int:
        return self.next_u64() >> 32

    def fill(self, length: int) -> bytes:
        """`length` little-endian bytes from consecutive next_u64 outputs (the Rust
        `fill` over 8-byte chunks)."""
        out = bytearray()
        while len(out) < length:
            out += self.next_u64().to_bytes(8, byteorder="little")
        return bytes(out[:length])


def _random_name(rng: _Rng) -> trevokeyless.AccountName:
    # `% 64` takes xorshift64*'s statistically weakest low bits — deliberately kept identical
    # across the Rust/Python/TS property suites and the fixture generator; changing it would
    # desynchronize the shared PRNG streams.
    # Every character comes from the allowed charset and exactly 10 are drawn, so the
    # AccountName constructor always accepts the result.
    return trevokeyless.AccountName("".join(_CHARSET[rng.next_u64() % 64] for _ in range(10)))


class TestPrng(unittest.TestCase):
    def test_prng_matches_the_reference_sequence(self: "TestPrng") -> None:
        # The first outputs of seed 0x5EED_0001, computed by the Rust implementation. A
        # porting mistake in the PRNG would silently change every sequence below; this
        # pin makes it fail loudly instead.
        rng = _Rng(0x5EED0001)
        self.assertEqual(
            [rng.next_u64() for _ in range(3)],
            [0xA2E8EC67270A2376, 0x44BE5980BAE0D9B9, 0x31076561BB9CC917],
        )


class TestRoundTripProperties(unittest.TestCase):
    """Mirrors prop_app_agent_roundtrip / prop_transactional_roundtrip /
    prop_named_roundtrip (Rust runs 1000 iterations; the streams are identical)."""

    ITERATIONS = 300

    def test_prop_app_agent_roundtrip(self: "TestRoundTripProperties") -> None:
        rng = _Rng(0x5EED0001)
        ids = [rng.next_u32() for _ in range(self.ITERATIONS)] + [0, 1, U32_MAX]
        for raw_app_agent_id in ids:
            app_agent_id = trevokeyless.AppAgentId(raw_app_agent_id)
            address = trevokeyless.encode_app_agent_address(app_agent_id, ss58_format=SS58_FORMAT)
            info = trevokeyless.decode_address(address, ss58_format=SS58_FORMAT)
            # `assert isinstance` rather than assertIsInstance: it asserts the same thing and
            # also narrows the BlockchainAccountInfo union for the type checker.
            assert isinstance(info, trevokeyless.AppAgentAccountInfo)  # noqa: S101 - isinstance narrowing for mypy
            self.assertEqual(info.app_agent_id, app_agent_id)
            self.assertEqual(trevokeyless.decode_app_agent_address(address, ss58_format=SS58_FORMAT), app_agent_id)
            # Both the wrong-type decoders must report the same (expected, found) pair -
            # the cross-language contract of AccountTypeMismatchError.
            for decode, expected in (
                (trevokeyless.decode_transactional_address, AccountType.Transactional),
                (trevokeyless.decode_named_address, AccountType.Named),
            ):
                with self.assertRaises(AccountTypeMismatchError) as ctx:
                    decode(address, ss58_format=SS58_FORMAT)
                self.assertIs(ctx.exception.expected, expected)
                self.assertIs(ctx.exception.found, AccountType.AppAgent)
            self.assertTrue(trevokeyless.is_keyless_address(address, ss58_format=SS58_FORMAT))
            # The account-id decoders agree with the address decoders.
            self.assertEqual(trevokeyless.decode_app_agent_account_id(info.account_id), app_agent_id)

    def test_prop_transactional_roundtrip(self: "TestRoundTripProperties") -> None:
        rng = _Rng(0x5EED0002)
        ids = [(rng.next_u32(), rng.next_u32()) for _ in range(self.ITERATIONS)]
        ids += [(0, 0), (0, U32_MAX), (U32_MAX, 0), (U32_MAX, U32_MAX)]
        for raw_app_agent_id, raw_ta_id in ids:
            app_agent_id = trevokeyless.AppAgentId(raw_app_agent_id)
            ta_id = trevokeyless.TransactionalId(raw_ta_id)
            address = trevokeyless.encode_transactional_address(
                app_agent_id=app_agent_id,
                ta_id=ta_id,
                ss58_format=SS58_FORMAT,
            )
            info = trevokeyless.decode_address(address, ss58_format=SS58_FORMAT)
            assert isinstance(info, trevokeyless.TransactionalAccountInfo)  # noqa: S101 - isinstance narrowing for mypy
            self.assertEqual((info.app_agent_id, info.ta_id), (app_agent_id, ta_id))
            self.assertEqual(
                trevokeyless.decode_transactional_address(address, ss58_format=SS58_FORMAT),
                (app_agent_id, ta_id),
            )
            for decode, expected in (
                (trevokeyless.decode_app_agent_address, AccountType.AppAgent),
                (trevokeyless.decode_named_address, AccountType.Named),
            ):
                with self.assertRaises(AccountTypeMismatchError) as ctx:
                    decode(address, ss58_format=SS58_FORMAT)
                self.assertIs(ctx.exception.expected, expected)
                self.assertIs(ctx.exception.found, AccountType.Transactional)
            self.assertTrue(trevokeyless.is_keyless_address(address, ss58_format=SS58_FORMAT))
            self.assertEqual(
                trevokeyless.decode_transactional_account_id(info.account_id),
                (app_agent_id, ta_id),
            )

    def test_prop_named_roundtrip(self: "TestRoundTripProperties") -> None:
        rng = _Rng(0x5EED0003)
        for _ in range(self.ITERATIONS):
            app_agent_id = trevokeyless.AppAgentId(rng.next_u32())
            name = _random_name(rng)
            address = trevokeyless.encode_named_address(
                app_agent_id=app_agent_id,
                account_name=name,
                ss58_format=SS58_FORMAT,
            )
            info = trevokeyless.decode_address(address, ss58_format=SS58_FORMAT)
            assert isinstance(info, trevokeyless.NamedAccountInfo)  # noqa: S101 - isinstance narrowing for mypy
            self.assertEqual((info.app_agent_id, info.account_name), (app_agent_id, name))
            self.assertEqual(
                trevokeyless.decode_named_address(address, ss58_format=SS58_FORMAT),
                (app_agent_id, name),
            )
            for decode, expected in (
                (trevokeyless.decode_app_agent_address, AccountType.AppAgent),
                (trevokeyless.decode_transactional_address, AccountType.Transactional),
            ):
                with self.assertRaises(AccountTypeMismatchError) as ctx:
                    decode(address, ss58_format=SS58_FORMAT)
                self.assertIs(ctx.exception.expected, expected)
                self.assertIs(ctx.exception.found, AccountType.Named)
            self.assertTrue(trevokeyless.is_keyless_address(address, ss58_format=SS58_FORMAT))
            self.assertEqual(
                trevokeyless.decode_named_account_id(info.account_id),
                (app_agent_id, name),
            )


class TestClassificationProperties(unittest.TestCase):
    def test_prop_decode_arbitrary_bytes_never_raises_and_is_regular(
        self: "TestClassificationProperties",
    ) -> None:
        # Mirrors prop_decode_arbitrary_bytes_never_panics_and_is_regular (Rust runs
        # 50,000 iterations over the same stream).
        rng = _Rng(0x5EED0004)
        for i in range(10_000):
            account = bytearray(rng.fill(32))
            # Cycle the discriminant byte through 0..=5 so every classification branch
            # (unknown discriminant and all three checksum verifications) is exercised.
            account[4] = i % 6
            account_id = "0x" + bytes(account).hex()
            info = trevokeyless.decode_account_id(account_id)
            # The whole value is pinned, not just the discriminant: a Regular result must
            # carry no leftover keyless identifiers, and (decoded from raw bytes) no
            # address.
            self.assertEqual(
                info,
                trevokeyless.RegularAccountInfo(address=None, account_id=account_id),
                "random bytes must classify as Regular (a checksum match has probability ~2^-136 at best)",
            )
            self.assertFalse(trevokeyless.is_keyless_account_id(account_id))

    def test_prop_single_byte_corruption_demotes_to_regular(
        self: "TestClassificationProperties",
    ) -> None:
        # Corrupting any single byte of a keyless account - open part or checksum -
        # must invalidate it. The construction (accounts, then one non-zero xor per
        # position) consumes the 0x5EED_0005 stream exactly like the Rust test.
        rng = _Rng(0x5EED0005)
        accounts = [
            trevokeyless.encode_app_agent_address(trevokeyless.AppAgentId(rng.next_u32()), ss58_format=SS58_FORMAT),
            trevokeyless.encode_transactional_address(
                app_agent_id=trevokeyless.AppAgentId(rng.next_u32()),
                ta_id=trevokeyless.TransactionalId(rng.next_u32()),
                ss58_format=SS58_FORMAT,
            ),
            trevokeyless.encode_named_address(
                app_agent_id=trevokeyless.AppAgentId(rng.next_u32()),
                account_name=_random_name(rng),
                ss58_format=SS58_FORMAT,
            ),
        ]
        for address in accounts:
            original = bytes.fromhex(trevokeyless.decode_address(address, ss58_format=SS58_FORMAT).account_id[2:])
            for position in range(32):
                xor = rng.next_u64() % 255 + 1  # non-zero
                corrupted = bytearray(original)
                corrupted[position] ^= xor
                corrupted_id = "0x" + bytes(corrupted).hex()
                info = trevokeyless.decode_account_id(corrupted_id)
                self.assertEqual(
                    info,
                    trevokeyless.RegularAccountInfo(address=None, account_id=corrupted_id),
                    f"corrupting byte {position} of {address} must demote it to Regular"
                    " (with no keyless identifiers left on the result)",
                )

    def test_prop_account_name_validation(self: "TestClassificationProperties") -> None:
        # Mirrors prop_account_name_construction: every generated name encodes, and
        # replacing any character with an out-of-charset byte is rejected (Rust runs
        # 500 iterations over the same stream).
        rng = _Rng(0x5EED0006)
        for _ in range(300):
            name = _random_name(rng)
            self.assertEqual(len(name), 10)
            trevokeyless.encode_named_address(
                app_agent_id=trevokeyless.AppAgentId(1),
                account_name=name,
                ss58_format=SS58_FORMAT,
            )

            position = rng.next_u64() % 10
            # `bad` is out of charset, so AccountName() would reject it and it cannot be
            # constructed. The raw `str` is passed on purpose — the type: ignore simulates an
            # untyped caller — to prove the encoder itself re-validates the name at runtime.
            bad = name[:position] + "@" + name[position + 1 :]
            with self.assertRaisesRegex(ValueError, "^Account name contains invalid characters$"):
                trevokeyless.encode_named_address(
                    app_agent_id=trevokeyless.AppAgentId(1),
                    account_name=bad,  # type: ignore[arg-type]
                    ss58_format=SS58_FORMAT,
                )


class TestSs58Properties(unittest.TestCase):
    def test_prop_ss58_roundtrip_random_accounts_and_formats(self: "TestSs58Properties") -> None:
        # Mirrors prop_ss58_roundtrip_random_accounts_and_formats (Rust runs 500
        # iterations over the same stream).
        rng = _Rng(0x5EED0007)
        for _ in range(300):
            account_id = "0x" + rng.fill(32).hex()
            ss58_format = rng.next_u64() % 16384
            if ss58_format in (46, 47):
                ss58_format = 48  # skip the reserved formats
            address = trevokeyless.encode_address(account_id, ss58_format=ss58_format)
            info = trevokeyless.decode_address(address, ss58_format=ss58_format)
            self.assertEqual(
                info.account_id,
                account_id,
                f"SS58 round trip must preserve the account and format {ss58_format}",
            )
            # The unknown-format decoder must agree on the account and report the format
            # that was encoded — the same pair the Rust from_ss58check_with_version and
            # TypeScript decodeAddressWithFormat assert over this stream.
            decoded = trevokeyless.decode_address_with_format(address)
            self.assertEqual(decoded.info, info, "the unknown-format decoder must agree with decode_address")
            self.assertEqual(decoded.ss58_format, ss58_format, "the format found must be the format encoded")


if __name__ == "__main__":
    unittest.main()
