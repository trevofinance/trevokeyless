/**
 * Property-style randomized tests mirroring the Rust suite (`tests_properties.rs`).
 *
 * The same deterministic xorshift64* PRNG construction and seeds are used in the Rust,
 * Python, and TypeScript suites, so a failure in one implementation reproduces
 * bit-for-bit in the others — the three suites together act as a cross-language
 * differential fuzzer (the "PRNG reference sequence" test below pins the stream). Keep
 * the construction byte-for-byte identical across the languages when refactoring, or
 * they silently stop testing the same inputs. Iteration counts are reduced where they
 * would otherwise dominate the suite runtime; since the PRNG streams are identical, the
 * reduced runs cover a prefix of the Rust runs.
 */
import { bytesToHex } from "@noble/hashes/utils.js";
import {
  AccountType,
  AccountTypeMismatchError,
  decodeAccountId,
  decodeAddress,
  decodeAppAgentAddress,
  decodeNamedAddress,
  decodeTransactionalAddress,
  encodeAddress,
  encodeAppAgentAddress,
  encodeNamedAddress,
  encodeTransactionalAddress,
  isKeylessAddress,
} from "../src/keyless";

const ss58Format = 5335;
const U32_MAX = 4294967295;

// The 64 characters allowed in an address name.
const CHARSET =
  "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ-#";

/**
 * Deterministic xorshift64* PRNG (same construction and seeds as the Rust
 * `tests_properties.rs` and the Python `test_properties.py`). Seeds must be non-zero.
 */
class Rng {
  private state: bigint;

  constructor(seed: bigint) {
    this.state = seed;
  }

  nextU64(): bigint {
    let x = this.state;
    x ^= x >> 12n;
    x = BigInt.asUintN(64, x ^ (x << 25n));
    x ^= x >> 27n;
    this.state = x;
    return BigInt.asUintN(64, x * 0x2545f4914f6cdd1dn);
  }

  nextU32(): number {
    return Number(this.nextU64() >> 32n);
  }

  /** `length` little-endian bytes from consecutive nextU64 outputs (the Rust `fill`
   * over 8-byte chunks). */
  fill(length: number): Uint8Array {
    const out = new Uint8Array(length);
    for (let offset = 0; offset < length; offset += 8) {
      let word = this.nextU64();
      for (let i = 0; i < Math.min(8, length - offset); i += 1) {
        out[offset + i] = Number(word & 0xffn);
        word >>= 8n;
      }
    }
    return out;
  }
}

function randomName(rng: Rng): string {
  let name = "";
  for (let i = 0; i < 10; i += 1) {
    name += CHARSET[Number(rng.nextU64() % 64n)];
  }
  return name;
}

const ITERATIONS = 300;

describe("PRNG reference sequence", () => {
  it("matches the Rust implementation", () => {
    // The first outputs of seed 0x5EED_0001, computed by the Rust implementation. A
    // porting mistake in the PRNG would silently change every sequence below; this
    // pin makes it fail loudly instead.
    const rng = new Rng(0x5eed0001n);
    expect([rng.nextU64(), rng.nextU64(), rng.nextU64()]).toEqual([
      0xa2e8ec67270a2376n,
      0x44be5980bae0d9b9n,
      0x31076561bb9cc917n,
    ]);
  });
});

describe("Round-trip properties", () => {
  // Mirrors prop_app_agent_roundtrip / prop_transactional_roundtrip /
  // prop_named_roundtrip (Rust runs 1000 iterations over the same streams).

  it("round-trips random AppAgent ids", () => {
    const rng = new Rng(0x5eed0001n);
    const ids = Array.from({ length: ITERATIONS }, () => rng.nextU32());
    ids.push(0, 1, U32_MAX);
    ids.forEach((appAgentId) => {
      const address = encodeAppAgentAddress(appAgentId, ss58Format);
      const info = decodeAddress(address, ss58Format);
      expect(info.accountType).toEqual(AccountType.AppAgent);
      expect(info.appAgentId).toEqual(appAgentId);
      expect(info.taId).toBeNull();
      expect(info.accountName).toBeNull();
      expect(decodeAppAgentAddress(address, ss58Format)).toEqual(appAgentId);
      expect(() => decodeTransactionalAddress(address, ss58Format)).toThrow(
        AccountTypeMismatchError,
      );
      expect(() => decodeNamedAddress(address, ss58Format)).toThrow(
        AccountTypeMismatchError,
      );
      expect(isKeylessAddress(address, ss58Format)).toBe(true);
    });
  });

  it("round-trips random Transactional id pairs", () => {
    const rng = new Rng(0x5eed0002n);
    const ids = Array.from({ length: ITERATIONS }, () => [
      rng.nextU32(),
      rng.nextU32(),
    ]);
    ids.push([0, 0], [0, U32_MAX], [U32_MAX, 0], [U32_MAX, U32_MAX]);
    ids.forEach(([appAgentId, taId]) => {
      const address = encodeTransactionalAddress(appAgentId, taId, ss58Format);
      const info = decodeAddress(address, ss58Format);
      expect(info.accountType).toEqual(AccountType.Transactional);
      expect(info.appAgentId).toEqual(appAgentId);
      expect(info.taId).toEqual(taId);
      expect(decodeTransactionalAddress(address, ss58Format)).toEqual({
        appAgentId,
        taId,
      });
      expect(() => decodeAppAgentAddress(address, ss58Format)).toThrow(
        AccountTypeMismatchError,
      );
      expect(isKeylessAddress(address, ss58Format)).toBe(true);
    });
  });

  it("round-trips random Named addresses", () => {
    const rng = new Rng(0x5eed0003n);
    for (let i = 0; i < ITERATIONS; i += 1) {
      const appAgentId = rng.nextU32();
      const name = randomName(rng);
      const address = encodeNamedAddress(appAgentId, name, ss58Format);
      const info = decodeAddress(address, ss58Format);
      expect(info.accountType).toEqual(AccountType.Named);
      expect(info.appAgentId).toEqual(appAgentId);
      expect(info.accountName).toEqual(name);
      expect(decodeNamedAddress(address, ss58Format)).toEqual({
        appAgentId,
        accountName: name,
      });
      expect(() => decodeAppAgentAddress(address, ss58Format)).toThrow(
        AccountTypeMismatchError,
      );
      expect(isKeylessAddress(address, ss58Format)).toBe(true);
    }
  });
});

