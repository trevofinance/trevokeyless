/**
 * Adversarial and boundary-value tests, mirroring the Rust test suite
 * (`tests_ss58.rs`, `tests_properties.rs`, `tests_encoding.rs`) and the Python
 * `test_adversarial.py`.
 *
 * The crafted inputs are built with the reference construction rules so that each test
 * documents exactly what is being fed to the decoder.
 */
import { blake2b } from "@noble/hashes/blake2.js";
import { bytesToHex, concatBytes } from "@noble/hashes/utils.js";
import { base58 } from "@scure/base";
import { ss58Encode } from "../src/ss58";
import type { AccountName, AppAgentId, TransactionalId } from "../src/keyless";
import {
  accountName as asAccountName,
  AccountNameError,
  AccountType,
  AccountTypeMismatchError,
  appAgentId as asAppAgentId,
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
  encodeAppAgentAccountId,
  encodeAppAgentAddress,
  encodeNamedAccountId,
  encodeNamedAddress,
  encodeTransactionalAccountId,
  encodeTransactionalAddress,
  isKeylessAccountId,
  isKeylessAddress,
  NotAnAddressError,
  ParseAccountIdError,
  Ss58Error,
  Ss58FormatMismatchError,
  SS58_MAX_ADDRESS_LENGTH,
  transactionalId as asTransactionalId,
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

/**
 * Asserts that `run` throws an `Ss58Error` of exactly the given kind (and, when given,
 * message). The kind is the cross-language error taxonomy: a bare `.toThrow()` would
 * also pass on e.g. a `RangeError` leaking out of parameter validation, which is exactly
 * the drift the taxonomy exists to prevent. The Python twins assert `.kind` the same way.
 */
function expectSs58Error(
  run: () => unknown,
  kind: string,
  message?: string,
): void {
  let thrown: unknown;
  try {
    run();
  } catch (error) {
    thrown = error;
  }
  expect(thrown).toBeInstanceOf(Ss58Error);
  expect((thrown as Ss58Error).kind).toEqual(kind);
  if (message !== undefined) {
    expect((thrown as Ss58Error).message).toEqual(message);
  }
}

describe("Boundary values", () => {
  it("round-trips AppAgent ids 0, 1 and u32::MAX", () => {
    [0, 1, U32_MAX].forEach((appAgentId) => {
      const address = encodeAppAgentAddress({
        appAgentId: asAppAgentId(appAgentId),
        ss58Format: ss58Format,
      });
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
      const address = encodeTransactionalAddress({
        appAgentId: asAppAgentId(appAgentId),
        taId: asTransactionalId(taId),
        ss58Format,
      });
      expect(decodeTransactionalAddress(address, ss58Format)).toEqual({
        appAgentId,
        taId,
      });
    });
  });

  it("round-trips Named boundary names", () => {
    ["0000000000", "##########", "----------", "ZZZZZZZZZZ"].forEach(
      (accountName) => {
        [0, U32_MAX].forEach((appAgentId) => {
          const address = encodeNamedAddress({
            appAgentId: asAppAgentId(appAgentId),
            accountName: asAccountName(accountName),
            ss58Format,
          });
          expect(decodeNamedAddress(address, ss58Format)).toEqual({
            appAgentId,
            accountName,
          });
        });
      },
    );
  });

  it("rejects out-of-range ids at encoding time", () => {
    [-1, U32_MAX + 1, 1.5, NaN].forEach((badId) => {
      expect(() =>
        encodeAppAgentAddress({
          appAgentId: badId as AppAgentId,
          ss58Format: ss58Format,
        }),
      ).toThrow(RangeError);
      expect(() =>
        encodeTransactionalAddress({
          appAgentId: asAppAgentId(1),
          taId: badId as TransactionalId,
          ss58Format,
        }),
      ).toThrow(RangeError);
      expect(() =>
        encodeNamedAddress({
          appAgentId: badId as AppAgentId,
          accountName: asAccountName("example123"),
          ss58Format,
        }),
      ).toThrow(RangeError);
    });
  });

  it("rejects invalid identifiers in the account-id encoders like the address encoders", () => {
    // The encode*AccountId trio builds the same open part as the address encoders, so
    // an invalid id or name must be rejected identically — a raw account id must never
    // be built from identifiers its address counterpart refuses. (The Python twin is
    // TestEncodeIdValidation in test_adversarial.py; the Rust implementation enforces
    // the types at compile time.)
    [-1, U32_MAX + 1, 1.5, NaN].forEach((badId) => {
      expect(() =>
        encodeAppAgentAccountId({ appAgentId: badId as AppAgentId }),
      ).toThrow(RangeError);
      expect(() =>
        encodeTransactionalAccountId({
          appAgentId: asAppAgentId(1),
          taId: badId as TransactionalId,
        }),
      ).toThrow(RangeError);
      expect(() =>
        encodeNamedAccountId({
          appAgentId: badId as AppAgentId,
          accountName: asAccountName("example123"),
        }),
      ).toThrow(RangeError);
    });
    expect(() =>
      encodeNamedAccountId({
        appAgentId: asAppAgentId(123),
        // An untyped JavaScript caller can bypass the brand; the encoder must still reject
        // the name, so the cast is what the test is about.
        accountName: "bad name!!" as AccountName,
      }),
    ).toThrow(new Error("Account name contains invalid characters"));
    ["short", "veeeerrrrryyyyloooongg"].forEach((badName) => {
      expect(() =>
        encodeNamedAccountId({
          appAgentId: asAppAgentId(123),
          accountName: badName as AccountName,
        }),
      ).toThrow(new Error("Account name must be exactly 10 characters long"));
    });
  });

  it("names the offending value or type in rejection messages", () => {
    // Numbers render verbatim; anything else renders by type, so a JavaScript caller
    // passing the string "123" is not shown a message that looks like a valid number
    // was rejected. (The Python implementation reports the type the same way; the
    // Rust implementation enforces the types at compile time.)
    expect(() =>
      encodeAppAgentAddress({ appAgentId: 1.5 as AppAgentId }),
    ).toThrow("got 1.5");
    expect(() =>
      encodeAppAgentAddress({ appAgentId: 1.5 as AppAgentId }),
    ).toThrow(RangeError);
    expect(() =>
      encodeAppAgentAddress({ appAgentId: NaN as AppAgentId }),
    ).toThrow("got NaN");
    expect(() =>
      encodeAppAgentAddress({ appAgentId: NaN as AppAgentId }),
    ).toThrow(RangeError);
    expect(() =>
      encodeAppAgentAddress({ appAgentId: "123" as unknown as AppAgentId }),
    ).toThrow("appAgentId must be an integer in [0, 4294967295], got string");
    expect(() =>
      encodeAppAgentAddress({ appAgentId: "123" as unknown as AppAgentId }),
    ).toThrow(RangeError);
    expect(() =>
      encodeTransactionalAddress({
        appAgentId: asAppAgentId(1),
        taId: null as unknown as TransactionalId,
      }),
    ).toThrow("taId must be an integer in [0, 4294967295], got null");
    expect(() =>
      encodeTransactionalAddress({
        appAgentId: asAppAgentId(1),
        taId: null as unknown as TransactionalId,
      }),
    ).toThrow(RangeError);
    expect(() =>
      encodeAppAgentAddress({
        appAgentId: asAppAgentId(123),
        ss58Format: "5335" as unknown as number,
      }),
    ).toThrow("ss58Format must be an integer in [0, 16383], got string");
    expect(() =>
      encodeAppAgentAddress({
        appAgentId: asAppAgentId(123),
        ss58Format: "5335" as unknown as number,
      }),
    ).toThrow(RangeError);
  });

  it("rejects negative zero identifiers", () => {
    // -0 is the one integral double JavaScript can tell apart from its integer twin
    // (123.0 *is* 123); it always signals a float-typed source, so it is rejected the
    // way the Python implementation rejects the float -0.0. Plain 0 stays accepted.
    expect(() =>
      encodeAppAgentAddress({ appAgentId: -0 as AppAgentId }),
    ).toThrow("appAgentId must be an integer in [0, 4294967295], got -0");
    expect(() =>
      encodeAppAgentAddress({ appAgentId: -0 as AppAgentId }),
    ).toThrow(RangeError);
    expect(() =>
      encodeTransactionalAddress({
        appAgentId: asAppAgentId(1),
        taId: -0 as TransactionalId,
      }),
    ).toThrow("taId must be an integer in [0, 4294967295], got -0");
    expect(() => asAppAgentId(-0)).toThrow(RangeError);
    expect(asAppAgentId(0)).toBe(0);
  });

  it("rejects a fractional ss58Format at encoding time", () => {
    // The SS58 layer range-checks the format but its bitwise prefix arithmetic would
    // silently truncate fractional values to an integer format.
    [5335.5, NaN].forEach((badFormat) => {
      expect(() =>
        encodeAppAgentAddress({
          appAgentId: asAppAgentId(123),
          ss58Format: badFormat,
        }),
      ).toThrow(/ss58Format must be an integer/);
      expect(() =>
        encodeAppAgentAddress({
          appAgentId: asAppAgentId(123),
          ss58Format: badFormat,
        }),
      ).toThrow(RangeError);
    });
  });

  it("validates appAgentId before the name in encodeNamedAddress", () => {
    // When both the id and the name are invalid, the id is reported first (argument
    // order), matching the Python implementation. "bad@name!!" is 10 characters but
    // carries out-of-charset bytes, so the order decides which error wins.
    expect(() =>
      encodeNamedAddress({
        appAgentId: -1 as AppAgentId,
        accountName: "bad@name!!" as AccountName,
      }),
    ).toThrow("appAgentId must be an integer in [0, 4294967295], got -1");
    expect(() =>
      encodeNamedAddress({
        appAgentId: -1 as AppAgentId,
        accountName: "bad@name!!" as AccountName,
      }),
    ).toThrow(RangeError);
  });
});

describe("ss58Format validation", () => {
  // The ss58Format parameter must be an integer in [0, 16383], mirroring the u16-typed
  // format of the Rust implementation and the runtime check of the Python
  // implementation.

  it("rejects out-of-range and non-integer formats at decoding time", () => {
    const address = encodeAppAgentAddress({
      appAgentId: asAppAgentId(123),
      ss58Format: ss58Format,
    });
    [-1, 16384, 1.5, NaN].forEach((badFormat) => {
      expect(() => decodeAddress(address, badFormat)).toThrow(
        /ss58Format must be an integer in \[0, 16383\]/,
      );
      expect(() => decodeAddress(address, badFormat)).toThrow(RangeError);
    });
  });

  it("rejects a negative-zero ss58Format on both the decode and encode paths", () => {
    // Same rule as the identifiers: -0 always signals a float-typed source. The message
    // spells it "-0" (String(-0) would render "0" and claim a valid format was rejected).
    const address = encodeAppAgentAddress({
      appAgentId: asAppAgentId(123),
      ss58Format: ss58Format,
    });
    expect(() => decodeAddress(address, -0)).toThrow(
      "ss58Format must be an integer in [0, 16383], got -0",
    );
    expect(() => decodeAddress(address, -0)).toThrow(RangeError);
    expect(() =>
      encodeAppAgentAddress({ appAgentId: asAppAgentId(123), ss58Format: -0 }),
    ).toThrow("ss58Format must be an integer in [0, 16383], got -0");
    // Plain zero is a valid SS58 format (Polkadot relay) and stays accepted.
    expect(() => decodeAddress(address, 0)).toThrow(Ss58FormatMismatchError);
  });

  it("rejects -1 instead of treating it as an any-format sentinel", () => {
    // Some Substrate decoders (e.g. @polkadot) interpret -1 as "accept any format";
    // here it must be rejected like any other out-of-range value, so an address of a
    // *different* format is never silently accepted.
    const substrateGenericAddress = encodeAppAgentAddress({
      appAgentId: asAppAgentId(123),
      ss58Format: 42,
    });
    expect(() => decodeAddress(substrateGenericAddress, -1)).toThrow(
      /ss58Format must be an integer in \[0, 16383\]/,
    );
    expect(() => decodeAddress(substrateGenericAddress, -1)).toThrow(
      RangeError,
    );
  });

  it("throws a RangeError for out-of-range formats (numeric-range consistency)", () => {
    // Numeric-range violations use RangeError across the whole API: u32 ids already
    // did, and the ss58Format parameter now does too (it threw a plain Error before the
    // 1.0 release), so a single `catch (e) { if (e instanceof RangeError) ... }` covers
    // every parameter-range misuse.
    const address = encodeAppAgentAddress({
      appAgentId: asAppAgentId(123),
      ss58Format: ss58Format,
    });
    expect(() => decodeAddress(address, 16384)).toThrow(RangeError);
    expect(() =>
      encodeAppAgentAddress({
        appAgentId: asAppAgentId(123),
        ss58Format: 16384,
      }),
    ).toThrow(RangeError);
    expect(() => encodeAddress(u8aToHex(appAgent123), 16384)).toThrow(
      RangeError,
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
      expect(() =>
        encodeAppAgentAddress({
          appAgentId: asAppAgentId(123),
          ss58Format: badFormat,
        }),
      ).toThrow(/ss58Format must be an integer in \[0, 16383\]/);
      expect(() =>
        encodeAppAgentAddress({
          appAgentId: asAppAgentId(123),
          ss58Format: badFormat,
        }),
      ).toThrow(RangeError);
    });
  });

  it("accepts the format boundaries 0 and 16383", () => {
    [0, 16383].forEach((goodFormat) => {
      const address = encodeAppAgentAddress({
        appAgentId: asAppAgentId(123),
        ss58Format: goodFormat,
      });
      expect(decodeAppAgentAddress(address, goodFormat)).toEqual(123);
    });
  });

  it("accepts reserved formats as expected-format decoder inputs", () => {
    const address = encodeAppAgentAddress({
      appAgentId: asAppAgentId(123),
      ss58Format: ss58Format,
    });
    [46, 47].forEach((reserved) => {
      let thrown: unknown;
      try {
        decodeAddress(address, reserved);
      } catch (error) {
        thrown = error;
      }
      expect(thrown).toBeInstanceOf(Ss58Error);
      expect(thrown).toMatchObject({
        kind: "UnexpectedFormat",
        expected: reserved,
        found: ss58Format,
      });
    });
  });
});

