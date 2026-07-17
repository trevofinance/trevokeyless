import { blake2b } from "@noble/hashes/blake2.js";
import { bytesToHex, concatBytes, hexToBytes } from "@noble/hashes/utils.js";
import type { Ss58ErrorKind } from "./ss58.js";
import {
  ACCOUNT_ID_LENGTH,
  SS58_FORMAT_MAX,
  SS58_MAX_ADDRESS_LENGTH,
  Ss58Error,
  Ss58FormatMismatchError,
  defineCrossRealmBrand,
  ss58Decode,
  ss58Encode,
  utf8Length,
} from "./ss58.js";

/**
 * The account-type discriminants stored at byte [4] of a keyless account id. They equal
 * the numeric values of the matching `AccountType` variants and carry the same names as
 * the Rust constants.
 */
const APP_AGENT_ACCOUNT_IDENTIFIER = 1;
const TRANSACTIONAL_ACCOUNT_IDENTIFIER = 2;
const NAMED_ACCOUNT_IDENTIFIER = 3;

/** Length of an account name, in UTF-8 bytes (the Rust `ACCOUNT_NAME_LENGTH`). */
const ACCOUNT_NAME_LENGTH = 10;

const ACCOUNT_NAME_ALLOWED_CHARS = /^[0-9a-zA-Z\-#]+$/;

// A 256-entry byte -> allowed lookup, derived once from ACCOUNT_NAME_ALLOWED_CHARS so the
// two cannot drift. isValidAccountNameBytes indexes it per byte instead of allocating a
// String.fromCharCode and running the regex ten times on every Named-account decode (the
// hot path for classifying chain state), mirroring the byte table used by Rust/Python.
const ACCOUNT_NAME_ALLOWED_BYTES: Uint8Array = (() => {
  const table = new Uint8Array(256);
  for (let byte = 0; byte < table.length; byte += 1) {
    if (ACCOUNT_NAME_ALLOWED_CHARS.test(String.fromCharCode(byte))) {
      table[byte] = 1;
    }
  }
  return table;
})();

const SS58_FORMAT_TREVO_ASSET_HUB = 5335;

// A raw account id accepted by decodeAccountId: 0x followed by exactly 64 hex
// characters. (The Python implementation uses the same expression; the Rust
// AccountId32 FromStr accepts the same string form.)
const HEX_ACCOUNT_ID = /^0x[0-9a-fA-F]{64}$/;

const U32_MAX = 4294967295;

// The identifier brands use *string-literal* keys, not `declare const ...: unique symbol`:
// the ESM and CJS builds each emit their own declaration file, and TypeScript treats every
// declaration file's `unique symbol` as a distinct nominal symbol, so the two builds' views
// of the same brand were mutually unassignable — the type-level twin of the dual-package
// hazard the runtime error brands (defineCrossRealmBrand) already solve. A namespaced
// string-literal key is structurally identical in both declaration files while remaining
// practically unforgeable: the property never exists at runtime, and no real object shape
// carries a "@trevofinance/trevokeyless.*" property by accident.

/** A validated u32 App Agent identifier, distinct from `TransactionalId` to prevent transposition. */
type AppAgentId = number & {
  readonly ["@trevofinance/trevokeyless.AppAgentId"]: true;
};

/** A validated u32 Transactional identifier, distinct from `AppAgentId` to prevent transposition. */
type TransactionalId = number & {
  readonly ["@trevofinance/trevokeyless.TransactionalId"]: true;
};

/**
 * A validated Named account name: exactly 10 UTF-8 bytes from `0-9 a-z A-Z - #`.
 *
 * Branded for the same reason `AppAgentId` is: an invalid name fails at the point of
 * construction, where the caller can see it, rather than inside an encoder. Build one with
 * `accountName()`. The counterpart of the Rust `AccountName` newtype and the Python
 * `AccountName` class.
 *
 * Names are case-sensitive, exact byte strings: `Treasury-1` and `treasury-1` are two different
 * accounts of the same App Agent. Never case-fold or trim a name.
 */
type AccountName = string & {
  readonly ["@trevofinance/trevokeyless.AccountName"]: true;
};

// Readability aliases for the two interchange string forms and the SS58 format number,
// mirroring the Python `BlockchainAddress` / `BlockchainAccountId` / `SS58Format`
// aliases. Plain aliases, deliberately NOT branded like the identifier types above:
// any string or number is assignable, they only name the intent in signatures — the
// values are runtime-validated data, not constructor-validated identifiers.

/** An SS58 address string — one of the two cross-language interchange forms. */
type BlockchainAddress = string;

/** A raw account id string (`0x` + 64 hex characters) — the other interchange form. */
type BlockchainAccountId = string;

/** An SS58 format (network prefix): an integer in [0, 16383] excluding 46 and 47. */
type Ss58Format = number;

/**
 * Classification of a blockchain address, as returned by `decodeAddress` and
 * `decodeAccountId`.
 *
 * An erasable `const` object plus union type rather than a TS `enum`, so the package
 * contains no non-erasable syntax. The numeric values match the Python `AccountType`
 * enum and the SCALE variant indices of the Rust `AccountType`; for the keyless
 * variants they coincide with the type discriminant stored at byte [4] of a keyless
 * account id. There is no numeric reverse lookup (`AccountType[1]`); derive names via
 * `Object.entries(AccountType)` if needed.
 */
const AccountType = Object.freeze({
  Regular: 0,
  AppAgent: 1,
  Transactional: 2,
  Named: 3,
} as const);
type AccountType = (typeof AccountType)[keyof typeof AccountType];

/** Name of each `AccountType` value, for error messages. */
const ACCOUNT_TYPE_NAMES: Record<AccountType, keyof typeof AccountType> = {
  [AccountType.Regular]: "Regular",
  [AccountType.AppAgent]: "AppAgent",
  [AccountType.Transactional]: "Transactional",
  [AccountType.Named]: "Named",
};

/** The indefinite article for an account type name ("an" before AppAgent). */
function accountTypeArticle(type: AccountType): string {
  return type === AccountType.AppAgent ? "an" : "a";
}

/**
 * Thrown by the typed decode functions when the address is of a different type.
 *
 * Carries the expected and the actual (found) classification of the address, mirroring
 * the `KeylessDecodeError` of the Rust implementation. The message is identical to the
 * Python implementation's.
 */
class AccountTypeMismatchError extends Error {
  readonly expected: AccountType;

  readonly found: AccountType;

  constructor(expected: AccountType, found: AccountType) {
    super(
      `Provided address is not ${accountTypeArticle(expected)} ${ACCOUNT_TYPE_NAMES[expected]} address but is ${accountTypeArticle(found)} ${ACCOUNT_TYPE_NAMES[found]} address`,
    );
    this.name = "AccountTypeMismatchError";
    this.expected = expected;
    this.found = found;
  }
}

/**
 * Thrown when a string that must be a raw account id is not `0x` followed by exactly 64
 * hex characters (`decodeAccountId`, `encodeAddress`, `isKeylessAccountId`).
 *
 * Mirrors the Rust `ParseAccountIdError` (the `FromStr`/`TryFrom` error of
 * `AccountId32`); the message is the Rust `Display` output verbatim.
 */
class ParseAccountIdError extends Error {
  constructor() {
    super("Invalid hex account id: expected 0x followed by 64 hex characters");
    this.name = "ParseAccountIdError";
  }
}

/**
 * Thrown by the SS58 address entry points (`decodeAddress`, `decodeAddressWithFormat`,
 * and the typed decoders built on them) when the input is a raw `0x` account id.
 *
 * SS58 addresses and raw account ids are separate input classes: a raw account id
 * carries no SS58 format, so accepting one here would silently bypass the
 * expected-format check. `decodeAccountId` is the entry point for raw account ids.
 */
class NotAnAddressError extends Error {
  constructor() {
    super("Raw account id hex is not an SS58 address; use decodeAccountId");
    this.name = "NotAnAddressError";
  }
}

type AccountNameErrorKind = "InvalidLength" | "InvalidCharacter";

/** Thrown when a Named account name violates the shared wire contract. */
class AccountNameError extends Error {
  readonly kind: AccountNameErrorKind;

  constructor(kind: AccountNameErrorKind) {
    super(
      kind === "InvalidLength"
        ? "Account name must be exactly 10 characters long"
        : "Account name contains invalid characters",
    );
    this.name = "AccountNameError";
    this.kind = kind;
  }
}

// Cross-realm `instanceof` for this module's error classes; see defineCrossRealmBrand in
// ss58.ts. A dual ESM/CommonJS package is loaded twice when an app imports it while a
// transitive dependency requires it, and plain prototype identity fails across that split.
defineCrossRealmBrand(AccountTypeMismatchError, "AccountTypeMismatchError");
defineCrossRealmBrand(ParseAccountIdError, "ParseAccountIdError");
defineCrossRealmBrand(NotAnAddressError, "NotAnAddressError");
defineCrossRealmBrand(AccountNameError, "AccountNameError");

/**
 * Fields common to every decoded account-info variant.
 *
 * The `address` field is the SS58 address the info was decoded from, or `null` when
 * the input was a raw account id (`decodeAccountId`): raw account bytes carry no SS58
 * format, so there is no address to report — render one explicitly with
 * `encodeAddress`. `accountId` is always the normalized `0x` + lowercase hex form.
 *
 * `address` is the input spelling verbatim, never re-encoded: for the one accepted
 * non-canonical spelling (the two-byte-prefix encoding of SS58 formats below 64, see
 * the security model) it differs from the canonical address of the same account. Key
 * caches and lookups on `accountId` — the canonical cross-language form — and render
 * the canonical address with `encodeAddress`.
 *
 * The type is parameterized on the address, because whether one exists is a hard invariant
 * of the entry point that produced the result, not something a caller has to test for:
 * `decodeAddress`/`decodeAddressWithFormat` always have the address they were given, and
 * `decodeAccountId` never does. Left as a plain `string | null` it forced every
 * `decodeAddress` caller to handle an unreachable `null`, and invited every
 * `decodeAccountId` caller to write `info.address!` on a value that is always `null`.
 */
interface AccountInfoBase<Address extends string | null = string | null> {
  readonly address: Address;
  readonly accountId: string;
}

/** Decoded identity of a regular (non-keyless) account. */
interface RegularAccountInfo<
  Address extends string | null = string | null,
> extends AccountInfoBase<Address> {
  readonly accountType: typeof AccountType.Regular;
}

/** Decoded identity of an AppAgent account. */
interface AppAgentAccountInfo<
  Address extends string | null = string | null,
> extends AccountInfoBase<Address> {
  readonly accountType: typeof AccountType.AppAgent;
  readonly appAgentId: AppAgentId;
}

/** Decoded identity of a Transactional account. */
interface TransactionalAccountInfo<
  Address extends string | null = string | null,
> extends AccountInfoBase<Address> {
  readonly accountType: typeof AccountType.Transactional;
  readonly appAgentId: AppAgentId;
  readonly taId: TransactionalId;
}

/** Decoded identity of a Named account. */
interface NamedAccountInfo<
  Address extends string | null = string | null,
> extends AccountInfoBase<Address> {
  readonly accountType: typeof AccountType.Named;
  readonly appAgentId: AppAgentId;
  readonly accountName: AccountName;
}

/**
 * Immutable, per-variant decoded account identity. Each variant contains only the
 * identifiers valid for its account type, making contradictory states unrepresentable.
 *
 * `Address` records whether the result carries the SS58 address it was decoded from:
 * `BlockchainAccountInfo<string>` from `decodeAddress` and `decodeAddressWithFormat`,
 * `BlockchainAccountInfo<null>` from `decodeAccountId`. The bare
 * `BlockchainAccountInfo` still means "either", so a consumer that stores or passes
 * around a decode result of unknown origin can keep naming it that.
 */
type BlockchainAccountInfo<Address extends string | null = string | null> =
  | RegularAccountInfo<Address>
  | AppAgentAccountInfo<Address>
  | TransactionalAccountInfo<Address>
  | NamedAccountInfo<Address>;

/**
 * Renders a rejected parameter for an error message: numbers verbatim (including NaN
 * and non-integers), `null` as `"null"`, arrays as `"array"`, anything else by its
 * type, so a JavaScript caller passing e.g. the string "123" sees `got string` rather
 * than a message that looks like a valid number was rejected. Matches the value
 * rendering of `validateString`/`validateOptions`. (The Python implementation reports
 * the type the same way; the Rust implementation enforces the types at compile time.)
 *
 * @param {unknown} value - The rejected value.
 * @returns {string} - The rendering for the error message.
 */
function describeRejectedValue(value: unknown): string {
  if (typeof value === "number") {
    // String(-0) renders as "0", which would make the message claim a valid value was
    // rejected; negative zero is the one rejected number that needs its own spelling.
    return Object.is(value, -0) ? "-0" : String(value);
  }
  if (value === null) {
    return "null";
  }
  if (Array.isArray(value)) {
    return "array";
  }
  return typeof value;
}

/**
 * Rejects a non-object where an options object is required.
 *
 * The option-object entry points destructure their argument, so a JavaScript caller passing
 * `null`, a number, or nothing at all would otherwise get a raw
 * "Cannot destructure property 'appAgentId' of 'object null' as it is null" — the one misuse
 * in this API that escapes the curated messages every other rejected argument produces.
 */
function validateOptions(
  value: unknown,
  label: string,
): asserts value is Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    const type =
      value === null ? "null" : Array.isArray(value) ? "array" : typeof value;
    throw new TypeError(`${label} must be an options object, got ${type}`);
  }
}

