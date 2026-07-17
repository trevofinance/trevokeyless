/** Compile-only assertions for identifier brands and decoded discriminated unions. */
import type { BlockchainAccountInfo, RegularAccountInfo } from "../src/keyless";
import {
  AccountType,
  accountName,
  appAgentId,
  decodeAccountId,
  decodeAddress,
  encodeAppAgentAccountId,
  encodeAppAgentAddress,
  encodeNamedAddress,
  encodeTransactionalAddress,
  transactionalId,
} from "../src/keyless";

const aaId = appAgentId(123);
const txId = transactionalId(7);
const name = accountName("Treasury-1");

encodeAppAgentAddress({ appAgentId: aaId });
encodeTransactionalAddress({ appAgentId: aaId, taId: txId });
encodeNamedAddress({ appAgentId: aaId, accountName: name });

encodeNamedAddress({
  appAgentId: aaId,
  // @ts-expect-error An unvalidated string must not be accepted as an AccountName.
  accountName: "Treasury-1",
});

// @ts-expect-error A TransactionalId must not be accepted as an AppAgentId.
encodeAppAgentAddress({ appAgentId: txId });
encodeTransactionalAddress({
  // @ts-expect-error Object fields preserve the distinct AppAgentId brand.
  appAgentId: txId,
  // @ts-expect-error Object fields preserve the distinct TransactionalId brand.
  taId: aaId,
});

const regular: RegularAccountInfo = {
  address: null,
  accountId: `0x${"00".repeat(32)}`,
  accountType: AccountType.Regular,
};

// @ts-expect-error Decoded values are immutable.
regular.accountId = "0xdeadbeef";

const invalidRegular: RegularAccountInfo = {
  address: null,
  accountId: `0x${"00".repeat(32)}`,
  accountType: AccountType.Regular,
  // @ts-expect-error Regular account info cannot carry an AppAgent identifier.
  appAgentId: aaId,
};
void invalidRegular;

declare const decoded: BlockchainAccountInfo;
if (decoded.accountType === AccountType.AppAgent) {
  void decoded.appAgentId;
  // @ts-expect-error An AppAgent variant cannot carry a Transactional identifier.
  void decoded.taId;
}

// The address is a hard invariant of the entry point, not something callers test for.
// decodeAddress always has one; decodeAccountId never does.
const fromAddress = decodeAddress(encodeAppAgentAddress({ appAgentId: aaId }));
// No null check, and no non-null assertion, is needed: it is a string.
const addressLength: number = fromAddress.address.length;
void addressLength;

const fromAccountId = decodeAccountId(
  encodeAppAgentAccountId({ appAgentId: aaId }),
);
// ...and here it is always null, so reading a string property off it is an error.
// @ts-expect-error decodeAccountId results never carry an address.
void fromAccountId.address.length;
