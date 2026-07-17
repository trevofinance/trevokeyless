import {
  encodeAppAgentAddress,
  encodeTransactionalAddress,
  encodeNamedAddress,
  AccountType,
  DecodedAddressWithFormat,
  Ss58Error,
  Ss58FormatMismatchError,
  decodeAppAgentAddress,
  decodeTransactionalAddress,
  decodeNamedAddress,
  decodeAddress,
  decodeAddressWithFormat,
  SS58_FORMAT_TREVO_ASSET_HUB,
  accountName,
  appAgentId,
  transactionalId,
} from "../src/keyless";

describe("AppAgent", () => {
  const app_agent_id = appAgentId(123);
  const account_id =
    "0x7b00000001293833058fc7db52fc03f6ce344bca98bd7825ff747743f1ff63e2";
  const blockchain_address =
    "ttowKp8AmQuGfbBGikG2pbdYNnErhHRaLrdktJeZEfJVeVnTp";
  const blockchain_address_ss58_42 =
    "5EqykH6EjAZ513RNK37NMagT2KPL4xr2vJXxGCRucbXqBSA7";

  it("Default SS58 format for encoding of AppAgent address is set to correct value.", () => {
    const encoded_address = encodeAppAgentAddress({ appAgentId: app_agent_id });
    expect(encoded_address).toEqual(blockchain_address);
  });

  it("Encoding of AppAgent ID to AppAgent address takes into account SS58 format.", () => {
    const encoded_address = encodeAppAgentAddress({
      appAgentId: app_agent_id,
      ss58Format: 42,
    });
    expect(encoded_address).toEqual(blockchain_address_ss58_42);
  });

  it("Default SS58 format for decoding of AppAgent address is set to correct value.", () => {
    const decoded_app_agent_id = decodeAppAgentAddress(blockchain_address);
    expect(decoded_app_agent_id).toEqual(app_agent_id);
  });

  it("Decoding of AppAgent address to AppAgent ID takes into account SS58 format.", () => {
    expect(() => decodeAppAgentAddress(blockchain_address, 42)).toThrow(
      new Error("Expected SS58 format 42 but found 5335"),
    );
    expect(() => decodeAppAgentAddress(blockchain_address, 42)).toThrow(
      Ss58Error,
    );
  });

  it("Default SS58 format for decoding of AppAgent address to address info is set to correct value.", () => {
    const decoded_address_info = decodeAddress(blockchain_address);
    const expected_address_info = {
      address: blockchain_address,
      accountId: account_id,
      accountType: AccountType.AppAgent,
      appAgentId: app_agent_id,
    };
    expect(decoded_address_info).toEqual(expected_address_info);
  });

  it("Decoding of AppAgent address to address info takes into account SS58 format.", () => {
    expect(() => decodeAddress(blockchain_address, 42)).toThrow(
      new Error("Expected SS58 format 42 but found 5335"),
    );
    expect(() => decodeAddress(blockchain_address, 42)).toThrow(Ss58Error);
  });
});