/** Rejects JavaScript values that TypeScript's compile-time string type cannot admit. */
function validateString(
  value: unknown,
  label: string,
): asserts value is string {
  if (typeof value !== "string") {
    const type =
      value === null ? "null" : Array.isArray(value) ? "array" : typeof value;
    throw new TypeError(`${label} must be a string, got ${type}`);
  }
}

/** The core u32 range predicate, shared by `validateU32` and the `is*Id` guards. */
function isValidU32(value: number): boolean {
  // `Object.is` separates -0 from 0. JavaScript cannot distinguish 123.0 from 123 —
  // they are the same IEEE-754 double — but -0 *is* distinguishable, always signals a
  // float-typed source, and is rejected, mirroring the Python implementation rejecting
  // the float -0.0 with a TypeError. (One consequence: JSON `-0` parses to an accepted
  // int 0 in Python but to a rejected -0 here; host-number classes like this cannot be
  // pinned by the JSON fixture — see conformance/README.md.)
  return (
    Number.isInteger(value) &&
    !Object.is(value, -0) &&
    value >= 0 &&
    value <= U32_MAX
  );
}

function validateU32(value: number, label: string): void {
  if (!isValidU32(value)) {
    throw new RangeError(
      `${label} must be an integer in [0, ${U32_MAX}], got ${describeRejectedValue(value)}`,
    );
  }
}

