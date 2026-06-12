/**
 * Adversarial and boundary-value tests, mirroring the Rust test suite
 * (`tests_ss58.rs`, `tests_properties.rs`, `tests_encoding.rs`) and the Python
 * `test_adversarial.py`.
 *
 * The crafted inputs are built with the reference construction rules so that each test
 * documents exactly what is being fed to the decoder.
 */
import { blake2b } from "@noble/hashes/blake2b";
import { bytesToHex, concatBytes } from "@noble/hashes/utils";
import { base58 } from "@scure/base";
import { ss58Encode } from "../src/ss58";
import {
  AddressType,
  AddressTypeMismatchError,
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
  Ss58Error,
  SS58_MAX_ADDRESS_LENGTH,
} from "../src/keyless";

const ss58Format = 5335;
const U32_MAX = 4294967295;

const SS58_PREFIX = Uint8Array.from("SS58PRE", (c) => c.charCodeAt(0));

/** "0x"-prefixed lowercase hex of the given bytes. */
function u8aToHex(bytes: Uint8Array): string {
  return `0x${bytesToHex(bytes)}`;
}

/** First bytes of blake2b-512("SS58PRE" ++ payload) are the SS58 checksum. */
function ss58Checksum(payload: Uint8Array): Uint8Array {
  return blake2b(concatBytes(SS58_PREFIX, payload), { dkLen: 64 });
}

/** Wraps a raw prefix+data payload in an SS58 envelope with a valid checksum. */
function rawSs58(payload: Uint8Array, checksumLength = 2): string {
  return base58.encode(
    concatBytes(payload, ss58Checksum(payload).slice(0, checksumLength)),
  );
}

/** The 1- or 2-byte SS58 prefix encoding of a format. */
function formatPrefix(format: number): Uint8Array {
  if (format < 64) {
    return Uint8Array.of(format);
  }
  return Uint8Array.of(
    ((format & 0b0000_0000_1111_1100) >> 2) | 0b0100_0000,
    (format >> 8) | ((format & 0b0000_0000_0000_0011) << 6),
  );
}

/** 32-byte keyless account id: open part + trailing bytes of its blake2b-256 hash. */
function keylessAccount(openPart: Uint8Array): Uint8Array {
  const checksum = blake2b(openPart, { dkLen: 32 });
  return concatBytes(openPart, checksum.slice(openPart.length));
}

// The AppAgent(123) account used by the shared golden vectors.
const appAgent123 = keylessAccount(Uint8Array.of(123, 0, 0, 0, 1));

describe("Boundary values", () => {
  it("round-trips AppAgent ids 0, 1 and u32::MAX", () => {
    [0, 1, U32_MAX].forEach((appAgentId) => {
      const address = encodeAppAgentAddress(appAgentId, ss58Format);
      expect(decodeAppAgentAddress(address, ss58Format)).toEqual(appAgentId);
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
      const address = encodeTransactionalAddress(appAgentId, taId, ss58Format);
      expect(decodeTransactionalAddress(address, ss58Format)).toEqual({
        appAgentId,
        taId,
      });
    });
  });

  it("round-trips Named boundary names", () => {
    ["0000000000", "##########", "----------", "ZZZZZZZZZZ"].forEach(
      (addressName) => {
        [0, U32_MAX].forEach((appAgentId) => {
          const address = encodeNamedAddress(
            appAgentId,
            addressName,
            ss58Format,
          );
          expect(decodeNamedAddress(address, ss58Format)).toEqual({
            appAgentId,
            addressName,
          });
        });
      },
    );
  });

  it("rejects out-of-range ids at encoding time", () => {
    [-1, U32_MAX + 1, 1.5, NaN].forEach((badId) => {
      expect(() => encodeAppAgentAddress(badId, ss58Format)).toThrow(
        RangeError,
      );
      expect(() => encodeTransactionalAddress(1, badId, ss58Format)).toThrow(
        RangeError,
      );
      expect(() => encodeNamedAddress(badId, "example123", ss58Format)).toThrow(
        RangeError,
      );
    });
  });

  it("rejects a fractional ss58Format at encoding time", () => {
    // The SS58 layer range-checks the format but its bitwise prefix arithmetic would
    // silently truncate fractional values to an integer format.
    [5335.5, NaN].forEach((badFormat) => {
      expect(() => encodeAppAgentAddress(123, badFormat)).toThrow(
        /ss58Format must be an integer/,
      );
    });
  });
});

