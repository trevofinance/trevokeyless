"""Differentially exercise valid and hostile inputs across all three implementations."""

from __future__ import annotations

import argparse
import hashlib
import json
import random
import shlex
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any

REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPOSITORY_ROOT / "keyless-python"))

from trevokeyless import (  # noqa: E402
    AccountName,
    AccountNameError,
    AccountType,
    AccountTypeMismatchError,
    AppAgentId,
    ParseAccountIdError,
    Ss58Error,
    Ss58FormatMismatchError,
    TransactionalId,
    decode_account_id,
    decode_address,
    decode_address_with_format,
    decode_app_agent_account_id,
    decode_named_account_id,
    decode_transactional_account_id,
    encode_address,
    encode_app_agent_account_id,
    encode_named_account_id,
    encode_transactional_account_id,
)
from trevokeyless._ss58 import _b58decode, _b58encode  # noqa: E402

# The Rust crate is the source of truth for the encoding; every other implementation is
# checked against its results.
ORACLE = "Rust"

VALID_NAME_CHARS = "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ-#"
FORMAT_BOUNDARIES = [0, 42, 63, 64, 5335, 16383]
U32_BOUNDARIES = [0, 1, 2**32 - 1]
TRANSACTIONAL_BOUNDARY_CASES = 12


# The hostile sections scale with --cases, so a larger budget deepens the hostile
# coverage instead of only re-running the same fixed sets against more encode cases.
# The ratios reproduce the historical fixed sizes (16/32/96/128/24) exactly at the
# default --cases=256; the max(1, ...) floors keep every comparison path alive on a
# tiny local run (--cases is already bounded below by the format boundaries).
def hostile_source_case_count(encode_cases: int) -> int:
    """Encode results seeded into the decode-corruption section."""
    return max(1, encode_cases // 16)


def decode_garbage_case_count(encode_cases: int) -> int:
    """Wholly generated garbage strings for the decode section."""
    return max(1, encode_cases // 8)


def name_fuzz_case_count(encode_cases: int) -> int:
    """Generated hostile account names."""
    return max(1, encode_cases * 3 // 8)


def classify_fuzz_case_count(encode_cases: int) -> int:
    """Generated raw account ids for the classify section."""
    return max(1, encode_cases // 2)


def hex_fuzz_case_count(encode_cases: int) -> int:
    """Generated raw account-id strings for the hex section."""
    return max(1, encode_cases * 3 // 32)


# Characters for the hostile-name generator. The account-name rule is defined on the UTF-8
# *byte* length, not the code-point count, so a generator that only ever emits ASCII cannot
# tell a correct implementation from one that used `len(name)` - the single most likely
# Rust-vs-JavaScript divergence in the codebase. These span every UTF-8 width.
NAME_FUZZ_CHARS = (
    # 1 byte, in the charset
    "0aZ-#"
    # 1 byte, outside the charset
    "_ !.*"
    # 2 bytes
    "éüñ"
    # 3 bytes
    "中あ★"
    # 4 bytes
    "\U0001f680\U0001d11e"
    # combining mark (2 bytes, and a separate code point from the letter it follows)
    "́"
)

# SS58 formats an encoder must reject. Both ways a format can be unusable are covered: the
# reserved ones and the out-of-range ones. Restricted to the u16 range because the Rust
# oracle's format parameter is a u16 - the sub-u16 rejections (negative, non-integer) are
# not expressible across the JSON boundary and stay pinned by `u32_invalid_encode` and each
# language's own suite.
REJECTED_FORMATS = [46, 47, 16384, 0xFFFF]

# Garbage decode strings: the base58 alphabet itself, its four excluded homoglyphs
# (0, O, I, l), base64-ish punctuation and whitespace, and 2-/3-/4-byte UTF-8 characters —
# so the InvalidBase58-vs-InvalidLength classification (including the 50-UTF-8-byte bound)
# is compared on arbitrary generated input, not only on the fixed ss58_invalid vectors.
DECODE_GARBAGE_CHARS = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz0OIl+/= .\té中\U0001f680"

JsonObject = dict[str, Any]
DifferentialResults = dict[str, list[JsonObject]]


def build_cases(seed: int, count: int) -> list[JsonObject]:
    """Valid encode inputs: every case here must round-trip in all three implementations.

    Reserved formats are excluded *because* they must not encode; their rejection is the
    subject of `build_format_cases`, and out-of-charset names that of `build_name_cases`.
    """
    rng = random.Random(seed)  # noqa: S311 - reproducibility, not cryptography
    cases: list[JsonObject] = []
    for index in range(count):
        kind = ("app_agent", "transactional", "named")[index % 3]
        app_agent_id = U32_BOUNDARIES[index] if index < len(U32_BOUNDARIES) else rng.getrandbits(32)
        if index < len(FORMAT_BOUNDARIES):
            ss58_format = FORMAT_BOUNDARIES[index]
        else:
            ss58_format = rng.randrange(0, 2**14)
            while ss58_format in (46, 47):
                ss58_format = rng.randrange(0, 2**14)
        case: JsonObject = {
            "kind": kind,
            "format": ss58_format,
            "app_agent_id": app_agent_id,
        }
        if kind == "transactional":
            case["ta_id"] = rng.choice(U32_BOUNDARIES) if index < TRANSACTIONAL_BOUNDARY_CASES else rng.getrandbits(32)
        elif kind == "named":
            case["account_name"] = "".join(rng.choice(VALID_NAME_CHARS) for _ in range(10))
        cases.append(case)
    return cases


def python_encode_results(cases: list[JsonObject]) -> list[JsonObject]:
    results = []
    for case in cases:
        aa_id = AppAgentId(case["app_agent_id"])
        if case["kind"] == "app_agent":
            account_id = encode_app_agent_account_id(aa_id)
        elif case["kind"] == "transactional":
            account_id = encode_transactional_account_id(
                app_agent_id=aa_id,
                ta_id=TransactionalId(case["ta_id"]),
            )
        else:
            account_id = encode_named_account_id(
                app_agent_id=aa_id,
                account_name=case["account_name"],
            )
        info = decode_account_id(account_id)
        results.append(
            {
                "kind": case["kind"],
                "account_id": account_id,
                "address": encode_address(account_id, ss58_format=case["format"]),
                "account_type": AccountType(info.account_type).name,
                "app_agent_id": getattr(info, "app_agent_id", None),
                "ta_id": getattr(info, "ta_id", None),
                "account_name": getattr(info, "account_name", None),
            }
        )
    return results


def different_valid_format(ss58_format: int) -> int:
    candidate = (ss58_format + 1) % (2**14)
    while candidate in (46, 47) or candidate == ss58_format:
        candidate = (candidate + 1) % (2**14)
    return candidate


def build_decode_cases(
    seed: int,
    encode_cases: list[JsonObject],
    encode_results: list[JsonObject],
    hostile_sources: int,
    garbage_cases: int,
) -> list[JsonObject]:
    fixture = json.loads((REPOSITORY_ROOT / "conformance" / "vectors.json").read_text(encoding="utf8"))
    rng = random.Random(seed ^ 0xDEC0DE5)  # noqa: S311 - reproducibility, not cryptography
    cases: list[JsonObject] = []

    for index, vector in enumerate(fixture["ss58_invalid"]):
        expected_error: JsonObject = {"status": "error", "kind": vector["error_kind"]}
        if vector["error_kind"] == "UnexpectedFormat":
            expected_error.update(expected=vector["expected"], found=vector["found"])
        cases.append(
            {
                "label": f"fixture-invalid-{index}: {vector['reason']}",
                "address": vector["ss58"],
                "format": vector["format"],
                "expected_error": expected_error,
            }
        )

    for index, vector in enumerate(fixture["ss58_decode_only"]):
        cases.append(
            {
                "label": f"fixture-decode-only-{index}: {vector['reason']}",
                "address": vector["ss58"],
                "format": vector["format"],
            }
        )

    sources = zip(
        encode_cases[:hostile_sources],
        encode_results[:hostile_sources],
        strict=True,
    )
    for index, (source_case, result) in enumerate(sources):
        address = result["address"]
        ss58_format = source_case["format"]
        decoded = bytearray(_b58decode(address))

        corrupted_checksum = bytearray(decoded)
        corrupted_checksum[-1] ^= 0x01
        cases.append(
            {
                "label": f"generated-{index}-checksum-byte-flip",
                "address": _b58encode(bytes(corrupted_checksum)),
                "format": ss58_format,
            }
        )
        cases.append(
            {
                "label": f"generated-{index}-wrong-expected-format",
                "address": address,
                "format": different_valid_format(ss58_format),
            }
        )
        cases.append(
            {
                "label": f"generated-{index}-trailing-space",
                "address": address + " ",
                "format": ss58_format,
            }
        )
        cases.append(
            {
                "label": f"generated-{index}-truncated",
                "address": address[:-1],
                "format": ss58_format,
            }
        )
        # Whitespace on the left and in the interior, not only trailing: the historically
        # vendored Python base58 codec right-stripped whitespace, so the three positions
        # are three different regressions.
        cases.append(
            {
                "label": f"generated-{index}-leading-space",
                "address": " " + address,
                "format": ss58_format,
            }
        )
        cases.append(
            {
                "label": f"generated-{index}-interior-space",
                "address": address[: len(address) // 2] + " " + address[len(address) // 2 :],
                "format": ss58_format,
            }
        )
        # Swap the case of one letter. Sometimes that is another valid base58 character
        # (a checksum or prefix failure), sometimes one of the excluded homoglyphs
        # (o -> O, i -> I: InvalidBase58); the classification must agree either way.
        alpha_positions = [position for position, char in enumerate(address) if char.isalpha()]
        position = alpha_positions[index % len(alpha_positions)]
        cases.append(
            {
                "label": f"generated-{index}-case-swap-{position}",
                "address": address[:position] + address[position].swapcase() + address[position + 1 :],
                "format": ss58_format,
            }
        )
        # A multi-byte character spliced into an otherwise valid address: out of the
        # alphabet, and — for addresses near the bound — pushing the UTF-8 byte length
        # past 50 while the code-point/UTF-16 fast paths still pass.
        for char in ("é", "\U0001f680"):
            cases.append(
                {
                    "label": f"generated-{index}-unicode-splice-{ord(char):04x}",
                    "address": address[: len(address) // 2] + char + address[len(address) // 2 :],
                    "format": ss58_format,
                }
            )

    # Wholly generated garbage input, spanning valid-alphabet runs, homoglyphs,
    # punctuation, whitespace, and multi-byte characters at lengths across the 50-byte
    # bound. No structure is intended; the point is that the rejection classification
    # agrees on arbitrary strings.
    for index in range(garbage_cases):
        length = rng.randint(0, 60)
        cases.append(
            {
                "label": f"garbage-{index}",
                "address": "".join(rng.choice(DECODE_GARBAGE_CHARS) for _ in range(length)),
                "format": rng.choice(FORMAT_BOUNDARIES),
            }
        )
    return cases


def normalized_info(info: Any, ss58_format: int) -> JsonObject:  # noqa: ANN401 - one of several decoded-info variants, probed with getattr
    return {
        "status": "ok",
        "format": ss58_format,
        "account_id": info.account_id,
        "account_type": AccountType(info.account_type).name,
        "app_agent_id": getattr(info, "app_agent_id", None),
        "ta_id": getattr(info, "ta_id", None),
        "account_name": getattr(info, "account_name", None),
    }


def normalized_error(error: Ss58Error) -> JsonObject:
    result: JsonObject = {"status": "error", "kind": error.kind}
    # The "UnexpectedFormat" kind is exactly the Ss58FormatMismatchError subclass, which is
    # the only one carrying the (expected, found) pair.
    if isinstance(error, Ss58FormatMismatchError):
        result.update(expected=error.expected, found=error.found)
    return result


def python_decode_results(cases: list[JsonObject]) -> list[JsonObject]:
    results = []
    for case in cases:
        try:
            expecting = normalized_info(decode_address(case["address"], ss58_format=case["format"]), case["format"])
        except Ss58Error as error:
            expecting = normalized_error(error)

        try:
            decoded = decode_address_with_format(case["address"])
            any_format = normalized_info(decoded.info, decoded.ss58_format)
        except Ss58Error as error:
            any_format = normalized_error(error)

        results.append({"label": case["label"], "expecting": expecting, "any_format": any_format})
    return results


def build_surrogate_cases() -> tuple[list[JsonObject], list[JsonObject]]:
    """Lone-surrogate host strings, compared Python <-> TypeScript directly.

    JSON *escape sequences* can carry lone surrogates: Python's ``json`` and JavaScript's
    ``JSON.parse`` both materialize ``"\\ud83d"`` as a host string containing the lone
    surrogate, but serde_json refuses it — so the Rust oracle can never see these cases.
    They travel in a separate file that only the TypeScript driver reads, and the
    comparison is two-sided (Python against TypeScript) rather than oracle-based. The
    contract under test is the JavaScript ``TextEncoder`` semantics both implement: a
    lone surrogate measures as the three-byte replacement character; a high/low pair
    (which the JSON decoders themselves combine into one astral scalar) as four bytes.
    """
    high = "\ud83d"
    low = "\ude00"
    name_cases: list[JsonObject] = [
        # 7 ASCII bytes + 3 (the replacement character) = 10: passes the byte-length
        # check and fails the charset -> InvalidCharacter.
        {"label": "lone-high-after-7-ascii", "name": "abcdefg" + high},
        {"label": "lone-low-before-7-ascii", "name": low + "abcdefg"},
        # 9 + 3 = 12 bytes -> InvalidLength.
        {"label": "lone-high-after-9-ascii", "name": "abcdefghi" + high},
        # A low before a high is *not* a pair: two lone surrogates (3 + 3) + 4 ASCII
        # = 10 bytes -> InvalidCharacter.
        {"label": "low-then-high-plus-4-ascii", "name": low + high + "abcd"},
        # The JSON decoders combine an escaped high/low pair into one astral scalar
        # (4 bytes): 6 + 4 = 10 -> InvalidCharacter; 7 + 4 = 11 -> InvalidLength.
        {"label": "escaped-pair-after-6-ascii", "name": "abcdef" + high + low},
        {"label": "escaped-pair-after-7-ascii", "name": "abcdefg" + high + low},
        {"label": "lone-high-alone", "name": high},
    ]
    for index, case in enumerate(name_cases):
        case["app_agent_id"] = 1000 + index

    decode_cases: list[JsonObject] = [
        # 3 bytes, within the length bound, out of the alphabet: InvalidBase58.
        {"label": "lone-high-alone", "address": high, "format": 5335},
        {"label": "lone-high-in-short-garbage", "address": "abc" + high + "def", "format": 5335},
        # 17 lone surrogates measure 51 UTF-8 bytes while both fast paths still pass
        # (17 code points in Python, 17 UTF-16 units in JavaScript): InvalidLength.
        {"label": "seventeen-lone-surrogates", "address": high * 17, "format": 5335},
        # 13 escaped pairs = 13 astral scalars = 52 bytes (26 UTF-16 units): InvalidLength.
        {"label": "thirteen-escaped-pairs", "address": (high + low) * 13, "format": 5335},
    ]
    return name_cases, decode_cases


def build_name_cases(seed: int, fuzz_cases: int) -> list[JsonObject]:
    """Account names, mostly invalid, spanning every UTF-8 width.

    The length rule is defined on the UTF-8 byte count, so the interesting cases are the
    ones where the byte count and the code-point count disagree: a name of 10 code points
    and 11 bytes is an InvalidLength, a name of 9 code points and 10 bytes reaches the
    charset check and is an InvalidCharacter. The kind is a cross-language contract
    (`named_invalid_encode` pins it per name), so it is compared, not just the rejection.

    JSON cannot carry a lone surrogate, so those stay pinned by each language's own suite.
    """
    rng = random.Random(seed ^ 0x4E414D45)  # noqa: S311 - reproducibility, not cryptography
    cases: list[JsonObject] = [
        {"label": "empty", "name": ""},
        {"label": "valid-ascii", "name": "hot-wallet"},
        {"label": "9-ascii", "name": "abcdefghi"},
        {"label": "11-ascii", "name": "abcdefghij0"},
        # 9 code points, 10 UTF-8 bytes: passes the byte-length check, fails the charset.
        {"label": "9cp-10b-accented", "name": "café12345"},
        # 10 code points, 11 UTF-8 bytes: fails the byte-length check.
        {"label": "10cp-11b-accented", "name": "héllo12345"},
        # 5 code points, 10 bytes.
        {"label": "5cp-10b-two-byte", "name": "é" * 5},
        # 10 code points, 30 bytes.
        {"label": "10cp-30b-three-byte", "name": "中" * 10},
        # 2 code points, 8 bytes; and 3 code points, 12 bytes.
        {"label": "2cp-8b-emoji", "name": "\U0001f680" * 2},
        {"label": "3cp-12b-emoji", "name": "\U0001f680" * 3},
        # A decomposed letter is two code points and three bytes.
        {"label": "combining-mark", "name": "abcdefgh" + "é"},
        {"label": "in-charset-boundary", "name": "##########"},
    ]
    for index in range(fuzz_cases):
        length = rng.randint(0, 14)
        name = "".join(rng.choice(NAME_FUZZ_CHARS) for _ in range(length))
        cases.append({"label": f"fuzz-{index}", "name": name})
    for case in cases:
        case["app_agent_id"] = rng.getrandbits(32)
    return cases


def python_name_results(cases: list[JsonObject]) -> list[JsonObject]:
    results = []
    for case in cases:
        try:
            account_id = encode_named_account_id(
                app_agent_id=AppAgentId(case["app_agent_id"]),
                account_name=AccountName(case["name"]),
            )
            result: JsonObject = {"status": "ok", "account_id": account_id}
        except AccountNameError as error:
            result = {"status": "error", "kind": error.kind}
        results.append({"label": case["label"], "result": result})
    return results


def build_format_cases(encode_results: list[JsonObject]) -> list[JsonObject]:
    """SS58 formats an encoder must reject, plus the valid boundaries it must accept.

    Only the *rejection* is compared: an unusable format is a rejected argument, and each
    language reports a bad argument its own way. What this pins is that the two ways a
    format can be unusable are not told apart — the regression being that Python and
    TypeScript once let the reserved formats fall through to the SS58 layer and raise their
    `Ss58Error`, the type that reports on an *address*. The runners below therefore catch
    only their own parameter-error type and let anything else escape, failing the check.
    """
    account_id = encode_results[0]["account_id"]
    cases = [{"label": f"rejected-{fmt}", "account_id": account_id, "format": fmt} for fmt in REJECTED_FORMATS]
    cases += [{"label": f"accepted-{fmt}", "account_id": account_id, "format": fmt} for fmt in FORMAT_BOUNDARIES]
    return cases


def python_format_results(cases: list[JsonObject]) -> list[JsonObject]:
    results = []
    for case in cases:
        try:
            address = encode_address(case["account_id"], ss58_format=case["format"])
            result: JsonObject = {"status": "ok", "address": address}
        except Ss58Error:
            # Deliberately not caught: an Ss58Error reports on an *address*, and the address
            # here is valid. Letting it escape fails the differential run loudly.
            raise
        except ValueError:
            result = {"status": "rejected"}
        results.append({"label": case["label"], "result": result})
    return results


def build_classify_cases(seed: int, encode_results: list[JsonObject], fuzz_cases: int) -> list[JsonObject]:
    """Raw 32-byte account ids fed to the classifier and to all three typed decoders.

    Covers what no other differential case does: arbitrary bytes (which must classify as
    Regular without panicking), keyless accounts whose checksum has been corrupted (which
    must *demote* to Regular), Named accounts carrying out-of-charset name bytes under a
    valid checksum (Regular again — anyone can craft one, no key is needed), and the typed
    decoders' (expected, found) mismatch pair, which docs/cross-language-contracts.md calls an explicit
    cross-language contract and which nothing else here exercises.
    """
    rng = random.Random(seed ^ 0xC1A551FD)  # noqa: S311 - reproducibility, not cryptography
    cases: list[JsonObject] = []

    # Every valid keyless account produced by the encode phase, so the typed decoders see
    # both their matching type and the two mismatching ones.
    for index, result in enumerate(encode_results[:24]):
        cases.append({"label": f"valid-{index}-{result['kind']}", "hex": result["account_id"]})
        # ...and the same account with one checksum bit flipped: must demote to Regular.
        raw = bytearray(bytes.fromhex(result["account_id"][2:]))
        raw[-1] ^= 0x01
        cases.append({"label": f"corrupted-{index}", "hex": "0x" + raw.hex()})
        # ...and one bit flipped at a seeded random position — open part, discriminant,
        # and checksum bytes alike. The fixture's classify group walks every byte of a
        # few fixed accounts; this varies the accounts too.
        random_raw = bytearray(bytes.fromhex(result["account_id"][2:]))
        position = rng.randrange(len(random_raw))
        random_raw[position] ^= 1 << rng.randrange(8)
        cases.append({"label": f"corrupted-{index}-byte-{position}", "hex": "0x" + random_raw.hex()})

    # A Named layout with a valid checksum over out-of-charset name bytes. No private key is
    # needed to build one, so every implementation must classify it as Regular rather than
    # trusting the checksum and returning a name it cannot represent.
    for index in range(4):
        open_part = bytearray(rng.getrandbits(32).to_bytes(4, "little"))
        open_part.append(3)  # NAMED_ACCOUNT_IDENTIFIER
        open_part += bytes(rng.randrange(0x80, 0x100) for _ in range(10))
        digest = hashlib.blake2b(bytes(open_part), digest_size=32).digest()
        cases.append({"label": f"named-out-of-charset-{index}", "hex": "0x" + (bytes(open_part) + digest[15:]).hex()})

    for index in range(fuzz_cases):
        raw = bytearray(rng.getrandbits(8) for _ in range(32))
        # Bias byte [4] towards the real discriminants, so the fuzzer spends its time in the
        # branches that actually do work rather than bailing out on an unknown type byte.
        raw[4] = rng.choice([0, 1, 2, 3, rng.getrandbits(8)])
        cases.append({"label": f"fuzz-{index}", "hex": "0x" + raw.hex()})
    return cases


def _typed_result(decode: Any, account_id: str) -> JsonObject:  # noqa: ANN401 - one of three typed decoders
    try:
        decode(account_id)
    except AccountTypeMismatchError as error:
        return {
            "status": "mismatch",
            "expected": AccountType(error.expected).name,
            "found": AccountType(error.found).name,
        }
    return {"status": "ok"}


def python_classify_results(cases: list[JsonObject]) -> list[JsonObject]:
    results = []
    for case in cases:
        info = decode_account_id(case["hex"])
        results.append(
            {
                "label": case["label"],
                "info": {
                    "account_id": info.account_id,
                    "account_type": AccountType(info.account_type).name,
                    "app_agent_id": getattr(info, "app_agent_id", None),
                    "ta_id": getattr(info, "ta_id", None),
                    "account_name": getattr(info, "account_name", None),
                },
                "typed": {
                    "app_agent": _typed_result(decode_app_agent_account_id, case["hex"]),
                    "transactional": _typed_result(decode_transactional_account_id, case["hex"]),
                    "named": _typed_result(decode_named_account_id, case["hex"]),
                },
            }
        )
    return results


def build_hex_cases(seed: int, encode_results: list[JsonObject], fuzz_cases: int) -> list[JsonObject]:
    """Raw account-id strings, valid and malformed, fed to the hex entry point.

    The "0x" + 64 hex contract is shared verbatim across the three; only the fixed
    `hex_invalid` vectors pinned it, so nothing exercised it on generated input.
    """
    rng = random.Random(seed ^ 0x8E00BAD)  # noqa: S311 - reproducibility, not cryptography
    good = encode_results[0]["account_id"]
    body = good[2:]
    cases: list[JsonObject] = [
        {"label": "canonical", "input": good},
        {"label": "uppercase-body", "input": "0x" + body.upper()},
        {"label": "mixed-case-body", "input": "0x" + body[:32].upper() + body[32:]},
        {"label": "uppercase-prefix", "input": "0X" + body},
        {"label": "no-prefix", "input": body},
        {"label": "short-63", "input": "0x" + body[:63]},
        {"label": "long-65", "input": "0x" + body + "a"},
        {"label": "trailing-space", "input": "0x" + body + " "},
        {"label": "leading-space", "input": " " + good},
        {"label": "empty", "input": ""},
        {"label": "just-prefix", "input": "0x"},
        {"label": "non-hex", "input": "0x" + body[:-1] + "g"},
        # Non-ASCII: the byte-vs-character question again, now in the hex parser.
        {"label": "non-ascii-digit", "input": "0x" + body[:-1] + "٣"},
        {"label": "an-address-not-an-account-id", "input": encode_results[0]["address"]},
    ]
    for index in range(fuzz_cases):
        length = rng.randint(60, 68)
        alphabet = "0123456789abcdefABCDEF" + "ghxyz_ "
        cases.append(
            {
                "label": f"fuzz-{index}",
                "input": "0x" + "".join(rng.choice(alphabet) for _ in range(length)),
            }
        )
    return cases


def python_hex_results(cases: list[JsonObject]) -> list[JsonObject]:
    results = []
    for case in cases:
        try:
            info = decode_account_id(case["input"])
            result: JsonObject = {
                "status": "ok",
                "account_id": info.account_id,
                "account_type": AccountType(info.account_type).name,
            }
        except ParseAccountIdError:
            result = {"status": "error"}
        results.append({"label": case["label"], "result": result})
    return results


def verify_pinned_errors(cases: list[JsonObject], results: list[JsonObject]) -> None:
    """Check the oracle itself against the error kinds pinned in the conformance fixture.

    The oracle is only a useful reference if it still agrees with the committed
    specification, so this runs against the Rust results before the other
    implementations are compared to them.
    """
    for case, result in zip(cases, results, strict=True):
        expected = case.get("expected_error")
        if expected is not None and result["expecting"] != expected:
            raise SystemExit(
                f"{ORACLE} disagrees with pinned hostile vector {case['label']}\n"
                f"expected: {json.dumps(expected, sort_keys=True)}\n"
                f"actual:   {json.dumps(result['expecting'], sort_keys=True)}"
            )


def command_results(command: list[str], *case_paths: Path) -> DifferentialResults:
    try:
        completed = subprocess.run(  # noqa: S603
            [*command, *map(str, case_paths)],
            cwd=REPOSITORY_ROOT,
            check=True,
            capture_output=True,
            text=True,
        )
    except subprocess.CalledProcessError as error:
        # Without this the harness that gates all three releases reports a Rust panic or a
        # TypeScript throw as nothing but "returned non-zero exit status 101": `check=True`
        # captures stderr into the exception, where it is never printed.
        print(
            f"{shlex.join(command)} failed with exit status {error.returncode}",
            file=sys.stderr,
        )
        for stream_name, stream in (("stdout", error.stdout), ("stderr", error.stderr)):
            if stream:
                print(f"--- {stream_name} ---\n{stream}", file=sys.stderr)
        raise SystemExit(1) from error

    try:
        results: DifferentialResults = json.loads(completed.stdout)
    except json.JSONDecodeError as error:
        # A harness that exits 0 but prints non-JSON is a bug worth naming, not a traceback.
        print(
            f"{shlex.join(command)} produced output that is not valid JSON: {error}",
            file=sys.stderr,
        )
        print(f"--- stdout ---\n{completed.stdout}", file=sys.stderr)
        raise SystemExit(1) from error
    return results


DEFAULT_SECTIONS = ("encode", "decode", "name", "format", "classify", "hex")


def first_difference(
    reference: DifferentialResults,
    actual: DifferentialResults,
    sections: tuple[str, ...] = DEFAULT_SECTIONS,
    reference_label: str | None = None,
) -> str:
    label = reference_label if reference_label is not None else f"{ORACLE} (source of truth)"
    for section in sections:
        left_results = reference.get(section, [])
        right_results = actual.get(section, [])
        if len(left_results) != len(right_results):
            return f"{section} result count {len(right_results)} != {len(left_results)}"
        for index, (left, right) in enumerate(zip(left_results, right_results, strict=True)):
            if left != right:
                return (
                    f"{section} case {index}\n{label}: {json.dumps(left, sort_keys=True)}\n"
                    f"actual: {json.dumps(right, sort_keys=True)}"
                )
    return "unknown difference"


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--seed", type=lambda value: int(value, 0), default=0x54524556)
    parser.add_argument("--cases", type=int, default=256)
    args = parser.parse_args()
    if args.cases < len(FORMAT_BOUNDARIES):
        parser.error("--cases must be at least 6 to cover all format boundaries")
    # Printed before anything runs, so every outcome — a divergence, an oracle
    # disagreement, a crashed harness — leaves the seed in the log and the run stays
    # reproducible locally with --seed. The success line at the end repeats it.
    print(f"cross-language differential check: seed={args.seed:#x}, encode_cases={args.cases}", flush=True)

    encode_cases = build_cases(args.seed, args.cases)
    encode_results = python_encode_results(encode_cases)
    decode_cases = build_decode_cases(
        args.seed,
        encode_cases,
        encode_results,
        hostile_source_case_count(args.cases),
        decode_garbage_case_count(args.cases),
    )
    name_cases = build_name_cases(args.seed, name_fuzz_case_count(args.cases))
    format_cases = build_format_cases(encode_results)
    classify_cases = build_classify_cases(args.seed, encode_results, classify_fuzz_case_count(args.cases))
    hex_cases = build_hex_cases(args.seed, encode_results, hex_fuzz_case_count(args.cases))
    surrogate_name_cases, surrogate_decode_cases = build_surrogate_cases()
    payload = {
        "encode_cases": encode_cases,
        "decode_cases": decode_cases,
        "name_cases": name_cases,
        "format_cases": format_cases,
        "classify_cases": classify_cases,
        "hex_cases": hex_cases,
    }

    with tempfile.TemporaryDirectory(prefix="trevokeyless-differential-") as directory:
        case_path = Path(directory) / "cases.json"
        case_path.write_text(json.dumps(payload), encoding="utf8")
        # The lone-surrogate cases travel in their own file: serde_json rejects
        # lone-surrogate escapes, so a single cases.json carrying them would crash the
        # Rust oracle on parse. Only the TypeScript driver reads this second file.
        surrogate_path = Path(directory) / "surrogate_cases.json"
        surrogate_path.write_text(
            json.dumps({"name_cases": surrogate_name_cases, "decode_cases": surrogate_decode_cases}),
            encoding="utf8",
        )
        # The Rust crate is the source of truth for the encoding, so it is the oracle:
        # Python (whose own uncaught exceptions — a leaked ValueError, say — still fail
        # this script) and TypeScript are each compared against it, never against each
        # other. Comparing the oracle to itself would prove nothing.
        oracle = command_results(
            [
                "cargo",
                "run",
                "--locked",
                "--quiet",
                "--manifest-path",
                "keyless-rust/Cargo.toml",
                "--example",
                "differential",
                "--features",
                "ss58,serde",
                "--",
            ],
            case_path,
        )
        typescript_results = command_results(["node", "keyless-ts/scripts/differential.mjs"], case_path, surrogate_path)
        surrogate_sections = ("surrogate_name", "surrogate_decode")
        typescript_surrogate = {section: typescript_results.pop(section) for section in surrogate_sections}
        python_surrogate = {
            "surrogate_name": python_name_results(surrogate_name_cases),
            "surrogate_decode": python_decode_results(surrogate_decode_cases),
        }
        implementations: dict[str, DifferentialResults] = {
            "Python": {
                "encode": encode_results,
                "decode": python_decode_results(decode_cases),
                "name": python_name_results(name_cases),
                "format": python_format_results(format_cases),
                "classify": python_classify_results(classify_cases),
                "hex": python_hex_results(hex_cases),
            },
            "TypeScript": typescript_results,
        }

    verify_pinned_errors(decode_cases, oracle["decode"])
    for language, actual in implementations.items():
        if actual != oracle:
            message = f"{language} differs from {ORACLE} (source of truth): {first_difference(oracle, actual)}"
            raise SystemExit(message)
    # The lone-surrogate parity has no oracle (the Rust implementation cannot represent
    # the inputs), so the two host languages must simply agree with each other — the
    # TextEncoder-semantics contract otherwise held only by the twin adversarial suites.
    if typescript_surrogate != python_surrogate:
        difference = first_difference(
            python_surrogate,
            typescript_surrogate,
            sections=surrogate_sections,
            reference_label="Python",
        )
        raise SystemExit(f"TypeScript differs from Python on lone-surrogate host strings: {difference}")
    print(
        f"cross-language differential check passed against the {ORACLE} source of truth: "
        f"implementations={'+'.join(implementations)}, seed={args.seed:#x}, "
        f"encode_cases={args.cases}, decode_cases={len(decode_cases)}, "
        f"name_cases={len(name_cases)}, format_cases={len(format_cases)}, "
        f"classify_cases={len(classify_cases)}, hex_cases={len(hex_cases)}, "
        f"surrogate_cases={len(surrogate_name_cases) + len(surrogate_decode_cases)} (Python<->TypeScript)"
    )


if __name__ == "__main__":
    main()
