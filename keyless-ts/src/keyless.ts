import {
  base58Decode,
  blake2AsU8a,
  decodeAddress as decodeAddressSs58,
  encodeAddress,
} from "@polkadot/util-crypto";
import { hexToU8a, u8aConcat, u8aEq, u8aToHex } from "@polkadot/util";

const APP_AGENT_ADDRESS_IDENTIFIER = 1;
const TRANSACTIONAL_ADDRESS_IDENTIFIER = 2;
const NAMED_ADDRESS_IDENTIFIER = 3;
const NAMED_ADDRESS_BYTES_LENGTH = 10;
const ADDRESS_NAME_ALLOWED_CHARS = /^[0-9a-zA-Z\-#]+$/;
const SS58_FORMAT__TREVO_ASSET_HUB = 5335;

// Length in bytes of a blockchain account id.
const ACCOUNT_ID_BYTES_LENGTH = 32;

// Length in bytes of the SS58 checksum of a 32-byte account id.
const SS58_CHECKSUM_LENGTH = 2;

// Maximum length in characters of an SS58 address of a 32-byte account id: the payload
// is at most 2 (prefix) + 32 (account id) + 2 (checksum) = 36 bytes = 288 bits, and each
// base58 character carries log2(58) ~ 5.858 bits, so a valid address has at most
// ceil(288 / 5.858) = 50 characters. Longer input is rejected before base58 decoding,
// whose cost grows quadratically with the input length. Mirrors the Rust implementation.
const SS58_MAX_ADDRESS_LENGTH = 50;

// A raw account id accepted by decodeAddress: 0x followed by exactly 64 hex characters.
const HEX_ACCOUNT_ID = /^0x[0-9a-fA-F]{64}$/;

const U32_MAX = 4294967295;

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
 * the `KeylessDecodeError` of the Rust implementation. The message is unchanged from
 * previous releases for backwards compatibility.
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
 * Validates that a name is exactly 10 characters long and contains only allowed ASCII
 * characters (`0-9 a-z A-Z - #`).
 *
 * @param {string} name - The name to validate.
 * @throws {Error} - Thrown if the name has a wrong length or contains invalid characters.
 */
function validateAddressName(name: string): void {
  if (name.length !== NAMED_ADDRESS_BYTES_LENGTH) {
    throw new Error("Name size not supported");
  }
  if (!ADDRESS_NAME_ALLOWED_CHARS.test(name)) {
    throw new Error("Name contains invalid characters");
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
  if (!Number.isInteger(ss58Format)) {
    // encodeAddress range-checks the format but its bitwise prefix arithmetic silently
    // truncates fractional values; the Rust (u16) and Python (bytes()) implementations
    // cannot encode a fractional format at all.
    throw new Error(`ss58Format must be an integer, got ${ss58Format}`);
  }
  const checksum = blake2AsU8a(openPart);
  const accountId = u8aConcat(openPart, checksum.slice(openPart.length));
  return encodeAddress(accountId, ss58Format);
}

/**
 * Encodes an application agent address and returns it.
 *
 * @param {number} appAgentId - The ID of the application agent.
 * @returns {string} - The encoded application agent address.
 */
function encodeAppAgent(
  appAgentId: number,
  ss58Format: number = SS58_FORMAT__TREVO_ASSET_HUB,
): string {
  const openPart = u8aConcat(
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
function encodeTransactional(
  appAgentId: number,
  taId: number,
  ss58Format: number = SS58_FORMAT__TREVO_ASSET_HUB,
): string {
  const openPart = u8aConcat(
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
function encodeNamed(
  appAgentId: number,
  name: string,
  ss58Format: number = SS58_FORMAT__TREVO_ASSET_HUB,
): string {
  validateAddressName(name);

  // The name is validated ASCII, so the char codes are exactly the encoded bytes.
  const nameBytes = Uint8Array.from(name, (c) => c.charCodeAt(0));
  const openPart = u8aConcat(
    u32ToLeBytes(appAgentId, "appAgentId"),
    Uint8Array.of(NAMED_ADDRESS_IDENTIFIER),
    nameBytes,
  );
  return encodeKeylessAccount(openPart, ss58Format);
}

/**
 * Resolves an address string to its raw 32-byte account id.
 *
 * @param {string} address - SS58 address (or `0x`-prefixed 32-byte account id hex).
 * @param {number} ss58Format - SS58 format (network prefix) the address must use; not
 *   applied to raw hex input, which carries no format.
 * @returns {Uint8Array} - The 32-byte account id.
 * @throws {Error} - Thrown if the input is not a valid SS58 address of a 32-byte account
 *   id in the expected format, or not a valid 32-byte hex account id.
 */
function decodeAccountId(address: string, ss58Format: number): Uint8Array {
  if (address.startsWith("0x")) {
    // Raw account id passthrough; the ss58Format argument does not apply.
    if (!HEX_ACCOUNT_ID.test(address)) {
      throw new Error(
        "Invalid hex account id: expected 0x followed by 64 hex characters",
      );
    }
    return hexToU8a(address);
  }

  if (address.length > SS58_MAX_ADDRESS_LENGTH) {
    // No valid address of a 32-byte account id is longer; rejecting early keeps the
    // cost of base58-decoding untrusted input bounded.
    throw new Error("Invalid address length");
  }
  const accountId = decodeAddressSs58(address, false, ss58Format);
  if (accountId.length !== ACCOUNT_ID_BYTES_LENGTH) {
    throw new Error(
      `Invalid account id length: expected ${ACCOUNT_ID_BYTES_LENGTH} bytes, got ${accountId.length}`,
    );
  }

  // decodeAddressSs58 validates base58, the SS58 checksum, and the expected format, but
  // it also accepts legacy payload shapes that the Rust and Python implementations
  // reject: a one-byte checksum after a two-byte prefix (which would give every account
  // a second, shorter address spelling with only 8 bits of checksum) and two-byte-prefix
  // encodings of the reserved formats 46 and 47. Enforce the canonical structure:
  // prefix, account id, and a full two-byte checksum, with a non-reserved format.
  /* eslint-disable no-bitwise */
  const decoded = base58Decode(address);
  const prefixLength = (decoded[0] & 0b0100_0000) === 0 ? 1 : 2;
  if (
    decoded.length !==
    prefixLength + ACCOUNT_ID_BYTES_LENGTH + SS58_CHECKSUM_LENGTH
  ) {
    throw new Error("Invalid address length");
  }
  const embeddedFormat =
    prefixLength === 1
      ? decoded[0]
      : ((decoded[0] & 0b0011_1111) << 2) |
        (decoded[1] >> 6) |
        ((decoded[1] & 0b0011_1111) << 8);
  /* eslint-enable no-bitwise */
  if (embeddedFormat === 46 || embeddedFormat === 47) {
    throw new Error(`${embeddedFormat} is a reserved SS58 format`);
  }

  return accountId;
}

/**
 * Decodes a blockchain address and classifies it.
 *
 * Accepts an SS58 address string. For backwards compatibility a raw account id in hex
 * form (`0x` followed by exactly 64 hex characters) is also accepted; such input
 * bypasses SS58 decoding — including the ss58Format check, which does not apply to raw
 * account bytes.
 *
 * @param {string} address - The blockchain address to decode.
 * @returns {BlockchainAddressInfo} - An object containing the decoded address type and data.
 * @throws {Error} - Thrown if the address is not a valid SS58 string of a 32-byte
 *   account id in the expected format, or not a valid 32-byte hex account id.
 */
function decodeAddress(
  address: string,
  ss58Format: number = SS58_FORMAT__TREVO_ASSET_HUB,
): BlockchainAddressInfo {
  const accountId = decodeAccountId(address, ss58Format);
  const accountIdHex = u8aToHex(accountId);

  // Read the byte that encodes type of the address
  const addressTypeByte = accountId[4];

  if (addressTypeByte === APP_AGENT_ADDRESS_IDENTIFIER) {
    // Verify checksum
    const openPartSize = 5;
    const openPart = accountId.slice(0, openPartSize);
    const checksum = accountId.slice(openPartSize);
    const checksumCalculated = blake2AsU8a(openPart).slice(openPartSize);
    if (u8aEq(checksum, checksumCalculated)) {
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
    const checksumCalculated = blake2AsU8a(openPart).slice(openPartSize);
    if (u8aEq(checksum, checksumCalculated)) {
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
    const checksumCalculated = blake2AsU8a(openPart).slice(openPartSize);
    // The name bytes must additionally pass charset validation: an address with a valid
    // checksum over invalid name bytes is classified as Regular, in line with the Rust
    // implementation.
    const nameBytes = accountId.slice(5, 15);
    if (
      u8aEq(checksum, checksumCalculated) &&
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
 * Decodes an application agent address.
 *
 * @param {string} address - The SS58 address (or 32-byte hex account id) to decode.
 * @returns {number} - The ID of the application agent.
 * @throws {AddressTypeMismatchError} - Thrown if the address is not an AppAgent address.
 */
function decodeAppAgent(
  address: string,
  ss58Format: number = SS58_FORMAT__TREVO_ASSET_HUB,
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

interface TransactionalAddressId {
  appAgentId: number;
  taId: number;
}

/**
 * Decodes a transactional address.
 *
 * @param {string} address - The SS58 address (or 32-byte hex account id) to decode.
 * @returns {TransactionalAddressId} - An object containing the IDs of the app agent and
 *   transactional address.
 * @throws {AddressTypeMismatchError} - Thrown if the address is not a Transactional address.
 */
function decodeTransactional(
  address: string,
  ss58Format: number = SS58_FORMAT__TREVO_ASSET_HUB,
): TransactionalAddressId {
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

interface NamedAddressId {
  appAgentId: number;
  addressName: string;
}

/**
 * Decodes a named address.
 *
 * @param {string} address - The SS58 address (or 32-byte hex account id) to decode.
 * @returns {NamedAddressId} - An object containing the ID of the application agent and
 *   the address name.
 * @throws {AddressTypeMismatchError} - Thrown if the address is not a Named address.
 */
function decodeNamed(
  address: string,
  ss58Format: number = SS58_FORMAT__TREVO_ASSET_HUB,
): NamedAddressId {
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
 * @param {string} address - The SS58 address (or 32-byte hex account id) to check.
 * @returns {boolean} - True if the address is a keyless address.
 * @throws {Error} - Thrown if the address cannot be decoded at all.
 */
function isKeylessAddress(
  address: string,
  ss58Format: number = SS58_FORMAT__TREVO_ASSET_HUB,
): boolean {
  return decodeAddress(address, ss58Format).addressType !== AddressType.Regular;
}

export {
  AddressType,
  AddressTypeMismatchError,
  BlockchainAddressInfo,
  TransactionalAddressId,
  NamedAddressId,
  encodeAppAgent,
  encodeTransactional,
  encodeNamed,
  decodeAppAgent,
  decodeTransactional,
  decodeNamed,
  decodeAddress,
  isKeylessAddress,
  ACCOUNT_ID_BYTES_LENGTH,
  NAMED_ADDRESS_BYTES_LENGTH,
  SS58_FORMAT__TREVO_ASSET_HUB,
  SS58_MAX_ADDRESS_LENGTH,
};
