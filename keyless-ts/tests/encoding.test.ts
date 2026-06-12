import { blake2b } from "@noble/hashes/blake2b";
import { ss58Encode } from "../src/ss58";
import {
  encodeAppAgent,
  encodeTransactional,
  encodeNamed,
  AddressType,
  BlockchainAddressInfo,
  decodeAppAgent,
  decodeTransactional,
  decodeNamed,
  decodeAddress,
} from "../src/keyless";

describe("AppAgent", () => {
  const ss58_format = 5335;

  const app_agent_id = 123;
  const account_id =
    "0x7b00000001293833058fc7db52fc03f6ce344bca98bd7825ff747743f1ff63e2";
  const blockchain_address =
    "ttowKp8AmQuGfbBGikG2pbdYNnErhHRaLrdktJeZEfJVeVnTp";

  it("Encodes AppAgent address correctly", () => {
    const encoded_address = encodeAppAgent(app_agent_id, ss58_format);
    expect(encoded_address).toEqual(blockchain_address);
  });

  it("Decodes AppAgent address correctly", () => {
    const decoded_app_agent_id = decodeAppAgent(
      blockchain_address,
      ss58_format,
    );
    expect(decoded_app_agent_id).toEqual(app_agent_id);
  });

  it("Decodes AppAgent address to address info correctly", () => {
    const decoded_info = decodeAddress(blockchain_address, ss58_format);
    const expected_info: BlockchainAddressInfo = {
      address: blockchain_address,
      accountId: account_id,
      addressType: AddressType.AppAgent,
      appAgentId: app_agent_id,
      taId: null,
      addressName: null,
    };
    expect(decoded_info).toEqual(expected_info);
  });
});

describe("Transactional", () => {
  const ss58_format = 5335;

  const app_agent_id = 123;
  const transactional_address_id = 456;
  const account_id =
    "0x7b00000002c801000033399aeb61b087f1a20a58c41ea5ff1b7bfb2fda27bfc0";
  const blockchain_address =
    "ttowKp8AmjjQh4GoN7xMiQWwVyyrU1Pu7GRf5HxFmV5t43TXG";

  it("Encodes Transactional address correctly", () => {
    const encoded_address = encodeTransactional(
      app_agent_id,
      transactional_address_id,
      ss58_format,
    );
    expect(encoded_address).toEqual(blockchain_address);
  });

  it("Decodes Transactional address correctly", () => {
    const decoded_ids = decodeTransactional(blockchain_address, ss58_format);
    expect(decoded_ids).toEqual({
      appAgentId: app_agent_id,
      taId: transactional_address_id,
    });
  });

  it("Decodes Transactional address to address info correctly", () => {
    const decoded_info = decodeAddress(blockchain_address, ss58_format);
    const expected_info: BlockchainAddressInfo = {
      address: blockchain_address,
      accountId: account_id,
      addressType: AddressType.Transactional,
      appAgentId: app_agent_id,
      taId: transactional_address_id,
      addressName: null,
    };
    expect(decoded_info).toEqual(expected_info);
  });
});

describe("Named", () => {
  const ss58_format = 5335;

  const app_agent_id = 123;
  const address_name = "example123";
  const account_id =
    "0x7b000000036578616d706c653132330a72315aba532de9903c05a71e7d2d0409";
  const blockchain_address =
    "ttowKp8Amrt2FQ2eNdsbMsW2EEeEsEmsT8KHLvPhVppp9zs8k";

  it("Encodes Named address correctly", () => {
    const encoded_address = encodeNamed(
      app_agent_id,
      address_name,
      ss58_format,
    );
    expect(encoded_address).toEqual(blockchain_address);
  });

  it("Decodes Named address correctly", () => {
    const decoded_ids = decodeNamed(blockchain_address, ss58_format);
    expect(decoded_ids).toEqual({
      appAgentId: app_agent_id,
      addressName: address_name,
    });
  });

  it("Decodes Named address to address info correctly", () => {
    const decoded_info = decodeAddress(blockchain_address, ss58_format);
    const expected_info: BlockchainAddressInfo = {
      address: blockchain_address,
      accountId: account_id,
      addressType: AddressType.Named,
      appAgentId: app_agent_id,
      taId: null,
      addressName: address_name,
    };
    expect(decoded_info).toEqual(expected_info);
  });
});

describe("Named address with invalid name bytes", () => {
  const ss58_format = 5335;

  // Builds the raw account id of a Named address by hand: anyone can construct an account
  // id that carries a valid checksum over arbitrary name bytes - no private key is needed.
  function buildNamedAddress(nameBytes: Uint8Array): string {
    const appAgentIdBytes = Buffer.alloc(4);
    appAgentIdBytes.writeUInt32LE(1, 0);
    const openPart = Buffer.concat([
      appAgentIdBytes,
      Buffer.from([AddressType.Named]),
      Buffer.from(nameBytes),
    ]);
    const checksum = blake2b(openPart, { dkLen: 32 });
    const accountId = Buffer.concat([
      openPart,
      Buffer.from(checksum.slice(openPart.length)),
    ]);
    return ss58Encode(accountId, ss58_format);
  }

  const invalidNames: [string, Uint8Array][] = [
    ["control character", Uint8Array.from([0x00, ...Buffer.from("nvalid123")])],
    ["invalid UTF-8", Uint8Array.from([0xff, ...Buffer.from("nvalid123")])],
    ["printable ASCII outside the allowed set", Buffer.from("invalid@12")],
  ];

  it.each(invalidNames)(
    "decodes address with %s in name bytes as Regular",
    (_description, nameBytes) => {
      const address = buildNamedAddress(nameBytes);
      const decoded_info = decodeAddress(address, ss58_format);
      expect(decoded_info.addressType).toEqual(AddressType.Regular);
      expect(decoded_info.appAgentId).toBeNull();
      expect(decoded_info.addressName).toBeNull();
    },
  );

  it.each(invalidNames)(
    "throws when address with %s in name bytes is decoded as Named",
    (_description, nameBytes) => {
      const address = buildNamedAddress(nameBytes);
      expect(() => decodeNamed(address, ss58_format)).toThrow(
        new Error(
          "Provided address is not a Named address but is instance of Regular",
        ),
      );
    },
  );

  it("decodes an address with valid name bytes built the same way as Named", () => {
    const address = buildNamedAddress(Buffer.from("example123"));
    const decoded_info = decodeAddress(address, ss58_format);
    expect(decoded_info.addressType).toEqual(AddressType.Named);
    expect(decoded_info.appAgentId).toEqual(1);
    expect(decoded_info.addressName).toEqual("example123");
  });
});

describe("Mismatch of address type", () => {
  const ss58_format = 5335;

  // Alice
  const account_id =
    "0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d";
  const regular_address = "ttqxHzRJmmjFBcE7Lb5Xs4GNMq2gFSt28JyvTEqjhqzE9EGP4";

  it("Checks that regular address can not be decoded as an AppAgent address", () => {
    expect(() => decodeAppAgent(regular_address, ss58_format)).toThrow(
      new Error(
        "Provided address is not an AppAgent address but is instance of Regular",
      ),
    );
  });

  it("Checks that regular address can not be decoded as a Transactional address", () => {
    expect(() => decodeTransactional(regular_address, ss58_format)).toThrow(
      new Error(
        "Provided address is not a Transactional address but is instance of Regular",
      ),
    );
  });

  it("Checks that regular address can not be decoded as a Named address", () => {
    expect(() => decodeNamed(regular_address, ss58_format)).toThrow(
      new Error(
        "Provided address is not a Named address but is instance of Regular",
      ),
    );
  });
});
