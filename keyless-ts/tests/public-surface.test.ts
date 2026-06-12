/**
 * Pins the public runtime surface of the package — the TypeScript counterpart of the
 * Python `tests/test_public_surface.py` (the Rust surface is pinned by the compiler).
 *
 * `src/keyless.ts` is the package's single entry point (`package.json` `exports`), so
 * its runtime exports are exactly what `require("@trevofinance/trevokeyless")`
 * resolves. The type-only exports (`BlockchainAddressInfo`,
 * `DecodedTransactionalAddress`, `DecodedNamedAddress`, `Ss58ErrorKind`) are erased at
 * compile time and must not appear at runtime.
 */
import * as keyless from "../src/keyless";

describe("Public surface", () => {
  it("exports exactly the documented runtime values", () => {
    const expected = [
      "ACCOUNT_ID_LENGTH",
      "AddressType",
      "AddressTypeMismatchError",
      "NAMED_ADDRESS_LENGTH",
      "SS58_FORMAT_TREVO_ASSET_HUB",
      "SS58_MAX_ADDRESS_LENGTH",
      "Ss58Error",
      "U32_MAX",
      "decodeAccountId",
      "decodeAddress",
      "decodeAppAgentAddress",
      "decodeNamedAddress",
      "decodeTransactionalAddress",
      "encodeAddress",
      "encodeAppAgentAddress",
      "encodeNamedAddress",
      "encodeTransactionalAddress",
      "isKeylessAddress",
    ];
    expect(Object.keys(keyless).sort()).toEqual(expected);
  });

  it("erases the type-only exports at runtime", () => {
    for (const typeOnly of [
      "BlockchainAddressInfo",
      "DecodedTransactionalAddress",
      "DecodedNamedAddress",
      "Ss58ErrorKind",
    ]) {
      expect(typeOnly in keyless).toBe(false);
    }
  });
});