/**
 * Encodes a u32 value to its 4-byte little-endian representation.
 *
 * @param {number} value - The value to encode.
 * @param {string} label - Name of the value, used in the error message.
 * @returns {Uint8Array} - The 4 little-endian bytes.
 * @throws {RangeError} - Thrown if the value is not an integer in [0, 4294967295].
 */
function u32ToLeBytes(value: number, label: string): Uint8Array {
  validateU32(value, label);
  const bytes = new Uint8Array(4);
  new DataView(bytes.buffer).setUint32(0, value, true);
  return bytes;
}

/** Validates and brands a numeric App Agent identifier. */
function appAgentId(value: number): AppAgentId {
  validateU32(value, "appAgentId");
  return value as AppAgentId;
}

/** Validates and brands a numeric Transactional identifier. */
function transactionalId(value: number): TransactionalId {
  validateU32(value, "taId");
  return value as TransactionalId;
}

/**
 * Validates and brands a Named account name.
 *
 * @throws {TypeError} if the value is not a string.
 * @throws {AccountNameError} if it is not 10 UTF-8 bytes long (`kind: "InvalidLength"`) or
 *   contains characters outside `0-9 a-z A-Z - #` (`kind: "InvalidCharacter"`).
 */
function accountName(value: string): AccountName {
  validateAccountName(value);
  return value as AccountName;
}

/**
 * Non-throwing counterpart of `appAgentId`: narrows to `AppAgentId` when the value is a
 * valid u32 App Agent identifier (an integer in [0, 4294967295]).
 */
function isAppAgentId(value: unknown): value is AppAgentId {
  return typeof value === "number" && isValidU32(value);
}

/**
 * Non-throwing counterpart of `transactionalId`: narrows to `TransactionalId` when the
 * value is a valid u32 Transactional identifier (an integer in [0, 4294967295]).
 */
function isTransactionalId(value: unknown): value is TransactionalId {
  return typeof value === "number" && isValidU32(value);
}

/**
 * Non-throwing counterpart of `accountName`: narrows to `AccountName` when the value is a
 * string of exactly 10 UTF-8 bytes from `0-9 a-z A-Z - #`.
 */
function isAccountName(value: unknown): value is AccountName {
  if (typeof value !== "string") {
    return false;
  }
  try {
    // Reuse the exact name rules (length in UTF-8 bytes, then charset) rather than
    // restating them; the branded return is discarded, only success/failure matters.
    accountName(value);
    return true;
  } catch {
    return false;
  }
}

/**
 * Reads a u32 from 4 little-endian bytes at the given offset of an account id.
 *
 * @param {Uint8Array} bytes - The bytes to read from.
 * @param {number} offset - Offset of the little-endian u32.
 * @returns {number} - The decoded value.
 */
function u32FromLeBytes(bytes: Uint8Array, offset: number): number {
  return new DataView(
    bytes.buffer,
    bytes.byteOffset,
    bytes.byteLength,
  ).getUint32(offset, true);
}

/**
 * Validates that an SS58 format is an integer in [0, 16383] (the prefix encodes
 * 14 bits).
 *
 * The Rust implementation enforces this through the u16 type plus a range check; the
 * Python implementation performs the equivalent runtime check. Reserved formats 46
 * and 47 remain valid expected-format inputs for decoders; the SS58 layer rejects them
 * only when an encoder tries to put one on the wire.
 *
 * @param {number} ss58Format - The SS58 format to validate.
 * @throws {RangeError} - Thrown if the format is not an integer in [0, 16383].
 */
function validateSs58Format(ss58Format: number): void {
  // -0 is rejected like the identifier validation rejects it (see `isValidU32`): the
  // one integral double JavaScript can tell apart from its integer twin.
  if (
    !Number.isInteger(ss58Format) ||
    Object.is(ss58Format, -0) ||
    ss58Format < 0 ||
    ss58Format > SS58_FORMAT_MAX
  ) {
    throw new RangeError(
      `ss58Format must be an integer in [0, ${SS58_FORMAT_MAX}], got ${describeRejectedValue(ss58Format)}`,
    );
  }
}

// The SS58 formats that can never appear in an address (per the SS58 registry).
const RESERVED_SS58_FORMATS = [46, 47];

/**
 * Validates an SS58 format that is about to be put on the wire.
 *
 * Everything `validateSs58Format` rejects, plus the reserved formats 46 and 47: no address
 * may carry them, so an encoder cannot honour the request. Decoders deliberately do *not*
 * apply this check — a reserved expected-format is a well-formed question whose answer is
 * "no address matches", i.e. an `Ss58FormatMismatchError`.
 *
 * An unusable format is a rejected *argument* however it is unusable, so it throws the same
 * `RangeError` as an out-of-range one rather than an `Ss58Error`, which is a report about an
 * *address*. The Rust encoder makes the same distinction with its dedicated
 * `InvalidSs58Format` type, and the Python one raises `ValueError` here.
 *
 * @param {number} ss58Format - The SS58 format to validate.
 * @throws {RangeError} - Thrown if the format is not an integer in [0, 16383], or is
 *   reserved (46, 47).
 */
function validateSs58FormatForEncoding(ss58Format: number): void {
  validateSs58Format(ss58Format);
  if (RESERVED_SS58_FORMATS.includes(ss58Format)) {
    throw new RangeError(
      `ss58Format must not be a reserved format (${RESERVED_SS58_FORMATS.join(", ")}), got ${ss58Format}`,
    );
  }
}