describe("ss58Format validation", () => {
  // The ss58Format parameter must be an integer in [0, 16383], mirroring the u16-typed
  // format of the Rust implementation and the runtime check of the Python
  // implementation.

  it("rejects out-of-range and non-integer formats at decoding time", () => {
    const address = encodeAppAgentAddress(123, ss58Format);
    [-1, 16384, 1.5, NaN].forEach((badFormat) => {
      expect(() => decodeAddress(address, badFormat)).toThrow(
        /ss58Format must be an integer in \[0, 16383\]/,
      );
    });
  });

  it("rejects -1 instead of treating it as an any-format sentinel", () => {
    // Some Substrate decoders (e.g. @polkadot) interpret -1 as "accept any format";
    // here it must be rejected like any other out-of-range value, so an address of a
    // *different* format is never silently accepted.
    const substrateGenericAddress = encodeAppAgentAddress(123, 42);
    expect(() => decodeAddress(substrateGenericAddress, -1)).toThrow(
      /ss58Format must be an integer in \[0, 16383\]/,
    );
  });

  it("rejects raw hex input in decodeAddress", () => {
    // SS58 addresses and raw account ids are separate input classes: a raw account id
    // carries no SS58 format, so accepting it here would silently bypass the
    // expected-format check. decodeAccountId (which takes no format) handles them.
    expect(() => decodeAddress(u8aToHex(appAgent123), ss58Format)).toThrow(
      new Error(
        "Raw account id hex is not an SS58 address; use decodeAccountId",
      ),
    );
    expect(() => isKeylessAddress(u8aToHex(appAgent123), ss58Format)).toThrow(
      /use decodeAccountId/,
    );
  });

  it("rejects out-of-range formats at encoding time", () => {
    [-1, 16384].forEach((badFormat) => {
      expect(() => encodeAppAgentAddress(123, badFormat)).toThrow(
        /ss58Format must be an integer in \[0, 16383\]/,
      );
    });
  });

  it("accepts the format boundaries 0 and 16383", () => {
    [0, 16383].forEach((goodFormat) => {
      const address = encodeAppAgentAddress(123, goodFormat);
      expect(decodeAppAgentAddress(address, goodFormat)).toEqual(123);
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
      const address = rawSs58(
        concatBytes(formatPrefix(ss58Format), payload),
        1,
      );
      expect(() => decodeAddress(address, ss58Format)).toThrow();
    });
  });

  it("rejects a 33-byte payload", () => {
    // A 33-byte payload (ECDSA-style public key) is legal generic SS58 but never a
    // 32-byte account id.
    const address = rawSs58(
      concatBytes(formatPrefix(ss58Format), appAgent123, Uint8Array.of(0)),
    );
    expect(() => decodeAddress(address, ss58Format)).toThrow();
  });

  it("rejects a 31-byte payload", () => {
    // Not a legal SS58 length at all; built manually since ss58Encode refuses it.
    const address = rawSs58(
      concatBytes(formatPrefix(ss58Format), appAgent123.slice(0, 31)),
    );
    expect(() => decodeAddress(address, ss58Format)).toThrow();
  });
});

