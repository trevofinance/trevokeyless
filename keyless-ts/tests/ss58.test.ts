import {
  encodeAppAgent,
  encodeTransactional,
  encodeNamed,
  AddressType,
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
  const blockchain_address_ss58_42 =
    "5EqykH6EjAZ513RNK37NMagT2KPL4xr2vJXxGCRucbXqBSA7";

  it("Default SS58 format for encoding of AppAgent address is set to correct value.", () => {
    const encoded_address = encodeAppAgent(app_agent_id);
    expect(encoded_address).toEqual(blockchain_address);
  });

  it("Encoding of AppAgent ID to AppAgent address takes into account SS58 format.", () => {
    const encoded_address = encodeAppAgent(app_agent_id, 42);
    expect(encoded_address).toEqual(blockchain_address_ss58_42);
  });

  it("Default SS58 format for decoding of AppAgent address is set to wrong value.", () => {
    const decoded_app_agent_id = decodeAppAgent(blockchain_address);
    expect(decoded_app_agent_id).toEqual(app_agent_id);
  });

  it("Decoding of AppAgent address to AppAgent ID takes into account SS58 format.", () => {
    expect(() => decodeAppAgent(blockchain_address, 42)).toThrow(
      new Error(
        "Decoding ttowKp8AmQuGfbBGikG2pbdYNnErhHRaLrdktJeZEfJVeVnTp: Expected ss58Format 42, received 5335",
      ),
    );
  });

  it("Default SS58 format for decoding of AppAgent address is set to wrong value.", () => {
    const decoded_address_info = decodeAddress(blockchain_address);
    const expected_address_info = {
      address: blockchain_address,
      accountId: account_id,
      addressType: AddressType.AppAgent,
      appAgentId: app_agent_id,
      taId: undefined,
      addressName: undefined,
    };
  });

  it("Decoding of AppAgent address to address info takes into account SS58 format.", () => {
    expect(() => decodeAddress(blockchain_address, 42)).toThrow(
      new Error(
        "Decoding ttowKp8AmQuGfbBGikG2pbdYNnErhHRaLrdktJeZEfJVeVnTp: Expected ss58Format 42, received 5335",
      ),
    );
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
  const blockchain_address_ss58_42 =
    "5EqykH6F3zh6U8x1gjSGAU5aE4P6nwAoL6S9FW8SSNvEj5KD";

  it("Default SS58 format for encoding of Transactional address is set to correct value.", () => {
    const encoded_address = encodeTransactional(
      app_agent_id,
      transactional_address_id,
    );
    expect(encoded_address).toEqual(blockchain_address);
  });

  it("Encoding of Transactional ID to Transactional address takes into account SS58 format.", () => {
    const encoded_address = encodeTransactional(
      app_agent_id,
      transactional_address_id,
      42,
    );
    expect(encoded_address).toEqual(blockchain_address_ss58_42);
  });

  it("Default SS58 format for decoding of Transactional address is set to wrong value.", () => {
    const decoded_ids = decodeTransactional(blockchain_address);
    expect(decoded_ids).toEqual({
      appAgentId: app_agent_id,
      taId: transactional_address_id,
    });
  });

  it("Decoding of Transactional address to Transactional ID takes into account SS58 format.", () => {
    expect(() => decodeTransactional(blockchain_address, 42)).toThrow(
      new Error(
        "Decoding ttowKp8AmjjQh4GoN7xMiQWwVyyrU1Pu7GRf5HxFmV5t43TXG: Expected ss58Format 42, received 5335",
      ),
    );
  });

  it("Default SS58 format for decoding of Transactional address is set to wrong value.", () => {
    const decoded_address_info = decodeAddress(blockchain_address);
    const expected_address_info = {
      address: blockchain_address,
      accountId: account_id,
      addressType: AddressType.Transactional,
      appAgentId: app_agent_id,
      taId: transactional_address_id,
      addressName: undefined,
    };
  });

  it("Decoding of Transactional address to address info takes into account SS58 format.", () => {
    expect(() => decodeAddress(blockchain_address, 42)).toThrow(
      new Error(
        "Decoding ttowKp8AmjjQh4GoN7xMiQWwVyyrU1Pu7GRf5HxFmV5t43TXG: Expected ss58Format 42, received 5335",
      ),
    );
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
  const blockchain_address_ss58_42 =
    "5EqykH6FB9Jeoto2CefudTAJUimW2K99Bz4QswaAn7rLge2y";

  it("Default SS58 format for encoding of Named address is set to correct value.", () => {
    const encoded_address = encodeNamed(app_agent_id, address_name);
    expect(encoded_address).toEqual(blockchain_address);
  });

  it("Encoding of address name to Named address doesn't take into account SS58 format.", () => {
    const encoded_address = encodeNamed(app_agent_id, address_name, 42);
    expect(encoded_address).toEqual(blockchain_address_ss58_42);
  });

  it("Default SS58 format for decoding of Named address is set to wrong value.", () => {
    const decoded_ids = decodeNamed(blockchain_address);
    expect(decoded_ids).toEqual({
      appAgentId: app_agent_id,
      addressName: address_name,
    });
  });

  it("Decoding of Named address to address name doesn't take into account SS58 format.", () => {
    expect(() => decodeNamed(blockchain_address, 42)).toThrow(
      new Error(
        "Decoding ttowKp8Amrt2FQ2eNdsbMsW2EEeEsEmsT8KHLvPhVppp9zs8k: Expected ss58Format 42, received 5335",
      ),
    );
  });

  it("Default SS58 format for decoding of Named address is set to wrong value.", () => {
    const decoded_address_info = decodeAddress(blockchain_address);
    const expected_address_info = {
      address: blockchain_address,
      accountId: account_id,
      addressType: AddressType.Named,
      appAgentId: app_agent_id,
      taId: undefined,
      addressName: address_name,
    };
  });

  it("Decoding of Named address to address info takes into account SS58 format.", () => {
    expect(() => decodeAddress(blockchain_address, 42)).toThrow(
      new Error(
        "Decoding ttowKp8Amrt2FQ2eNdsbMsW2EEeEsEmsT8KHLvPhVppp9zs8k: Expected ss58Format 42, received 5335",
      ),
    );
  });
});