/**
 * Validates that a name is exactly `ACCOUNT_NAME_LENGTH` UTF-8 bytes long and contains
 * only allowed ASCII characters (`0-9 a-z A-Z - #`).
 *
 * The length is measured in **UTF-8 bytes**, exactly like the Rust `AccountName::new`,
 * which validates the encoded name bytes that go on the wire: the name occupies bytes
 * [5..15] of the account id, so a name of the wrong byte length is a length failure
 * regardless of how many characters (or UTF-16 code units) it happens to have. A name
 * that is 10 bytes long but not pure ASCII then fails the charset check.
 *
 * @param {string} name - The name to validate.
 * @throws {AccountNameError} - Thrown if the name has a wrong byte length
 *   (`InvalidLength`) or contains invalid characters (`InvalidCharacter`).
 */
function validateAccountName(name: string): void {
  validateString(name, "accountName");
  // Fast-reject on the UTF-16 unit count before walking the string: every unit encodes
  // to at least one UTF-8 byte, so a name longer than ACCOUNT_NAME_LENGTH units cannot
  // be ACCOUNT_NAME_LENGTH bytes. Without this, a caller-supplied name (the natural
  // input of encodeNamedAddress) was scanned in full — a linear amplification vector,
  // and the one entry point that did not bound its input up front. The Python
  // implementation applies the same short-circuit; the classification is unchanged.
  if (
    name.length > ACCOUNT_NAME_LENGTH ||
    utf8Length(name) !== ACCOUNT_NAME_LENGTH
  ) {
    throw new AccountNameError("InvalidLength");
  }
  if (!ACCOUNT_NAME_ALLOWED_CHARS.test(name)) {
    throw new AccountNameError("InvalidCharacter");
  }
}

/**
 * Checks that the given bytes form a valid account name: exactly 10 bytes, each one of the
 * allowed characters (`0-9 a-z A-Z - #`).
 *
 * Anyone can construct an account id that carries a valid checksum over arbitrary name
 * bytes (no private key is needed), so the name bytes must be validated during decoding.
 *
 * @param {Uint8Array} nameBytes - The raw name bytes of a Named address.
 * @returns {boolean} - True if the bytes are a valid account name.
 */
function isValidAccountNameBytes(nameBytes: Uint8Array): boolean {
  return (
    nameBytes.length === ACCOUNT_NAME_LENGTH &&
    nameBytes.every((byte) => ACCOUNT_NAME_ALLOWED_BYTES[byte] === 1)
  );
}

/**
 * The 32-byte blake2b-256 hash used for keyless account checksums.
 *
 * @param {Uint8Array} data - The data to hash.
 * @returns {Uint8Array} - The 32-byte digest.
 */
function blake2b256(data: Uint8Array): Uint8Array {
  return blake2b(data, { dkLen: 32 });
}

/**
 * Compares two byte arrays for equality. Constant time is not required: every input of
 * this library is a public identifier, no secrets are processed anywhere.
 *
 * @param {Uint8Array} a - The first array.
 * @param {Uint8Array} b - The second array.
 * @returns {boolean} - True if the arrays have equal length and contents.
 */
function bytesEqual(a: Uint8Array, b: Uint8Array): boolean {
  if (a.length !== b.length) {
    return false;
  }
  for (let i = 0; i < a.length; i += 1) {
    if (a[i] !== b[i]) {
      return false;
    }
  }
  return true;
}

/**
 * Builds the 32-byte keyless account id from its open part: the open part followed by the
 * trailing bytes of its blake2b-256 hash (the checksum).
 *
 * @param {Uint8Array} openPart - The open (plain-text) part of the account id.
 * @returns {Uint8Array} - The 32-byte account id.
 */
function accountIdFromOpenPart(openPart: Uint8Array): Uint8Array {
  const checksum = blake2b256(openPart);
  return concatBytes(openPart, checksum.slice(openPart.length));
}

/** The normalized `0x` + lowercase hex account id for the given open part. */
function hexAccountId(openPart: Uint8Array): string {
  return `0x${bytesToHex(accountIdFromOpenPart(openPart))}`;
}

/**
 * Builds a keyless account id from its open part and wraps it in an SS58 address.
 *
 * @param {Uint8Array} openPart - The open (plain-text) part of the account id.
 * @param {number} ss58Format - SS58 format (network prefix) to use for the address.
 * @returns {string} - The encoded SS58 address.
 */
function encodeKeylessAccount(
  openPart: Uint8Array,
  ss58Format: number,
): string {
  validateSs58FormatForEncoding(ss58Format);
  return ss58Encode(accountIdFromOpenPart(openPart), ss58Format);
}

/** The open part of an AppAgent account: AppAgent ID (LE) + the type identifier. */
function appAgentOpenPart(appAgentId: AppAgentId): Uint8Array {
  return concatBytes(
    u32ToLeBytes(appAgentId, "appAgentId"),
    Uint8Array.of(APP_AGENT_ACCOUNT_IDENTIFIER),
  );
}

/** The open part of a Transactional account: AppAgent ID, type identifier, TA ID. */
function transactionalOpenPart(
  appAgentId: AppAgentId,
  taId: TransactionalId,
): Uint8Array {
  return concatBytes(
    u32ToLeBytes(appAgentId, "appAgentId"),
    Uint8Array.of(TRANSACTIONAL_ACCOUNT_IDENTIFIER),
    u32ToLeBytes(taId, "taId"),
  );
}

/** The open part of a Named account: AppAgent ID, type identifier, the 10 name bytes. */
function namedOpenPart(appAgentId: AppAgentId, name: string): Uint8Array {
  // appAgentId is validated before the name, matching the Python implementation: when
  // both arguments are invalid, the offending id is reported first, in declaration order.
  const appAgentIdBytes = u32ToLeBytes(appAgentId, "appAgentId");
  validateAccountName(name);
  // The name is validated ASCII, so the char codes are exactly the encoded bytes.
  const nameBytes = Uint8Array.from(name, (c) => c.charCodeAt(0));
  return concatBytes(
    appAgentIdBytes,
    Uint8Array.of(NAMED_ACCOUNT_IDENTIFIER),
    nameBytes,
  );
}

/** The branded App Agent id, and the optional SS58 format of the resulting address. */
interface AppAgentAddressOptions {
  appAgentId: AppAgentId;
  ss58Format?: number;
}

/**
 * Encodes an App Agent root address.
 *
 * @param {AppAgentAddressOptions} options - The branded App Agent id and optional format.
 * @returns {string} - The encoded App Agent address.
 * @throws {TypeError} - Thrown if `options` is not an options object.
 * @throws {RangeError} - Thrown if appAgentId is not an integer in [0, 4294967295], or
 *   ss58Format is not an integer in [0, 16383] or is reserved (46, 47).
 */
function encodeAppAgentAddress(options: AppAgentAddressOptions): string {
  validateOptions(options, "options");
  const { appAgentId, ss58Format = SS58_FORMAT_TREVO_ASSET_HUB } = options;
  return encodeKeylessAccount(appAgentOpenPart(appAgentId), ss58Format);
}

/** The branded identifier that defines an App Agent root account. */
type AppAgentAccountIdentifiers = Omit<AppAgentAddressOptions, "ss58Format">;

