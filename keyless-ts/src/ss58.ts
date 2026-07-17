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
import { blake2b } from "@noble/hashes/blake2.js";
import { concatBytes } from "@noble/hashes/utils.js";
import { base58 } from "@scure/base";

// Largest valid SS58 format: the prefix encodes 14 bits. Mirrors the SS58_FORMAT_MAX
// bound of the Rust implementation (private there too). keyless.ts imports this one
// for its parameter validation, so the two layers cannot drift into disagreeing about
// what is valid (see the export-site note below).
const SS58_FORMAT_MAX = 0b0011_1111_1111_1111;

// Length in bytes of the SS58 checksum of a 32-byte account id.
const SS58_CHECKSUM_LENGTH = 2;

// Length in bytes of a blockchain account id.
const ACCOUNT_ID_LENGTH = 32;

// Conservative pre-decode length bound for an SS58 address of a 32-byte account id.
// The generic 36-byte payload-size calculation gives 50 base58 characters. Longer input
// is rejected before base58 decoding, whose cost grows quadratically with the input
// length. The bound is applied to the
// UTF-8 byte length, exactly like the Rust reference decoder (bytes and UTF-16 units
// coincide on the all-ASCII base58 alphabet). This is deliberately a safe
// over-approximation, not the maximum achievable valid spelling: the prefix makes the
// true maximum 49 characters (pinned by the all-0xFF/format-16383 conformance vector).
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

// The five kinds of the shared cross-language taxonomy. The constructor rejects anything
// else at runtime, so a JavaScript caller cannot mint an Ss58Error with a `kind` outside
// the taxonomy every implementation (and the conformance fixture) agrees on. Mirrors the
// closed enum the Rust and Python implementations enforce by construction.
const SS58_ERROR_KINDS: readonly Ss58ErrorKind[] = [
  "InvalidBase58",
  "InvalidLength",
  "InvalidChecksum",
  "InvalidFormat",
  "UnexpectedFormat",
];

/**
 * Makes `instanceof` work across the ESM/CommonJS boundary and across realms.
 *
 * A dual package is loaded twice in a process that mixes `import` and `require` (an app that
 * imports us while a transitive dependency requires us, or a bundler resolving the `browser`
 * condition for app code and CommonJS for a server module). The two copies have distinct class
 * objects, so a plain `e instanceof Ss58Error` is `false` for an error thrown by the other copy
 * — silently defeating the error handling this package's README teaches. The same failure
 * occurs across *realms* (a vm context, an iframe, a worker reached through a non-cloning
 * channel), where even `Error.prototype` itself is a different object per realm.
 *
 * Brand the *prototype* with a `Symbol.for` key (a process-wide registry, so both copies agree
 * on the symbol) and match on that instead of on prototype identity. Subclass prototypes inherit
 * the base brand, so `Ss58FormatMismatchError` still satisfies `instanceof Ss58Error`.
 *
 * `Symbol.hasInstance` is installed on the *constructor*, and statics are inherited down the
 * `extends` chain. A consumer's `class MyError extends Ss58Error {}` would therefore inherit
 * this check and report `true` for *every* Ss58Error in the process, wrecking the `instanceof`
 * of their own class. So the check only answers for the exact constructor it was installed on,
 * and defers to the ordinary prototype walk for anything further down the chain.
 *
 * A brand alone also says nothing about the *shape* of the value: matching on the symbol only
 * would let a bare `{ [Symbol.for("…")]: true }` — a JSON payload, say — pass
 * `instanceof Ss58Error`, after which TypeScript narrows it to `Ss58Error` and the caller reads
 * `.message` and `.kind` off an object that has neither. A real `Error` is required as well —
 * from any realm; see `isRealError`.
 *
 * The registry key carries the package's major version. `Symbol.for` is a process-wide registry,
 * so without it two different majors coexisting in one dependency tree — which are precisely the
 * two that are *not* interchangeable — would cross-brand each other's errors.
 */
const BRAND_NAMESPACE = "@trevofinance/trevokeyless@2";

