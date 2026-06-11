/**
 * Adversarial and boundary-value tests, mirroring the Rust test suite
 * (`tests_ss58.rs`, `tests_properties.rs`, `tests_encoding.rs`) and the Python
 * `test_adversarial.py`.
 *
 * The crafted inputs are built with the reference construction rules so that each test
 * documents exactly what is being fed to the decoder.
 */
import {
  base58Decode,
  base58Encode,
  blake2AsU8a,
  encodeAddress,
} from "@polkadot/util-crypto";
import { u8aConcat, u8aToHex } from "@polkadot/util";
import {
  AddressType,
  AddressTypeMismatchError,
  decodeAddress,
  decodeAppAgent,
  decodeNamed,
  decodeTransactional,
  encodeAppAgent,
  encodeNamed,
  encodeTransactional,
  isKeylessAddress,
  SS58_MAX_ADDRESS_LENGTH,
} from "../src/keyless";

const ss58Format = 5335;
const U32_MAX = 4294967295;

const SS58_PREFIX = Uint8Array.from("SS58PRE", (c) => c.charCodeAt(0));

/** First bytes of blake2b-512("SS58PRE" ++ payload) are the SS58 checksum. */
function ss58Checksum(payload: Uint8Array): Uint8Array {
  return blake2AsU8a(u8aConcat(SS58_PREFIX, payload), 512);
}

/** Wraps a raw prefix+data payload in an SS58 envelope with a valid checksum. */
function rawSs58(payload: Uint8Array, checksumLength = 2): string {
  return base58Encode(
    u8aConcat(payload, ss58Checksum(payload).slice(0, checksumLength)),
  );
}

/** The 1- or 2-byte SS58 prefix encoding of a format. */
function formatPrefix(format: number): Uint8Array {
  if (format < 64) {
    return Uint8Array.of(format);
  }
  /* eslint-disable no-bitwise */
  return Uint8Array.of(
    ((format & 0b0000_0000_1111_1100) >> 2) | 0b0100_0000,
    (format >> 8) | ((format & 0b0000_0000_0000_0011) << 6),
  );
  /* eslint-enable no-bitwise */
}

/** 32-byte keyless account id: open part + trailing bytes of its blake2b-256 hash. */
function keylessAccount(openPart: Uint8Array): Uint8Array {
  const checksum = blake2AsU8a(openPart);
  return u8aConcat(openPart, checksum.slice(openPart.length));
}

// The AppAgent(123) account used by the shared golden vectors.
const appAgent123 = keylessAccount(Uint8Array.of(123, 0, 0, 0, 1));

describe("Boundary values", () => {
  it("round-trips AppAgent ids 0, 1 and u32::MAX", () => {
    [0, 1, U32_MAX].forEach((appAgentId) => {
      const address = encodeAppAgent(appAgentId, ss58Format);
      expect(decodeAppAgent(address, ss58Format)).toEqual(appAgentId);
    });
  });

  it("round-trips Transactional id boundaries", () => {
    (
      [
        [0, 0],
        [0, U32_MAX],
        [U32_MAX, 0],
        [U32_MAX, U32_MAX],
      ] as [number, number][]
    ).forEach(([appAgentId, taId]) => {
      const address = encodeTransactional(appAgentId, taId, ss58Format);
      expect(decodeTransactional(address, ss58Format)).toEqual({
        appAgentId,
        taId,
      });
    });
  });

  it("round-trips Named boundary names", () => {
    ["0000000000", "##########", "----------", "ZZZZZZZZZZ"].forEach(
      (addressName) => {
        [0, U32_MAX].forEach((appAgentId) => {
          const address = encodeNamed(appAgentId, addressName, ss58Format);
          expect(decodeNamed(address, ss58Format)).toEqual({
            appAgentId,
            addressName,
          });
        });
      },
    );
  });

  it("rejects out-of-range ids at encoding time", () => {
    [-1, U32_MAX + 1, 1.5, NaN].forEach((badId) => {
      expect(() => encodeAppAgent(badId, ss58Format)).toThrow(RangeError);
      expect(() => encodeTransactional(1, badId, ss58Format)).toThrow(
        RangeError,
      );
      expect(() => encodeNamed(badId, "example123", ss58Format)).toThrow(
        RangeError,
      );
    });
  });

  it("rejects a fractional ss58Format at encoding time", () => {
    // encodeAddress range-checks the format but its bitwise prefix arithmetic would
    // silently truncate fractional values to an integer format.
    [5335.5, NaN].forEach((badFormat) => {
      expect(() => encodeAppAgent(123, badFormat)).toThrow(
        /ss58Format must be an integer/,
      );
    });
  });
});

