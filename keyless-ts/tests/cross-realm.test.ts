/**
 * Pins that `instanceof` — the error-discrimination mechanism this package's README
 * teaches — keeps working when the module is loaded twice in one process.
 *
 * A dual ESM/CommonJS package really is instantiated twice whenever an application
 * `import`s it while a transitive dependency `require`s it, or when a bundler resolves the
 * `browser` condition for application code and CommonJS for a server module. The two copies
 * have distinct class objects, so plain prototype-identity `instanceof` silently returns
 * `false` for an error thrown by the other copy and the error escapes as "unknown".
 *
 * Jest's module registry gives us the same split without needing both build outputs: two
 * `isolateModules` registries produce two independent copies of the module, exactly as the
 * ESM/CommonJS duplication does. The classes must be *different objects* (otherwise this
 * test proves nothing) yet still recognize each other's instances, via the `Symbol.for`
 * brand in `defineCrossRealmBrand`.
 *
 * The brand also survives a *real* realm boundary (a `node:vm` context here; an iframe or
 * a worker in a browser), where even `Error.prototype` differs per realm — that is the
 * `isRealError` fallback, exercised by its own describe block below.
 */
import type { AccountName } from "../src/keyless";
import * as direct from "../src/keyless";

type Module = typeof import("../src/keyless");

function loadCopy(): Module {
  let copy: Module | undefined;
  jest.isolateModules(() => {
    // eslint-disable-next-line @typescript-eslint/no-require-imports
    copy = require("../src/keyless") as Module;
  });
  if (copy === undefined) {
    throw new Error("failed to load an isolated copy of the module");
  }
  return copy;
}

const other = loadCopy();

const VALID_OTHER_FORMAT_ADDRESS =
  "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY";

function caught(run: () => unknown): unknown {
  try {
    run();
  } catch (error) {
    return error;
  }
  throw new Error("expected the call to throw");
}

describe("instanceof across two copies of the module", () => {
  it("really loads two distinct copies", () => {
    // Guards the premise: if the copies were the same object every assertion below would
    // pass trivially and the test would be worthless.
    expect(other.Ss58Error).not.toBe(direct.Ss58Error);
    expect(other.AccountNameError).not.toBe(direct.AccountNameError);
  });

  it("recognizes an Ss58Error thrown by the other copy", () => {
    const error = caught(() => direct.decodeAddress("not-an-address"));
    expect(error).toBeInstanceOf(other.Ss58Error);
  });

  it("recognizes the Ss58FormatMismatchError subclass, and its base class", () => {
    const error = caught(() =>
      direct.decodeAddress(VALID_OTHER_FORMAT_ADDRESS),
    );
    expect(error).toBeInstanceOf(other.Ss58FormatMismatchError);
    expect(error).toBeInstanceOf(other.Ss58Error);
  });

  it("recognizes the keyless module's own error classes", () => {
    const nameError = caught(() =>
      direct.encodeNamedAddress({
        appAgentId: direct.appAgentId(1),
        // Deliberately invalid: the cast simulates an untyped JavaScript caller, so the
        // encoder throws the AccountNameError this test is about.
        accountName: "too-long-a-name" as AccountName,
      }),
    );
    expect(nameError).toBeInstanceOf(other.AccountNameError);

    const parseError = caught(() => direct.decodeAccountId("0xnothex"));
    expect(parseError).toBeInstanceOf(other.ParseAccountIdError);

    const notAnAddress = caught(() =>
      direct.decodeAddress(`0x${"11".repeat(32)}`),
    );
    expect(notAnAddress).toBeInstanceOf(other.NotAnAddressError);

    const mismatch = caught(() =>
      direct.decodeNamedAddress(
        direct.encodeAppAgentAddress({ appAgentId: direct.appAgentId(7) }),
      ),
    );
    expect(mismatch).toBeInstanceOf(other.AccountTypeMismatchError);
  });

  it("does not brand unrelated errors", () => {
    // The brand must discriminate, not blanket-accept: a plain Error, and one error class
    // against another, must still be told apart.
    expect(new Error("plain")).not.toBeInstanceOf(other.Ss58Error);
    expect(new TypeError("plain")).not.toBeInstanceOf(other.AccountNameError);

    const ss58Error = caught(() => direct.decodeAddress("not-an-address"));
    expect(ss58Error).not.toBeInstanceOf(other.AccountNameError);
    expect(ss58Error).not.toBeInstanceOf(other.ParseAccountIdError);

    const plainSs58Error = caught(() => direct.decodeAddress("not-an-address"));
    // A non-mismatch Ss58Error is not the UnexpectedFormat subclass.
    expect(plainSs58Error).not.toBeInstanceOf(other.Ss58FormatMismatchError);
  });
});

