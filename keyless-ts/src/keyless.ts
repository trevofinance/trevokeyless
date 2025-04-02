import { blake2AsU8a } from "@polkadot/util-crypto";
import {
  decodeAddress as decodeAddressSs58,
  encodeAddress,
} from "@polkadot/keyring";
import { strict as assert } from "assert";

const APP_AGENT_ADDRESS_IDENTIFIER = 1;
const TRANSACTIONAL_ADDRESS_IDENTIFIER = 2;
const NAMED_ADDRESS_IDENTIFIER = 3;
const NAMED_ADDRESS_BYTES_LENGTH = 10;
const SS58_FORMAT__TREVO_ASSET_HUB = 5335;

enum AddressType {
  Regular = 0,
  AppAgent = 1,
  Transactional = 2,
  Named = 3,
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
 * Validates that a name contains only allowed ASCII characters.
 *
 * @param {string} name - The name to validate.
 * @returns {boolean} - True if the name is valid
 */
function validateAddressName(name: string): void {
  assert(name.length === NAMED_ADDRESS_BYTES_LENGTH, "Name size not supported");
  const allowedChars = /^[0-9a-zA-Z\-#]+$/;
  assert(allowedChars.test(name), "Name contains invalid characters");
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
  const appAgentIdBytes = Buffer.alloc(4);
  appAgentIdBytes.writeUInt32LE(appAgentId, 0);

  const appAgentIdBuffer = Buffer.from([APP_AGENT_ADDRESS_IDENTIFIER]);
  const openPart = Buffer.concat([appAgentIdBytes, appAgentIdBuffer]);
  const checksum = blake2AsU8a(openPart);
  const address = Buffer.concat([openPart, checksum.slice(openPart.length)]);

  return encodeAddress(address, ss58Format);
}

/**
 * Encodes a transactional agent address and returns it.
 *
 * @param {number} appAgentId - The ID of the application agent.
 * @param {number} taId - The ID of the transactional agent.
 * @returns {string} - The encoded transactional agent address.
 */
function encodeTransactional(
  appAgentId: number,
  taId: number,
  ss58Format: number = SS58_FORMAT__TREVO_ASSET_HUB,
): string {
  const appAgentIdBytes = Buffer.alloc(4);
  appAgentIdBytes.writeUInt32LE(appAgentId, 0);

  const taIdBytes = Buffer.alloc(4);
  taIdBytes.writeUInt32LE(taId, 0);

  const taIdBuffer = Buffer.from([TRANSACTIONAL_ADDRESS_IDENTIFIER]);
  const openPart = Buffer.concat([appAgentIdBytes, taIdBuffer, taIdBytes]);
  const checksum = blake2AsU8a(openPart);
  const address = Buffer.concat([openPart, checksum.slice(openPart.length)]);

  return encodeAddress(address, ss58Format);
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

  const appAgentIdBytes = Buffer.alloc(4);
  appAgentIdBytes.writeUInt32LE(appAgentId, 0);

  const nameBytes = Buffer.from(name);

  const taIdBuffer = Buffer.from([NAMED_ADDRESS_IDENTIFIER]);
  const openPart = Buffer.concat([appAgentIdBytes, taIdBuffer, nameBytes]);
  const checksum = blake2AsU8a(openPart);
  const address = Buffer.concat([openPart, checksum.slice(openPart.length)]);

  return encodeAddress(address, ss58Format);
}

function isEqualArrays(a: Uint8Array, b: Uint8Array): boolean {
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
 * Decodes a keyless address and determines its type.
 *
 * @param {string} address - The blockchain address to decode.
 * @returns {BlockchainAddressInfo} - An object containing the decoded address type and data.
 * @throws {Error} - Thrown if the address cannot be decoded or is of an unsupported type.
 */
function decodeAddress(
  address: string,
  ss58Format: number = SS58_FORMAT__TREVO_ASSET_HUB,
): BlockchainAddressInfo {
  const accountId = decodeAddressSs58(address, false, ss58Format);
  const accountIdBuffer = Buffer.from(accountId);

  // Read the byte that encodes type of the address
  const addressTypeByte = accountId[4];

  if (addressTypeByte === APP_AGENT_ADDRESS_IDENTIFIER) {
    // Verify checksum
    const openPartSize = 5;
    const openPart = accountId.slice(0, openPartSize);
    const checksum = accountId.slice(openPartSize);
    const checksumCalculated = blake2AsU8a(openPart).slice(openPartSize);
    if (isEqualArrays(checksum, checksumCalculated)) {
      // Extract and decode appAgentId
      const appAgentId = accountIdBuffer.subarray(0, 4).readUint32LE(0);

      return {
        address,
        accountId: `0x${accountIdBuffer.toString("hex")}`,
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
    if (isEqualArrays(checksum, checksumCalculated)) {
      // Extract and decode appAgentId_bytes and taId_bytes
      const appAgentId = accountIdBuffer.subarray(0, 4).readUint32LE(0);
      const taId = accountIdBuffer.subarray(5, 9).readUint32LE(0);

      return {
        address,
        accountId: `0x${accountIdBuffer.toString("hex")}`,
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
    if (isEqualArrays(checksum, checksumCalculated)) {
      // Extract and decode appAgentId and address_name
      const appAgentId = accountIdBuffer.subarray(0, 4).readUint32LE(0);
      const addressName = accountIdBuffer.subarray(5, 15).toString();

      return {
        address,
        accountId: `0x${accountIdBuffer.toString("hex")}`,
        addressType: AddressType.Named,
        appAgentId,
        taId: null,
        addressName,
      };
    }
  }

  return {
    address,
    accountId: `0x${accountIdBuffer.toString("hex")}`,
    addressType: AddressType.Regular,
    appAgentId: null,
    taId: null,
    addressName: null,
  };
}

/**
 * Decodes an application agent address.
 *
 * @param {string} address
 * @returns {number} - The ID of the application agent.
 */
function decodeAppAgent(
  address: string,
  ss58Format: number = SS58_FORMAT__TREVO_ASSET_HUB,
): number {
  const decodingResult = decodeAddress(address, ss58Format);

  if (decodingResult.addressType !== AddressType.AppAgent) {
    throw new Error(
      `Provided address is not an AppAgent address but is instance of ${AddressType[decodingResult.addressType]}`,
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
 * @param {string} address - The JSON representation of the encoded transactional address.
 * @returns {Object} - An object containing the IDs of the app agent and transactional agent.
 */
function decodeTransactional(
  address: string,
  ss58Format: number = SS58_FORMAT__TREVO_ASSET_HUB,
): TransactionalAddressId {
  const decodingResult = decodeAddress(address, ss58Format);

  if (decodingResult.addressType !== AddressType.Transactional) {
    throw new Error(
      `Provided address is not a Transactional address but is instance of ${AddressType[decodingResult.addressType]}`,
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
 * @param {string} address - The JSON representation of the encoded named address.
 * @returns {BlockchainAddressInfo} - An object containing the IDs of the application agent and name.
 */
function decodeNamed(
  address: string,
  ss58Format: number = SS58_FORMAT__TREVO_ASSET_HUB,
): NamedAddressId {
  const decodingResult = decodeAddress(address, ss58Format);

  if (decodingResult.addressType !== AddressType.Named) {
    throw new Error(
      `Provided address is not a Named address but is instance of ${AddressType[decodingResult.addressType]}`,
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

export {
  AddressType,
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
  SS58_FORMAT__TREVO_ASSET_HUB,
};
