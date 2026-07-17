import { spawnSync } from "node:child_process";

const archive = process.argv[2];
if (!archive) {
  console.error("usage: node scripts/check-package.mjs <npm-pack.tgz>");
  process.exit(2);
}

const listed = spawnSync("tar", ["-tf", archive], { encoding: "utf8" });
if (listed.status !== 0) {
  process.stderr.write(listed.stderr);
  process.exit(listed.status ?? 1);
}

const actual = listed.stdout.trim().split("\n").sort();
const expected = [
  "package/CHANGELOG.md",
  "package/LICENSE",
  "package/README.md",
  "package/dist/cjs/keyless.d.ts",
  "package/dist/cjs/keyless.js",
  "package/dist/cjs/ss58.d.ts",
  "package/dist/cjs/ss58.js",
  "package/dist/esm/keyless.d.ts",
  "package/dist/esm/keyless.js",
  "package/dist/esm/package.json",
  "package/dist/esm/ss58.d.ts",
  "package/dist/esm/ss58.js",
  "package/package.json",
].sort();

if (JSON.stringify(actual) !== JSON.stringify(expected)) {
  console.error("npm archive contents differ from the exact allowlist");
  console.error(
    `expected:\n${expected.join("\n")}\nactual:\n${actual.join("\n")}`,
  );
  process.exit(1);
}

console.log(`npm archive contents OK: ${actual.length} files`);
