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
import type { AccountName } from "../src/keyless";
import {
  AccountType,
  AccountTypeMismatchError,
  decodeAccountId,
  decodeAddress,
  decodeAddressWithFormat,
  decodeAppAgentAccountId,
  decodeAppAgentAddress,
  decodeNamedAccountId,
  decodeNamedAddress,
  decodeTransactionalAccountId,
  decodeTransactionalAddress,
  encodeAddress,
  encodeAppAgentAddress,
  encodeNamedAddress,
  encodeTransactionalAddress,
  isKeylessAddress,
  accountName as asAccountName,
  appAgentId as asAppAgentId,
  transactionalId as asTransactionalId,
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
    // `% 64` takes xorshift64*'s statistically weakest low bits — deliberately kept
    // identical across the Rust/Python/TS property suites and the fixture generator;
    // changing it would desynchronize the shared PRNG streams.
    name += CHARSET[Number(rng.nextU64() % 64n)];
  }
  return name;
}

const ITERATIONS = 300;

/**
 * Asserts that `run` throws an `AccountTypeMismatchError` carrying exactly the given
 * (expected, found) pair — the cross-language contract pinned by the `type_mismatch`
 * fixture group. Asserting only the class would let a decoder report the wrong pair
 * (the Python property suite checks the pair too).
 */