/**
 * Encodes an AppAgent account id (the raw `0x` + 64 hex interchange form).
 *
 * The account-id counterpart of `encodeAppAgentAddress`: it returns the raw account id
 * (which carries no SS58 format) instead of an SS58 address — the form accepted by
 * `decodeAccountId` and `encodeAddress` and exposed as the `accountId` field of a decode
 * result. Mirrors `encode_app_agent_account` of the Rust implementation.
 *
 * @param {AppAgentAccountIdentifiers} options - The branded App Agent id.
 * @returns {string} - The account id: `0x` followed by 64 lowercase hex characters.
 * @throws {TypeError} - Thrown if `options` is not an options object.
 * @throws {RangeError} - Thrown if appAgentId is not an integer in [0, 4294967295].
 */
function encodeAppAgentAccountId(options: AppAgentAccountIdentifiers): string {
  validateOptions(options, "options");
  const { appAgentId } = options;
  return hexAccountId(appAgentOpenPart(appAgentId));
}

/** The branded identifiers, and the optional SS58 format of the resulting address. */
interface TransactionalAddressOptions {
  appAgentId: AppAgentId;
  taId: TransactionalId;
  ss58Format?: number;
}

/**
 * Encodes a Transactional address.
 *
 * @param {TransactionalAddressOptions} options - The branded identifiers and optional format.
 * @returns {string} - The encoded Transactional address.
 * @throws {TypeError} - Thrown if `options` is not an options object.
 * @throws {RangeError} - Thrown if appAgentId or taId is not an integer in [0, 4294967295],
 *   or ss58Format is not an integer in [0, 16383] or is reserved (46, 47).
 */
function encodeTransactionalAddress(
  options: TransactionalAddressOptions,
): string {
  validateOptions(options, "options");
  const {
    appAgentId,
    taId,
    ss58Format = SS58_FORMAT_TREVO_ASSET_HUB,
  } = options;
  return encodeKeylessAccount(
    transactionalOpenPart(appAgentId, taId),
    ss58Format,
  );
}

/** The branded identifiers that define a Transactional account. */
type TransactionalAccountIdentifiers = Omit<
  TransactionalAddressOptions,
  "ss58Format"
>;

/**
 * Encodes a Transactional account id (the raw `0x` + 64 hex interchange form).
 *
 * The account-id counterpart of `encodeTransactionalAddress`. Mirrors
 * `encode_transactional_account` of the Rust implementation.
 *
 * @param {TransactionalAccountIdentifiers} options - The branded account identifiers.
 * @returns {string} - The account id: `0x` followed by 64 lowercase hex characters.
 * @throws {TypeError} - Thrown if `options` is not an options object.
 * @throws {RangeError} - Thrown if appAgentId or taId is not an integer in [0, 4294967295].
 */
function encodeTransactionalAccountId(
  options: TransactionalAccountIdentifiers,
): string {
  validateOptions(options, "options");
  const { appAgentId, taId } = options;
  return hexAccountId(transactionalOpenPart(appAgentId, taId));
}

/** The App Agent and account name, and the optional SS58 format of the resulting address. */
interface NamedAddressOptions {
  appAgentId: AppAgentId;
  accountName: AccountName;
  ss58Format?: number;
}

/**
 * Encodes a Named address.
 *
 * @param {NamedAddressOptions} options - The App Agent, account name, and optional format.
 * @returns {string} - The encoded Named address.
 * @throws {TypeError} - Thrown if `options` is not an options object.
 * @throws {AccountNameError} - Thrown if the account name is invalid.
 * @throws {RangeError} - Thrown if appAgentId is not an integer in [0, 4294967295], or
 *   ss58Format is not an integer in [0, 16383] or is reserved (46, 47).
 */
function encodeNamedAddress(options: NamedAddressOptions): string {
  validateOptions(options, "options");
  const {
    appAgentId,
    accountName,
    ss58Format = SS58_FORMAT_TREVO_ASSET_HUB,
  } = options;
  return encodeKeylessAccount(
    namedOpenPart(appAgentId, accountName),
    ss58Format,
  );
}

/** The branded identifiers that define a Named account. */
type NamedAccountIdentifiers = Omit<NamedAddressOptions, "ss58Format">;

/**
 * Encodes a Named account id (the raw `0x` + 64 hex interchange form).
 *
 * The account-id counterpart of `encodeNamedAddress`. Mirrors `encode_named_account` of
 * the Rust implementation.
 *
 * @param {NamedAccountIdentifiers} options - The App Agent and account name.
 * @returns {string} - The account id: `0x` followed by 64 lowercase hex characters.
 * @throws {TypeError} - Thrown if `options` is not an options object.
 * @throws {AccountNameError} - Thrown if the account name is invalid.
 * @throws {RangeError} - Thrown if appAgentId is not an integer in [0, 4294967295].
 */
function encodeNamedAccountId(options: NamedAccountIdentifiers): string {
  validateOptions(options, "options");
  const { appAgentId, accountName } = options;
  return hexAccountId(namedOpenPart(appAgentId, accountName));
}

/**
 * Resolves an SS58 address string to its raw 32-byte account id.
 *
 * @param {string} address - SS58 address of a 32-byte account id.
 * @param {number} ss58Format - SS58 format (network prefix) the address must use.
 * @returns {Uint8Array} - The 32-byte account id.
 * @throws {RangeError} - Thrown if ss58Format is not an integer in [0, 16383].
 * @throws {NotAnAddressError} - Thrown if the input is a raw `0x` account id.
 * @throws {Ss58Error} - Thrown if the input is not a valid SS58 address of a 32-byte
 *   account id in the expected format.
 */
function decodeSs58AccountId(address: string, ss58Format: number): Uint8Array {
  validateSs58Format(ss58Format);
  validateString(address, "address");

  if (address.startsWith("0x")) {
    // Raw account ids are deliberately not accepted here: they carry no SS58 format
    // and would silently bypass the expected-format check. decodeAccountId is the
    // entry point for raw account ids.
    throw new NotAnAddressError();
  }

  // ss58Decode enforces the canonical structure exactly like the Rust reference
  // implementation: the 50-character pre-bound, a 1- or 2-byte prefix, a 32-byte
  // account id, a full two-byte checksum, and a non-reserved format.
  const { accountId, format } = ss58Decode(address);
  if (format !== ss58Format) {
    throw Ss58Error.unexpectedFormat(ss58Format, format);
  }

  return accountId;
}

/**
 * Classifies a 32-byte account id and extracts any embedded keyless identifiers.
 *
 * Generic in the address so the caller's invariant carries into the result: the address
 * entry points hand it a `string`, `decodeAccountId` hands it `null`, and each gets back a
 * result whose `address` field is typed accordingly.
 *
 * @param {string | null} address - The SS58 address the account id was decoded from,
 *   stored in the `address` field of the result; null when the input was a raw
 *   account id.
 * @param {Uint8Array} accountId - The 32-byte account id.
 * @returns {BlockchainAccountInfo} - An object containing the decoded address type and data.
 */
