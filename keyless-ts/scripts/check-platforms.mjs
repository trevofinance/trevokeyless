import { build } from "esbuild";
import { chromium } from "playwright";
import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import vm from "node:vm";

const args = process.argv.slice(2);
const resolverOnly = args.includes("--resolver-only");
const runtime = args.includes("--runtime") || !resolverOnly;
const browserOnly = args.includes("--browser-only");
const hermesOnly = args.includes("--hermes-only");
if (browserOnly && hermesOnly) {
  throw new Error("--browser-only and --hermes-only are mutually exclusive");
}
const packageArgument = args.find((argument) => !argument.startsWith("--"));
const packageRoot = packageArgument
  ? `${resolve(packageArgument)}/`
  : `${fileURLToPath(new URL("../", import.meta.url))}/`;

const smokeMarker = "__TREVO_KEYLESS_SMOKE_OK__";

// The transactional address for appAgentId=123, taId=7 in the default SS58 format
// (5335, Trevo Asset Hub). Pinned from the Rust source of truth — the same encoding the
// conformance fixture is generated from — and asserted verbatim below.
//
// Without this golden value the check proves only self-consistency: a runtime whose
// blake2b or base58 is *consistently* wrong (a broken TypedArray, a subtly different
// bigint) encodes wrong, decodes its own wrong output back to 123/7, and passes green.
// That is precisely the class of bug this browser/Hermes check exists to catch.
const goldenAddress = "ttowKp8AmayDzZndoxrpir8aoWExipuzTu9WFEimiRp9XdH85";

// The Named path is the one built from platform-sensitive constructs — the iterator spread of
// a Uint8Array in String.fromCharCode(...nameBytes), Uint8Array.from(name, ...), and a per-byte
// RegExp — none of which the Transactional golden above exercises. decodeAccountId
// (hex parsing, no SS58) is likewise never otherwise run outside Node. Both goldens are the
// Rust source of truth's output for AppAgent 123 / name "Treasury-1".
const goldenNamedAddress = "ttowKp8Amr7CWoMKAJY3HB7jTYkNwtizyuK85oxmPzeH3EQzx";
const goldenNamedAccountId =
  "0x7b0000000354726561737572792d31fb632a3b888b21fa1146108d0912111e7e";
const goldenAccountName = "Treasury-1";

const smokeSource = `
  import {
    AccountType,
    appAgentId,
    transactionalId,
    encodeTransactionalAddress,
    encodeNamedAddress,
    decodeAddress,
    decodeAccountId,
  } from "@trevofinance/trevokeyless";

  const address = encodeTransactionalAddress({
    appAgentId: appAgentId(123),
    taId: transactionalId(7),
  });
  if (address !== ${JSON.stringify(goldenAddress)}) {
    throw new Error(
      "platform bundle encoded the wrong address: expected " +
        ${JSON.stringify(goldenAddress)} +
        ", got " +
        address,
    );
  }
  const decoded = decodeAddress(address);
  if (
    decoded.accountType !== AccountType.Transactional ||
    decoded.appAgentId !== 123 ||
    decoded.taId !== 7
  ) {
    throw new Error("platform bundle round-trip failed");
  }

  const namedAddress = encodeNamedAddress({
    appAgentId: appAgentId(123),
    accountName: ${JSON.stringify(goldenAccountName)},
  });
  if (namedAddress !== ${JSON.stringify(goldenNamedAddress)}) {
    throw new Error(
      "platform bundle encoded the wrong Named address: expected " +
        ${JSON.stringify(goldenNamedAddress)} +
        ", got " +
        namedAddress,
    );
  }
  const decodedName = decodeAddress(namedAddress);
  if (
    decodedName.accountType !== AccountType.Named ||
    decodedName.appAgentId !== 123 ||
    decodedName.accountName !== ${JSON.stringify(goldenAccountName)}
  ) {
    throw new Error("platform bundle Named round-trip failed");
  }
  if (decodedName.accountId !== ${JSON.stringify(goldenNamedAccountId)}) {
    throw new Error(
      "platform bundle produced the wrong Named account id: got " +
        decodedName.accountId,
    );
  }

  const fromAccountId = decodeAccountId(${JSON.stringify(goldenNamedAccountId)});
  if (
    fromAccountId.accountType !== AccountType.Named ||
    fromAccountId.accountName !== ${JSON.stringify(goldenAccountName)} ||
    fromAccountId.address !== null
  ) {
    throw new Error("platform bundle decodeAccountId failed");
  }

  globalThis.${smokeMarker} = true;
  if (typeof print === "function") print("${smokeMarker}");
`;

