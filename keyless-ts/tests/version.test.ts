import fs from "node:fs";
import path from "node:path";

import { Ss58Error } from "../src/ss58";

/**
 * Pins the version declarations of this package to each other — the per-package
 * counterpart of the Python `tests/test_version.py`. The repository-wide release helper
 * (`.github/scripts/release_metadata.py`, run by check-docs) verifies the full set of
 * sync points; these tests make a TS-only drift fail this package's own suite instead
 * of only the docs gate.
 */
describe("Version sync", () => {
  const read = (file: string): Record<string, unknown> =>
    JSON.parse(
      fs.readFileSync(path.join(__dirname, "..", file), "utf8"),
    ) as Record<string, unknown>;

  const manifest = read("package.json");
  const lock = read("package-lock.json");
  const lockRootPackage = (lock.packages as Record<string, unknown>)[
    ""
  ] as Record<string, unknown>;

  it("package-lock.json pins the package.json version, in both places", () => {
    // `npm version --no-git-tag-version` (what release_metadata.py --set ts runs)
    // rewrites all three; a hand-edited manifest leaves the lock behind.
    expect(lock.version).toBe(manifest.version);
    expect(lockRootPackage.version).toBe(manifest.version);
  });

  it("package-lock.json carries the manifest's Node engine range", () => {
    // The lock's engines copy is what `npm ci` enforces for developers; a manifest-only
    // engines bump would silently leave it stale.
    expect(lockRootPackage.engines).toEqual(manifest.engines);
  });

  it("the cross-realm brand namespace carries the manifest's major version", () => {
    // BRAND_NAMESPACE (src/ss58.ts) embeds the package major precisely so two
    // coexisting majors do not cross-brand each other's errors — but it is a
    // hand-written constant that `release_metadata.py --set` never touches. Pin it
    // here through observable behavior: the brand symbol on the error prototype is
    // registered process-wide, so its `Symbol.keyFor` key must spell the manifest's
    // name and major. A major release without the matching namespace bump fails this
    // test instead of silently re-enabling cross-major misrecognition.
    const brandKeys = Object.getOwnPropertySymbols(Ss58Error.prototype)
      .map((symbol) => Symbol.keyFor(symbol))
      .filter((key): key is string => key !== undefined);
    const major = (manifest.version as string).split(".")[0];
    expect(brandKeys).toEqual([
      `${manifest.name as string}@${major}:Ss58Error`,
    ]);
  });
});