function classifyAccountId<Address extends string | null>(
  address: Address,
  accountId: Uint8Array,
): BlockchainAccountInfo<Address> {
  const accountIdHex = `0x${bytesToHex(accountId)}`;

  // Read the byte that encodes type of the address
  const accountTypeByte = accountId[4];

  if (accountTypeByte === APP_AGENT_ACCOUNT_IDENTIFIER) {
    // Verify checksum
    const openPartSize = 5;
    const openPart = accountId.slice(0, openPartSize);
    const checksum = accountId.slice(openPartSize);
    const checksumCalculated = blake2b256(openPart).slice(openPartSize);
    if (bytesEqual(checksum, checksumCalculated)) {
      // Extract and decode appAgentId
      const decodedAppAgentId = appAgentId(u32FromLeBytes(accountId, 0));

      return Object.freeze({
        address,
        accountId: accountIdHex,
        accountType: AccountType.AppAgent,
        appAgentId: decodedAppAgentId,
      });
    }
  }

  if (accountTypeByte === TRANSACTIONAL_ACCOUNT_IDENTIFIER) {
    // Verify checksum
    const openPartSize = 9;
    const openPart = accountId.slice(0, openPartSize);
    const checksum = accountId.slice(openPartSize);
    const checksumCalculated = blake2b256(openPart).slice(openPartSize);
    if (bytesEqual(checksum, checksumCalculated)) {
      // Extract and decode appAgentId and taId
      const decodedAppAgentId = appAgentId(u32FromLeBytes(accountId, 0));
      const decodedTaId = transactionalId(u32FromLeBytes(accountId, 5));

      return Object.freeze({
        address,
        accountId: accountIdHex,
        accountType: AccountType.Transactional,
        appAgentId: decodedAppAgentId,
        taId: decodedTaId,
      });
    }
  }

  if (accountTypeByte === NAMED_ACCOUNT_IDENTIFIER) {
    // Verify checksum
    const openPartSize = 15;
    const openPart = accountId.slice(0, openPartSize);
    const checksum = accountId.slice(openPartSize);
    const checksumCalculated = blake2b256(openPart).slice(openPartSize);
    // The name bytes must additionally pass charset validation: an address with a valid
    // checksum over invalid name bytes is classified as Regular, in line with the Rust
    // implementation.
    const nameBytes = accountId.slice(5, 15);
    if (
      bytesEqual(checksum, checksumCalculated) &&
      isValidAccountNameBytes(nameBytes)
    ) {
      // Extract and decode appAgentId and accountName. The name bytes have just passed
      // charset validation and are exactly 10 ASCII bytes, so the brand is sound without
      // re-running accountName().
      const decodedAppAgentId = appAgentId(u32FromLeBytes(accountId, 0));
      const decodedName = String.fromCharCode(...nameBytes) as AccountName;

      return Object.freeze({
        address,
        accountId: accountIdHex,
        accountType: AccountType.Named,
        appAgentId: decodedAppAgentId,
        accountName: decodedName,
      });
    }
  }

  return Object.freeze({
    address,
    accountId: accountIdHex,
    accountType: AccountType.Regular,
  });
}

/**
 * Decodes an SS58 blockchain address and classifies it.
 *
 * Accepts an SS58 address string of a 32-byte account id in the expected format. Raw
 * account ids in hex form are rejected here because they carry no SS58 format; use
 * `decodeAccountId` for those.
 *
 * @param {string} address - The SS58 address to decode.
 * @returns {BlockchainAccountInfo} - An object containing the decoded address type and data.
 * @throws {TypeError} - Thrown if the address is not a string.
 * @throws {RangeError} - Thrown if ss58Format is not an integer in [0, 16383].
 * @throws {NotAnAddressError} - Thrown if the input is a raw `0x` account id.
 * @throws {Ss58Error} - Thrown if the address is not a valid SS58 string of a 32-byte
 *   account id in the expected format; `Ss58FormatMismatchError` for a valid address of
 *   another format.
 */
function decodeAddress(
  address: BlockchainAddress,
  ss58Format: Ss58Format = SS58_FORMAT_TREVO_ASSET_HUB,
): BlockchainAccountInfo<string> {
  return classifyAccountId(address, decodeSs58AccountId(address, ss58Format));
}

/**
 * The result of `decodeAddressWithFormat`: the classification of the address and the
 * SS58 format found in it. (The Python implementation returns the same pair as the
 * `DecodedAddressWithFormat` named tuple; the Rust `from_ss58check_with_version`
 * returns the raw `(AccountId32, u16)` pair.)
 */
interface DecodedAddressWithFormat {
  readonly info: BlockchainAccountInfo<string>;
  readonly ss58Format: number;
}

/**
 * Decodes an SS58 blockchain address of *any* (non-reserved) format and classifies it.
 *
 * The counterpart of `decodeAddress` for addresses whose SS58 format is not known up
 * front: instead of requiring an expected format, it returns the format found in the
 * address alongside the classification. Mirrors
 * `AccountId32::from_ss58check_with_version` of the Rust implementation (the Python
 * implementation exposes the same entry point as `decode_address_with_format`).
 *
 * The address is validated exactly like `decodeAddress` (canonical structure, length,
 * checksum; raw `0x` account ids are rejected outright) — only the expected-format
 * comparison is skipped, so an `Ss58Error` of kind `"UnexpectedFormat"` is never
 * thrown. Prefer `decodeAddress` whenever the expected format is known: accepting any
 * format silently admits addresses of other networks.
 *
 * @param {string} address - The SS58 address to decode.
 * @returns {DecodedAddressWithFormat} - The classification of the address (`info`) and
 *   the SS58 format found in it (`ss58Format`).
 * @throws {Ss58Error} - Thrown if the address is not a valid SS58 string of a 32-byte
 *   account id; its `kind` carries the failure classification.
 * @throws {NotAnAddressError} - Thrown if the input is a raw `0x` account id (use
 *   `decodeAccountId`).
 */
function decodeAddressWithFormat(
  address: BlockchainAddress,
): DecodedAddressWithFormat {
  validateString(address, "address");
  if (address.startsWith("0x")) {
    // Raw account ids are deliberately not accepted here: they carry no SS58 format
    // for this function to report. decodeAccountId is the entry point for raw
    // account ids.
    throw new NotAnAddressError();
  }
  const { accountId, format } = ss58Decode(address);
  return Object.freeze({
    info: classifyAccountId(address, accountId),
    ss58Format: format,
  });
}

/**
 * Decodes a raw 32-byte account id and classifies it.
 *
 * Mirrors `decode_account_info` of the Rust implementation, which takes the raw
 * `AccountId32` bytes. Raw account bytes carry no SS58 format, so unlike
 * `decodeAddress` this function has no ss58Format parameter.
 *
 * @param {string} accountId - The account id: `0x` followed by exactly 64 hex
 *   characters.
 * @returns {BlockchainAccountInfo} - An object containing the decoded address type and
 *   data; its `address` field is `null` (raw account bytes carry no address; render
 *   one with `encodeAddress`).
 * @throws {ParseAccountIdError} - Thrown if the input is not `0x` followed by exactly 64
 *   hex characters.
 */