describe("Keyless encoder validation order", () => {
  it("reports the identifier/name option before an invalid ss58Format", () => {
    // The keyless encoders build the open part before encodeKeylessAccount examines
    // the format, so their identifier/name options are validated first, in
    // declaration order — unlike encodeAddress and the decoders, which check the
    // format parameter first. The Python implementation orders identically; pinning
    // the both-invalid cases keeps a well-meaning reordering in one language from
    // silently diverging from the other with every suite green.
    expect(() =>
      encodeAppAgentAddress({ appAgentId: -1 as never, ss58Format: 99999 }),
    ).toThrow(/^appAgentId must be an integer/);
    expect(() =>
      encodeAppAgentAddress({ appAgentId: -1 as never, ss58Format: 99999 }),
    ).toThrow(RangeError);
    expect(() =>
      encodeTransactionalAddress({
        appAgentId: -1 as never,
        taId: -1 as never,
        ss58Format: 99999,
      }),
    ).toThrow(/^appAgentId must be an integer/);
    expect(() =>
      encodeTransactionalAddress({
        appAgentId: asAppAgentId(1),
        taId: -1 as never,
        ss58Format: 99999,
      }),
    ).toThrow(/^taId must be an integer/);
    expect(() =>
      encodeNamedAddress({
        appAgentId: asAppAgentId(1),
        accountName: "bad" as AccountName,
        ss58Format: 99999,
      }),
    ).toThrow(AccountNameError);
    // appAgentId before the name: declaration order within the open part too.
    expect(() =>
      encodeNamedAddress({
        appAgentId: -1 as never,
        accountName: "bad" as AccountName,
        ss58Format: 99999,
      }),
    ).toThrow(/^appAgentId must be an integer/);
    // With only the format wrong, the format error still surfaces.
    expect(() =>
      encodeAppAgentAddress({
        appAgentId: asAppAgentId(123),
        ss58Format: 99999,
      }),
    ).toThrow(/^ss58Format must be an integer/);
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
      expectSs58Error(
        () => decodeAddress(address, ss58Format),
        "InvalidLength",
        "Address has an invalid length",
      );
    });
  });

  it("rejects a 33-byte payload", () => {
    // A 33-byte payload (ECDSA-style public key) is legal generic SS58 but never a
    // 32-byte account id.
    const address = rawSs58(
      concatBytes(formatPrefix(ss58Format), appAgent123, Uint8Array.of(0)),
    );
    expectSs58Error(
      () => decodeAddress(address, ss58Format),
      "InvalidLength",
      "Address has an invalid length",
    );
  });

  it("rejects a 31-byte payload", () => {
    // Not a legal SS58 length at all; built manually since ss58Encode refuses it.
    const address = rawSs58(
      concatBytes(formatPrefix(ss58Format), appAgent123.slice(0, 31)),
    );
    expectSs58Error(
      () => decodeAddress(address, ss58Format),
      "InvalidLength",
      "Address has an invalid length",
    );
  });

  it("refuses to encode account ids that are not exactly 32 bytes", () => {
    // The strict codec is 32-byte account ids only, at encode time too — the reason
    // the 31-byte payload above had to be built manually. (The Python twin is
    // test_ss58_encode_accepts_bytes_only in test_adversarial.py.)
    [new Uint8Array(0), new Uint8Array(31), new Uint8Array(33)].forEach(
      (badAccountId) => {
        let thrown: unknown;
        try {
          ss58Encode(badAccountId, ss58Format);
        } catch (e) {
          thrown = e;
        }
        expect(thrown).toBeInstanceOf(Ss58Error);
        expect((thrown as Ss58Error).kind).toEqual("InvalidLength");
      },
    );
  });
});