/**
 * True for real `Error` objects from any realm.
 *
 * `value instanceof Error` covers the common case — including the dual-load one — but is
 * realm-bound: an error created in another realm inherits from *that realm's*
 * `Error.prototype` and fails it. The fallback reads the `[[ErrorData]]` internal slot
 * through `Object.prototype.toString`, which every realm's `Error` carries. A plain object
 * can fake the `"[object Error]"` tag only by installing `Symbol.toStringTag`, which
 * requires running code — and code that can do that can construct a genuinely branded
 * error anyway. The brand is a safety net against *accidental* matches (JSON payloads and
 * the like, which can carry neither symbol keys nor the tag), not a security boundary
 * against in-process code.
 */
function isRealError(value: unknown): boolean {
  return (
    value instanceof Error ||
    Object.prototype.toString.call(value) === "[object Error]"
  );
}

function defineCrossRealmBrand(
  constructor: new (...args: never[]) => Error,
  key: string,
): void {
  const brand = Symbol.for(`${BRAND_NAMESPACE}:${key}`);
  Object.defineProperty(constructor.prototype, brand, { value: true });
  Object.defineProperty(constructor, Symbol.hasInstance, {
    value: function (this: unknown, value: unknown): boolean {
      if (this !== constructor) {
        // A subclass inherited this static. Answer for it the ordinary way.
        return Function.prototype[Symbol.hasInstance].call(this, value);
      }
      return (
        isRealError(value) && (value as Record<symbol, unknown>)[brand] === true
      );
    },
  });
}

/**
 * Thrown by the SS58 decoding (and encoding, for invalid formats) functions.
 *
 * Mirrors the `Ss58Error` enum of the Rust implementation: `kind` is the variant name
 * and the message matches the Rust `Display` output. For `kind === "UnexpectedFormat"`
 * the `expected` and `found` fields carry the two formats; that case is the
 * `Ss58FormatMismatchError` subclass below (the Python implementation exposes the same
 * class), so it can be caught by `instanceof` and not only by inspecting `kind`.
 */
class Ss58Error extends Error {
  readonly kind: Ss58ErrorKind;

  // `declare`, not a plain field: a declared class field is *emitted* as a defineProperty, so
  // it would exist — as `undefined` — on every Ss58Error, and `"expected" in error` would be
  // true for all of them. `declare` is type-only, so the property exists exactly when the
  // constructor assigns it, which is exactly for `kind === "UnexpectedFormat"`.

  /** The format required by the caller (`kind === "UnexpectedFormat"` only). */
  declare readonly expected?: number;

  /** The format found in the address (`kind === "UnexpectedFormat"` only). */
  declare readonly found?: number;

  constructor(
    kind: Ss58ErrorKind,
    message: string,
    expected?: number,
    found?: number,
  ) {
    super(message);
    if (!SS58_ERROR_KINDS.includes(kind)) {
      throw new RangeError(`Unknown Ss58Error kind: ${String(kind)}`);
    }
    // "UnexpectedFormat" *is* the Ss58FormatMismatchError case (the class doc and the
    // README promise so): a bare Ss58Error with that kind would carry no
    // expected/found pair and fail the subclass instanceof. Reject direct
    // construction — the Python Ss58Error.__init__ enforces the same — but let the
    // subclass (and consumer subclasses of it, which new.target reveals) through.
    // Widened to Function: TypeScript otherwise rejects the comparison of the two
    // constructor types as overlap-free, which is exactly the case being tested.
    const target: Function = new.target; // eslint-disable-line @typescript-eslint/no-unsafe-function-type
    if (
      kind === "UnexpectedFormat" &&
      target !== Ss58FormatMismatchError &&
      !Object.prototype.isPrototypeOf.call(
        Ss58FormatMismatchError.prototype,
        target.prototype as object,
      )
    ) {
      throw new RangeError(
        "kind 'UnexpectedFormat' carries an expected/found format pair; " +
          "construct Ss58FormatMismatchError(expected, found) instead",
      );
    }
    this.name = "Ss58Error";
    this.kind = kind;
    // Assigned only when they carry a value. An unconditional assignment would make
    // `"expected" in error` true — with the value `undefined` — for every Ss58Error, and
    // that structural check is the natural JS idiom (it is the one this package uses for
    // BlockchainAccountInfo: `"appAgentId" in info`).
    if (expected !== undefined) {
      this.expected = expected;
    }
    if (found !== undefined) {
      this.found = found;
    }
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

  static unexpectedFormat(
    expected: number,
    found: number,
  ): Ss58FormatMismatchError {
    return new Ss58FormatMismatchError(expected, found);
  }
}

/**
 * The `"UnexpectedFormat"` case of `Ss58Error`: the address is a valid SS58 address, but
 * of a different (non-expected) format. Carries both formats, always defined here.
 *
 * The Python implementation exposes the same class; the Rust implementation has a single
 * `Ss58Error::UnexpectedFormat` variant carrying the same pair. `kind` stays
 * `"UnexpectedFormat"`, so `catch (e) { if (e instanceof Ss58Error) e.kind }` keeps
 * working unchanged.
 */
class Ss58FormatMismatchError extends Ss58Error {
  declare readonly expected: number;

