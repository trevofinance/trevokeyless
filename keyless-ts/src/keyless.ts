import { blake2b } from "@noble/hashes/blake2.js";
import { bytesToHex, concatBytes, hexToBytes } from "@noble/hashes/utils.js";
import {
  SS58_MAX_ADDRESS_LENGTH,
  Ss58Error,
  Ss58ErrorKind,
  ss58Decode,
  ss58Encode,
} from "./ss58";

const APP_AGENT_ADDRESS_IDENTIFIER = 1;
const TRANSACTIONAL_ADDRESS_IDENTIFIER = 2;
const NAMED_ADDRESS_IDENTIFIER = 3;
const NAMED_ADDRESS_LENGTH = 10;
const ADDRESS_NAME_ALLOWED_CHARS = /^[0-9a-zA-Z\-#]+$/;
const SS58_FORMAT_TREVO_ASSET_HUB = 5335;

// Length in bytes of a blockchain account id.
const ACCOUNT_ID_LENGTH = 32;

// A raw account id accepted by decodeAccountId: 0x followed by exactly 64 hex
// characters. (The Python implementation uses the same expression; the Rust
// implementation has no string form at all - AccountId32 is constructed from bytes.)
const HEX_ACCOUNT_ID = /^0x[0-9a-fA-F]{64}$/;

const U32_MAX = 4294967295;

// Largest valid SS58 format: the prefix encodes 14 bits. Mirrors the SS58_FORMAT_MAX
// bound of the Rust implementation.
const SS58_FORMAT_MAX = 16383;

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
const AccountType = {
  Regular: 0,
  AppAgent: 1,
  Transactional: 2,
  Named: 3,
} as const;
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
 * Represents the information related to a blockchain address.
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
 */
interface BlockchainAccountInfo {
  address: string | null;
  accountId: string;
  accountType: AccountType;
  appAgentId: number | null;
  taId: number | null;
  accountName: string | null;
}

/**
 * Renders a rejected parameter for an error message: numbers verbatim (including NaN
 * and non-integers), anything else by its type, so a JavaScript caller passing e.g.
 * the string "123" sees `got string` rather than a message that looks like a valid
 * number was rejected. (The Python implementation reports the type the same way; the
 * Rust implementation enforces the types at compile time.)
 *
 * @param {unknown} value - The rejected value.
 * @returns {string} - The rendering for the error message.
 */
function describeRejectedValue(value: unknown): string {
  return typeof value === "number" ? String(value) : typeof value;
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
  if (!Number.isInteger(value) || value < 0 || value > U32_MAX) {
    throw new RangeError(
      `${label} must be an integer in [0, ${U32_MAX}], got ${describeRejectedValue(value)}`,
    );
  }
  const bytes = new Uint8Array(4);
  new DataView(bytes.buffer).setUint32(0, value, true);
  return bytes;
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
 * Python implementation performs the equivalent runtime check. Rejecting the format up
 * front gives the caller a clear message naming the parameter; the SS58 layer rejects
 * the reserved formats 46 and 47 separately.
 *
 * @param {number} ss58Format - The SS58 format to validate.
 * @throws {RangeError} - Thrown if the format is not an integer in [0, 16383].
 */
function validateSs58Format(ss58Format: number): void {
  if (
    !Number.isInteger(ss58Format) ||
    ss58Format < 0 ||
    ss58Format > SS58_FORMAT_MAX
  ) {
    throw new RangeError(
      `ss58Format must be an integer in [0, ${SS58_FORMAT_MAX}], got ${describeRejectedValue(ss58Format)}`,
    );
  }
}

/**
 * Validates that a name is exactly 10 characters long and contains only allowed ASCII
 * characters (`0-9 a-z A-Z - #`).
 *
 * @param {string} name - The name to validate.
 * @throws {Error} - Thrown if the name has a wrong length or contains invalid characters.
 */
function validateAddressName(name: string): void {
  if (name.length !== NAMED_ADDRESS_LENGTH) {
    throw new Error("Address name must be exactly 10 characters long");
  }
  if (!ADDRESS_NAME_ALLOWED_CHARS.test(name)) {
    throw new Error("Address name contains invalid characters");
  }
}

/**
 * Checks that the given bytes form a valid address name: exactly 10 bytes, each one of the
 * allowed characters (`0-9 a-z A-Z - #`).
 *
 * Anyone can construct an account id that carries a valid checksum over arbitrary name
 * bytes (no private key is needed), so the name bytes must be validated during decoding.
 *
 * @param {Uint8Array} nameBytes - The raw name bytes of a Named address.
 * @returns {boolean} - True if the bytes are a valid address name.
 */
function isValidAddressNameBytes(nameBytes: Uint8Array): boolean {
  return (
    nameBytes.length === NAMED_ADDRESS_LENGTH &&
    nameBytes.every((byte) =>
      ADDRESS_NAME_ALLOWED_CHARS.test(String.fromCharCode(byte)),
    )
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
  validateSs58Format(ss58Format);
  return ss58Encode(accountIdFromOpenPart(openPart), ss58Format);
}

/** The open part of an AppAgent account: AppAgent ID (LE) + the type identifier. */
function appAgentOpenPart(appAgentId: number): Uint8Array {
  return concatBytes(
    u32ToLeBytes(appAgentId, "appAgentId"),
    Uint8Array.of(APP_AGENT_ADDRESS_IDENTIFIER),
  );
}

/** The open part of a Transactional account: AppAgent ID, type identifier, TA ID. */
function transactionalOpenPart(appAgentId: number, taId: number): Uint8Array {
  return concatBytes(
    u32ToLeBytes(appAgentId, "appAgentId"),
    Uint8Array.of(TRANSACTIONAL_ADDRESS_IDENTIFIER),
    u32ToLeBytes(taId, "taId"),
  );
}

/** The open part of a Named account: AppAgent ID, type identifier, the 10 name bytes. */
function namedOpenPart(appAgentId: number, name: string): Uint8Array {
  // appAgentId is validated before the name, matching the Python implementation: when
  // both arguments are invalid, the offending id is reported first, in declaration order.
  const appAgentIdBytes = u32ToLeBytes(appAgentId, "appAgentId");
  validateAddressName(name);
  // The name is validated ASCII, so the char codes are exactly the encoded bytes.
  const nameBytes = Uint8Array.from(name, (c) => c.charCodeAt(0));
  return concatBytes(
    appAgentIdBytes,
    Uint8Array.of(NAMED_ADDRESS_IDENTIFIER),
    nameBytes,
  );
}

/**
 * Encodes an application agent address and returns it.
 *
 * @param {number} appAgentId - The ID of the application agent.
 * @returns {string} - The encoded application agent address.
 */
function encodeAppAgentAddress(
  appAgentId: number,
  ss58Format: number = SS58_FORMAT_TREVO_ASSET_HUB,
): string {
  return encodeKeylessAccount(appAgentOpenPart(appAgentId), ss58Format);
}

/**
 * Encodes an AppAgent account id (the raw `0x` + 64 hex interchange form).
 *
 * The account-id counterpart of `encodeAppAgentAddress`: it returns the raw account id
 * (which carries no SS58 format) instead of an SS58 address — the form accepted by
 * `decodeAccountId` and `encodeAddress` and exposed as the `accountId` field of a decode
 * result. Mirrors `encode_app_agent_account` of the Rust implementation.
 *
 * @param {number} appAgentId - The ID of the application agent.
 * @returns {string} - The account id: `0x` followed by 64 lowercase hex characters.
 */
function encodeAppAgentAccountId(appAgentId: number): string {
  return hexAccountId(appAgentOpenPart(appAgentId));
}

/**
 * Encodes a transactional address and returns it.
 *
 * @param {number} appAgentId - The ID of the application agent.
 * @param {number} taId - The ID of the transactional address.
 * @returns {string} - The encoded transactional address.
 */
function encodeTransactionalAddress(
  appAgentId: number,
  taId: number,
  ss58Format: number = SS58_FORMAT_TREVO_ASSET_HUB,
): string {
  return encodeKeylessAccount(
    transactionalOpenPart(appAgentId, taId),
    ss58Format,
  );
}

/**
 * Encodes a Transactional account id (the raw `0x` + 64 hex interchange form).
 *
 * The account-id counterpart of `encodeTransactionalAddress`. Mirrors
 * `encode_transactional_account` of the Rust implementation.
 *
 * @param {number} appAgentId - The ID of the application agent.
 * @param {number} taId - The ID of the transactional address.
 * @returns {string} - The account id: `0x` followed by 64 lowercase hex characters.
 */
function encodeTransactionalAccountId(
  appAgentId: number,
  taId: number,
): string {
  return hexAccountId(transactionalOpenPart(appAgentId, taId));
}

/**
 * Encodes a named address and returns it.
 *
 * @param {number} appAgentId - The ID of the application agent.
 * @param {string} name - The name of the address.
 * @returns {string} - The encoded named address.
 */
function encodeNamedAddress(
  appAgentId: number,
  name: string,
  ss58Format: number = SS58_FORMAT_TREVO_ASSET_HUB,
): string {
  return encodeKeylessAccount(namedOpenPart(appAgentId, name), ss58Format);
}

/**
 * Encodes a Named account id (the raw `0x` + 64 hex interchange form).
 *
 * The account-id counterpart of `encodeNamedAddress`. Mirrors `encode_named_account` of
 * the Rust implementation.
 *
 * @param {number} appAgentId - The ID of the application agent.
 * @param {string} name - The name of the address.
 * @returns {string} - The account id: `0x` followed by 64 lowercase hex characters.
 */
function encodeNamedAccountId(appAgentId: number, name: string): string {
  return hexAccountId(namedOpenPart(appAgentId, name));
}

/**
 * Resolves an SS58 address string to its raw 32-byte account id.
 *
 * @param {string} address - SS58 address of a 32-byte account id.
 * @param {number} ss58Format - SS58 format (network prefix) the address must use.
 * @returns {Uint8Array} - The 32-byte account id.
 * @throws {Error} - Thrown if ss58Format is not an integer in [0, 16383], or the input
 *   is not a valid SS58 address of a 32-byte account id in the expected format.
 */
function decodeSs58AccountId(address: string, ss58Format: number): Uint8Array {
  validateSs58Format(ss58Format);

  if (address.startsWith("0x")) {
    // Raw account ids are deliberately not accepted here: they carry no SS58 format
    // and would silently bypass the expected-format check. decodeAccountId is the
    // entry point for raw account ids.
    throw new Error(
      "Raw account id hex is not an SS58 address; use decodeAccountId",
    );
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
 * @param {string | null} address - The SS58 address the account id was decoded from,
 *   stored in the `address` field of the result; null when the input was a raw
 *   account id.
 * @param {Uint8Array} accountId - The 32-byte account id.
 * @returns {BlockchainAccountInfo} - An object containing the decoded address type and data.
 */
function classifyAccountId(
  address: string | null,
  accountId: Uint8Array,
): BlockchainAccountInfo {
  const accountIdHex = `0x${bytesToHex(accountId)}`;

  // Read the byte that encodes type of the address
  const accountTypeByte = accountId[4];

  if (accountTypeByte === APP_AGENT_ADDRESS_IDENTIFIER) {
    // Verify checksum
    const openPartSize = 5;
    const openPart = accountId.slice(0, openPartSize);
    const checksum = accountId.slice(openPartSize);
    const checksumCalculated = blake2b256(openPart).slice(openPartSize);
    if (bytesEqual(checksum, checksumCalculated)) {
      // Extract and decode appAgentId
      const appAgentId = u32FromLeBytes(accountId, 0);

      return {
        address,
        accountId: accountIdHex,
        accountType: AccountType.AppAgent,
        appAgentId,
        taId: null,
        accountName: null,
      };
    }
  }

  if (accountTypeByte === TRANSACTIONAL_ADDRESS_IDENTIFIER) {
    // Verify checksum
    const openPartSize = 9;
    const openPart = accountId.slice(0, openPartSize);
    const checksum = accountId.slice(openPartSize);
    const checksumCalculated = blake2b256(openPart).slice(openPartSize);
    if (bytesEqual(checksum, checksumCalculated)) {
      // Extract and decode appAgentId and taId
      const appAgentId = u32FromLeBytes(accountId, 0);
      const taId = u32FromLeBytes(accountId, 5);

      return {
        address,
        accountId: accountIdHex,
        accountType: AccountType.Transactional,
        appAgentId,
        taId,
        accountName: null,
      };
    }
  }

  if (accountTypeByte === NAMED_ADDRESS_IDENTIFIER) {
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
      isValidAddressNameBytes(nameBytes)
    ) {
      // Extract and decode appAgentId and accountName
      const appAgentId = u32FromLeBytes(accountId, 0);
      const accountName = String.fromCharCode(...nameBytes);

      return {
        address,
        accountId: accountIdHex,
        accountType: AccountType.Named,
        appAgentId,
        taId: null,
        accountName,
      };
    }
  }

  return {
    address,
    accountId: accountIdHex,
    accountType: AccountType.Regular,
    appAgentId: null,
    taId: null,
    accountName: null,
  };
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
 * @throws {Error} - Thrown if ss58Format is not an integer in [0, 16383], or the
 *   address is not a valid SS58 string of a 32-byte account id in the expected format.
 */
function decodeAddress(
  address: string,
  ss58Format: number = SS58_FORMAT_TREVO_ASSET_HUB,
): BlockchainAccountInfo {
  return classifyAccountId(address, decodeSs58AccountId(address, ss58Format));
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
 * @throws {Error} - Thrown if the input is not `0x` followed by exactly 64 hex
 *   characters.
 */
function decodeAccountId(accountId: string): BlockchainAccountInfo {
  if (!HEX_ACCOUNT_ID.test(accountId)) {
    throw new Error(
      "Invalid hex account id: expected 0x followed by 64 hex characters",
    );
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
 * @throws {Error} - Thrown if the input is not `0x` followed by exactly 64 hex
 *   characters, or ss58Format is not an integer in [0, 16383].
 * @throws {Ss58Error} - Kind `InvalidFormat` if ss58Format is reserved (46, 47).
 */
function encodeAddress(
  accountId: string,
  ss58Format: number = SS58_FORMAT_TREVO_ASSET_HUB,
): string {
  if (!HEX_ACCOUNT_ID.test(accountId)) {
    throw new Error(
      "Invalid hex account id: expected 0x followed by 64 hex characters",
    );
  }
  validateSs58Format(ss58Format);
  return ss58Encode(hexToBytes(accountId.slice(2)), ss58Format);
}

/**
 * Decodes an application agent address.
 *
 * @param {string} address - The SS58 address to decode.
 * @returns {number} - The ID of the application agent.
 * @throws {AccountTypeMismatchError} - Thrown if the address is not an AppAgent address.
 */
function decodeAppAgentAddress(
  address: string,
  ss58Format: number = SS58_FORMAT_TREVO_ASSET_HUB,
): number {
  const decodingResult = decodeAddress(address, ss58Format);

  if (decodingResult.accountType !== AccountType.AppAgent) {
    throw new AccountTypeMismatchError(
      AccountType.AppAgent,
      decodingResult.accountType,
    );
  }
  if (
    decodingResult.appAgentId === undefined ||
    decodingResult.appAgentId === null
  ) {
    throw new Error(
      "Internal error in trevokeyless - appAgentId of AppAgent address is undefined",
    );
  }

  return decodingResult.appAgentId;
}

/**
 * The identifiers embedded in a Transactional address, as returned by
 * `decodeTransactionalAddress`. (The Python implementation returns the same pair as a tuple;
 * its `TransactionalId` type alias is the plain `taId` integer instead.)
 */
interface DecodedTransactionalAddress {
  appAgentId: number;
  taId: number;
}

/**
 * Decodes a transactional address.
 *
 * @param {string} address - The SS58 address to decode.
 * @returns {DecodedTransactionalAddress} - An object containing the IDs of the app agent
 *   and transactional address.
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
  if (
    decodingResult.appAgentId === undefined ||
    decodingResult.appAgentId === null
  ) {
    throw new Error(
      "Internal error in trevokeyless - appAgentId of Transactional address is undefined",
    );
  }
  if (decodingResult.taId === undefined || decodingResult.taId === null) {
    throw new Error(
      "Internal error in trevokeyless - taId of Transactional address is undefined",
    );
  }

  return {
    appAgentId: decodingResult.appAgentId,
    taId: decodingResult.taId,
  };
}

/**
 * The identifiers embedded in a Named address, as returned by `decodeNamedAddress`. (The
 * Python implementation returns the same pair as a tuple.)
 */
interface DecodedNamedAddress {
  appAgentId: number;
  accountName: string;
}

/**
 * Decodes a named address.
 *
 * @param {string} address - The SS58 address to decode.
 * @returns {DecodedNamedAddress} - An object containing the ID of the application agent
 *   and the address name.
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
  if (
    decodingResult.appAgentId === undefined ||
    decodingResult.appAgentId === null
  ) {
    throw new Error(
      "Internal error in trevokeyless - appAgentId of Named address is undefined",
    );
  }
  if (
    decodingResult.accountName === undefined ||
    decodingResult.accountName === null
  ) {
    throw new Error(
      "Internal error in trevokeyless - accountName of Named address is undefined",
    );
  }

  return {
    appAgentId: decodingResult.appAgentId,
    accountName: decodingResult.accountName,
  };
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
 * @throws {Error} - Thrown if the address cannot be decoded at all.
 */
function isKeylessAddress(
  address: string,
  ss58Format: number = SS58_FORMAT_TREVO_ASSET_HUB,
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
 * @throws {Error} - Thrown if the input is not `0x` followed by exactly 64 hex characters.
 */
function isKeylessAccountId(accountId: string): boolean {
  return decodeAccountId(accountId).accountType !== AccountType.Regular;
}

export {
  AccountType,
  AccountTypeMismatchError,
  Ss58Error,
  Ss58ErrorKind,
  BlockchainAccountInfo,
  DecodedTransactionalAddress,
  DecodedNamedAddress,
  encodeAppAgentAddress,
  encodeTransactionalAddress,
  encodeNamedAddress,
  encodeAppAgentAccountId,
  encodeTransactionalAccountId,
  encodeNamedAccountId,
  decodeAppAgentAddress,
  decodeTransactionalAddress,
  decodeNamedAddress,
  decodeAddress,
  decodeAccountId,
  encodeAddress,
  isKeylessAddress,
  isKeylessAccountId,
  ACCOUNT_ID_LENGTH,
  NAMED_ADDRESS_LENGTH,
  SS58_FORMAT_TREVO_ASSET_HUB,
  SS58_MAX_ADDRESS_LENGTH,
  U32_MAX,
};