describe("Malformed SS58 input", () => {
  it("rejects a tampered SS58 checksum", () => {
    const address = encodeAppAgentAddress(123, ss58Format);
    const payload = base58.decode(address);
    payload[payload.length - 1] ^= 0xff;
    const tampered = base58.encode(payload);
    expect(() => decodeAddress(tampered, ss58Format)).toThrow(
      new Error("Address has an invalid checksum"),
    );
  });

  it("decodes a tampered keyless checksum as Regular", () => {
    // Corrupt a byte in the keyless checksum region (open part of an AppAgent account
    // is bytes [0..5]) and rewrap in a valid SS58 envelope: the SS58 layer accepts it,
    // the keyless layer must demote it to Regular.
    const account = Uint8Array.from(appAgent123);
    account[10] ^= 0xff;
    const address = ss58Encode(account, ss58Format);
    const info = decodeAddress(address, ss58Format);
    expect(info.addressType).toEqual(AddressType.Regular);
    expect(info.appAgentId).toBeNull();
  });

  it("rejects the reserved format 46, crafted with a valid checksum", () => {
    const address = rawSs58(concatBytes(formatPrefix(46), appAgent123));
    expect(() => decodeAddress(address, 46)).toThrow();
  });

  it("rejects two-byte-prefix encodings of the reserved formats 46 and 47", () => {
    // Permissive Substrate decoders (e.g. @polkadot) accept these; like the Rust and
    // Python implementations, reserved formats are rejected in any prefix encoding.
    [46, 47].forEach((reserved) => {
      const prefix = Uint8Array.of(
        ((reserved & 0b0000_0000_1111_1100) >> 2) | 0b0100_0000,
        (reserved >> 8) | ((reserved & 0b0000_0000_0000_0011) << 6),
      );
      const address = rawSs58(concatBytes(prefix, appAgent123));
      expect(() => decodeAddress(address, reserved)).toThrow(
        new Error("SS58 format is invalid or reserved"),
      );
    });
  });

  it("rejects a one-byte-checksum spelling of a canonical address", () => {
    // Permissive Substrate decoders accept a 35-byte decoded payload (two-byte prefix
    // + 32-byte account + one-byte checksum), which would give every account a second,
    // shorter address string; like Rust and Python, only the full two-byte checksum
    // structure decodes.
    const address = rawSs58(
      concatBytes(formatPrefix(ss58Format), appAgent123),
      1,
    );
    expect(() => decodeAddress(address, ss58Format)).toThrow(
      new Error("Address has an invalid length"),
    );
  });

  it("rejects a prefix byte in the reserved range 128..=255", () => {
    // Per the SS58 spec the first byte must be 0..=63 or 64..=127.
    const address = rawSs58(concatBytes(Uint8Array.of(0x80), appAgent123));
    expect(() => decodeAddress(address, 128)).toThrow();
  });

  it("accepts a non-canonical two-byte prefix of a small format", () => {
    // The two-byte SS58 prefix can also (non-canonically) encode formats below 64.
    // Like the Rust and upstream Substrate codecs, decoding accepts such addresses.
    const smallFormat = 42;
    const nonCanonicalPrefix = Uint8Array.of(
      ((smallFormat & 0b0000_0000_1111_1100) >> 2) | 0b0100_0000,
      (smallFormat >> 8) | ((smallFormat & 0b0000_0000_0000_0011) << 6),
    );
    const address = rawSs58(concatBytes(nonCanonicalPrefix, appAgent123));
    const info = decodeAddress(address, smallFormat);
    expect(info.addressType).toEqual(AddressType.AppAgent);
    expect(info.appAgentId).toEqual(123);
  });

  it("rejects garbage input", () => {
    ["", " ", "   ", "0OIl", "x", "1"].forEach((garbage) => {
      expect(() => decodeAddress(garbage, ss58Format)).toThrow();
    });
  });

  it("rejects surrounding whitespace", () => {
    // An address with surrounding whitespace is not the canonical spelling of the
    // account; whitespace is outside the base58 alphabet (pinned cross-language by the
    // ss58_invalid conformance vectors).
    const address = encodeAppAgentAddress(123, ss58Format);
    [`${address} `, ` ${address}`, `${address}\n`, `${address}\t`].forEach(
      (spelling) => {
        expect(() => decodeAddress(spelling, ss58Format)).toThrow();
      },
    );
  });

  it("rejects oversized input before base58 decoding", () => {
    const oversized = "1".repeat(SS58_MAX_ADDRESS_LENGTH + 1);
    expect(() => decodeAddress(oversized, ss58Format)).toThrow(
      new Error("Address has an invalid length"),
    );
  });

  it("still decodes the longest legitimate address", () => {
    // The longest legitimate addresses (two-byte prefix, high account id bytes) stay
    // within the bound and must still decode.
    const maxFormat = 16383;
    const address = ss58Encode(new Uint8Array(32).fill(0xff), maxFormat);
    expect(address.length).toBeLessThanOrEqual(SS58_MAX_ADDRESS_LENGTH);
    const info = decodeAddress(address, maxFormat);
    expect(info.addressType).toEqual(AddressType.Regular);
  });
});