describe("Malformed SS58 input", () => {
  it("rejects a tampered SS58 checksum", () => {
    const address = encodeAppAgentAddress({
      appAgentId: asAppAgentId(123),
      ss58Format: ss58Format,
    });
    const payload = base58.decode(address);
    const last = payload.length - 1;
    payload[last] = (payload[last] as number) ^ 0xff;
    const tampered = base58.encode(payload);
    expect(() => decodeAddress(tampered, ss58Format)).toThrow(
      new Error("Address has an invalid checksum"),
    );
    expect(() => decodeAddress(tampered, ss58Format)).toThrow(Ss58Error);
  });

  it("decodes a tampered keyless checksum as Regular", () => {
    // Corrupt a byte in the keyless checksum region (open part of an AppAgent account
    // is bytes [0..5]) and rewrap in a valid SS58 envelope: the SS58 layer accepts it,
    // the keyless layer must demote it to Regular.
    const account = Uint8Array.from(appAgent123);
    account[10] = (account[10] as number) ^ 0xff;
    const address = ss58Encode(account, ss58Format);
    const info = decodeAddress(address, ss58Format);
    expect(info.accountType).toEqual(AccountType.Regular);
    expect("appAgentId" in info).toBe(false);
  });

  it("rejects the reserved format 46, crafted with a valid checksum", () => {
    const address = rawSs58(concatBytes(formatPrefix(46), appAgent123));
    expectSs58Error(
      () => decodeAddress(address, 46),
      "InvalidFormat",
      "SS58 format is invalid or reserved",
    );
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
      expect(() => decodeAddress(address, reserved)).toThrow(Ss58Error);
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
    expect(() => decodeAddress(address, ss58Format)).toThrow(Ss58Error);
  });

  it("rejects a prefix byte in the reserved range 128..=255", () => {
    // Per the SS58 spec the first byte must be 0..=63 or 64..=127.
    const address = rawSs58(concatBytes(Uint8Array.of(0x80), appAgent123));
    expectSs58Error(
      () => decodeAddress(address, 128),
      "InvalidFormat",
      "SS58 format is invalid or reserved",
    );
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
    expect(info.accountType).toEqual(AccountType.AppAgent);
    if (info.accountType !== AccountType.AppAgent) {
      throw new Error("Expected an AppAgent account-info variant");
    }
    expect(info.appAgentId).toEqual(123);
  });

  it("rejects garbage input with its taxonomy kind", () => {
    // Every rejection carries its concrete Ss58Error kind: strings outside the base58
    // alphabet are InvalidBase58, strings that decode to something that is not a
    // 32-byte account payload are InvalidLength. A bare "it throws" assertion would
    // also accept a RangeError escaping from parameter validation.
    const garbage: [string, string][] = [
      ["", "InvalidLength"], // decodes to zero bytes
      [" ", "InvalidBase58"],
      ["   ", "InvalidBase58"],
      ["0OIl", "InvalidBase58"], // the four characters excluded from the alphabet
      ["x", "InvalidLength"], // valid base58, one byte
      ["1", "InvalidLength"], // valid base58, one zero byte
    ];
    garbage.forEach(([input, kind]) => {
      expectSs58Error(() => decodeAddress(input, ss58Format), kind);
    });
  });

  it("rejects surrounding whitespace", () => {
    // An address with surrounding whitespace is not the canonical spelling of the
    // account; whitespace is outside the base58 alphabet (pinned cross-language by the
    // ss58_invalid conformance vectors).
    const address = encodeAppAgentAddress({
      appAgentId: asAppAgentId(123),
      ss58Format,
    });
    [`${address} `, ` ${address}`, `${address}\n`, `${address}\t`].forEach(
      (spelling) => {
        // Whitespace is outside the base58 alphabet: InvalidBase58, with the shared
        // cross-language message (pinned verbatim in Rust and Python too).
        expectSs58Error(
          () => decodeAddress(spelling, ss58Format),
          "InvalidBase58",
          "Address is not valid base58",
        );
      },
    );
  });

  it("pins the SS58_MAX_ADDRESS_LENGTH bound to 50", () => {
    // The oversized-input tests use SS58_MAX_ADDRESS_LENGTH as the boundary; pin its
    // exported value so it cannot silently drift from the Rust constant (50), which
    // would make those tests exercise a different boundary than the reference decoder.
    expect(SS58_MAX_ADDRESS_LENGTH).toBe(50);
  });

  it("rejects oversized input before base58 decoding", () => {
    const oversized = "1".repeat(SS58_MAX_ADDRESS_LENGTH + 1);
    expect(() => decodeAddress(oversized, ss58Format)).toThrow(
      new Error("Address has an invalid length"),
    );
    expect(() => decodeAddress(oversized, ss58Format)).toThrow(Ss58Error);
  });

  it("applies the length bound to UTF-8 bytes, not UTF-16 units", () => {
    // The 50-character bound is defined in UTF-8 bytes, like the Rust reference
    // decoder: input can stay within 50 UTF-16 units (or 50 code points) and still
    // exceed the bound. Also pinned cross-language by the ss58_invalid vectors.
    ["é".repeat(26) /* 52 bytes */, "𝄞".repeat(26) /* 104 bytes */].forEach(
      (overBound) => {
        let error: unknown;
        try {
          decodeAddress(overBound, ss58Format);
        } catch (e) {
          error = e;
        }
        expect(error).toBeInstanceOf(Ss58Error);
        expect((error as Ss58Error).kind).toEqual("InvalidLength");
      },
    );
    // A single non-ASCII character is within the bound and fails the base58 charset.
    let error: unknown;
    try {
      decodeAddress("é", ss58Format);
    } catch (e) {
      error = e;
    }
    expect(error).toBeInstanceOf(Ss58Error);
    expect((error as Ss58Error).kind).toEqual("InvalidBase58");
  });

  it("classifies unpaired surrogates via the SS58 error taxonomy", () => {
    // JavaScript strings can hold lone surrogates; TextEncoder encodes each one as
    // the 3-byte replacement character, so the byte bound and the base58 charset
    // classify them like any other non-ASCII garbage. The Python implementation's
    // matching length helper counts the same 3 bytes; Rust cannot represent such strings
    // at all (&str is always valid UTF-8), so the classification is pinned here and
    // in the Python suite rather than in the conformance fixture (the generator's
    // serde_json cannot emit lone surrogates).
    let error: unknown;
    try {
      // One lone surrogate: 1 UTF-16 unit, 3 bytes - within the length bound.
      decodeAddress("\ud800", ss58Format);
    } catch (e) {
      error = e;
    }
    expect(error).toBeInstanceOf(Ss58Error);
    expect((error as Ss58Error).kind).toEqual("InvalidBase58");

    let overBoundError: unknown;
    try {
      // 50 lone surrogates: 50 UTF-16 units but 150 bytes - over the 50-byte bound.
      decodeAddress("\ud800".repeat(50), ss58Format);
    } catch (e) {
      overBoundError = e;
    }
    expect(overBoundError).toBeInstanceOf(Ss58Error);
    expect((overBoundError as Ss58Error).kind).toEqual("InvalidLength");
  });

  it("counts programmatic surrogate pairs like TextEncoder", () => {
    // Nine UTF-16 high/low pairs are nine four-byte scalars (36 bytes), not 18
    // replacement characters (54 bytes), so charset rejection wins over the length
    // bound. This is the exact artificial case pinned by the Python twin.
    let error: unknown;
    try {
      decodeAddress("\ud83d\ude00".repeat(9), ss58Format);
    } catch (caught) {
      error = caught;
    }
    expect(error).toBeInstanceOf(Ss58Error);
    expect((error as Ss58Error).kind).toEqual("InvalidBase58");
  });

  it("classifies lone-surrogate names via the name taxonomy", () => {
    // Account names measure the same TextEncoder UTF-8 bytes as addresses (the
    // surrogate contract covers "addresses and account names alike", per the
    // conformance README), so hostile host strings classify through the
    // AccountNameError taxonomy, identically to the Python twin. Nine ASCII + one
    // lone surrogate: 10 UTF-16 units but 9 + 3 = 12 bytes, so the byte rule
    // reports the length.
    let error: unknown;
    try {
      asAccountName("abcdefghi\ud800");
    } catch (caught) {
      error = caught;
    }
    expect(error).toBeInstanceOf(AccountNameError);
    expect((error as AccountNameError).kind).toEqual("InvalidLength");

    // One lone surrogate + seven ASCII: 3 + 7 = exactly 10 bytes, so the length
    // gate passes and the charset check reports the surrogate itself.
    let charsetError: unknown;
    try {
      asAccountName("\ud800abcdefg");
    } catch (caught) {
      charsetError = caught;
    }
    expect(charsetError).toBeInstanceOf(AccountNameError);
    expect((charsetError as AccountNameError).kind).toEqual("InvalidCharacter");

    // Ten lone surrogates: exactly 10 UTF-16 units but 30 bytes. An implementation
    // measuring String.length would reach the charset check; the byte rule must
    // report the length.
    let boundError: unknown;
    try {
      asAccountName("\udc00".repeat(10));
    } catch (caught) {
      boundError = caught;
    }
    expect(boundError).toBeInstanceOf(AccountNameError);
    expect((boundError as AccountNameError).kind).toEqual("InvalidLength");
  });

  it("counts programmatic surrogate pairs in names like TextEncoder", () => {
    // An explicit UTF-16 pair combines into one four-byte scalar (TextEncoder), so
    // pair + six ASCII is 8 UTF-16 units but exactly 10 bytes: the length gate
    // passes and the charset check fires \u2014 not InvalidLength, which a
    // replacement-per-unit measurement (3 + 3 + 6 = 12 bytes) would report. This is
    // the exact artificial case pinned by the Python twin.
    const pairedName = "\ud83d\ude00aaaaaa";
    expect(pairedName.length).toEqual(8);
    let error: unknown;
    try {
      asAccountName(pairedName);
    } catch (caught) {
      error = caught;
    }
    expect(error).toBeInstanceOf(AccountNameError);
    expect((error as AccountNameError).kind).toEqual("InvalidCharacter");
  });

  it("still decodes the longest legitimate address", () => {
    // The longest legitimate addresses (two-byte prefix, high account id bytes) stay
    // within the bound and must still decode.
    const maxFormat = 16383;
    const address = ss58Encode(new Uint8Array(32).fill(0xff), maxFormat);
    expect(address.length).toBeLessThanOrEqual(SS58_MAX_ADDRESS_LENGTH);
    const info = decodeAddress(address, maxFormat);
    expect(info.accountType).toEqual(AccountType.Regular);
  });
});