  declare readonly found: number;

  constructor(expected: number, found: number) {
    super(
      "UnexpectedFormat",
      `Expected SS58 format ${expected} but found ${found}`,
      expected,
      found,
    );
    this.name = "Ss58FormatMismatchError";
  }
}

defineCrossRealmBrand(Ss58Error, "Ss58Error");
defineCrossRealmBrand(Ss58FormatMismatchError, "Ss58FormatMismatchError");

const SS58_PREFIX = Uint8Array.of(83, 83, 53, 56, 80, 82, 69); // ASCII "SS58PRE"

/**
 * UTF-8 byte length with the same surrogate handling as TextEncoder, without requiring
 * TextEncoder to exist globally (not all React Native runtimes provide it).
 */
function utf8Length(value: string): number {
  let length = 0;
  for (let index = 0; index < value.length; index += 1) {
    const first = value.charCodeAt(index);
    if (first <= 0x7f) {
      length += 1;
    } else if (first <= 0x7ff) {
      length += 2;
    } else if (first >= 0xd800 && first <= 0xdbff && index + 1 < value.length) {
      const second = value.charCodeAt(index + 1);
      if (second >= 0xdc00 && second <= 0xdfff) {
        length += 4;
        index += 1;
      } else {
        length += 3;
      }
    } else {
      // BMP scalars and lone surrogates both encode to three bytes (the latter via U+FFFD).
      length += 3;
    }
  }
  return length;
}

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
  if (accountId.length !== ACCOUNT_ID_LENGTH) {
    throw Ss58Error.invalidLength();
  }
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
  // The bound is defined in UTF-8 bytes, like the Rust reference decoder (which checks
  // the byte length of the &str): non-ASCII input can stay within 50 UTF-16 units yet
  // exceed 50 bytes and must classify as a length failure in every implementation. The
  // unit check above keeps the cost O(1) for huge input; this exact check only ever
  // examines at most 50 UTF-16 units.
  if (utf8Length(address) > SS58_MAX_ADDRESS_LENGTH) {
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

  // `data` is non-empty (checked above), so index 0 is in bounds.
  const first = data[0] as number;
  let prefixLength: number;
  let format: number;
  if (first <= 63) {
    prefixLength = 1;
    format = first;
  } else if (first <= 127) {
    if (data.length < 2) {
      throw Ss58Error.invalidLength();
    }
    // The `data.length < 2` guard above makes index 1 in bounds.
    const second = data[1] as number;
    prefixLength = 2;
    format =
      ((first & 0b0011_1111) << 2) |
      (second >> 6) |
      ((second & 0b0011_1111) << 8);
  } else {
    // First bytes 128..=255 are not a valid SS58 prefix encoding.
    throw Ss58Error.invalidFormat();
  }
  if (formatInvalid(format)) {
    throw Ss58Error.invalidFormat();
  }
  if (data.length !== prefixLength + ACCOUNT_ID_LENGTH + SS58_CHECKSUM_LENGTH) {
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

export type { Ss58ErrorKind };

export {
  Ss58Error,
  Ss58FormatMismatchError,
  // Package-internal: keyless.ts brands its own error classes with the same scheme.
  defineCrossRealmBrand,
  // Package-internal: keyless.ts validates the ss58Format parameter against the same bound
  // the encoder enforces, so the two cannot drift into disagreeing about what is valid.
  SS58_FORMAT_MAX,
  ACCOUNT_ID_LENGTH,
  ss58Decode,
  ss58Encode,
  // Package-internal: the account-name length rule of keyless.ts is defined in UTF-8
  // bytes (like the Rust AccountName::new), and reuses this helper.
  utf8Length,
};
