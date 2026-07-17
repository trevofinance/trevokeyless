/**
 * Pins the public runtime surface of the package — the TypeScript counterpart of the
 * Python `tests/test_public_surface.py` (the Rust surface is pinned by the compiler).
 *
 * `src/keyless.ts` is the package's single entry point (`package.json` `exports`), so
 * its runtime exports are exactly what `require("@trevofinance/trevokeyless")`
 * resolves. The type-only exports (`AccountName`, `AppAgentId`, `TransactionalId`, the `*Options` /
 * `*Identifiers` argument shapes, `BlockchainAccountInfo` and its four variants,
 * `DecodedAddressWithFormat`, `DecodedTransactionalAddress`, `DecodedNamedAddress`,
 * `Ss58ErrorKind`, `AccountNameErrorKind`) are erased at compile time and must not
 * appear at runtime.
 */
import * as keyless from "../src/keyless";

describe("Public surface", () => {
  it("exports exactly the documented runtime values", () => {
    const expected = [
      "ACCOUNT_ID_LENGTH",
      "ACCOUNT_NAME_LENGTH",
      "APP_AGENT_ACCOUNT_IDENTIFIER",
      "AccountNameError",
      "AccountType",
      "AccountTypeMismatchError",
      "NAMED_ACCOUNT_IDENTIFIER",
      "NotAnAddressError",
      "ParseAccountIdError",
      "SS58_FORMAT_TREVO_ASSET_HUB",
      "SS58_MAX_ADDRESS_LENGTH",
      "Ss58Error",
      "Ss58FormatMismatchError",
      "TRANSACTIONAL_ACCOUNT_IDENTIFIER",
      "U32_MAX",
      "accountName",
      "appAgentId",
      "decodeAccountId",
      "decodeAddress",
      "decodeAddressWithFormat",
      "decodeAppAgentAccountId",
      "decodeAppAgentAddress",
      "decodeNamedAccountId",
      "decodeNamedAddress",
      "decodeTransactionalAccountId",
      "decodeTransactionalAddress",
      "encodeAddress",
      "encodeAppAgentAccountId",
      "encodeAppAgentAddress",
      "encodeNamedAccountId",
      "encodeNamedAddress",
      "encodeTransactionalAccountId",
      "encodeTransactionalAddress",
      "isAccountName",
      "isAppAgentId",
      "isKeylessAccountId",
      "isKeylessAddress",
      "isTransactionalId",
      "transactionalId",
    ];
    expect(Object.keys(keyless).sort()).toEqual(expected);
  });

  it("pins the exported constant values", () => {
    // The values are part of the public API (callers size buffers and validate
    // inputs against them); pinning them here keeps a drift in the source from
    // passing while every other test re-declares its own literals.
    expect(keyless.ACCOUNT_ID_LENGTH).toBe(32);
    expect(keyless.ACCOUNT_NAME_LENGTH).toBe(10);
    expect(keyless.SS58_FORMAT_TREVO_ASSET_HUB).toBe(5335);
    expect(keyless.SS58_MAX_ADDRESS_LENGTH).toBe(50);
    expect(keyless.U32_MAX).toBe(4294967295);
    expect(keyless.accountName("Treasury-1")).toBe("Treasury-1");
    expect(keyless.appAgentId(123)).toBe(123);
    expect(keyless.transactionalId(7)).toBe(7);
    expect(Object.isFrozen(keyless.AccountType)).toBe(true);
  });

  it("pins the account-type discriminants to their AccountType values", () => {
    // The byte-[4] discriminant of a keyless account id equals the numeric AccountType
    // value of its variant, in all three implementations; the constants carry the same
    // names as the Rust ones.
    expect(keyless.APP_AGENT_ACCOUNT_IDENTIFIER).toBe(1);
    expect(keyless.TRANSACTIONAL_ACCOUNT_IDENTIFIER).toBe(2);
    expect(keyless.NAMED_ACCOUNT_IDENTIFIER).toBe(3);
    expect(keyless.APP_AGENT_ACCOUNT_IDENTIFIER).toBe(
      keyless.AccountType.AppAgent,
    );
    expect(keyless.TRANSACTIONAL_ACCOUNT_IDENTIFIER).toBe(
      keyless.AccountType.Transactional,
    );
    expect(keyless.NAMED_ACCOUNT_IDENTIFIER).toBe(keyless.AccountType.Named);
  });

  it("exports the error classes with their names and prototype chain", () => {
    // Each error class is programmatically catchable via instanceof (a caller must never
    // have to string-match a message), and Ss58FormatMismatchError is the
    // "UnexpectedFormat" case of Ss58Error, so catching Ss58Error still sees it.
    expect(new keyless.ParseAccountIdError().name).toBe("ParseAccountIdError");
    expect(new keyless.ParseAccountIdError()).toBeInstanceOf(Error);
    expect(new keyless.NotAnAddressError().name).toBe("NotAnAddressError");
    expect(new keyless.NotAnAddressError()).toBeInstanceOf(Error);
    const mismatch = new keyless.Ss58FormatMismatchError(5335, 42);
    expect(mismatch).toBeInstanceOf(keyless.Ss58Error);
    expect(mismatch.name).toBe("Ss58FormatMismatchError");
    expect(mismatch.kind).toBe("UnexpectedFormat");
    expect([mismatch.expected, mismatch.found]).toEqual([5335, 42]);
    // The remaining three classes pin their .name too: a caller logging or routing
    // on error.name must see the class name, and nothing else asserted these.
    const ss58Error = new keyless.Ss58Error(
      "InvalidChecksum",
      "Address has an invalid checksum",
    );
    expect(ss58Error.name).toBe("Ss58Error");
    expect(ss58Error).toBeInstanceOf(Error);
    const nameError = new keyless.AccountNameError("InvalidLength");
    expect(nameError.name).toBe("AccountNameError");
    expect(nameError).toBeInstanceOf(Error);
    const typeMismatch = new keyless.AccountTypeMismatchError(
      keyless.AccountType.AppAgent,
      keyless.AccountType.Named,
    );
    expect(typeMismatch.name).toBe("AccountTypeMismatchError");
    expect(typeMismatch).toBeInstanceOf(Error);
  });

  it("refuses a bare Ss58Error with the UnexpectedFormat kind", () => {
    // "UnexpectedFormat" *is* the Ss58FormatMismatchError case: a bare Ss58Error with
    // that kind would carry no expected/found pair and fail the subclass instanceof
    // the class documentation promises. The Python constructor enforces the same;
    // consumer subclasses of the mismatch class stay constructible.
    expect(() => new keyless.Ss58Error("UnexpectedFormat", "forged")).toThrow(
      RangeError,
    );
    class ConsumerMismatch extends keyless.Ss58FormatMismatchError {}
    const consumer = new ConsumerMismatch(5335, 42);
    expect(consumer.kind).toBe("UnexpectedFormat");
    expect(consumer).toBeInstanceOf(keyless.Ss58Error);
  });

  it("erases the type-only exports at runtime", () => {
    for (const typeOnly of [
      "BlockchainAccountInfo",
      "RegularAccountInfo",
      "AppAgentAccountInfo",
      "TransactionalAccountInfo",
      "NamedAccountInfo",
      "AccountName",
      "AppAgentId",
      "TransactionalId",
      "AppAgentAddressOptions",
      "AppAgentAccountIdentifiers",
      "TransactionalAddressOptions",
      "TransactionalAccountIdentifiers",
      "NamedAddressOptions",
      "NamedAccountIdentifiers",
      "DecodedAddressWithFormat",
      "DecodedTransactionalAddress",
      "DecodedNamedAddress",
      "Ss58ErrorKind",
      "AccountNameErrorKind",
      "BlockchainAddress",
      "BlockchainAccountId",
      "Ss58Format",
    ]) {
      expect(typeOnly in keyless).toBe(false);
    }
  });
});