describe("String parameter types", () => {
  const nonStrings: readonly unknown[] = [123, true, null, undefined, [], {}];

  it("rejects non-primitive address strings deliberately", () => {
    const address = encodeAppAgentAddress({ appAgentId: asAppAgentId(123) });
    [...nonStrings, new String(address)].forEach((value) => {
      expect(() => decodeAddress(value as never)).toThrow(TypeError);
      expect(() => decodeAddress(value as never)).toThrow(
        /address must be a string/,
      );
      expect(() => decodeAddressWithFormat(value as never)).toThrow(TypeError);
      expect(() => decodeAppAgentAddress(value as never)).toThrow(TypeError);
      expect(() => decodeTransactionalAddress(value as never)).toThrow(
        TypeError,
      );
      expect(() => decodeNamedAddress(value as never)).toThrow(TypeError);
      expect(() => isKeylessAddress(value as never)).toThrow(TypeError);
    });
  });

  it("validates the expected format before the address type", () => {
    expect(() => decodeAddress(123 as never, -1)).toThrow(RangeError);
  });

  it("rejects non-primitive account-id strings without RegExp coercion", () => {
    const accountId = u8aToHex(appAgent123);
    [...nonStrings, new String(accountId)].forEach((value) => {
      expect(() => decodeAccountId(value as never)).toThrow(TypeError);
      expect(() => decodeAccountId(value as never)).toThrow(
        /accountId must be a string/,
      );
      expect(() => encodeAddress(value as never)).toThrow(TypeError);
      expect(() => isKeylessAccountId(value as never)).toThrow(TypeError);
    });
  });

  it("rejects non-primitive account names without RegExp coercion", () => {
    [...nonStrings, new String("example123")].forEach((value) => {
      expect(() =>
        encodeNamedAddress({
          appAgentId: asAppAgentId(123),
          accountName: value as never,
        }),
      ).toThrow(TypeError);
      expect(() =>
        encodeNamedAccountId({
          appAgentId: asAppAgentId(123),
          accountName: value as never,
        }),
      ).toThrow(/accountName must be a string/);
    });
  });
});

