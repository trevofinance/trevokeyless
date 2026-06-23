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

// Catches a silent extraction regression (e.g. the fence tags being renamed): the
// README currently holds the import prelude plus seven examples.
if (blocks.length < 8) {
  console.error(
    `expected the README's JavaScript examples to be found, got ${blocks.length} blocks`,
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
try {
  examples.forEach((example, index) => {
    const file = join(workDir, `example-${index}.cjs`);
    writeFileSync(file, `${prelude}\n${example}`);
    const result = spawnSync(process.execPath, [file], {
      stdio: ["ignore", "pipe", "pipe"],
      encoding: "utf8",
    });
    if (result.status !== 0) {
      console.error(`README example ${index} failed:\n${example}`);
      console.error(result.stderr);
      process.exit(1);
    }
  });
  console.log(`README examples OK: ${examples.length} blocks executed`);
} finally {
  rmSync(workDir, { recursive: true, force: true });
}