describe("Classification properties", () => {
  it("classifies arbitrary bytes as Regular without throwing", () => {
    // Mirrors prop_decode_arbitrary_bytes_never_panics_and_is_regular (Rust runs
    // 50,000 iterations over the same stream).
    const rng = new Rng(0x5eed0004n);
    for (let i = 0; i < 10_000; i += 1) {
      const account = rng.fill(32);
      // Cycle the discriminant byte through 0..=5 so every classification branch
      // (unknown discriminant and all three checksum verifications) is exercised.
      account[4] = i % 6;
      const info = decodeAccountId(`0x${bytesToHex(account)}`);
      // Random bytes must classify as Regular (a checksum match has probability
      // ~2^-136 at best).
      expect(info.accountType).toEqual(AccountType.Regular);
    }
  });

  it("demotes any single-byte corruption to Regular", () => {
    // Corrupting any single byte of a keyless account - open part or checksum - must
    // invalidate it. The construction (accounts, then one non-zero xor per position)
    // consumes the 0x5EED_0005 stream exactly like the Rust test.
    const rng = new Rng(0x5eed0005n);
    const addresses = [
      encodeAppAgentAddress(rng.nextU32(), ss58Format),
      encodeTransactionalAddress(rng.nextU32(), rng.nextU32(), ss58Format),
      encodeNamedAddress(rng.nextU32(), randomName(rng), ss58Format),
    ];
    addresses.forEach((address) => {
      const accountIdHex = decodeAddress(address, ss58Format).accountId.slice(
        2,
      );
      for (let position = 0; position < 32; position += 1) {
        const xor = Number(rng.nextU64() % 255n) + 1; // non-zero
        const corrupted = Uint8Array.from(
          accountIdHex.match(/../g)!.map((byte) => parseInt(byte, 16)),
        );
        corrupted[position] ^= xor;
        const info = decodeAccountId(`0x${bytesToHex(corrupted)}`);
        expect(info.accountType).toEqual(AccountType.Regular);
      }
    });
  });

  it("validates random address names", () => {
    // Mirrors prop_account_name_construction: every generated name encodes, and
    // replacing any character with an out-of-charset byte is rejected (Rust runs 500
    // iterations over the same stream).
    const rng = new Rng(0x5eed0006n);
    for (let i = 0; i < ITERATIONS; i += 1) {
      const name = randomName(rng);
      expect(name).toHaveLength(10);
      encodeNamedAddress(1, name, ss58Format);

      const position = Number(rng.nextU64() % 10n);
      const bad = `${name.slice(0, position)}@${name.slice(position + 1)}`;
      expect(() => encodeNamedAddress(1, bad, ss58Format)).toThrow(
        "Address name contains invalid characters",
      );
    }
  });
});

describe("SS58 properties", () => {
  it("round-trips random accounts across random formats", () => {
    // Mirrors prop_ss58_roundtrip_random_accounts_and_formats (Rust runs 500
    // iterations over the same stream).
    const rng = new Rng(0x5eed0007n);
    for (let i = 0; i < ITERATIONS; i += 1) {
      const accountId = `0x${bytesToHex(rng.fill(32))}`;
      let format = Number(rng.nextU64() % 16384n);
      if (format === 46 || format === 47) {
        format = 48; // skip the reserved formats
      }
      const address = encodeAddress(accountId, format);
      const info = decodeAddress(address, format);
      // The SS58 round trip must preserve the account and format.
      expect(info.accountId).toEqual(accountId);
    }
  });
});