const targets = {
  browser: {
    platform: "browser",
    conditions: ["browser", "import", "default"],
    mainFields: ["browser", "module", "main"],
  },
  reactNative: {
    platform: "neutral",
    conditions: ["react-native", "import", "default"],
    mainFields: ["react-native", "module", "main"],
  },
};

async function bundle(target, languageTarget) {
  const result = await build({
    absWorkingDir: packageRoot,
    bundle: true,
    conditions: target.conditions,
    format: "iife",
    logLevel: "silent",
    mainFields: target.mainFields,
    platform: target.platform,
    stdin: { contents: smokeSource, resolveDir: packageRoot },
    target: languageTarget,
    write: false,
  });
  return result.outputFiles[0].text;
}

async function checkResolvers() {
  for (const [name, target] of Object.entries(targets)) {
    const source = await bundle(target, "es2022");
    const context = vm.createContext({});
    vm.runInContext(source, context, { filename: `${name}-bundle.js` });
    if (context[smokeMarker] !== true) {
      throw new Error(`${name} bundle did not execute in the resolver smoke`);
    }
  }
  console.log("browser and React Native export resolver checks passed");
}

async function checkBrowserRuntime() {
  const source = await bundle(targets.browser, "es2022");
  const browser = await chromium.launch({ headless: true });
  const version = browser.version();
  try {
    const page = await browser.newPage();
    const pageErrors = [];
    page.on("pageerror", (error) => pageErrors.push(error));
    await page.setContent("<!doctype html><html><body></body></html>");
    await page.addScriptTag({ content: source });
    if (pageErrors.length > 0) {
      throw pageErrors[0];
    }
    if (
      (await page.evaluate((marker) => globalThis[marker], smokeMarker)) !==
      true
    ) {
      throw new Error("browser bundle did not execute in Chromium");
    }
  } finally {
    await browser.close();
  }
  console.log(`browser runtime check passed in Chromium ${version}`);
}

function defaultHermesBinary() {
  const platformDirectory = {
    linux: "linux64-bin",
    darwin: "osx-bin",
    win32: "win64-bin",
  }[process.platform];
  if (!platformDirectory || !["x64"].includes(process.arch)) {
    throw new Error(
      "No bundled Hermes CLI is available for this host; set HERMES_BINARY to a Hermes executable",
    );
  }
  const executable = process.platform === "win32" ? "hermes.exe" : "hermes";
  return fileURLToPath(
    new URL(
      `../node_modules/hermes-engine-cli/${platformDirectory}/${executable}`,
      import.meta.url,
    ),
  );
}

async function checkHermesRuntime() {
  const hermesBinary = resolve(
    process.env.HERMES_BINARY ?? defaultHermesBinary(),
  );
  if (!existsSync(hermesBinary)) {
    throw new Error(`Hermes executable not found: ${hermesBinary}`);
  }
  const source = await bundle(targets.reactNative, "es2015");
  const result = spawnSync(hermesBinary, [], {
    encoding: "utf8",
    input: source,
    maxBuffer: 10 * 1024 * 1024,
  });
  if (result.error) {
    throw result.error;
  }
  if (result.status !== 0 || !result.stdout.includes(smokeMarker)) {
    throw new Error(
      `Hermes runtime check failed (exit ${String(result.status)}):\n${result.stderr || result.stdout}`,
    );
  }
  const version = spawnSync(hermesBinary, ["-version"], { encoding: "utf8" });
  if (version.error || version.status !== 0) {
    throw version.error ?? new Error("Hermes did not report its version");
  }
  console.log(
    `React Native runtime check passed in ${version.stdout.trim().split("\n")[0]}`,
  );
}

if (resolverOnly) {
  await checkResolvers();
}
if (runtime) {
  if (!hermesOnly) {
    await checkBrowserRuntime();
  }
  if (!browserOnly) {
    await checkHermesRuntime();
  }
}
