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

describe("Address name", () => {
  const ss58_format = 5335;

  it("Encodes valid names correctly", () => {
    const valid_names = [
      "example123",
      "abcdEFGHIJ",
      "1234567890",
      "name-#test",
    ];
    for (let i = 0; i < valid_names.length; i++) {
      expect(() => encodeNamed(123, valid_names[i], ss58_format)).not.toThrow();
    }
  });

  it("Encodes valid names correctly", () => {
    const invalid_names = [
      "example!23",
      "name@12345",
      "abcd*EFGHI",
      "with space",
      "with_under",
    ];
    for (let i = 0; i < invalid_names.length; i++) {
      expect(() => encodeNamed(123, invalid_names[i], ss58_format)).toThrow(
        new Error("Name contains invalid characters"),
      );
    }

    const invalidLengthNames = ["short", "veeeerrrrryyyyloooongg"];
    for (let i = 0; i < invalidLengthNames.length; i++) {
      expect(() =>
        encodeNamed(123, invalidLengthNames[i], ss58_format),
      ).toThrow(new Error("Name size not supported"));
    }
  });
});