describe("Account id length", () => {
  // Account ids must be exactly 32 bytes; the Rust AccountId32 makes this structural.

  it("rejects short SS58 payloads with valid envelopes", () => {
    // Valid SS58 envelopes (correct checksum, format 5335) around 1-, 4- and 8-byte
    // payloads: legal "account index" lengths in the generic SS58 scheme, but never a
    // 32-byte account id. Previously these decoded as Regular with a tiny account id.
    [
      Uint8Array.of(7),
      Uint8Array.of(0xde, 0xad, 0xbe, 0xef),
      new Uint8Array(8),
    ].forEach((payload) => {
      const address = rawSs58(u8aConcat(formatPrefix(ss58Format), payload), 1);
      expect(() => decodeAddress(address, ss58Format)).toThrow();
    });
  });

  it("rejects a 33-byte payload", () => {
    // A 33-byte payload (ECDSA-style public key) is legal generic SS58 but never a
    // 32-byte account id.
    const address = encodeAddress(
      u8aConcat(appAgent123, Uint8Array.of(0)),
      ss58Format,
    );
    expect(() => decodeAddress(address, ss58Format)).toThrow();
  });

  it("rejects a 31-byte payload", () => {
    // Not a legal SS58 length at all; built manually since encodeAddress refuses it.
    const address = rawSs58(
      u8aConcat(formatPrefix(ss58Format), appAgent123.slice(0, 31)),
    );
    expect(() => decodeAddress(address, ss58Format)).toThrow();
  });
});

describe("Malformed SS58 input", () => {
  it("rejects a tampered SS58 checksum", () => {
    const address = encodeAppAgent(123, ss58Format);
    const payload = base58Decode(address);
    /* eslint-disable no-bitwise */
    payload[payload.length - 1] ^= 0xff;
    /* eslint-enable no-bitwise */
    const tampered = base58Encode(payload);
    expect(() => decodeAddress(tampered, ss58Format)).toThrow(
      /Invalid decoded address checksum/,
    );
  });

  it("decodes a tampered keyless checksum as Regular", () => {
    // Corrupt a byte in the keyless checksum region (open part of an AppAgent account
    // is bytes [0..5]) and rewrap in a valid SS58 envelope: the SS58 layer accepts it,
    // the keyless layer must demote it to Regular.
    const account = Uint8Array.from(appAgent123);
    /* eslint-disable no-bitwise */
    account[10] ^= 0xff;
    /* eslint-enable no-bitwise */
    const address = encodeAddress(account, ss58Format);
    const info = decodeAddress(address, ss58Format);
    expect(info.addressType).toEqual(AddressType.Regular);
    expect(info.appAgentId).toBeNull();
  });

  it("rejects the reserved format 46, crafted with a valid checksum", () => {
    const address = rawSs58(u8aConcat(formatPrefix(46), appAgent123));
    expect(() => decodeAddress(address, 46)).toThrow();
  });

  it("rejects two-byte-prefix encodings of the reserved formats 46 and 47", () => {
    // The underlying @polkadot decoder accepts these; the Rust and Python
    // implementations reject reserved formats in any prefix encoding.
    /* eslint-disable no-bitwise */
    [46, 47].forEach((reserved) => {
      const prefix = Uint8Array.of(
        ((reserved & 0b0000_0000_1111_1100) >> 2) | 0b0100_0000,
        (reserved >> 8) | ((reserved & 0b0000_0000_0000_0011) << 6),
      );
      const address = rawSs58(u8aConcat(prefix, appAgent123));
      expect(() => decodeAddress(address, reserved)).toThrow(
        new Error(`${reserved} is a reserved SS58 format`),
      );
    });
    /* eslint-enable no-bitwise */
  });

  it("rejects a one-byte-checksum spelling of a canonical address", () => {
    // The underlying @polkadot decoder accepts a 35-byte decoded payload (two-byte
    // prefix + 32-byte account + one-byte checksum), which would give every account a
    // second, shorter address string; the Rust and Python implementations reject it.
    const address = rawSs58(
      u8aConcat(formatPrefix(ss58Format), appAgent123),
      1,
    );
    expect(() => decodeAddress(address, ss58Format)).toThrow(
      new Error("Invalid address length"),
    );
  });

  it("rejects a prefix byte in the reserved range 128..=255", () => {
    // Per the SS58 spec the first byte must be 0..=63 or 64..=127.
    const address = rawSs58(u8aConcat(Uint8Array.of(0x80), appAgent123));
    expect(() => decodeAddress(address, 128)).toThrow();
  });

  it("accepts a non-canonical two-byte prefix of a small format", () => {
    // The two-byte SS58 prefix can also (non-canonically) encode formats below 64.
    // Like the Rust and upstream Substrate codecs, decoding accepts such addresses.
    const smallFormat = 42;
    /* eslint-disable no-bitwise */
    const nonCanonicalPrefix = Uint8Array.of(
      ((smallFormat & 0b0000_0000_1111_1100) >> 2) | 0b0100_0000,
      (smallFormat >> 8) | ((smallFormat & 0b0000_0000_0000_0011) << 6),
    );
    /* eslint-enable no-bitwise */
    const address = rawSs58(u8aConcat(nonCanonicalPrefix, appAgent123));
    const info = decodeAddress(address, smallFormat);
    expect(info.addressType).toEqual(AddressType.AppAgent);
    expect(info.appAgentId).toEqual(123);
  });

  it("rejects garbage input", () => {
    ["", " ", "   ", "0OIl", "x", "1"].forEach((garbage) => {
      expect(() => decodeAddress(garbage, ss58Format)).toThrow();
    });
  });

  it("rejects oversized input before base58 decoding", () => {
    const oversized = "1".repeat(SS58_MAX_ADDRESS_LENGTH + 1);
    expect(() => decodeAddress(oversized, ss58Format)).toThrow(
      new Error("Invalid address length"),
    );
  });

  it("still decodes the longest legitimate address", () => {
    // The longest legitimate addresses (two-byte prefix, high account id bytes) stay
    // within the bound and must still decode.
    const maxFormat = 16383;
    const address = encodeAddress(new Uint8Array(32).fill(0xff), maxFormat);
    expect(address.length).toBeLessThanOrEqual(SS58_MAX_ADDRESS_LENGTH);
    const info = decodeAddress(address, maxFormat);
    expect(info.addressType).toEqual(AddressType.Regular);
  });
});

