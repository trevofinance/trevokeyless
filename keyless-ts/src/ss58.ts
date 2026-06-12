/**
 * SS58 address encoding and decoding for 32-byte account ids.
 *
 * SS58 is the address format used by Substrate-based chains: a base58 string carrying a
 * network format prefix, the 32-byte account id, and a 2-byte checksum
 * (`blake2b_512("SS58PRE" ++ prefix ++ account_id)[0..2]`).
 *
 * This module is a port of the Rust reference implementation
 * (`keyless-rust/src/ss58.rs`) on top of the audited zero-dependency primitives
 * `@noble/hashes` (blake2b) and `@scure/base` (base58). Decoding accepts only the
 * canonical structure: a 1- or 2-byte prefix, a 32-byte account id, and a full two-byte
 * checksum, with a non-reserved format. The module is internal to the package; only the
 * keyless entry points are part of the public API.
 */
import { blake2b } from "@noble/hashes/blake2b";
import { concatBytes, utf8ToBytes } from "@noble/hashes/utils";
import { base58 } from "@scure/base";

// Largest valid SS58 format: the prefix encodes 14 bits. Mirrors the SS58_FORMAT_MAX
// bound of the Rust implementation.
export const SS58_FORMAT_MAX = 0b0011_1111_1111_1111;

// Length in bytes of the SS58 checksum of a 32-byte account id.
const SS58_CHECKSUM_LENGTH = 2;

// Length in bytes of a blockchain account id.
const ACCOUNT_ID_BYTES_LENGTH = 32;

// Maximum length in characters of an SS58 address of a 32-byte account id: the payload
// is at most 2 (prefix) + 32 (account id) + 2 (checksum) = 36 bytes = 288 bits, and each
// base58 character carries log2(58) ~ 5.858 bits, so a valid address has at most
// ceil(288 / 5.858) = 50 characters. Longer input is rejected before base58 decoding,
// whose cost grows quadratically with the input length. Mirrors the Rust implementation.
export const SS58_MAX_ADDRESS_LENGTH = 50;

/**
 * The failure classification of an `Ss58Error`; the names mirror the variants of the
 * Rust `Ss58Error` enum.
 */
type Ss58ErrorKind =
  | "InvalidBase58"
  | "InvalidLength"
  | "InvalidChecksum"
  | "InvalidFormat"
  | "UnexpectedFormat";

/**
 * Thrown by the SS58 decoding (and encoding, for invalid formats) functions.
 *
 * Mirrors the `Ss58Error` enum of the Rust implementation: `kind` is the variant name
 * and the message matches the Rust `Display` output. For `kind === "UnexpectedFormat"`
 * the `expected` and `found` fields carry the two formats (the Python implementation
 * reports the same pair via `Ss58FormatMismatchError`).
 */
class Ss58Error extends Error {
  readonly kind: Ss58ErrorKind;

  /** The format required by the caller (`kind === "UnexpectedFormat"` only). */
  readonly expected?: number;

  /** The format found in the address (`kind === "UnexpectedFormat"` only). */
  readonly found?: number;

  constructor(
    kind: Ss58ErrorKind,
    message: string,
    expected?: number,
    found?: number,
  ) {
    super(message);
    this.name = "Ss58Error";
    this.kind = kind;
    this.expected = expected;
    this.found = found;
  }

  static invalidBase58(): Ss58Error {
    return new Ss58Error("InvalidBase58", "Address is not valid base58");
  }

  static invalidLength(): Ss58Error {
    return new Ss58Error("InvalidLength", "Address has an invalid length");
  }

  static invalidChecksum(): Ss58Error {
    return new Ss58Error("InvalidChecksum", "Address has an invalid checksum");
  }

  static invalidFormat(): Ss58Error {
    return new Ss58Error("InvalidFormat", "SS58 format is invalid or reserved");
  }

  static unexpectedFormat(expected: number, found: number): Ss58Error {
    return new Ss58Error(
      "UnexpectedFormat",
      `Expected SS58 format ${expected} but found ${found}`,
      expected,
      found,
    );
  }
}

const SS58_PREFIX = utf8ToBytes("SS58PRE");

/**
 * Returns true for SS58 formats that can never be used: reserved (46, 47), out of the
 * 14-bit range, or not an integer at all.
 */