function decodeAccountId(
  accountId: BlockchainAccountId,
): BlockchainAccountInfo<null> {
  validateString(accountId, "accountId");
  if (!HEX_ACCOUNT_ID.test(accountId)) {
    throw new ParseAccountIdError();
  }
  return classifyAccountId(null, hexToBytes(accountId.slice(2)));
}

/**
 * Encodes a raw 32-byte account id as an SS58 address.
 *
 * The counterpart of `decodeAddress`: any account id obtained from
 * `BlockchainAccountInfo.accountId` (or from chain state) renders as its canonical
 * address string. Mirrors `AccountId32::to_ss58check_with_version` of the Rust
 * implementation. The input contract is `decodeAccountId`'s: `0x` followed by exactly
 * 64 hex characters (both digit cases accepted).
 *
 * @param {string} accountId - The account id: `0x` followed by exactly 64 hex
 *   characters.
 * @param {number} ss58Format - SS58 format (network prefix) to use for the address.
 * @returns {string} - The SS58 address.
 * @throws {ParseAccountIdError} - Thrown if the input is not `0x` followed by exactly 64
 *   hex characters.
 * @throws {RangeError} - Thrown if ss58Format is not an integer in [0, 16383], or is
 *   reserved (46, 47).
 */
function encodeAddress(
  accountId: BlockchainAccountId,
  ss58Format: Ss58Format = SS58_FORMAT_TREVO_ASSET_HUB,
): BlockchainAddress {
  // ss58Format first, then the payload — the same order as the decode path. The format is
  // a caller *parameter* (a programmer error) while the account id is *data* (frequently
  // untrusted), and a caller who got both wrong should hear about their own bug first.
  // Validating the payload first meant encodeAddress and decodeAddress disagreed about
  // which of two bad arguments to report.
  validateSs58FormatForEncoding(ss58Format);
  validateString(accountId, "accountId");
  if (!HEX_ACCOUNT_ID.test(accountId)) {
    throw new ParseAccountIdError();
  }
  return ss58Encode(hexToBytes(accountId.slice(2)), ss58Format);
}

/**
 * Decodes an application agent address.
 *
 * @param {string} address - The SS58 address to decode.
 * @returns {number} - The ID of the application agent.
 * @throws {TypeError} - Thrown if the address is not a string.
 * @throws {RangeError} - Thrown if ss58Format is not an integer in [0, 16383].
 * @throws {NotAnAddressError} - Thrown if the input is a raw `0x` account id.
 * @throws {Ss58Error} - Thrown if the address is not a valid SS58 string of a 32-byte
 *   account id in the expected format.
 * @throws {AccountTypeMismatchError} - Thrown if the address is not an AppAgent address.
 */
function decodeAppAgentAddress(
  address: string,
  ss58Format: number = SS58_FORMAT_TREVO_ASSET_HUB,
): AppAgentId {
  const decodingResult = decodeAddress(address, ss58Format);

  if (decodingResult.accountType !== AccountType.AppAgent) {
    throw new AccountTypeMismatchError(
      AccountType.AppAgent,
      decodingResult.accountType,
    );
  }
  return decodingResult.appAgentId;
}

/**
 * The identifiers embedded in a Transactional address, as returned by
 * `decodeTransactionalAddress`. The Python implementation returns the same branded pair as a
 * named tuple, and the Rust implementation uses `TransactionalAccountIdentifiers`.
 */
interface DecodedTransactionalAddress {
  readonly appAgentId: AppAgentId;
  readonly taId: TransactionalId;
}

/**
 * Decodes a transactional address.
 *
 * @param {string} address - The SS58 address to decode.
 * @returns {DecodedTransactionalAddress} - An object containing the IDs of the app agent
 *   and transactional address.
 * @throws {TypeError} - Thrown if the address is not a string.
 * @throws {RangeError} - Thrown if ss58Format is not an integer in [0, 16383].
 * @throws {NotAnAddressError} - Thrown if the input is a raw `0x` account id.
 * @throws {Ss58Error} - Thrown if the address is not a valid SS58 string of a 32-byte
 *   account id in the expected format.
 * @throws {AccountTypeMismatchError} - Thrown if the address is not a Transactional address.
 */
function decodeTransactionalAddress(
  address: string,
  ss58Format: number = SS58_FORMAT_TREVO_ASSET_HUB,
): DecodedTransactionalAddress {
  const decodingResult = decodeAddress(address, ss58Format);

  if (decodingResult.accountType !== AccountType.Transactional) {
    throw new AccountTypeMismatchError(
      AccountType.Transactional,
      decodingResult.accountType,
    );
  }
  return Object.freeze({
    appAgentId: decodingResult.appAgentId,
    taId: decodingResult.taId,
  });
}

/**
 * The identifiers embedded in a Named address, as returned by `decodeNamedAddress`. (The
 * Python implementation returns the same pair as a tuple.)
 */
interface DecodedNamedAddress {
  readonly appAgentId: AppAgentId;
  readonly accountName: AccountName;
}

/**
 * Decodes a named address.
 *
 * @param {string} address - The SS58 address to decode.
 * @returns {DecodedNamedAddress} - An object containing the ID of the application agent
 *   and the address name.
 * @throws {TypeError} - Thrown if the address is not a string.
 * @throws {RangeError} - Thrown if ss58Format is not an integer in [0, 16383].
 * @throws {NotAnAddressError} - Thrown if the input is a raw `0x` account id.
 * @throws {Ss58Error} - Thrown if the address is not a valid SS58 string of a 32-byte
 *   account id in the expected format.
 * @throws {AccountTypeMismatchError} - Thrown if the address is not a Named address.
 */
function decodeNamedAddress(
  address: string,
  ss58Format: number = SS58_FORMAT_TREVO_ASSET_HUB,
): DecodedNamedAddress {
  const decodingResult = decodeAddress(address, ss58Format);

  if (decodingResult.accountType !== AccountType.Named) {
    throw new AccountTypeMismatchError(
      AccountType.Named,
      decodingResult.accountType,
    );
  }
  return Object.freeze({
    appAgentId: decodingResult.appAgentId,
    accountName: decodingResult.accountName,
  });
}

/**
 * Decodes an AppAgent account id (the raw `0x` + 64 hex interchange form).
 *
 * The account-id counterpart of `decodeAppAgentAddress`, mirroring
 * `decode_app_agent_account` of the Rust implementation: it takes the raw account id
 * (which carries no SS58 format) instead of an SS58 address, so it has no ss58Format
 * parameter.
 *
 * @param {string} accountId - The account id: `0x` followed by exactly 64 hex characters.
 * @returns {AppAgentId} - The ID of the application agent.
 * @throws {ParseAccountIdError} - Thrown if the input is not `0x` followed by exactly 64
 *   hex characters.
 * @throws {AccountTypeMismatchError} - Thrown if the account id is not an AppAgent
 *   account id.
 */