describe("Hex account id contract", () => {
  // A raw account id ("0x" + 64 hex chars) is accepted and validated; anything else
  // hex-like is rejected.

  it("classifies a valid hex account id", () => {
    const info = decodeAddress(u8aToHex(appAgent123), ss58Format);
    expect(info.addressType).toEqual(AddressType.AppAgent);
    expect(info.appAgentId).toEqual(123);
    expect(info.accountId).toEqual(u8aToHex(appAgent123));
  });

  it("normalizes uppercase hex", () => {
    const upper = `0x${u8aToHex(appAgent123).slice(2).toUpperCase()}`;
    const info = decodeAddress(upper, ss58Format);
    expect(info.addressType).toEqual(AddressType.AppAgent);
    expect(info.accountId).toEqual(u8aToHex(appAgent123));
  });

  it("rejects hex of the wrong length", () => {
    ["0x", "0xdeadbeef", `${u8aToHex(appAgent123)}00`].forEach((badHex) => {
      expect(() => decodeAddress(badHex, ss58Format)).toThrow(
        /Invalid hex account id/,
      );
    });
  });

  it("rejects invalid hex", () => {
    ["0xabc", `0xzz${"00".repeat(31)}`].forEach((badHex) => {
      expect(() => decodeAddress(badHex, ss58Format)).toThrow(
        /Invalid hex account id/,
      );
    });
  });
});

describe("isKeylessAddress", () => {
  // Mirrors the Rust `test_is_keyless_account` test.

  it("returns true for keyless addresses", () => {
    [
      encodeAppAgent(1, ss58Format),
      encodeTransactional(1, 2, ss58Format),
      encodeNamed(1, "test123456", ss58Format),
    ].forEach((address) => {
      expect(isKeylessAddress(address, ss58Format)).toBe(true);
    });
  });

  it("returns false for regular addresses", () => {
    // Alice's well-known development account.
    const alice = "ttqxHzRJmmjFBcE7Lb5Xs4GNMq2gFSt28JyvTEqjhqzE9EGP4";
    expect(isKeylessAddress(alice, ss58Format)).toBe(false);
    // A zeroed account has no valid keyless discriminant.
    expect(isKeylessAddress(`0x${"00".repeat(32)}`, ss58Format)).toBe(false);
  });
});

describe("AddressTypeMismatchError", () => {
  it("carries the expected and found address types", () => {
    const namedAddress = encodeNamed(1, "test123456", ss58Format);

    let thrown: unknown;
    try {
      decodeAppAgent(namedAddress, ss58Format);
    } catch (error) {
      thrown = error;
    }

    expect(thrown).toBeInstanceOf(AddressTypeMismatchError);
    const mismatch = thrown as AddressTypeMismatchError;
    expect(mismatch.expected).toEqual(AddressType.AppAgent);
    expect(mismatch.found).toEqual(AddressType.Named);
    // Backwards compatibility: the message is unchanged from previous releases.
    expect(mismatch.message).toEqual(
      "Provided address is not an AppAgent address but is instance of Named",
    );
  });
});
