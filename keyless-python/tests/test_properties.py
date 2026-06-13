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

# The 64 characters allowed in an address name.
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


def _random_name(rng: _Rng) -> str:
    return "".join(_CHARSET[rng.next_u64() % 64] for _ in range(10))


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
        for app_agent_id in ids:
            address = trevokeyless.encode_app_agent_address(app_agent_id, SS58_FORMAT)
            info = trevokeyless.decode_address(address, SS58_FORMAT)
            self.assertIs(info.account_type, AccountType.AppAgent)
            self.assertEqual(info.app_agent_id, app_agent_id)
            self.assertIsNone(info.ta_id)
            self.assertIsNone(info.account_name)
            self.assertEqual(trevokeyless.decode_app_agent_address(address, SS58_FORMAT), app_agent_id)
            with self.assertRaises(AccountTypeMismatchError) as ctx:
                trevokeyless.decode_transactional_address(address, SS58_FORMAT)
            self.assertIs(ctx.exception.found, AccountType.AppAgent)
            with self.assertRaises(AccountTypeMismatchError):
                trevokeyless.decode_named_address(address, SS58_FORMAT)
            self.assertTrue(trevokeyless.is_keyless_address(address, SS58_FORMAT))

    def test_prop_transactional_roundtrip(self: "TestRoundTripProperties") -> None:
        rng = _Rng(0x5EED0002)
        ids = [(rng.next_u32(), rng.next_u32()) for _ in range(self.ITERATIONS)]
        ids += [(0, 0), (0, U32_MAX), (U32_MAX, 0), (U32_MAX, U32_MAX)]
        for app_agent_id, ta_id in ids:
            address = trevokeyless.encode_transactional_address(app_agent_id, ta_id, SS58_FORMAT)
            info = trevokeyless.decode_address(address, SS58_FORMAT)
            self.assertIs(info.account_type, AccountType.Transactional)
            self.assertEqual((info.app_agent_id, info.ta_id), (app_agent_id, ta_id))
            self.assertEqual(
                trevokeyless.decode_transactional_address(address, SS58_FORMAT),
                (app_agent_id, ta_id),
            )
            with self.assertRaises(AccountTypeMismatchError) as ctx:
                trevokeyless.decode_app_agent_address(address, SS58_FORMAT)
            self.assertIs(ctx.exception.found, AccountType.Transactional)
            self.assertTrue(trevokeyless.is_keyless_address(address, SS58_FORMAT))

    def test_prop_named_roundtrip(self: "TestRoundTripProperties") -> None:
        rng = _Rng(0x5EED0003)
        for _ in range(self.ITERATIONS):
            app_agent_id = rng.next_u32()
            name = _random_name(rng)
            address = trevokeyless.encode_named_address(app_agent_id, name, SS58_FORMAT)
            info = trevokeyless.decode_address(address, SS58_FORMAT)
            self.assertIs(info.account_type, AccountType.Named)
            self.assertEqual((info.app_agent_id, info.account_name), (app_agent_id, name))
            self.assertEqual(
                trevokeyless.decode_named_address(address, SS58_FORMAT),
                (app_agent_id, name),
            )
            with self.assertRaises(AccountTypeMismatchError) as ctx:
                trevokeyless.decode_app_agent_address(address, SS58_FORMAT)
            self.assertIs(ctx.exception.found, AccountType.Named)
            self.assertTrue(trevokeyless.is_keyless_address(address, SS58_FORMAT))


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
            info = trevokeyless.decode_account_id("0x" + bytes(account).hex())
            self.assertIs(
                info.account_type,
                AccountType.Regular,
                "random bytes must classify as Regular (a checksum match has probability ~2^-136 at best)",
            )

    def test_prop_single_byte_corruption_demotes_to_regular(
        self: "TestClassificationProperties",
    ) -> None:
        # Corrupting any single byte of a keyless account - open part or checksum -
        # must invalidate it. The construction (accounts, then one non-zero xor per
        # position) consumes the 0x5EED_0005 stream exactly like the Rust test.
        rng = _Rng(0x5EED0005)
        accounts = [
            trevokeyless.encode_app_agent_address(rng.next_u32(), SS58_FORMAT),
            trevokeyless.encode_transactional_address(rng.next_u32(), rng.next_u32(), SS58_FORMAT),
            trevokeyless.encode_named_address(rng.next_u32(), _random_name(rng), SS58_FORMAT),
        ]
        for address in accounts:
            original = bytes.fromhex(trevokeyless.decode_address(address, SS58_FORMAT).account_id[2:])
            for position in range(32):
                xor = rng.next_u64() % 255 + 1  # non-zero
                corrupted = bytearray(original)
                corrupted[position] ^= xor
                info = trevokeyless.decode_account_id("0x" + bytes(corrupted).hex())
                self.assertIs(
                    info.account_type,
                    AccountType.Regular,
                    f"corrupting byte {position} of {address} must demote it to Regular",
                )

    def test_prop_account_name_validation(self: "TestClassificationProperties") -> None:
        # Mirrors prop_account_name_construction: every generated name encodes, and
        # replacing any character with an out-of-charset byte is rejected (Rust runs
        # 500 iterations over the same stream).
        rng = _Rng(0x5EED0006)
        for _ in range(300):
            name = _random_name(rng)
            self.assertEqual(len(name), 10)
            trevokeyless.encode_named_address(1, name, SS58_FORMAT)

            position = rng.next_u64() % 10
            bad = name[:position] + "@" + name[position + 1 :]
            with self.assertRaisesRegex(ValueError, "^Address name contains invalid characters$"):
                trevokeyless.encode_named_address(1, bad, SS58_FORMAT)


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
            address = trevokeyless.encode_address(account_id, ss58_format)
            info = trevokeyless.decode_address(address, ss58_format)
            self.assertEqual(
                info.account_id,
                account_id,
                f"SS58 round trip must preserve the account and format {ss58_format}",
            )


if __name__ == "__main__":
    unittest.main()