describe("instanceof across a real realm boundary", () => {
  // A vm context is a genuine second realm: its Error.prototype is a different object,
  // so `value instanceof Error` is false there and only the [[ErrorData]] fallback in
  // isRealError can recognize the value as an Error. The Symbol.for registry is shared
  // across realms within one process, so branding the foreign error with the registered
  // key reproduces exactly what a copy of this package loaded in that realm would throw.
  const key = "@trevofinance/trevokeyless@2:Ss58Error";

  function inForeignRealm(code: string): unknown {
    // eslint-disable-next-line @typescript-eslint/no-require-imports
    const vm = require("node:vm") as typeof import("node:vm");
    return vm.runInNewContext(code, { foreignBrandKey: key });
  }

  it("recognizes a branded Error created in another realm", () => {
    const foreign = inForeignRealm(`
      const error = new Error("Address is not valid base58");
      error.name = "Ss58Error";
      error.kind = "InvalidBase58";
      Object.defineProperty(error, Symbol.for(foreignBrandKey), { value: true });
      error;
    `);
    // Premise guard: the value really is from another realm.
    expect(foreign instanceof Error).toBe(false);
    expect(foreign).toBeInstanceOf(direct.Ss58Error);
    expect(foreign).toBeInstanceOf(other.Ss58Error);
  });

  it("still rejects foreign non-errors and unbranded foreign errors", () => {
    const forgedPlain = inForeignRealm(`
      const forged = { kind: "InvalidBase58" };
      Object.defineProperty(forged, Symbol.for(foreignBrandKey), { value: true });
      forged;
    `);
    expect(forgedPlain).not.toBeInstanceOf(direct.Ss58Error);

    const unbranded = inForeignRealm(`new Error("plain foreign error");`);
    expect(unbranded).not.toBeInstanceOf(direct.Ss58Error);
  });
});

describe("the brand does not blanket-accept anything carrying the symbol", () => {
  // The brand is looked up in the process-wide `Symbol.for` registry, so it is reachable by
  // anyone. It must therefore be a *necessary* condition for `instanceof`, never a sufficient
  // one: `instanceof Ss58Error` narrows the value to Ss58Error in TypeScript, and the caller
  // goes straight on to read `.message` and `.kind`.
  const key = "@trevofinance/trevokeyless@2:Ss58Error";

  it("rejects a plain object carrying the brand symbol", () => {
    const forged = { [Symbol.for(key)]: true };
    expect(forged).not.toBeInstanceOf(direct.Ss58Error);
    expect(forged).not.toBeInstanceOf(other.Ss58Error);
  });

  it("rejects a non-Error with the brand and a plausible shape", () => {
    // What a JSON-deserialized "error" looks like.
    const forged = {
      [Symbol.for(key)]: true,
      kind: "InvalidChecksum",
      message: "Address has an invalid checksum",
    };
    expect(forged).not.toBeInstanceOf(direct.Ss58Error);
  });
});

describe("Ss58Error only carries expected/found when they mean something", () => {
  it("does not define them on a non-mismatch error", () => {
    const error = caught(() => direct.decodeAddress("not-an-address"));
    expect(error).toBeInstanceOf(direct.Ss58Error);
    // `"expected" in e` is the natural structural check, and it must not lie.
    expect("expected" in (error as object)).toBe(false);
    expect("found" in (error as object)).toBe(false);
  });

  it("defines them on the UnexpectedFormat subclass", () => {
    const error = caught(() =>
      direct.decodeAddress(VALID_OTHER_FORMAT_ADDRESS),
    );
    expect(error).toBeInstanceOf(direct.Ss58FormatMismatchError);
    expect("expected" in (error as object)).toBe(true);
    expect("found" in (error as object)).toBe(true);
    expect(
      (error as InstanceType<typeof direct.Ss58FormatMismatchError>).expected,
    ).toBe(direct.SS58_FORMAT_TREVO_ASSET_HUB);
  });
});

describe("instanceof for a consumer's own subclass", () => {
  // Wrapping or extending a library's error classes is ordinary practice. Because
  // `Symbol.hasInstance` lives on the constructor and statics are inherited, a subclass would
  // inherit the base class's brand check and answer `true` for every error of the base class
  // in the process — silently destroying the subclass's own `instanceof`.
  class ConsumerError extends direct.Ss58Error {}
  class ConsumerNameError extends direct.AccountNameError {}

  it("does not report a library error as an instance of a consumer subclass", () => {
    const ss58Error = caught(() => direct.decodeAddress("not-an-address"));
    expect(ss58Error).toBeInstanceOf(direct.Ss58Error);
    expect(ss58Error).not.toBeInstanceOf(ConsumerError);

    const nameError = caught(() => direct.accountName("too-long-a-name"));
    expect(nameError).toBeInstanceOf(direct.AccountNameError);
    expect(nameError).not.toBeInstanceOf(ConsumerNameError);
  });

  it("still recognizes the consumer's own instances, and their base classes", () => {
    const mine = new ConsumerError("InvalidBase58", "mine");
    expect(mine).toBeInstanceOf(ConsumerError);
    expect(mine).toBeInstanceOf(direct.Ss58Error);
    // ...including across the ESM/CommonJS split the brand exists for.
    expect(mine).toBeInstanceOf(other.Ss58Error);
  });
});