describe("Transactional", () => {
  const app_agent_id = appAgentId(123);
  const transactional_address_id = transactionalId(456);
  const account_id =
    "0x7b00000002c801000033399aeb61b087f1a20a58c41ea5ff1b7bfb2fda27bfc0";
  const blockchain_address =
    "ttowKp8AmjjQh4GoN7xMiQWwVyyrU1Pu7GRf5HxFmV5t43TXG";
  const blockchain_address_ss58_42 =
    "5EqykH6F3zh6U8x1gjSGAU5aE4P6nwAoL6S9FW8SSNvEj5KD";

  it("Default SS58 format for encoding of Transactional address is set to correct value.", () => {
    const encoded_address = encodeTransactionalAddress({
      appAgentId: app_agent_id,
      taId: transactional_address_id,
    });
    expect(encoded_address).toEqual(blockchain_address);
  });

  it("Encoding of Transactional ID to Transactional address takes into account SS58 format.", () => {
    const encoded_address = encodeTransactionalAddress({
      appAgentId: app_agent_id,
      taId: transactional_address_id,
      ss58Format: 42,
    });
    expect(encoded_address).toEqual(blockchain_address_ss58_42);
  });

  it("Default SS58 format for decoding of Transactional address is set to correct value.", () => {
    const decoded_ids = decodeTransactionalAddress(blockchain_address);
    expect(decoded_ids).toEqual({
      appAgentId: app_agent_id,
      taId: transactional_address_id,
    });
  });

  it("Decoding of Transactional address to Transactional ID takes into account SS58 format.", () => {
    expect(() => decodeTransactionalAddress(blockchain_address, 42)).toThrow(
      new Error("Expected SS58 format 42 but found 5335"),
    );
    expect(() => decodeTransactionalAddress(blockchain_address, 42)).toThrow(
      Ss58Error,
    );
  });

  it("Default SS58 format for decoding of Transactional address to address info is set to correct value.", () => {
    const decoded_address_info = decodeAddress(blockchain_address);
    const expected_address_info = {
      address: blockchain_address,
      accountId: account_id,
      accountType: AccountType.Transactional,
      appAgentId: app_agent_id,
      taId: transactional_address_id,
    };
    expect(decoded_address_info).toEqual(expected_address_info);
  });

  it("Decoding of Transactional address to address info takes into account SS58 format.", () => {
    expect(() => decodeAddress(blockchain_address, 42)).toThrow(
      new Error("Expected SS58 format 42 but found 5335"),
    );
    expect(() => decodeAddress(blockchain_address, 42)).toThrow(Ss58Error);
  });
});

describe("Named", () => {
  const app_agent_id = appAgentId(123);
  const address_name = accountName("example123");
  const account_id =
    "0x7b000000036578616d706c653132330a72315aba532de9903c05a71e7d2d0409";
  const blockchain_address =
    "ttowKp8Amrt2FQ2eNdsbMsW2EEeEsEmsT8KHLvPhVppp9zs8k";
  const blockchain_address_ss58_42 =
    "5EqykH6FB9Jeoto2CefudTAJUimW2K99Bz4QswaAn7rLge2y";

  it("Default SS58 format for encoding of Named address is set to correct value.", () => {
    const encoded_address = encodeNamedAddress({
      appAgentId: app_agent_id,
      accountName: address_name,
    });
    expect(encoded_address).toEqual(blockchain_address);
  });

  it("Encoding of address name to Named address takes into account SS58 format.", () => {
    const encoded_address = encodeNamedAddress({
      appAgentId: app_agent_id,
      accountName: address_name,
      ss58Format: 42,
    });
    expect(encoded_address).toEqual(blockchain_address_ss58_42);
  });

  it("Default SS58 format for decoding of Named address is set to correct value.", () => {
    const decoded_ids = decodeNamedAddress(blockchain_address);
    expect(decoded_ids).toEqual({
      appAgentId: app_agent_id,
      accountName: address_name,
    });
  });

  it("Decoding of Named address to address name takes into account SS58 format.", () => {
    expect(() => decodeNamedAddress(blockchain_address, 42)).toThrow(
      new Error("Expected SS58 format 42 but found 5335"),
    );
    expect(() => decodeNamedAddress(blockchain_address, 42)).toThrow(Ss58Error);
  });

  it("Default SS58 format for decoding of Named address to address info is set to correct value.", () => {
    const decoded_address_info = decodeAddress(blockchain_address);
    const expected_address_info = {
      address: blockchain_address,
      accountId: account_id,
      accountType: AccountType.Named,
      appAgentId: app_agent_id,
      accountName: address_name,
    };
    expect(decoded_address_info).toEqual(expected_address_info);
  });

  it("Decoding of Named address to address info takes into account SS58 format.", () => {
    expect(() => decodeAddress(blockchain_address, 42)).toThrow(
      new Error("Expected SS58 format 42 but found 5335"),
    );
    expect(() => decodeAddress(blockchain_address, 42)).toThrow(Ss58Error);
  });
});