function formatInvalid(format: number): boolean {
  return (
    !Number.isInteger(format) ||
    format < 0 ||
    format > SS58_FORMAT_MAX ||
    format === 46 ||
    format === 47
  );
}

/**
 * Computes the 64-byte blake2b hash of `"SS58PRE" ++ data`, of which the first
 * `SS58_CHECKSUM_LENGTH` bytes are the SS58 checksum.
 */
function ss58Hash(data: Uint8Array): Uint8Array {
  return blake2b(concatBytes(SS58_PREFIX, data), { dkLen: 64 });
}

/**
 * Encodes a 32-byte account id as an SS58 address with the given format.
 *
 * @throws {Ss58Error} - Kind `InvalidFormat` if the format is reserved (46, 47) or not
 *   an integer in [0, 16383]; kind `InvalidLength` if the account id is not 32 bytes.
 */
function ss58Encode(accountId: Uint8Array, format: number): string {
  if (formatInvalid(format)) {
    throw Ss58Error.invalidFormat();
  }
  if (accountId.length !== ACCOUNT_ID_BYTES_LENGTH) {
    throw Ss58Error.invalidLength();
  }
  /* eslint-disable no-bitwise */
  const prefix =
    format < 64
      ? Uint8Array.of(format)
      : // Two-byte prefix: the first byte carries bits 2..=7 of the format (tagged with
        // 0b01 in its upper two bits), the second byte carries bits 8..=13 in its lower
        // six bits and bits 0..=1 in its upper two bits.
        Uint8Array.of(
          ((format & 0b0000_0000_1111_1100) >> 2) | 0b0100_0000,
          (format >> 8) | ((format & 0b0000_0000_0000_0011) << 6),
        );
  /* eslint-enable no-bitwise */
  const payload = concatBytes(prefix, accountId);
  return base58.encode(
    concatBytes(payload, ss58Hash(payload).slice(0, SS58_CHECKSUM_LENGTH)),
  );
}

/**
 * Decodes an SS58 address of any (non-reserved) format, returning the 32-byte account
 * id and the format found in the address.
 *
 * Input longer than the maximum length of a valid address (50 characters) is rejected
 * before base58 decoding, so the cost of decoding untrusted input stays bounded.
 *
 * @throws {Ss58Error} - With the `kind` describing the failure, exactly like the Rust
 *   `from_ss58check_with_version`.
 */
function ss58Decode(address: string): {
  accountId: Uint8Array;
  format: number;
} {
  if (address.length > SS58_MAX_ADDRESS_LENGTH) {
    throw Ss58Error.invalidLength();
  }
  let data: Uint8Array;
  try {
    data = base58.decode(address);
  } catch {
    throw Ss58Error.invalidBase58();
  }
  if (data.length === 0) {
    throw Ss58Error.invalidLength();
  }

  /* eslint-disable no-bitwise */
  let prefixLength: number;
  let format: number;
  if (data[0] <= 63) {
    prefixLength = 1;
    [format] = data;
  } else if (data[0] <= 127) {
    if (data.length < 2) {
      throw Ss58Error.invalidLength();
    }
    prefixLength = 2;
    format =
      ((data[0] & 0b0011_1111) << 2) |
      (data[1] >> 6) |
      ((data[1] & 0b0011_1111) << 8);
  } else {
    // First bytes 128..=255 are not a valid SS58 prefix encoding.
    throw Ss58Error.invalidFormat();
  }
  /* eslint-enable no-bitwise */
  if (formatInvalid(format)) {
    throw Ss58Error.invalidFormat();
  }
  if (
    data.length !==
    prefixLength + ACCOUNT_ID_BYTES_LENGTH + SS58_CHECKSUM_LENGTH
  ) {
    throw Ss58Error.invalidLength();
  }

  const checksumStart = data.length - SS58_CHECKSUM_LENGTH;
  const expectedChecksum = ss58Hash(data.subarray(0, checksumStart));
  if (
    data[checksumStart] !== expectedChecksum[0] ||
    data[checksumStart + 1] !== expectedChecksum[1]
  ) {
    throw Ss58Error.invalidChecksum();
  }

  return { accountId: data.slice(prefixLength, checksumStart), format };
}

export { Ss58Error, Ss58ErrorKind, ss58Decode, ss58Encode };
