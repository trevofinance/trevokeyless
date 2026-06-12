import { blake2b } from "@noble/hashes/blake2b";
import { bytesToHex, concatBytes, hexToBytes } from "@noble/hashes/utils";
import {
  SS58_MAX_ADDRESS_LENGTH,
  Ss58Error,
  ss58Decode,
  ss58Encode,
} from "./ss58";

const APP_AGENT_ADDRESS_IDENTIFIER = 1;
const TRANSACTIONAL_ADDRESS_IDENTIFIER = 2;
const NAMED_ADDRESS_IDENTIFIER = 3;
const NAMED_ADDRESS_BYTES_LENGTH = 10;
const ADDRESS_NAME_ALLOWED_CHARS = /^[0-9a-zA-Z\-#]+$/;
const SS58_FORMAT_TREVO_ASSET_HUB = 5335;

// Length in bytes of a blockchain account id.
const ACCOUNT_ID_BYTES_LENGTH = 32;

// A raw account id accepted by decodeAccountId: 0x followed by exactly 64 hex
// characters. (The Python implementation uses the same expression; the Rust
// implementation has no string form at all - AccountId32 is constructed from bytes.)
const HEX_ACCOUNT_ID = /^0x[0-9a-fA-F]{64}$/;

const U32_MAX = 4294967295;

// Largest valid SS58 format: the prefix encodes 14 bits. Mirrors the SS58_FORMAT_MAX
// bound of the Rust implementation.
const SS58_FORMAT_MAX = 16383;

enum AddressType {
  Regular = 0,
  AppAgent = 1,
  Transactional = 2,
  Named = 3,
}

/**
 * Thrown by the typed decode functions when the address is of a different type.
 *
 * Carries the expected and the actual (found) classification of the address, mirroring
 * the `KeylessDecodeError` of the Rust implementation. The message is identical to the
 * Python implementation's.
 */
class AddressTypeMismatchError extends Error {
  readonly expected: AddressType;

  readonly found: AddressType;

  constructor(expected: AddressType, found: AddressType) {
    const article = expected === AddressType.AppAgent ? "an" : "a";
    super(
      `Provided address is not ${article} ${AddressType[expected]} address but is instance of ${AddressType[found]}`,
    );
    this.name = "AddressTypeMismatchError";
    this.expected = expected;
    this.found = found;
  }
}

// Represents the information related to a blockchain address.
interface BlockchainAddressInfo {
  address: string;
  accountId: string;
  addressType: AddressType;
  appAgentId: number | null;
  taId: number | null;
  addressName: string | null;
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
      `${label} must be an integer in [0, ${U32_MAX}], got ${value}`,
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
 * @throws {Error} - Thrown if the format is not an integer in [0, 16383].
 */
function validateSs58Format(ss58Format: number): void {
  if (
    !Number.isInteger(ss58Format) ||
    ss58Format < 0 ||
    ss58Format > SS58_FORMAT_MAX
  ) {
    throw new Error(
      `ss58Format must be an integer in [0, ${SS58_FORMAT_MAX}], got ${ss58Format}`,
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
  if (name.length !== NAMED_ADDRESS_BYTES_LENGTH) {
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
    nameBytes.length === NAMED_ADDRESS_BYTES_LENGTH &&
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
 * Builds a keyless account id from its open part and wraps it in an SS58 address.
 *
 * The 32-byte account id is `openPart ++ blake2_256(openPart)[openPart.length..]` —
 * the trailing bytes of the hash serve as the checksum.
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
  const checksum = blake2b256(openPart);
  const accountId = concatBytes(openPart, checksum.slice(openPart.length));
  return ss58Encode(accountId, ss58Format);
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
  const openPart = concatBytes(
    u32ToLeBytes(appAgentId, "appAgentId"),
    Uint8Array.of(APP_AGENT_ADDRESS_IDENTIFIER),
  );
  return encodeKeylessAccount(openPart, ss58Format);
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
  const openPart = concatBytes(
    u32ToLeBytes(appAgentId, "appAgentId"),
    Uint8Array.of(TRANSACTIONAL_ADDRESS_IDENTIFIER),
    u32ToLeBytes(taId, "taId"),
  );
  return encodeKeylessAccount(openPart, ss58Format);
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
  validateAddressName(name);

  // The name is validated ASCII, so the char codes are exactly the encoded bytes.
  const nameBytes = Uint8Array.from(name, (c) => c.charCodeAt(0));
  const openPart = concatBytes(
    u32ToLeBytes(appAgentId, "appAgentId"),
    Uint8Array.of(NAMED_ADDRESS_IDENTIFIER),
    nameBytes,
  );
  return encodeKeylessAccount(openPart, ss58Format);
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
 * @param {string} address - The original input (SS58 address or account id hex),
 *   stored verbatim in the `address` field of the result.
 * @param {Uint8Array} accountId - The 32-byte account id.
 * @returns {BlockchainAddressInfo} - An object containing the decoded address type and data.
 */
function classifyAccountId(
  address: string,
  accountId: Uint8Array,
): BlockchainAddressInfo {
  const accountIdHex = `0x${bytesToHex(accountId)}`;

  // Read the byte that encodes type of the address
  const addressTypeByte = accountId[4];

  if (addressTypeByte === APP_AGENT_ADDRESS_IDENTIFIER) {
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
        addressType: AddressType.AppAgent,
        appAgentId,
        taId: null,
        addressName: null,
      };
    }
  }

  if (addressTypeByte === TRANSACTIONAL_ADDRESS_IDENTIFIER) {
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
        addressType: AddressType.Transactional,
        appAgentId,
        taId,
        addressName: null,
      };
    }
  }

  if (addressTypeByte === NAMED_ADDRESS_IDENTIFIER) {
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
      // Extract and decode appAgentId and addressName
      const appAgentId = u32FromLeBytes(accountId, 0);
      const addressName = String.fromCharCode(...nameBytes);

      return {
        address,
        accountId: accountIdHex,
        addressType: AddressType.Named,
        appAgentId,
        taId: null,
        addressName,
      };
    }
  }

  return {
    address,
    accountId: accountIdHex,
    addressType: AddressType.Regular,
    appAgentId: null,
    taId: null,
    addressName: null,
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
 * @returns {BlockchainAddressInfo} - An object containing the decoded address type and data.
 * @throws {Error} - Thrown if ss58Format is not an integer in [0, 16383], or the
 *   address is not a valid SS58 string of a 32-byte account id in the expected format.
 */
function decodeAddress(
  address: string,
  ss58Format: number = SS58_FORMAT_TREVO_ASSET_HUB,
): BlockchainAddressInfo {
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
 * @returns {BlockchainAddressInfo} - An object containing the decoded address type and
 *   data; its `address` field holds the input verbatim.
 * @throws {Error} - Thrown if the input is not `0x` followed by exactly 64 hex
 *   characters.
 */
function decodeAccountId(accountId: string): BlockchainAddressInfo {
  if (!HEX_ACCOUNT_ID.test(accountId)) {
    throw new Error(
      "Invalid hex account id: expected 0x followed by 64 hex characters",
    );
  }
  return classifyAccountId(accountId, hexToBytes(accountId.slice(2)));
}

/**
 * Encodes a raw 32-byte account id as an SS58 address.
 *
 * The counterpart of `decodeAddress`: any account id obtained from
 * `BlockchainAddressInfo.accountId` (or from chain state) renders as its canonical
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
 * @throws {AddressTypeMismatchError} - Thrown if the address is not an AppAgent address.
 */
function decodeAppAgentAddress(
  address: string,
  ss58Format: number = SS58_FORMAT_TREVO_ASSET_HUB,
): number {
  const decodingResult = decodeAddress(address, ss58Format);

  if (decodingResult.addressType !== AddressType.AppAgent) {
    throw new AddressTypeMismatchError(
      AddressType.AppAgent,
      decodingResult.addressType,
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
 * its `TransactionalAddressId` type alias is the plain `taId` integer instead.)
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
 * @throws {AddressTypeMismatchError} - Thrown if the address is not a Transactional address.
 */
function decodeTransactionalAddress(
  address: string,
  ss58Format: number = SS58_FORMAT_TREVO_ASSET_HUB,
): DecodedTransactionalAddress {
  const decodingResult = decodeAddress(address, ss58Format);

  if (decodingResult.addressType !== AddressType.Transactional) {
    throw new AddressTypeMismatchError(
      AddressType.Transactional,
      decodingResult.addressType,
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
  addressName: string;
}

/**
 * Decodes a named address.
 *
 * @param {string} address - The SS58 address to decode.
 * @returns {DecodedNamedAddress} - An object containing the ID of the application agent
 *   and the address name.
 * @throws {AddressTypeMismatchError} - Thrown if the address is not a Named address.
 */
function decodeNamedAddress(
  address: string,
  ss58Format: number = SS58_FORMAT_TREVO_ASSET_HUB,
): DecodedNamedAddress {
  const decodingResult = decodeAddress(address, ss58Format);

  if (decodingResult.addressType !== AddressType.Named) {
    throw new AddressTypeMismatchError(
      AddressType.Named,
      decodingResult.addressType,
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
    decodingResult.addressName === undefined ||
    decodingResult.addressName === null
  ) {
    throw new Error(
      "Internal error in trevokeyless - addressName of Named address is undefined",
    );
  }

  return {
    appAgentId: decodingResult.appAgentId,
    addressName: decodingResult.addressName,
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
  return decodeAddress(address, ss58Format).addressType !== AddressType.Regular;
}

export {
  AddressType,
  AddressTypeMismatchError,
  Ss58Error,
  BlockchainAddressInfo,
  DecodedTransactionalAddress,
  DecodedNamedAddress,
  encodeAppAgentAddress,
  encodeTransactionalAddress,
  encodeNamedAddress,
  decodeAppAgentAddress,
  decodeTransactionalAddress,
  decodeNamedAddress,
  decodeAddress,
  decodeAccountId,
  encodeAddress,
  isKeylessAddress,
  ACCOUNT_ID_BYTES_LENGTH,
  NAMED_ADDRESS_BYTES_LENGTH,
  SS58_FORMAT_TREVO_ASSET_HUB,
  SS58_MAX_ADDRESS_LENGTH,
  U32_MAX,
};
