/**
 * Executes every JavaScript example of the package README against the installed
 * package.
 *
 * The Rust README is compiled as a doctest, so it cannot drift from the crate; this
 * script gives the TypeScript README the same protection. CI runs it from the
 * interop-smoke directory (where the packed tarball is installed), so the examples'
 * `require("@trevofinance/trevokeyless")` resolves exactly the artifact a user would
 * install.
 *
 * The README's first `javascript` block is the import prelude; every following block
 * is one self-contained example. Each example runs in its own Node process (prelude +
 * block), exactly like a user pasting one example under the documented imports.
 *
 * Usage: node run-readme.mjs <path-to-README.md>  (cwd must resolve the package)
 */
import { spawnSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const readmePath = process.argv[2];
if (!readmePath) {
  console.error("usage: node run-readme.mjs <path-to-README.md>");
  process.exit(2);
}

const fence = /^```javascript\s*$\n([\s\S]*?)^```\s*$/gm;
const blocks = [...readFileSync(readmePath, "utf8").matchAll(fence)].map(
  (m) => m[1],
);

// The exact block count is pinned so a silent extraction regression (e.g. the fence
// tags being renamed) or a removed/unparsed example cannot pass silently: the README
// currently holds the import prelude plus eleven examples. Update the number when
// adding a README example.
if (blocks.length !== 12) {
  console.error(
    `expected exactly 12 JavaScript blocks in the README, got ${blocks.length}`,
  );
  process.exit(1);
}
const [prelude, ...examples] = blocks;
if (!prelude.includes("require(")) {
  console.error("the first README block must be the require() import prelude");
  process.exit(1);
}

// The example files must live under the cwd: CommonJS resolves require() from the
// script file's directory chain, so the files need the cwd's node_modules (where the
// packed tarball is installed) above them.
const workDir = mkdtempSync(join(process.cwd(), "readme-examples-"));
// process.exit() does not unwind, so exiting inside the try block would skip the
// finally cleanup; the failure path only sets a flag and exits after the temp dir
// has been removed.
let failed = false;
try {
  for (const [index, example] of examples.entries()) {
    const file = join(workDir, `example-${index}.cjs`);
    writeFileSync(file, `${prelude}\n${example}`);
    const result = spawnSync(process.execPath, [file], {
      stdio: ["ignore", "pipe", "pipe"],
      encoding: "utf8",
    });
    if (result.status !== 0) {
      console.error(`README example ${index} failed:\n${example}`);
      // The example's stdout carries its console.log diagnostics — print it too.
      if (result.stdout) {
        console.error(`--- stdout ---\n${result.stdout}`);
      }
      console.error(`--- stderr ---\n${result.stderr}`);
      failed = true;
      break;
    }
  }
} finally {
  rmSync(workDir, { recursive: true, force: true });
}
if (failed) {
  process.exit(1);
}
console.log(`README examples OK: ${examples.length} blocks executed`);