describe("Hex account id contract", () => {
  // decodeAccountId accepts a raw account id ("0x" + exactly 64 hex chars); anything
  // else is rejected. decodeAddress accepts SS58 addresses only.

  it("classifies a valid hex account id", () => {
    const info = decodeAccountId(u8aToHex(appAgent123));
    expect(info.accountType).toEqual(AccountType.AppAgent);
    if (info.accountType !== AccountType.AppAgent) {
      throw new Error("Expected an AppAgent account-info variant");
    }
    expect(info.appAgentId).toEqual(123);
    expect(info.accountId).toEqual(u8aToHex(appAgent123));
    // Raw account bytes carry no address; encodeAddress renders one explicitly.
    expect(info.address).toBeNull();
  });

  it("normalizes uppercase hex", () => {
    const upper = `0x${u8aToHex(appAgent123).slice(2).toUpperCase()}`;
    const info = decodeAccountId(upper);
    expect(info.accountType).toEqual(AccountType.AppAgent);
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

  it("rejects hex with whitespace at any position", () => {
    // The contract is exactly 64 hex characters; whitespace anywhere must be rejected
    // (mirrors the Python test_rejects_hex_with_whitespace).
    const clean = u8aToHex(appAgent123).slice(2);
    [
      `0x${clean.slice(0, 8)} ${clean.slice(8)}`, // space inside
      `0x${clean} `, // trailing space
      `0x ${clean}`, // space after the prefix
      `0x${clean.slice(0, 32)}\t${clean.slice(32)}`, // tab inside
      `0x${clean}\n`, // trailing newline
    ].forEach((spaced) => {
      expect(() => decodeAccountId(spaced)).toThrow(/Invalid hex account id/);
    });
  });
});

describe("encodeAddress", () => {
  // encodeAddress renders a raw account id ("0x" + exactly 64 hex chars) as its
  // canonical SS58 address, mirroring AccountId32::to_ss58check_with_version.

  it("round-trips with decodeAddress", () => {
    const hexAccount = u8aToHex(appAgent123);
    const address = encodeAddress(hexAccount, ss58Format);
    expect(address).toEqual(
      encodeAppAgentAddress({
        appAgentId: asAppAgentId(123),
        ss58Format: ss58Format,
      }),
    );
    expect(decodeAddress(address, ss58Format).accountId).toEqual(hexAccount);
  });

  it("defaults to the Trevo format", () => {
    expect(encodeAddress(u8aToHex(appAgent123))).toEqual(
      encodeAppAgentAddress({ appAgentId: asAppAgentId(123) }),
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
      expect(() => encodeAddress(hexAccount, badFormat)).toThrow(RangeError);
    });
    // A reserved format is unusable for exactly the same reason an out-of-range one is —
    // no address can carry it — so it is the same kind of rejected argument and throws the
    // same RangeError. It must *not* throw Ss58Error, which reports on an address: that
    // would mean `catch (e) { if (e instanceof RangeError) }` caught 16384 but not 46.
    [46, 47].forEach((reserved) => {
      let thrown: unknown;
      try {
        encodeAddress(hexAccount, reserved);
      } catch (e) {
        thrown = e;
      }
      expect(thrown).toBeInstanceOf(RangeError);
      expect(thrown).not.toBeInstanceOf(Ss58Error);
      expect((thrown as RangeError).message).toMatch(
        /ss58Format must not be a reserved format/,
      );
    });
  });

  it("validates ss58Format before the account id, like the decode path", () => {
    // The validation order is contractual and mirrored across languages: a caller
    // *parameter* is checked before the *data*, so a caller who got both wrong hears about
    // their own bug first. encodeAddress used to check the payload first, so it disagreed
    // with decodeAddress about which of two bad arguments to report.
    expect(() => encodeAddress("not-an-account-id", 99999)).toThrow(RangeError);
    expect(() => decodeAddress("not-an-address", 99999)).toThrow(RangeError);

    // ...and with only the payload wrong, the payload error still surfaces.
    expect(() => encodeAddress("not-an-account-id")).toThrow(
      ParseAccountIdError,
    );
  });
});

describe("isKeylessAddress", () => {
  // Mirrors the Rust `test_is_keyless_account` test.

  it("returns true for keyless addresses", () => {
    [
      encodeAppAgentAddress({
        appAgentId: asAppAgentId(1),
        ss58Format: ss58Format,
      }),
      encodeTransactionalAddress({
        appAgentId: asAppAgentId(1),
        taId: asTransactionalId(2),
        ss58Format,
      }),
      encodeNamedAddress({
        appAgentId: asAppAgentId(1),
        accountName: asAccountName("test123456"),
        ss58Format,
      }),
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
    expect(info.accountType).toEqual(AccountType.Regular);
  });
});

describe("Decoded value semantics", () => {
  it("freezes every per-variant account-info value and exposes only valid fields", () => {
    const variants = [
      decodeAccountId(`0x${"00".repeat(32)}`),
      decodeAccountId(encodeAppAgentAccountId({ appAgentId: asAppAgentId(1) })),
      decodeAccountId(
        encodeTransactionalAccountId({
          appAgentId: asAppAgentId(1),
          taId: asTransactionalId(2),
        }),
      ),
      decodeAccountId(
        encodeNamedAccountId({
          appAgentId: asAppAgentId(1),
          accountName: asAccountName("test123456"),
        }),
      ),
    ];

    expect(variants.map((info) => Object.keys(info).sort())).toEqual([
      ["accountId", "accountType", "address"],
      ["accountId", "accountType", "address", "appAgentId"],
      ["accountId", "accountType", "address", "appAgentId", "taId"],
      ["accountId", "accountName", "accountType", "address", "appAgentId"],
    ]);
    variants.forEach((info) => expect(Object.isFrozen(info)).toBe(true));
  });

  it("freezes compound decode results", () => {
    const appAddress = encodeAppAgentAddress({
      appAgentId: asAppAgentId(1),
      ss58Format: ss58Format,
    });
    const transactionalAddress = encodeTransactionalAddress({
      appAgentId: asAppAgentId(1),
      taId: asTransactionalId(2),
      ss58Format,
    });
    const namedAddress = encodeNamedAddress({
      appAgentId: asAppAgentId(1),
      accountName: asAccountName("test123456"),
      ss58Format,
    });

    expect(Object.isFrozen(decodeAddressWithFormat(appAddress))).toBe(true);
    expect(
      Object.isFrozen(
        decodeTransactionalAddress(transactionalAddress, ss58Format),
      ),
    ).toBe(true);
    expect(Object.isFrozen(decodeNamedAddress(namedAddress, ss58Format))).toBe(
      true,
    );
  });
});

describe("Ss58Error", () => {
  // The SS58 errors mirror the Rust Ss58Error enum: `kind` is the variant name and the
  // message matches the Rust Display output. The wrong-format error carries the
  // expected and the found format (the Python Ss58FormatMismatchError reports the same
  // pair).

  it("carries the expected and found formats on a format mismatch", () => {
    const substrateGeneric = encodeAppAgentAddress({
      appAgentId: asAppAgentId(123),
      ss58Format: 42,
    });

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

  it("reports a checksum failure rather than a format mismatch for a corrupt address", () => {
    // The expected format is compared only after the checksum verifies (like the Rust
    // from_ss58check): a corrupt address of another format must raise its validation
    // error, never claim to be "a valid address of another network". Mirrors the Python
    // test_checksum_failure_wins_over_format_mismatch.
    const payload = base58.decode(
      encodeAppAgentAddress({ appAgentId: asAppAgentId(123), ss58Format: 42 }),
    );
    const last = payload.length - 1;
    payload[last] = (payload[last] as number) ^ 0xff;
    let thrown: unknown;
    try {
      decodeAddress(base58.encode(payload), ss58Format);
    } catch (error) {
      thrown = error;
    }
    expect(thrown).toBeInstanceOf(Ss58Error);
    expect((thrown as Ss58Error).kind).toEqual("InvalidChecksum");
  });

  it("reports a length failure rather than a format mismatch for a truncated payload", () => {
    // Same ordering for structurally invalid payloads of a non-expected format. Mirrors
    // the Python test_length_failure_wins_over_format_mismatch.
    const address = rawSs58(
      concatBytes(formatPrefix(42), appAgent123.slice(0, 31)),
    );
    let thrown: unknown;
    try {
      decodeAddress(address, ss58Format);
    } catch (error) {
      thrown = error;
    }
    expect(thrown).toBeInstanceOf(Ss58Error);
    expect((thrown as Ss58Error).kind).toEqual("InvalidLength");
  });
});

describe("AccountTypeMismatchError", () => {
  it("carries the expected and found address types", () => {
    const namedAddress = encodeNamedAddress({
      appAgentId: asAppAgentId(1),
      accountName: asAccountName("test123456"),
      ss58Format,
    });

    let thrown: unknown;
    try {
      decodeAppAgentAddress(namedAddress, ss58Format);
    } catch (error) {
      thrown = error;
    }

    expect(thrown).toBeInstanceOf(AccountTypeMismatchError);
    const mismatch = thrown as AccountTypeMismatchError;
    expect(mismatch.expected).toEqual(AccountType.AppAgent);
    expect(mismatch.found).toEqual(AccountType.Named);
    // The message is identical across the Python and TypeScript implementations.
    expect(mismatch.message).toEqual(
      "Provided address is not an AppAgent address but is a Named address",
    );
  });

  it("uses the article 'an' when the found type is AppAgent", () => {
    // The article must agree with both type names, not only the expected one.
    const appAgentAddress = encodeAppAgentAddress({
      appAgentId: asAppAgentId(1),
      ss58Format: ss58Format,
    });

    let thrown: unknown;
    try {
      decodeTransactionalAddress(appAgentAddress, ss58Format);
    } catch (error) {
      thrown = error;
    }

    expect(thrown).toBeInstanceOf(AccountTypeMismatchError);
    const mismatch = thrown as AccountTypeMismatchError;
    expect(mismatch.expected).toEqual(AccountType.Transactional);
    expect(mismatch.found).toEqual(AccountType.AppAgent);
    expect(mismatch.message).toEqual(
      "Provided address is not a Transactional address but is an AppAgent address",
    );
  });
});

describe("Typed account-id decoders", () => {
  // The account-id counterparts of the typed address decoders (the Rust
  // decode_app_agent_account / decode_transactional_account / decode_named_account):
  // they classify the raw "0x" + 64 hex form, which carries no SS58 format.

  const appAgentAccountId = encodeAppAgentAccountId({
    appAgentId: asAppAgentId(123),
  });
  const transactionalAccountId = encodeTransactionalAccountId({
    appAgentId: asAppAgentId(123),
    taId: asTransactionalId(456),
  });
  const namedAccountId = encodeNamedAccountId({
    appAgentId: asAppAgentId(123),
    accountName: asAccountName("hot-wallet"),
  });

  it("decodes each keyless account id to its identifiers", () => {
    expect(decodeAppAgentAccountId(appAgentAccountId)).toEqual(123);
    expect(decodeTransactionalAccountId(transactionalAccountId)).toEqual({
      appAgentId: 123,
      taId: 456,
    });
    expect(decodeNamedAccountId(namedAccountId)).toEqual({
      appAgentId: 123,
      accountName: asAccountName("hot-wallet"),
    });
  });

  it("freezes the compound results", () => {
    expect(
      Object.isFrozen(decodeTransactionalAccountId(transactionalAccountId)),
    ).toBe(true);
    expect(Object.isFrozen(decodeNamedAccountId(namedAccountId))).toBe(true);
  });

  it("throws AccountTypeMismatchError with the (expected, found) pair", () => {
    let thrown: unknown;
    try {
      decodeAppAgentAccountId(namedAccountId);
    } catch (error) {
      thrown = error;
    }
    expect(thrown).toBeInstanceOf(AccountTypeMismatchError);
    expect(thrown).toMatchObject({
      expected: AccountType.AppAgent,
      found: AccountType.Named,
    });

    let regularThrown: unknown;
    try {
      decodeNamedAccountId(`0x${"00".repeat(32)}`);
    } catch (error) {
      regularThrown = error;
    }
    expect(regularThrown).toBeInstanceOf(AccountTypeMismatchError);
    expect(regularThrown).toMatchObject({
      expected: AccountType.Named,
      found: AccountType.Regular,
    });
  });

  it("rejects malformed account ids with ParseAccountIdError", () => {
    ["0xdeadbeef", "", u8aToHex(appAgent123).slice(2)].forEach((badHex) => {
      expect(() => decodeAppAgentAccountId(badHex)).toThrow(
        ParseAccountIdError,
      );
      expect(() => decodeTransactionalAccountId(badHex)).toThrow(
        ParseAccountIdError,
      );
      expect(() => decodeNamedAccountId(badHex)).toThrow(ParseAccountIdError);
    });
  });

  it("rejects an SS58 address (it is not a raw account id)", () => {
    const address = encodeAppAgentAddress({ appAgentId: asAppAgentId(123) });
    expect(() => decodeAppAgentAccountId(address)).toThrow(ParseAccountIdError);
  });
});

describe("Error classes", () => {
  // Every failure class is programmatically discriminable: a caller must never have to
  // string-match a message to tell them apart.

  it("throws ParseAccountIdError from the raw-account-id entry points", () => {
    ["0xnothex", "0x", "not-hex-at-all-but-no-0x-prefix"].forEach((badHex) => {
      expect(() => decodeAccountId(badHex)).toThrow(ParseAccountIdError);
      expect(() => encodeAddress(badHex)).toThrow(ParseAccountIdError);
      expect(() => isKeylessAccountId(badHex)).toThrow(ParseAccountIdError);
    });
    // It is a plain Error subclass, not an Ss58Error: a handler catching only the SS58
    // taxonomy must not swallow a malformed account id.
    let thrown: unknown;
    try {
      decodeAccountId("0xdeadbeef");
    } catch (error) {
      thrown = error;
    }
    expect(thrown).toBeInstanceOf(ParseAccountIdError);
    expect(thrown).not.toBeInstanceOf(Ss58Error);
    expect((thrown as Error).name).toEqual("ParseAccountIdError");
  });

  it("throws NotAnAddressError from the SS58 address entry points", () => {
    const rawAccountId = u8aToHex(appAgent123);
    expect(() => decodeAddress(rawAccountId)).toThrow(NotAnAddressError);
    expect(() => decodeAddressWithFormat(rawAccountId)).toThrow(
      NotAnAddressError,
    );
    expect(() => isKeylessAddress(rawAccountId)).toThrow(NotAnAddressError);
    expect(() => decodeAppAgentAddress(rawAccountId)).toThrow(
      NotAnAddressError,
    );
    expect(() => decodeTransactionalAddress(rawAccountId)).toThrow(
      NotAnAddressError,
    );
    expect(() => decodeNamedAddress(rawAccountId)).toThrow(NotAnAddressError);
  });

  it("throws Ss58FormatMismatchError for the UnexpectedFormat case", () => {
    // The subclass is the "UnexpectedFormat" kind of Ss58Error (as in Python), so it is
    // catchable by instanceof; catching the Ss58Error base class still sees it.
    const substrateGeneric = encodeAppAgentAddress({
      appAgentId: asAppAgentId(123),
      ss58Format: 42,
    });
    let thrown: unknown;
    try {
      decodeAddress(substrateGeneric, ss58Format);
    } catch (error) {
      thrown = error;
    }
    expect(thrown).toBeInstanceOf(Ss58FormatMismatchError);
    expect(thrown).toBeInstanceOf(Ss58Error);
    const mismatch = thrown as Ss58FormatMismatchError;
    expect(mismatch.kind).toEqual("UnexpectedFormat");
    expect(mismatch.expected).toEqual(ss58Format);
    expect(mismatch.found).toEqual(42);
    expect(mismatch.message).toEqual("Expected SS58 format 5335 but found 42");

    // The other kinds stay on the Ss58Error base class.
    let base58Error: unknown;
    try {
      decodeAddress("0OIl", ss58Format);
    } catch (error) {
      base58Error = error;
    }
    expect(base58Error).toBeInstanceOf(Ss58Error);
    expect(base58Error).not.toBeInstanceOf(Ss58FormatMismatchError);
  });
});

describe("options object rejection", () => {
  // The option-object encoders destructure their argument, so a JavaScript caller passing a
  // non-object would otherwise get a raw "Cannot destructure property ..." TypeError instead
  // of the curated message every other rejected argument in this API produces.
  const encoders: ReadonlyArray<[string, (options: never) => string]> = [
    ["encodeAppAgentAddress", encodeAppAgentAddress],
    ["encodeTransactionalAddress", encodeTransactionalAddress],
    ["encodeNamedAddress", encodeNamedAddress],
    // encodeAppAgentAccountId used to be the one positional encoder, and so was the one
    // encoder absent from this table.
    ["encodeAppAgentAccountId", encodeAppAgentAccountId],
    ["encodeTransactionalAccountId", encodeTransactionalAccountId],
    ["encodeNamedAccountId", encodeNamedAccountId],
  ];

  const rejected: ReadonlyArray<[unknown, string]> = [
    [null, "null"],
    [undefined, "undefined"],
    [123, "number"],
    ["0x00", "string"],
    [[], "array"],
  ];

  it.each(encoders)("%s rejects a non-object argument", (_name, encode) => {
    for (const [value, described] of rejected) {
      expect(() => encode(value as never)).toThrow(TypeError);
      expect(() => encode(value as never)).toThrow(
        `options must be an options object, got ${described}`,
      );
    }
  });
});
