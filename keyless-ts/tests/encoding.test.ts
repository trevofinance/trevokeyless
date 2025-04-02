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
