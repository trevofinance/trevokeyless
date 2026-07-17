import type { AccountName } from "../src/keyless";
import {
  AccountNameError,
  accountName,
  appAgentId,
  encodeNamedAddress,
  isAccountName,
  isAppAgentId,
  isTransactionalId,
  U32_MAX,
} from "../src/keyless";

describe("Account name", () => {
  const ss58_format = 5335;

  it("Encodes valid names correctly", () => {
    const valid_names = [
      "example123",
      "abcdEFGHIJ",
      "1234567890",
      "name-#test",
    ];
    for (const name of valid_names) {
      expect(() =>
        encodeNamedAddress({
          appAgentId: appAgentId(123),
          accountName: accountName(name),
          ss58Format: ss58_format,
        }),
      ).not.toThrow();
    }
  });

  it("Rejects invalid names", () => {
    // The names below are invalid by construction, so they are cast rather than passed
    // through `accountName()`: the cast is what an untyped JavaScript caller does, and
    // the point of the test is that the *encoder* rejects the name.
    const invalid_names = [
      "example!23",
      "name@12345",
      "abcd*EFGHI",
      "with space",
      "with_under",
    ];
    for (let i = 0; i < invalid_names.length; i++) {
      let thrown: unknown;
      try {
        encodeNamedAddress({
          appAgentId: appAgentId(123),
          accountName: invalid_names[i] as AccountName,
          ss58Format: ss58_format,
        });
      } catch (error) {
        thrown = error;
      }
      expect(thrown).toBeInstanceOf(AccountNameError);
      expect((thrown as AccountNameError).kind).toBe("InvalidCharacter");
      expect(String((thrown as Error).message)).toBe(
        "Account name contains invalid characters",
      );
    }

    const invalidLengthNames = ["short", "veeeerrrrryyyyloooongg"];
    for (let i = 0; i < invalidLengthNames.length; i++) {
      let thrown: unknown;
      try {
        encodeNamedAddress({
          appAgentId: appAgentId(123),
          accountName: invalidLengthNames[i] as AccountName,
          ss58Format: ss58_format,
        });
      } catch (error) {
        thrown = error;
      }
      expect(thrown).toBeInstanceOf(AccountNameError);
      expect((thrown as AccountNameError).kind).toBe("InvalidLength");
      expect(String((thrown as Error).message)).toBe(
        "Account name must be exactly 10 characters long",
      );
    }
  });

  it("measures the name length in UTF-8 bytes, like the Rust AccountName", () => {
    // The name occupies bytes [5..15] of the account id, so the length rule is defined
    // on the UTF-8 *byte* length of the name (Rust's AccountName::new takes &[u8]), not
    // on JavaScript's UTF-16 code-unit count. Non-ASCII names are rejected either way,
    // but the two rules disagree on the *kind*, which is a pinned cross-language
    // contract (the named_invalid_encode fixture group carries an error_kind per name).
    const cases: [string, string][] = [
      ["héllo12345", "InvalidLength"], // 11 UTF-8 bytes, 10 UTF-16 units
      ["café12345", "InvalidCharacter"], // 10 UTF-8 bytes, 9 UTF-16 units
      ["🙂aaaaaaaa", "InvalidLength"], // 12 UTF-8 bytes, 10 UTF-16 units
      ["🙂aaaaaa", "InvalidCharacter"], // 10 UTF-8 bytes, 8 UTF-16 units
      ["ααααα", "InvalidCharacter"], // 10 UTF-8 bytes, 5 UTF-16 units
    ];
    cases.forEach(([name, kind]) => {
      let thrown: unknown;
      try {
        encodeNamedAddress({
          appAgentId: appAgentId(123),
          accountName: name as AccountName,
          ss58Format: ss58_format,
        });
      } catch (error) {
        thrown = error;
      }
      expect(thrown).toBeInstanceOf(AccountNameError);
      expect(`${name}: ${(thrown as AccountNameError).kind}`).toBe(
        `${name}: ${kind}`,
      );
    });
  });
});