describe("decodeAddressWithFormat", () => {
  // decodeAddressWithFormat decodes an address of any (non-reserved) format,
  // returning the classification and the format found in the address - the
  // counterpart of the Rust from_ss58check_with_version for addresses whose format
  // is not known up front.
  const app_agent_id = 123;
  const account_id =
    "0x7b00000001293833058fc7db52fc03f6ce344bca98bd7825ff747743f1ff63e2";
  const blockchain_address =
    "ttowKp8AmQuGfbBGikG2pbdYNnErhHRaLrdktJeZEfJVeVnTp";
  const blockchain_address_ss58_42 =
    "5EqykH6EjAZ513RNK37NMagT2KPL4xr2vJXxGCRucbXqBSA7";

  it("Decodes a Trevo-format address to the same info as decodeAddress, plus the format.", () => {
    const result: DecodedAddressWithFormat =
      decodeAddressWithFormat(blockchain_address);
    expect(result.ss58Format).toEqual(SS58_FORMAT_TREVO_ASSET_HUB);
    expect(result.info).toEqual(decodeAddress(blockchain_address));
  });

  it("Decodes an address of a foreign format without an expected format.", () => {
    // decodeAddress would throw UnexpectedFormat here.
    const result = decodeAddressWithFormat(blockchain_address_ss58_42);
    expect(result.ss58Format).toEqual(42);
    expect(result.info.accountId).toEqual(account_id);
    expect(result.info.accountType).toEqual(AccountType.AppAgent);
    if (result.info.accountType !== AccountType.AppAgent) {
      throw new Error("Expected an AppAgent account-info variant");
    }
    expect(result.info.appAgentId).toEqual(app_agent_id);
    // The address field is the input spelling verbatim.
    expect(result.info.address).toEqual(blockchain_address_ss58_42);
  });

  it("Rejects an invalid address with its validation kind, never UnexpectedFormat.", () => {
    let error: unknown;
    try {
      decodeAddressWithFormat(blockchain_address.slice(0, -1));
    } catch (e) {
      error = e;
    }
    if (!(error instanceof Ss58Error)) {
      throw new Error(`expected Ss58Error, got ${String(error)}`);
    }
    expect(error.kind).not.toEqual("UnexpectedFormat");
  });

  it("Rejects raw account id hex outright (it carries no SS58 format to report).", () => {
    expect(() => decodeAddressWithFormat(account_id)).toThrow(
      new Error(
        "Raw account id hex is not an SS58 address; use decodeAccountId",
      ),
    );
  });
});

describe("Ss58Error constructor taxonomy guard", () => {
  it("accepts each of the four directly constructible kinds", () => {
    for (const kind of [
      "InvalidBase58",
      "InvalidLength",
      "InvalidChecksum",
      "InvalidFormat",
    ] as const) {
      const error = new Ss58Error(kind, "message");
      expect(error.kind).toBe(kind);
    }
  });

  it("rejects a bare UnexpectedFormat: that kind is the subclass's", () => {
    // A bare Ss58Error with the "UnexpectedFormat" kind would carry no expected/found
    // pair and fail the `instanceof Ss58FormatMismatchError` the documentation
    // promises for that kind; the Python constructor rejects the same misuse.
    expect(() => new Ss58Error("UnexpectedFormat", "message")).toThrow(
      RangeError,
    );
    expect(() => new Ss58Error("UnexpectedFormat", "message")).toThrow(
      /Ss58FormatMismatchError/,
    );
  });

  it("rejects a kind outside the shared taxonomy", () => {
    expect(() => new Ss58Error("Bogus" as never, "message")).toThrow(
      RangeError,
    );
    expect(() => new Ss58Error("Bogus" as never, "message")).toThrow(
      /Unknown Ss58Error kind/,
    );
  });

  it("still constructs the UnexpectedFormat subclass (its super() kind is accepted)", () => {
    const error = new Ss58FormatMismatchError(5335, 42);
    expect(error.kind).toBe("UnexpectedFormat");
    expect(error).toBeInstanceOf(Ss58Error);
    expect([error.expected, error.found]).toEqual([5335, 42]);
  });
});