function decodeAppAgentAccountId(accountId: string): AppAgentId {
  const decodingResult = decodeAccountId(accountId);

  if (decodingResult.accountType !== AccountType.AppAgent) {
    throw new AccountTypeMismatchError(
      AccountType.AppAgent,
      decodingResult.accountType,
    );
  }
  return decodingResult.appAgentId;
}

/**
 * Decodes a Transactional account id (the raw `0x` + 64 hex interchange form).
 *
 * The account-id counterpart of `decodeTransactionalAddress`, mirroring
 * `decode_transactional_account` of the Rust implementation.
 *
 * @param {string} accountId - The account id: `0x` followed by exactly 64 hex characters.
 * @returns {DecodedTransactionalAddress} - The App Agent and Transactional identifiers.
 * @throws {ParseAccountIdError} - Thrown if the input is not `0x` followed by exactly 64
 *   hex characters.
 * @throws {AccountTypeMismatchError} - Thrown if the account id is not a Transactional
 *   account id.
 */
function decodeTransactionalAccountId(
  accountId: string,
): DecodedTransactionalAddress {
  const decodingResult = decodeAccountId(accountId);

  if (decodingResult.accountType !== AccountType.Transactional) {
    throw new AccountTypeMismatchError(
      AccountType.Transactional,
      decodingResult.accountType,
    );
  }
  return Object.freeze({
    appAgentId: decodingResult.appAgentId,
    taId: decodingResult.taId,
  });
}

/**
 * Decodes a Named account id (the raw `0x` + 64 hex interchange form).
 *
 * The account-id counterpart of `decodeNamedAddress`, mirroring `decode_named_account`
 * of the Rust implementation.
 *
 * @param {string} accountId - The account id: `0x` followed by exactly 64 hex characters.
 * @returns {DecodedNamedAddress} - The App Agent identifier and the account name.
 * @throws {ParseAccountIdError} - Thrown if the input is not `0x` followed by exactly 64
 *   hex characters.
 * @throws {AccountTypeMismatchError} - Thrown if the account id is not a Named account
 *   id.
 */
function decodeNamedAccountId(accountId: string): DecodedNamedAddress {
  const decodingResult = decodeAccountId(accountId);

  if (decodingResult.accountType !== AccountType.Named) {
    throw new AccountTypeMismatchError(
      AccountType.Named,
      decodingResult.accountType,
    );
  }
  return Object.freeze({
    appAgentId: decodingResult.appAgentId,
    accountName: decodingResult.accountName,
  });
}

/**
 * Returns true if the address passes the keyless checksum verification (AppAgent,
 * Transactional, or Named), false for any other (Regular) address — including a Named
 * layout whose checksum is valid but whose name bytes fail charset validation.
 *
 * Mirrors `is_keyless_account` of the Rust implementation.
 *
 * @param {string} address - The SS58 address to check.
 * @returns {boolean} - True if the address is a keyless address.
 * @throws {TypeError} - Thrown if the address is not a string.
 * @throws {RangeError} - Thrown if ss58Format is not an integer in [0, 16383].
 * @throws {NotAnAddressError} - Thrown if the input is a raw `0x` account id.
 * @throws {Ss58Error} - Thrown if the address cannot be decoded at all.
 */
function isKeylessAddress(
  address: BlockchainAddress,
  ss58Format: Ss58Format = SS58_FORMAT_TREVO_ASSET_HUB,
): boolean {
  return decodeAddress(address, ss58Format).accountType !== AccountType.Regular;
}

/**
 * Returns true if the raw account id passes the keyless checksum verification (AppAgent,
 * Transactional, or Named), false for a Regular account id — including a Named layout
 * whose checksum is valid but whose name bytes fail charset validation.
 *
 * The account-id counterpart of `isKeylessAddress`: it classifies the raw `0x` + 64 hex
 * account id form (which carries no SS58 format) instead of an SS58 address, and is total
 * over well-formed account ids. Mirrors `is_keyless_account` of the Rust implementation.
 *
 * @param {string} accountId - The account id: `0x` followed by exactly 64 hex characters.
 * @returns {boolean} - True if the account id is keyless.
 * @throws {ParseAccountIdError} - Thrown if the input is not `0x` followed by exactly 64
 *   hex characters.
 */
function isKeylessAccountId(accountId: BlockchainAccountId): boolean {
  return decodeAccountId(accountId).accountType !== AccountType.Regular;
}

// Types are re-exported through `export type` so every file of this package stays erasable:
// a bundler that transpiles file-by-file (esbuild, swc, babel) cannot know that these names
// are types, and would emit a runtime re-export of a binding that does not exist. Pinned by
// `isolatedModules` in tsconfig.json. `AccountType` is deliberately in the value list below:
// it is a const object *and* a type, and `export {}` carries both meanings.
export type {
  AccountName,
  AppAgentId,
  TransactionalId,
  BlockchainAddress,
  BlockchainAccountId,
  Ss58Format,
  AppAgentAddressOptions,
  AppAgentAccountIdentifiers,
  TransactionalAddressOptions,
  TransactionalAccountIdentifiers,
  NamedAddressOptions,
  NamedAccountIdentifiers,
  AccountNameErrorKind,
  Ss58ErrorKind,
  BlockchainAccountInfo,
  RegularAccountInfo,
  AppAgentAccountInfo,
  TransactionalAccountInfo,
  NamedAccountInfo,
  DecodedAddressWithFormat,
  DecodedTransactionalAddress,
  DecodedNamedAddress,
};

export {
  accountName,
  AccountNameError,
  AccountType,
  AccountTypeMismatchError,
  NotAnAddressError,
  ParseAccountIdError,
  Ss58Error,
  Ss58FormatMismatchError,
  appAgentId,
  transactionalId,
  isAppAgentId,
  isTransactionalId,
  isAccountName,
  encodeAppAgentAddress,
  encodeTransactionalAddress,
  encodeNamedAddress,
  encodeAppAgentAccountId,
  encodeTransactionalAccountId,
  encodeNamedAccountId,
  decodeAppAgentAddress,
  decodeTransactionalAddress,
  decodeNamedAddress,
  decodeAppAgentAccountId,
  decodeTransactionalAccountId,
  decodeNamedAccountId,
  decodeAddress,
  decodeAddressWithFormat,
  decodeAccountId,
  encodeAddress,
  isKeylessAddress,
  isKeylessAccountId,
  ACCOUNT_ID_LENGTH,
  ACCOUNT_NAME_LENGTH,
  APP_AGENT_ACCOUNT_IDENTIFIER,
  TRANSACTIONAL_ACCOUNT_IDENTIFIER,
  NAMED_ACCOUNT_IDENTIFIER,
  SS58_FORMAT_TREVO_ASSET_HUB,
  SS58_MAX_ADDRESS_LENGTH,
  U32_MAX,
};