describe("Hex account id contract", () => {
  // decodeAccountId accepts a raw account id ("0x" + exactly 64 hex chars); anything
  // else is rejected. decodeAddress accepts SS58 addresses only.

  it("classifies a valid hex account id", () => {
    const info = decodeAccountId(u8aToHex(appAgent123));
    expect(info.addressType).toEqual(AddressType.AppAgent);
    expect(info.appAgentId).toEqual(123);
    expect(info.accountId).toEqual(u8aToHex(appAgent123));
    // Raw account bytes carry no address; encodeAddress renders one explicitly.
    expect(info.address).toBeNull();
  });

  it("normalizes uppercase hex", () => {
    const upper = `0x${u8aToHex(appAgent123).slice(2).toUpperCase()}`;
    const info = decodeAccountId(upper);
    expect(info.addressType).toEqual(AddressType.AppAgent);
    expect(info.accountId).toEqual(u8aToHex(appAgent123));
    expect(info.address).toBeNull();
  });

  it("rejects hex of the wrong length", () => {
    ["0x", "0xdeadbeef", `${u8aToHex(appAgent123)}00`, "0xabc"].forEach(
      (badHex) => {
        expect(() => decodeAccountId(badHex)).toThrow(/Invalid hex account id/);
      },
    );
  });

  it("rejects invalid hex", () => {
    // Wrong charset, an uppercase "0X" prefix, and a missing prefix are all rejected.
    [
      `0xzz${"00".repeat(31)}`,
      `0X${u8aToHex(appAgent123).slice(2)}`,
      u8aToHex(appAgent123).slice(2),
    ].forEach((badHex) => {
      expect(() => decodeAccountId(badHex)).toThrow(/Invalid hex account id/);
    });
  });
});

describe("encodeAddress", () => {
  // encodeAddress renders a raw account id ("0x" + exactly 64 hex chars) as its
  // canonical SS58 address, mirroring AccountId32::to_ss58check_with_version.

  it("round-trips with decodeAddress", () => {
    const hexAccount = u8aToHex(appAgent123);
    const address = encodeAddress(hexAccount, ss58Format);
    expect(address).toEqual(encodeAppAgentAddress(123, ss58Format));
    expect(decodeAddress(address, ss58Format).accountId).toEqual(hexAccount);
  });

  it("defaults to the Trevo format", () => {
    expect(encodeAddress(u8aToHex(appAgent123))).toEqual(
      encodeAppAgentAddress(123),
    );
  });

  it("rejects invalid hex", () => {
    const clean = u8aToHex(appAgent123).slice(2);
    [
      "",
      "0x",
      `0x${clean.slice(0, 62)}`,
      `0x${clean}00`,
      `0X${clean}`,
      clean,
      `0x${clean.slice(0, 62)}zz`,
    ].forEach((badHex) => {
      expect(() => encodeAddress(badHex)).toThrow(/Invalid hex account id/);
    });
  });

  it("rejects invalid and reserved formats", () => {
    const hexAccount = u8aToHex(appAgent123);
    [-1, 16384, 5335.5, NaN].forEach((badFormat) => {
      expect(() => encodeAddress(hexAccount, badFormat)).toThrow(
        /ss58Format must be an integer/,
      );
    });
    [46, 47].forEach((reserved) => {
      let thrown: unknown;
      try {
        encodeAddress(hexAccount, reserved);
      } catch (e) {
        thrown = e;
      }
      expect(thrown).toBeInstanceOf(Ss58Error);
      expect((thrown as Ss58Error).kind).toEqual("InvalidFormat");
    });
  });
});

