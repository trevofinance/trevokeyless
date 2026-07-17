import { readFileSync } from "node:fs";
import {
  AccountNameError,
  AccountType,
  AccountTypeMismatchError,
  ParseAccountIdError,
  Ss58Error,
  accountName,
  appAgentId,
  decodeAccountId,
  decodeAddress,
  decodeAddressWithFormat,
  decodeAppAgentAccountId,
  decodeNamedAccountId,
  decodeTransactionalAccountId,
  encodeAppAgentAccountId,
  encodeAddress,
  encodeNamedAccountId,
  encodeTransactionalAccountId,
  transactionalId,
} from "../dist/esm/keyless.js";

const path = process.argv[2];
const surrogatePath = process.argv[3];
if (!path) {
  console.error(
    "usage: node scripts/differential.mjs <cases.json> [surrogate-cases.json]",
  );
  process.exit(2);
}

const accountTypeNames = {
  [AccountType.Regular]: "Regular",
  [AccountType.AppAgent]: "AppAgent",
  [AccountType.Transactional]: "Transactional",
  [AccountType.Named]: "Named",
};

function normalizedInfo(info, ss58Format) {
  return {
    status: "ok",
    format: ss58Format,
    account_id: info.accountId,
    account_type: accountTypeNames[info.accountType],
    app_agent_id: "appAgentId" in info ? info.appAgentId : null,
    ta_id: "taId" in info ? info.taId : null,
    account_name: "accountName" in info ? info.accountName : null,
  };
}

function normalizedError(error) {
  if (!(error instanceof Ss58Error)) {
    throw error;
  }
  const result = { status: "error", kind: error.kind };
  if (error.kind === "UnexpectedFormat") {
    result.expected = error.expected;
    result.found = error.found;
  }
  return result;
}

function encodeResult(testCase) {
  const aaId = appAgentId(testCase.app_agent_id);
  let accountId;
  switch (testCase.kind) {
    case "app_agent":
      accountId = encodeAppAgentAccountId({ appAgentId: aaId });
      break;
    case "transactional":
      accountId = encodeTransactionalAccountId({
        appAgentId: aaId,
        taId: transactionalId(testCase.ta_id),
      });
      break;
    case "named":
      accountId = encodeNamedAccountId({
        appAgentId: aaId,
        accountName: testCase.account_name,
      });
      break;
    default:
      throw new Error(`unknown case kind ${testCase.kind}`);
  }
  const info = decodeAccountId(accountId);
  return {
    kind: testCase.kind,
    account_id: accountId,
    address: encodeAddress(accountId, testCase.format),
    account_type: accountTypeNames[info.accountType],
    app_agent_id: "appAgentId" in info ? info.appAgentId : null,
    ta_id: "taId" in info ? info.taId : null,
    account_name: "accountName" in info ? info.accountName : null,
  };
}

function decodeResult(testCase) {
  let expecting;
  try {
    expecting = normalizedInfo(
      decodeAddress(testCase.address, testCase.format),
      testCase.format,
    );
  } catch (error) {
    expecting = normalizedError(error);
  }

  let anyFormat;
  try {
    const decoded = decodeAddressWithFormat(testCase.address);
    anyFormat = normalizedInfo(decoded.info, decoded.ss58Format);
  } catch (error) {
    anyFormat = normalizedError(error);
  }

  return {
    label: testCase.label,
    expecting,
    any_format: anyFormat,
  };
}

// Account names, valid and hostile. The AccountNameError kind is a cross-language contract,
// so it is reported rather than a bare rejection — this is what catches a `String.length`
// name rule masquerading as the UTF-8 byte rule.
function nameResult(testCase) {
  let result;
  try {
    result = {
      status: "ok",
      account_id: encodeNamedAccountId({
        appAgentId: appAgentId(testCase.app_agent_id),
        accountName: accountName(testCase.name),
      }),
    };
  } catch (error) {
    if (!(error instanceof AccountNameError)) {
      throw error;
    }
    result = { status: "error", kind: error.kind };
  }
  return { label: testCase.label, result };
}

// SS58 formats an encoder must accept or reject. Only the rejection is compared: the
// classification of a rejected *argument* is per-language. An Ss58Error is deliberately not
// caught — it reports on an *address*, and the address here is valid, so letting it escape
// fails the differential run loudly. That is the regression this pins.
function formatResult(testCase) {
  let result;
  try {
    result = {
      status: "ok",
      address: encodeAddress(testCase.account_id, testCase.format),
    };
  } catch (error) {
    if (error instanceof Ss58Error || !(error instanceof RangeError)) {
      throw error;
    }
    result = { status: "rejected" };
  }
  return { label: testCase.label, result };
}

function typedResult(decode, accountId) {
  try {
    decode(accountId);
  } catch (error) {
    if (!(error instanceof AccountTypeMismatchError)) {
      throw error;
    }
    return {
      status: "mismatch",
      expected: accountTypeNames[error.expected],
      found: accountTypeNames[error.found],
    };
  }
  return { status: "ok" };
}

// Raw 32-byte account ids through the classifier and all three typed decoders.
function classifyResult(testCase) {
  const info = decodeAccountId(testCase.hex);
  return {
    label: testCase.label,
    info: {
      account_id: info.accountId,
      account_type: accountTypeNames[info.accountType],
      app_agent_id: "appAgentId" in info ? info.appAgentId : null,
      ta_id: "taId" in info ? info.taId : null,
      account_name: "accountName" in info ? info.accountName : null,
    },
    typed: {
      app_agent: typedResult(decodeAppAgentAccountId, testCase.hex),
      transactional: typedResult(decodeTransactionalAccountId, testCase.hex),
      named: typedResult(decodeNamedAccountId, testCase.hex),
    },
  };
}

// The raw account-id string contract: "0x" + exactly 64 hex characters, both digit cases.
function hexResult(testCase) {
  let result;
  try {
    const info = decodeAccountId(testCase.input);
    result = {
      status: "ok",
      account_id: info.accountId,
      account_type: accountTypeNames[info.accountType],
    };
  } catch (error) {
    if (!(error instanceof ParseAccountIdError)) {
      throw error;
    }
    result = { status: "error" };
  }
  return { label: testCase.label, result };
}

const payload = JSON.parse(readFileSync(path, "utf8"));
const results = {
  encode: payload.encode_cases.map(encodeResult),
  decode: payload.decode_cases.map(decodeResult),
  name: payload.name_cases.map(nameResult),
  format: payload.format_cases.map(formatResult),
  classify: payload.classify_cases.map(classifyResult),
  hex: payload.hex_cases.map(hexResult),
};

// Lone-surrogate host strings travel in a separate file: serde_json rejects
// lone-surrogate escapes, so the Rust oracle never sees these cases and the harness
// compares this output directly against the Python side instead
// (see .github/scripts/check_cross_language.py).
if (surrogatePath) {
  const surrogates = JSON.parse(readFileSync(surrogatePath, "utf8"));
  results.surrogate_name = surrogates.name_cases.map(nameResult);
  results.surrogate_decode = surrogates.decode_cases.map(decodeResult);
}

process.stdout.write(JSON.stringify(results));
