# Changelog — trevokeyless (Python)

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); the
package follows [semantic versioning](https://semver.org). The byte encoding itself is
frozen and never changes at any version.

## [2.0.0] — 2026-07-21

Everything below is the delta against 1.0.0, the previous — and only other — release of
this package on PyPI.

### Added

- `decode_address_with_format` and its `DecodedAddressWithFormat` result: decode an
  SS58 address of *any* (non-reserved) format and return the classification together
  with the format found in the address — the counterpart of the Rust
  `AccountId32::from_ss58check_with_version` for addresses whose format is not known
  up front. Prefer `decode_address` whenever the expected format is known.
- Typed raw-account-id decoders `decode_app_agent_account_id`,
  `decode_transactional_account_id`, and `decode_named_account_id` — the account-id
  counterparts of the typed address decoders, mirroring the Rust
  `decode_app_agent_account` / `decode_transactional_account` / `decode_named_account`.
  They raise `AccountTypeMismatchError` on a classification mismatch, exactly like the
  address versions.
- `ParseAccountIdError` and `NotAnAddressError`, both `ValueError` subclasses (so
  existing `except ValueError` handlers keep working): the raw-account-id entry points
  (`decode_account_id`, `encode_address`, `is_keyless_account_id`, `decode_*_account_id`)
  now raise the former for input that is not `0x` + 64 hex characters — mirroring the
  Rust `ParseAccountIdError` — and the SS58 address entry points raise the latter when
  handed a raw account id. The messages are unchanged.
- The byte-`[4]` account type discriminants are public under their Rust names:
  `APP_AGENT_ACCOUNT_IDENTIFIER` (1), `TRANSACTIONAL_ACCOUNT_IDENTIFIER` (2),
  `NAMED_ACCOUNT_IDENTIFIER` (3).
- `is_app_agent_id`, `is_transactional_id`, and `is_account_name`: non-throwing
  counterparts of the validating constructors, mirroring the TypeScript
  `isAppAgentId` / `isTransactionalId` / `isAccountName` type guards. Each returns
  `True` exactly when the constructor would accept the value and narrows it for a
  type checker (`typing.TypeGuard`).
- Per-version PyPI classifiers for Python 3.10–3.14.

### Changed

- **Breaking:** `Ss58Error` takes only `kind` — the optional `message` parameter is gone,
  so a caller can no longer construct an error whose message contradicts its kind (the
  wording is a cross-language contract; the docstring already promised this). The
  messages themselves are unchanged, derived from the kind. `Ss58FormatMismatchError`
  still renders its pair-specific message. `AccountTypeMismatchError` now coerces its
  `(expected, found)` pair through `AccountType`, so an invalid argument fails with the
  enum's own `ValueError` instead of leaking an `AttributeError` from the message
  rendering; the numeric values 0–3 are accepted and coerce to the members.
- **Breaking:** `AppAgentId` and `TransactionalId` are distinct validating `int`
  subclasses; 1.0.0 annotated them as plain `int` aliases that validated nothing. The
  constructor requires the value and rejects out-of-range ints and `bool`, so
  `AppAgentId(-1)` now raises at the point of construction (the TypeScript
  `appAgentId(-1)` already threw). They remain usable anywhere an `int` is, and the
  encoders keep validating raw ints from untyped callers.
- **Breaking:** `AccountName` is a validating `str` subclass (10 UTF-8 bytes from
  `0-9 a-z A-Z - #`), the counterpart of the Rust newtype and the TypeScript
  `accountName()` factory; 1.0.0 annotated names as a plain `str` alias that validated
  nothing. Passing a raw `str` still works at runtime; the encoders validate whatever
  they are handed.
- **Breaking:** decoded account info is a union of four frozen, slotted, keyword-only
  dataclasses (`RegularAccountInfo | AppAgentAccountInfo | TransactionalAccountInfo |
  NamedAccountInfo`); every variant contains only the identifiers valid for its
  classification. 1.0.0 returned a single dataclass with nullable fields. Multi-identifier
  encoders are keyword-only and Named encoders consistently use `account_name`,
  preventing positional transposition.
- **Breaking:** account-name validation now measures the **UTF-8 byte length** of the
  name, like the Rust implementation (the source of truth), instead of its number of
  code points. The set of accepted names is unchanged — every allowed character is
  one-byte ASCII — but a rejected non-ASCII name can now get the other
  `AccountNameError` kind: `"café12345"` (9 characters, 10 bytes) is an
  `"InvalidCharacter"` where it used to be an `"InvalidLength"`, and `"héllo12345"`
  (10 characters, 11 bytes) stays an `"InvalidLength"`. This removes a real
  cross-language divergence in the error taxonomy.
- **Breaking:** `ss58_format` is keyword-only in the decoders (`decode_address`,
  `decode_app_agent_address`, `decode_transactional_address`, `decode_named_address`,
  `is_keyless_address`) and in `encode_address`, as it already was in the encoders.
  Callers passing it positionally must now pass `ss58_format=...`.
- **Breaking:** `encode_address` validates `ss58_format` before the account-id payload,
  exactly like the decode path: the format is a caller *parameter* (a programmer error)
  while the account id is *data*, so when both are invalid the caller's own bug is
  reported first. Previously it validated the payload first and disagreed with
  `decode_address` about which of two bad arguments to report. (The keyless encoders
  are different, deliberately: their identifier/name arguments are validated before the
  format, in declaration order, matching the TypeScript implementation.)
- **Breaking:** the encoding functions reject the reserved SS58 formats 46 and 47 with
  the same `ValueError` as any other unusable format. Previously they fell through to
  the SS58 layer and raised `Ss58Error` kind `"InvalidFormat"` — the type that reports
  on an *address* — for what is a rejected *argument*. Decoders still accept 46 and 47
  as expected-format inputs (no valid address can carry them, so the result is the
  address's own validation error or `Ss58FormatMismatchError`).
- **Breaking:** the public constant `NAMED_ADDRESS_LENGTH` is renamed
  `ACCOUNT_NAME_LENGTH` — it is the length of an account *name*, not of an address —
  matching the name Rust and TypeScript export.
- **Breaking:** `Ss58Error` and `AccountNameError` reject an out-of-taxonomy `kind`. The five
  SS58 kinds and two name kinds are a pinned cross-language contract, and an unknown kind was
  being silently mapped to the wrong message.
- `AccountType` is an `IntEnum` (1.0.0 used a plain `Enum`); its numeric values
  (Regular = 0, AppAgent = 1, Transactional = 2, Named = 3) are shared with the Rust
  and TypeScript implementations, and the keyless values equal the byte-`[4]`
  discriminants.
- Named-account validation raises `AccountNameError` with a stable
  `InvalidLength`/`InvalidCharacter` kind (a `ValueError` subclass, so existing
  handlers keep working).
- The name-validation error messages now read "Account name must be exactly
  10 characters long" / "Account name contains invalid characters" (previously
  "Address name …"), unifying the wording shared verbatim by the Rust, Python, and
  TypeScript implementations.

### Fixed

- Every public exception now survives `pickle` and `copy` with its structured data intact.
  `BaseException.__reduce__` rebuilds an exception as `cls(*args)` where `args` is just the
  message, so `Ss58Error`, `Ss58FormatMismatchError`, `AccountTypeMismatchError`,
  `ParseAccountIdError` and `NotAnAddressError` raised an opaque `TypeError` from the
  unpickler — and `AccountNameError` unpickled "successfully" with its message silently
  flipped to the `InvalidCharacter` text while its `kind` still said `InvalidLength`. This
  affects any consumer propagating an error across a process boundary (`multiprocessing`,
  `ProcessPoolExecutor`, Celery) or deep-copying a result that carries one.
- Account-name validation short-circuits on the code-point count before walking the string:
  rejecting a 20-million-character name took ~1.2 s and now takes constant time.
- UTF-8 length checks now classify programmatically constructed surrogate pairs the
  same way as JavaScript's `TextEncoder`, while retaining the defined behavior for
  lone surrogates.
- `Ss58Error("UnexpectedFormat")` — the one kind with no fixed standard message — raises a
  descriptive `ValueError` pointing at `Ss58FormatMismatchError(expected, found)` instead
  of escaping as a `KeyError` from the internal message table.
- Out-of-range identifiers and `ss58_format` values beyond 64 bits are reported by bit
  length (`got a 33220-bit int`) instead of echoed verbatim. Echoing a huge int tripped
  CPython's int→str conversion limit, replacing the documented message with "Exceeds the
  limit (4300 digits) for integer string conversion" (and Pythons without the limit
  echoed megabytes of adversarial input back at the caller). Plausible wrong-width
  mistakes (`2**40`, `2**53 + 1`, `-(2**32)`) are still echoed verbatim.
- Repository URLs in the package metadata pointed at a nonexistent repository
  (`trevo-keyless` instead of `trevokeyless`).

### Internal

- The conformance suite pins the `error_kind` of every `named_invalid_encode` vector and
  the exact vector count of every fixture group; the raw-account-id encoders and the new
  typed account-id decoders have rejection tests of their own.
- Development and audit dependencies are fully hash-locked; CI audits that lock
  rather than incidental runner packages.
- Wheel and sdist contents are checked against exact allowlists, and the generated
  bytecode accidentally committed to the repository was removed.
- Added a seeded Rust/Python/TypeScript differential repository check.

## [1.0.0] — 2026-07-05

### Added

- Initial stable release: keyless address encoding/decoding/classification
  (`encode_*_address`, `decode_address`, typed `decode_*_address`,
  `is_keyless_address`), the raw account-id entry points (`encode_*_account_id`,
  `decode_account_id`, `encode_address`, `is_keyless_account_id`), the strict
  vendored SS58 codec (zero runtime dependencies), and verification against the
  cross-language conformance fixture.