describe("isKeylessAddress", () => {
  // Mirrors the Rust `test_is_keyless_account` test.

  it("returns true for keyless addresses", () => {
    [
      encodeAppAgentAddress(1, ss58Format),
      encodeTransactionalAddress(1, 2, ss58Format),
      encodeNamedAddress(1, "test123456", ss58Format),
    ].forEach((address) => {
      expect(isKeylessAddress(address, ss58Format)).toBe(true);
    });
  });

  it("returns false for regular addresses", () => {
    // Alice's well-known development account.
    const alice = "ttqxHzRJmmjFBcE7Lb5Xs4GNMq2gFSt28JyvTEqjhqzE9EGP4";
    expect(isKeylessAddress(alice, ss58Format)).toBe(false);
    // A zeroed account has no valid keyless discriminant; raw account ids are
    // classified via decodeAccountId.
    const info = decodeAccountId(`0x${"00".repeat(32)}`);
    expect(info.addressType).toEqual(AddressType.Regular);
  });
});

describe("Ss58Error", () => {
  // The SS58 errors mirror the Rust Ss58Error enum: `kind` is the variant name and the
  // message matches the Rust Display output. The wrong-format error carries the
  // expected and the found format (the Python Ss58FormatMismatchError reports the same
  // pair).

  it("carries the expected and found formats on a format mismatch", () => {
    const substrateGeneric = encodeAppAgentAddress(123, 42);

    let thrown: unknown;
    try {
      decodeAddress(substrateGeneric, ss58Format);
    } catch (error) {
      thrown = error;
    }

    expect(thrown).toBeInstanceOf(Ss58Error);
    const mismatch = thrown as Ss58Error;
    expect(mismatch.kind).toEqual("UnexpectedFormat");
    expect(mismatch.expected).toEqual(ss58Format);
    expect(mismatch.found).toEqual(42);
    expect(mismatch.message).toEqual("Expected SS58 format 5335 but found 42");
  });

  it("classifies the failure via kind", () => {
    const cases: [string, string][] = [
      ["0OIl", "InvalidBase58"],
      ["1".repeat(SS58_MAX_ADDRESS_LENGTH + 1), "InvalidLength"],
      [rawSs58(concatBytes(formatPrefix(46), appAgent123)), "InvalidFormat"],
    ];
    cases.forEach(([input, expectedKind]) => {
      let thrown: unknown;
      try {
        decodeAddress(input, ss58Format);
      } catch (error) {
        thrown = error;
      }
      expect(thrown).toBeInstanceOf(Ss58Error);
      expect((thrown as Ss58Error).kind).toEqual(expectedKind);
    });
  });
});

describe("AddressTypeMismatchError", () => {
  it("carries the expected and found address types", () => {
    const namedAddress = encodeNamedAddress(1, "test123456", ss58Format);

    let thrown: unknown;
    try {
      decodeAppAgentAddress(namedAddress, ss58Format);
    } catch (error) {
      thrown = error;
    }

    expect(thrown).toBeInstanceOf(AddressTypeMismatchError);
    const mismatch = thrown as AddressTypeMismatchError;
    expect(mismatch.expected).toEqual(AddressType.AppAgent);
    expect(mismatch.found).toEqual(AddressType.Named);
    // The message is identical across the Python and TypeScript implementations.
    expect(mismatch.message).toEqual(
      "Provided address is not an AppAgent address but is instance of Named",
    );
  });
});
