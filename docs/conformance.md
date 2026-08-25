# Conformance fixture maintenance

`conformance/vectors.json` is the generated executable cross-language specification. The
Rust crate is the source of truth; all three suites consume the fixture, and CI compares
fresh generator output byte-for-byte with the committed file.

Never hand-edit the fixture. Generate from `keyless-rust/`:

```console
cargo run --example gen_conformance --features ss58 > ../conformance/vectors.json
```

A change to a value field of an existing vector—such as `hex`, `ss58`, `account_id`, or
`expected`—changes the frozen on-chain format. Stop and obtain explicit confirmation
before proceeding. Adding vectors or groups is the normal way to pin newly specified
behavior.

## Adding a vector or group

1. Add the vector or group in `keyless-rust/examples/gen_conformance.rs`. Derive values
   from the implementation and add a generator self-check that would fail if the emitted
   vector disagreed with Rust.
2. For a new group, add its `print_group(...)` call; only the final group passes `true`.
3. Regenerate the fixture and review the complete diff.
4. Update exact count pins in:
   - `keyless-rust/src/tests/tests_conformance.rs`;
   - `keyless-python/tests/test_conformance.py`;
   - `keyless-ts/tests/conformance.test.ts`.
5. For a new group, add a consuming test in all three suites, its TypeScript fixture
   interface member, and its entry in [conformance-schema.md](conformance-schema.md).
6. Run all three suites and the cross-language differential check.

All parts land with the behavior in one pull request.

## Running without the fixture

Published artifacts do not contain the repository-root fixture.

- `TREVOKEYLESS_CONFORMANCE_SKIP` explicitly skips Rust or Python conformance tests outside
  the repository. Rust also skips by default when the entire repository conformance
  directory is absent, as in a published crate.
- `TREVOKEYLESS_CONFORMANCE_REQUIRE` turns an unexpected skip into a failure and is set by
  Rust and Python CI jobs.

Inside the repository, set neither variable; a missing fixture is a failure. TypeScript
always runs from the checkout and reads the fixture unconditionally.

The complete group inventory, field definitions, language entrypoints, and host-boundary
limitations are in [conformance-schema.md](conformance-schema.md).