describe("accountName()", () => {
  // The branding factory: the counterpart of appAgentId()/transactionalId() for the
  // Named account name. It validates once, at the point the name enters the type system,
  // so an unvalidated string cannot reach an encoder.

  it("accepts a valid name and returns it unchanged", () => {
    ["example123", "abcdEFGHIJ", "1234567890", "name-#test"].forEach((name) => {
      expect(accountName(name)).toBe(name);
    });
  });

  it("rejects a wrong-length name with AccountNameError InvalidLength", () => {
    ["", "short", "example12", "example1234", "veeeerrrrryyyyloooongg"].forEach(
      (name) => {
        let thrown: unknown;
        try {
          accountName(name);
        } catch (error) {
          thrown = error;
        }
        expect(thrown).toBeInstanceOf(AccountNameError);
        expect((thrown as AccountNameError).kind).toBe("InvalidLength");
        expect((thrown as Error).message).toBe(
          "Account name must be exactly 10 characters long",
        );
      },
    );
  });

  it("rejects an attacker-length name without scanning it", () => {
    // The UTF-16 unit count bounds the work before utf8Length walks the string: a
    // 100-million-character name must take the O(1) fast path, not an O(n) scan (which
    // takes hundreds of milliseconds — the linear amplification the Python
    // implementation's short-circuit already closes). The bound below is deliberately
    // generous so runner noise cannot flake it while a full scan still trips it.
    const huge = "a".repeat(100_000_000);
    const start = performance.now();
    let thrown: unknown;
    try {
      accountName(huge);
    } catch (error) {
      thrown = error;
    }
    const elapsed = performance.now() - start;
    expect(thrown).toBeInstanceOf(AccountNameError);
    expect((thrown as AccountNameError).kind).toBe("InvalidLength");
    expect(elapsed).toBeLessThan(250);
  });

  it("rejects an out-of-charset name with AccountNameError InvalidCharacter", () => {
    ["example!23", "with space", "with_under", "café12345"].forEach((name) => {
      let thrown: unknown;
      try {
        accountName(name);
      } catch (error) {
        thrown = error;
      }
      expect(thrown).toBeInstanceOf(AccountNameError);
      expect((thrown as AccountNameError).kind).toBe("InvalidCharacter");
      expect((thrown as Error).message).toBe(
        "Account name contains invalid characters",
      );
    });
  });

  it("rejects a non-string with a TypeError", () => {
    // Same contract as the other string parameters: non-primitive strings are rejected
    // deliberately rather than coerced.
    [123, true, null, undefined, [], {}, new String("example123")].forEach(
      (value) => {
        expect(() => accountName(value as never)).toThrow(TypeError);
        expect(() => accountName(value as never)).toThrow(
          /accountName must be a string/,
        );
      },
    );
  });
});

describe("Non-throwing type guards", () => {
  it("isAccountName narrows exactly the values accountName() accepts", () => {
    for (const valid of [
      "Treasury-1",
      "example123",
      "name-#test",
      "1234567890",
    ]) {
      expect(isAccountName(valid)).toBe(true);
    }
    // Wrong byte length, out-of-charset, and non-strings are all rejected without throwing.
    for (const invalid of [
      "short",
      "toolongname11",
      "example!23",
      "café12345",
      123,
      null,
      undefined,
      ["Treasury-1"],
      { length: 10 },
    ]) {
      expect(isAccountName(invalid)).toBe(false);
    }
  });

  it("isAppAgentId and isTransactionalId narrow exactly a valid u32", () => {
    for (const guard of [isAppAgentId, isTransactionalId]) {
      expect(guard(0)).toBe(true);
      expect(guard(123)).toBe(true);
      expect(guard(U32_MAX)).toBe(true);
      // Out of range, non-integer, and non-number are all rejected without throwing.
      expect(guard(-1)).toBe(false);
      expect(guard(U32_MAX + 1)).toBe(false);
      expect(guard(1.5)).toBe(false);
      expect(guard(NaN)).toBe(false);
      // Negative zero signals a float-typed source (plain 0 is accepted above).
      expect(guard(-0)).toBe(false);
      expect(guard("123")).toBe(false);
      expect(guard(null)).toBe(false);
      expect(guard(undefined)).toBe(false);
    }
  });
});