function expectTypeMismatch(
  run: () => unknown,
  expected: AccountType,
  found: AccountType,
): void {
  let thrown: unknown;
  try {
    run();
  } catch (error) {
    thrown = error;
  }
  expect(thrown).toBeInstanceOf(AccountTypeMismatchError);
  expect((thrown as AccountTypeMismatchError).expected).toEqual(expected);
  expect((thrown as AccountTypeMismatchError).found).toEqual(found);
}

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
    ids.forEach((rawAppAgentId) => {
      const appAgentId = asAppAgentId(rawAppAgentId);
      const address = encodeAppAgentAddress({
        appAgentId: appAgentId,
        ss58Format: ss58Format,
      });
      const info = decodeAddress(address, ss58Format);
      expect(info.accountType).toEqual(AccountType.AppAgent);
      if (info.accountType !== AccountType.AppAgent) {
        throw new Error("Expected an AppAgent account-info variant");
      }
      expect(info.appAgentId).toEqual(appAgentId);
      expect(decodeAppAgentAddress(address, ss58Format)).toEqual(appAgentId);
      expectTypeMismatch(
        () => decodeTransactionalAddress(address, ss58Format),
        AccountType.Transactional,
        AccountType.AppAgent,
      );
      expectTypeMismatch(
        () => decodeNamedAddress(address, ss58Format),
        AccountType.Named,
        AccountType.AppAgent,
      );
      expect(isKeylessAddress(address, ss58Format)).toBe(true);
      // The account-id decoder agrees with the address decoder (the Python property
      // suite pins the same agreement over this stream).
      expect(decodeAppAgentAccountId(info.accountId)).toEqual(appAgentId);
    });
  });

  it("round-trips random Transactional id pairs", () => {
    const rng = new Rng(0x5eed0002n);
    const ids = Array.from({ length: ITERATIONS }, (): [number, number] => [
      rng.nextU32(),
      rng.nextU32(),
    ]);
    ids.push([0, 0], [0, U32_MAX], [U32_MAX, 0], [U32_MAX, U32_MAX]);
    ids.forEach(([rawAppAgentId, rawTaId]) => {
      const appAgentId = asAppAgentId(rawAppAgentId);
      const taId = asTransactionalId(rawTaId);
      const address = encodeTransactionalAddress({
        appAgentId,
        taId,
        ss58Format,
      });
      const info = decodeAddress(address, ss58Format);
      expect(info.accountType).toEqual(AccountType.Transactional);
      if (info.accountType !== AccountType.Transactional) {
        throw new Error("Expected a Transactional account-info variant");
      }
      expect(info.appAgentId).toEqual(appAgentId);
      expect(info.taId).toEqual(taId);
      expect(decodeTransactionalAddress(address, ss58Format)).toEqual({
        appAgentId,
        taId,
      });
      expectTypeMismatch(
        () => decodeAppAgentAddress(address, ss58Format),
        AccountType.AppAgent,
        AccountType.Transactional,
      );
      expectTypeMismatch(
        () => decodeNamedAddress(address, ss58Format),
        AccountType.Named,
        AccountType.Transactional,
      );
      expect(isKeylessAddress(address, ss58Format)).toBe(true);
      expect(decodeTransactionalAccountId(info.accountId)).toEqual({
        appAgentId,
        taId,
      });
    });
  });

  it("round-trips random Named addresses", () => {
    const rng = new Rng(0x5eed0003n);
    for (let i = 0; i < ITERATIONS; i += 1) {
      const appAgentId = asAppAgentId(rng.nextU32());
      const name = randomName(rng);
      const address = encodeNamedAddress({
        appAgentId,
        accountName: asAccountName(name),
        ss58Format,
      });
      const info = decodeAddress(address, ss58Format);
      expect(info.accountType).toEqual(AccountType.Named);
      if (info.accountType !== AccountType.Named) {
        throw new Error("Expected a Named account-info variant");
      }
      expect(info.appAgentId).toEqual(appAgentId);
      expect(info.accountName).toEqual(name);
      expect(decodeNamedAddress(address, ss58Format)).toEqual({
        appAgentId,
        accountName: name,
      });
      expectTypeMismatch(
        () => decodeAppAgentAddress(address, ss58Format),
        AccountType.AppAgent,
        AccountType.Named,
      );
      expectTypeMismatch(
        () => decodeTransactionalAddress(address, ss58Format),
        AccountType.Transactional,
        AccountType.Named,
      );
      expect(isKeylessAddress(address, ss58Format)).toBe(true);
      expect(decodeNamedAccountId(info.accountId)).toEqual({
        appAgentId,
        accountName: name,
      });
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
      const accountId = `0x${bytesToHex(account)}`;
      const info = decodeAccountId(accountId);
      // Random bytes must classify as Regular (a checksum match has probability
      // ~2^-136 at best). The whole info object is pinned, not only the type: a Regular
      // classification must not leak a stale identifier from a partially matched
      // keyless layout.
      expect(info).toEqual({
        address: null,
        accountId,
        accountType: AccountType.Regular,
      });
    }
  });

  it("demotes any single-byte corruption to Regular", () => {
    // Corrupting any single byte of a keyless account - open part or checksum - must
    // invalidate it. The construction (accounts, then one non-zero xor per position)
    // consumes the 0x5EED_0005 stream exactly like the Rust test.
    const rng = new Rng(0x5eed0005n);
    const addresses = [
      encodeAppAgentAddress({
        appAgentId: asAppAgentId(rng.nextU32()),
        ss58Format: ss58Format,
      }),
      encodeTransactionalAddress({
        appAgentId: asAppAgentId(rng.nextU32()),
        taId: asTransactionalId(rng.nextU32()),
        ss58Format,
      }),
      encodeNamedAddress({
        appAgentId: asAppAgentId(rng.nextU32()),
        accountName: asAccountName(randomName(rng)),
        ss58Format,
      }),
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
        corrupted[position] = (corrupted[position] as number) ^ xor;
        const corruptedId = `0x${bytesToHex(corrupted)}`;
        const info = decodeAccountId(corruptedId);
        // The demoted value carries no identifiers of the keyless layout it came from.
        expect(info).toEqual({
          address: null,
          accountId: corruptedId,
          accountType: AccountType.Regular,
        });
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
      encodeNamedAddress({
        appAgentId: asAppAgentId(1),
        accountName: asAccountName(name),
        ss58Format,
      });

      const position = Number(rng.nextU64() % 10n);
      const bad = `${name.slice(0, position)}@${name.slice(position + 1)}`;
      expect(() =>
        encodeNamedAddress({
          appAgentId: asAppAgentId(1),
          // Deliberately out of charset: the cast simulates an untyped JavaScript caller,
          // so the encoder is what rejects the name.
          accountName: bad as AccountName,
          ss58Format,
        }),
      ).toThrow("Account name contains invalid characters");
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
      // The SS58 round trip must preserve the account *and* the format: re-reading the
      // address without an expected format must report the format it was encoded with
      // (the Rust property checks the same (account, format) pair).
      expect(info.accountId).toEqual(accountId);
      expect(decodeAddressWithFormat(address)).toEqual({
        info,
        ss58Format: format,
      });
    }
  });
});
