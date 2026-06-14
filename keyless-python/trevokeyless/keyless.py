"""keyless.py

This module provides functionality for encoding and decoding keyless addresses.

A keyless address is a type of address that doesn't rely on cryptographic key pairs for identification.
Instead, it is derived from a combination of identifiers and checksums, making it suitable for various
use cases such as tracking and verification.

Examples:
    # Encode an AppAgent keyless address
    app_agent_id = 123
    app_agent_address = encode_app_agent_address(app_agent_id)

    # Decode the keyless address
    decoded_app_agent_id = decode_app_agent_address(app_agent_address)
    assert decoded_app_agent_id == app_agent_id
"""

import re
from dataclasses import dataclass
from enum import Enum
from hashlib import blake2b
from typing import NamedTuple, TypeAlias

from ._ss58 import SS58_FORMAT_MAX as _SS58_FORMAT_MAX
from ._ss58 import Ss58Error, ss58_decode, ss58_encode

# Constants
NAMED_ADDRESS_LENGTH = 10
SS58_FORMAT_TREVO_ASSET_HUB = 5335

# Largest value of a u32 identifier (AppAgent ID, TA ID). Mirrors the `u32` types of the
# Rust implementation and the `U32_MAX` bound of the TypeScript implementation.
U32_MAX = 4294967295

# Length in bytes of a blockchain account id.
ACCOUNT_ID_LENGTH = 32

# A raw account id accepted by decode_account_id: "0x" followed by exactly 64 hex
# characters. bytes.fromhex is lenient about ASCII whitespace, so the shape must be
# checked explicitly to enforce the documented contract (the TypeScript implementation
# uses the same expression; the Rust implementation has no string form at all -
# AccountId32 is constructed from bytes).
_HEX_ACCOUNT_ID = re.compile(r"0x[0-9a-fA-F]{64}")


# Types of address identifiers
BlockchainAddress: TypeAlias = str
BlockchainAccountId: TypeAlias = str
AppAgentId: TypeAlias = int
TransactionalId: TypeAlias = int
AccountName: TypeAlias = str
SS58Format: TypeAlias = int


class AccountType(Enum):
    Regular = 0
    AppAgent = 1
    Transactional = 2
    Named = 3


def _account_type_article(account_type: AccountType) -> str:
    """The indefinite article for an account type name ("an" before AppAgent)."""
    return "an" if account_type is AccountType.AppAgent else "a"


class AccountTypeMismatchError(ValueError):
    """Raised by the typed decode_* functions when the address is of a different type.

    Carries the expected and the actual (found) classification of the address, mirroring
    the ``KeylessDecodeError`` of the Rust implementation. Subclasses ``ValueError`` so
    ``except ValueError`` handlers treat it like any other decoding failure. The message
    is identical to the TypeScript implementation's.

    Attributes:
        expected (AccountType): The address type required by the caller.
        found (AccountType): The actual classification of the address.
    """

    def __init__(self, expected: AccountType, found: AccountType) -> None:
        self.expected = expected
        self.found = found
        super().__init__(
            f"Provided address is not {_account_type_article(expected)} {expected.name} "
            f"address but is {_account_type_article(found)} {found.name} address"
        )


class Ss58FormatMismatchError(Ss58Error):
    """Raised when an address is a valid SS58 string but uses a different format than expected.

    The ``"UnexpectedFormat"`` case of :class:`Ss58Error`, mirroring the
    ``Ss58Error::UnexpectedFormat`` variant of the Rust implementation (the TypeScript
    implementation reports the same pair). Raised only after the address has fully
    validated — structure, length, and checksum — exactly like the Rust implementation,
    which compares the format last; an address that fails validation raises the
    corresponding :class:`Ss58Error` instead.

    Attributes:
        expected (SS58Format): The SS58 format required by the caller.
        found (SS58Format): The format embedded in the address.
    """

    def __init__(self, expected: SS58Format, found: SS58Format) -> None:
        self.expected = expected
        self.found = found
        super().__init__("UnexpectedFormat", f"Expected SS58 format {expected} but found {found}")


@dataclass(frozen=True)
class BlockchainAccountInfo:
    """Decoded identity of a blockchain address.

    Immutable (and therefore hashable), mirroring the value semantics of the Rust
    ``BlockchainAccountInfo`` (``Copy + Eq + Hash``).

    The ``address`` field is the SS58 address the info was decoded from, or ``None``
    when the input was a raw account id (``decode_account_id``): raw account bytes
    carry no SS58 format, so there is no address to report — render one explicitly
    with ``encode_address``. ``account_id`` is always the normalized ``0x`` +
    lowercase hex form.

    ``address`` is the input spelling verbatim, never re-encoded: for the one accepted
    non-canonical spelling (the two-byte-prefix encoding of SS58 formats below 64, see
    the security model) it differs from the canonical address of the same account. Key
    caches and lookups on ``account_id`` — the canonical cross-language form — and
    render the canonical address with ``encode_address``.
    """

    address: BlockchainAddress | None
    account_id: BlockchainAccountId
    account_type: AccountType
    app_agent_id: AppAgentId | None
    ta_id: TransactionalId | None
    account_name: AccountName | None


def _blake2_256(data: bytes) -> bytes:
    """
    Calculates the 32-byte Blake2b hash of the provided data.

    Args:
        data (bytes): The data to hash.

    Returns:
        bytes: The 32-byte Blake2b digest.
    """
    return blake2b(data, digest_size=32).digest()


def _require_str(value: object, label: str) -> None:
    """
    Validates that a string parameter actually is a str.

    Public entry points raise TypeError - rather than leaking an AttributeError from a
    string method lookup - when a string parameter has the wrong type. (The TypeScript
    implementation surfaces a native TypeError the same way; the Rust implementation
    enforces the types at compile time.) mypy enforces the type for annotated callers;
    the guard serves untyped ones.

    Args:
        value (object): The value to validate.
        label (str): Name of the parameter, used in the error message.

    Raises:
        TypeError: If the value is not a str.
    """
    if not isinstance(value, str):
        msg = f"{label} must be a str, got {type(value).__name__}"
        raise TypeError(msg)


def _validate_u32(value: int, label: str) -> None:
    """
    Validates that a value is an integer in the u32 range [0, 4294967295].

    The Rust implementation enforces this through the `u32` type; the TypeScript
    implementation performs the equivalent runtime check (RangeError, its closest
    native fit for both failure modes). Here the failure modes split along Python
    convention: a non-int raises TypeError, an out-of-range int raises ValueError.
    ``bool`` is rejected as a wrong type: it is an ``int`` subclass, but silently
    encoding ``True`` as 1 would hide a caller bug that the other implementations
    surface.

    Args:
        value (int): The value to validate.
        label (str): Name of the value, used in the error message.

    Raises:
        TypeError: If the value is not an int (``bool`` included).
        ValueError: If the value is outside [0, 4294967295].
    """
    if isinstance(value, bool) or not isinstance(value, int):
        msg = f"{label} must be an int, got {type(value).__name__}"
        raise TypeError(msg)
    if value < 0 or value > U32_MAX:
        msg = f"{label} must be an integer in [0, {U32_MAX}], got {value!r}"
        raise ValueError(msg)


def _validate_ss58_format(ss58_format: int) -> None:
    """
    Validates that an SS58 format is an integer in [0, 16383] (the prefix encodes
    14 bits).

    The Rust implementation enforces this through the `u16` type plus a range check; the
    TypeScript implementation performs the equivalent runtime check. The failure modes
    split along Python convention: a non-int (``bool`` included, for the same reason as
    in `_validate_u32`) raises a deliberate TypeError - not one leaked from the prefix
    byte arithmetic - and an out-of-range int raises ValueError. The reserved formats
    46 and 47 are rejected separately by the SS58 layer.

    Args:
        ss58_format (int): The SS58 format to validate.

    Raises:
        TypeError: If the format is not an int (``bool`` included).
        ValueError: If the format is outside [0, 16383].
    """
    if isinstance(ss58_format, bool) or not isinstance(ss58_format, int):
        msg = f"ss58_format must be an int, got {type(ss58_format).__name__}"
        raise TypeError(msg)
    if not 0 <= ss58_format <= _SS58_FORMAT_MAX:
        msg = f"ss58_format must be an integer in [0, {_SS58_FORMAT_MAX}], got {ss58_format!r}"
        raise ValueError(msg)


__allowed_chars = set("0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ-#")


def _validate_account_name(name: str) -> None:
    """
    Validates that a name is exactly 10 characters long and contains only allowed ASCII
    characters (`0-9 a-z A-Z - #`).

    Args:
        name (str): The name to validate.

    Raises:
        TypeError: If the name is not a str.
        ValueError: If the name has a wrong length or contains invalid characters.
    """
    _require_str(name, "name")
    if len(name) != NAMED_ADDRESS_LENGTH:
        msg = "Address name must be exactly 10 characters long"
        raise ValueError(msg)

    if not all(c in __allowed_chars for c in name):
        msg = "Address name contains invalid characters"
        raise ValueError(msg)


def _decode_account_name(name_bytes: bytes) -> AccountName | None:
    """
    Decodes the raw name bytes of a Named address, validating the character set.

    Anyone can construct an account id that carries a valid checksum over arbitrary name
    bytes (no private key is needed), so the name bytes must be validated during decoding.

    Args:
        name_bytes (bytes): The raw 10 name bytes of the Named address.

    Returns:
        str | None: The decoded name, or None if the bytes are not a valid address name.
    """
    if len(name_bytes) != NAMED_ADDRESS_LENGTH:
        return None
    if not all(chr(b) in __allowed_chars for b in name_bytes):
        return None
    return name_bytes.decode("ascii")


def _encode_address(open_part: bytes, ss58_format: SS58Format) -> BlockchainAddress:
    """
    Builds a keyless account id from its open part and wraps it in an SS58 address.

    The 32-byte account id is `open_part + blake2_256(open_part)[len(open_part):]` —
    the trailing bytes of the hash serve as the checksum.

    Args:
        open_part (bytes): The open (plain-text) part of the account id.
        ss58_format (int): SS58 format (network prefix) to use for the address.

    Returns:
        str: Encoded SS58 address.

    Raises:
        TypeError: If ss58_format is not an int.
        ValueError: If ss58_format is outside [0, 16383], or is reserved.
    """
    _validate_ss58_format(ss58_format)

    # Calculate checksum using blake2_256
    checksum = _blake2_256(open_part)

    # Construct address_encoded
    address_encoded = open_part + checksum[len(open_part) :]

    return ss58_encode(address_encoded, ss58_format)


def encode_app_agent_address(
    app_agent_id: AppAgentId, ss58_format: SS58Format = SS58_FORMAT_TREVO_ASSET_HUB
) -> BlockchainAddress:
    """
    Encode an AppAgent address.

    Args:
        app_agent_id (int): AppAgent ID.

    Returns:
        str: Encoded AppAgent address.

    Raises:
        TypeError: If app_agent_id or ss58_format is not an int.
        ValueError: If app_agent_id is outside [0, 4294967295], or ss58_format is
            not a valid SS58 format.
    """
    _validate_u32(app_agent_id, "app_agent_id")

    # Convert app_agent_id to little-endian bytes
    app_agent_id_bytes = app_agent_id.to_bytes(4, byteorder="little")

    # Construct open_part
    open_part = app_agent_id_bytes + bytes([AccountType.AppAgent.value])

    return _encode_address(open_part, ss58_format)


def decode_app_agent_address(
    encoded_address: BlockchainAddress, ss58_format: SS58Format = SS58_FORMAT_TREVO_ASSET_HUB
) -> AppAgentId:
    """
    Decode an encoded AppAgent address.

    Args:
        encoded_address (str): Encoded AppAgent address.

    Returns:
        int: Decoded AppAgent ID.

    Raises:
        AccountTypeMismatchError: If the address is not an AppAgent address.
        ValueError: If the address cannot be decoded at all.
    """
    decoding_result = decode_address(encoded_address, ss58_format)

    if decoding_result.account_type is not AccountType.AppAgent:
        raise AccountTypeMismatchError(AccountType.AppAgent, decoding_result.account_type)
    if decoding_result.app_agent_id is None:
        msg = "Internal error in trevokeyless - app_agent_id of AppAgent address is None"
        raise ValueError(msg)

    return decoding_result.app_agent_id


def encode_transactional_address(
    app_agent_id: AppAgentId,
    ta_id: TransactionalId,
    ss58_format: SS58Format = SS58_FORMAT_TREVO_ASSET_HUB,
) -> BlockchainAddress:
    """
    Encode a Transactional address.

    Args:
        app_agent_id (int): AppAgent ID.
        ta_id (int): Transactional address ID.

    Returns:
        str: Encoded Transactional address.

    Raises:
        TypeError: If app_agent_id, ta_id, or ss58_format is not an int.
        ValueError: If app_agent_id or ta_id is outside [0, 4294967295], or
            ss58_format is not a valid SS58 format.
    """
    _validate_u32(app_agent_id, "app_agent_id")
    _validate_u32(ta_id, "ta_id")

    # Convert app_agent_id and ta_id to little-endian bytes
    app_agent_id_bytes = app_agent_id.to_bytes(4, byteorder="little")
    ta_id_bytes = ta_id.to_bytes(4, byteorder="little")

    # Construct open_part
    open_part = app_agent_id_bytes + bytes([AccountType.Transactional.value]) + ta_id_bytes

    return _encode_address(open_part, ss58_format)


class DecodedTransactionalAddress(NamedTuple):
    """The identifiers embedded in a Transactional address, returned by
    :func:`decode_transactional_address`.

    A :class:`typing.NamedTuple` rather than a bare ``(app_agent_id, ta_id)`` tuple, so
    the two integer fields can be accessed by name and not silently transposed; it stays
    backward compatible with tuple unpacking and indexing. Mirrors the object the
    TypeScript ``decodeTransactionalAddress`` returns and the Rust
    ``DecodedTransactionalAccount`` struct.
    """

    app_agent_id: AppAgentId
    ta_id: TransactionalId


def decode_transactional_address(
    encoded_address: BlockchainAddress, ss58_format: SS58Format = SS58_FORMAT_TREVO_ASSET_HUB
) -> DecodedTransactionalAddress:
    """
    Decode an encoded Transactional address.

    Args:
        encoded_address (str): Encoded Transactional address.

    Returns:
        DecodedTransactionalAddress: The decoded AppAgent ID and Transactional address ID
            (a named tuple; unpacks as ``app_agent_id, ta_id``).

    Raises:
        AccountTypeMismatchError: If the address is not a Transactional address.
        ValueError: If the address cannot be decoded at all.
    """
    decoding_result = decode_address(encoded_address, ss58_format)

    if decoding_result.account_type is not AccountType.Transactional:
        raise AccountTypeMismatchError(AccountType.Transactional, decoding_result.account_type)
    if decoding_result.app_agent_id is None:
        msg = "Internal error in trevokeyless - app_agent_id of Transactional address is None"
        raise ValueError(msg)
    if decoding_result.ta_id is None:
        msg = "Internal error in trevokeyless - ta_id of Transactional address is None"
        raise ValueError(msg)

    return DecodedTransactionalAddress(decoding_result.app_agent_id, decoding_result.ta_id)


def encode_named_address(
    app_agent_id: AppAgentId, name: AccountName, ss58_format: SS58Format = SS58_FORMAT_TREVO_ASSET_HUB
) -> BlockchainAddress:
    """
    Encode a Named address.

    Args:
        app_agent_id (int): AppAgent ID.
        name (str): Address name.

    Returns:
        str: Encoded Named address.

    Raises:
        TypeError: If app_agent_id or ss58_format is not an int, or name is not a str.
        ValueError: If app_agent_id is outside [0, 4294967295], the name has a wrong
            length or contains invalid characters, or ss58_format is not a valid
            SS58 format.
    """
    _validate_u32(app_agent_id, "app_agent_id")
    _validate_account_name(name)

    # Convert app_agent_id to little-endian bytes
    app_agent_id_bytes = app_agent_id.to_bytes(4, byteorder="little")

    # Construct open_part
    open_part = app_agent_id_bytes + bytes([AccountType.Named.value]) + name.encode()

    return _encode_address(open_part, ss58_format)


class DecodedNamedAddress(NamedTuple):
    """The identifiers embedded in a Named address, returned by
    :func:`decode_named_address`.

    A :class:`typing.NamedTuple` rather than a bare ``(app_agent_id, account_name)``
    tuple; stays backward compatible with tuple unpacking and indexing. Mirrors the
    object the TypeScript ``decodeNamedAddress`` returns and the Rust
    ``DecodedNamedAccount`` struct.
    """

    app_agent_id: AppAgentId
    account_name: AccountName


def decode_named_address(
    encoded_address: BlockchainAddress, ss58_format: SS58Format = SS58_FORMAT_TREVO_ASSET_HUB
) -> DecodedNamedAddress:
    """
    Decode an encoded Named address.

    Args:
        encoded_address (str): Encoded Named address.

    Returns:
        DecodedNamedAddress: The decoded AppAgent ID and address name (a named tuple;
            unpacks as ``app_agent_id, account_name``).

    Raises:
        AccountTypeMismatchError: If the address is not a Named address.
        ValueError: If the address cannot be decoded at all.
    """
    decoding_result = decode_address(encoded_address, ss58_format)

    if decoding_result.account_type is not AccountType.Named:
        raise AccountTypeMismatchError(AccountType.Named, decoding_result.account_type)
    if decoding_result.app_agent_id is None:
        msg = "Internal error in trevokeyless - app_agent_id of Named address is None"
        raise ValueError(msg)
    if decoding_result.account_name is None:
        msg = "Internal error in trevokeyless - name of Named address is None"
        raise ValueError(msg)

    return DecodedNamedAddress(decoding_result.app_agent_id, decoding_result.account_name)


def _decode_ss58_account_id_bytes(blockchain_address: BlockchainAddress, ss58_format: SS58Format) -> bytes:
    """
    Resolves an SS58 address string to its raw 32-byte account id.

    Args:
        blockchain_address (str): SS58 address of a 32-byte account id.
        ss58_format (int): SS58 format (network prefix) the address must use.

    Returns:
        bytes: The 32-byte account id.

    Raises:
        TypeError: If blockchain_address is not a str, or ss58_format is not an int.
        Ss58Error: If the input is not a valid SS58 address of a 32-byte account
            id (``Ss58FormatMismatchError`` if it is valid but uses a different format
            than expected).
        ValueError: If ss58_format is outside [0, 16383].
    """
    # Validate the ss58_format parameter before the address, matching the Rust
    # from_ss58check_expecting (which checks the expected format up front) and the
    # TypeScript decodeSs58AccountId: when both the address and the format are invalid,
    # every implementation reports the format error.
    _validate_ss58_format(ss58_format)
    _require_str(blockchain_address, "blockchain_address")

    if blockchain_address.startswith("0x"):
        # Raw account ids are deliberately not accepted here: they carry no SS58 format
        # and would silently bypass the expected-format check. decode_account_id is the
        # entry point for raw account ids.
        msg = "Raw account id hex is not an SS58 address; use decode_account_id"
        raise ValueError(msg)

    account_id_bytes, found_format = ss58_decode(blockchain_address)
    # The expected format is compared only after the address has fully validated
    # (structure, length, checksum), exactly like the Rust from_ss58check: an invalid
    # address must raise its validation error, not a misleading format mismatch.
    if found_format != ss58_format:
        raise Ss58FormatMismatchError(ss58_format, found_format)
    return account_id_bytes


def decode_address(
    blockchain_address: BlockchainAddress, ss58_format: SS58Format = SS58_FORMAT_TREVO_ASSET_HUB
) -> BlockchainAccountInfo:
    """
    Decode an SS58 blockchain address and classify it.

    Accepts an SS58 address string of a 32-byte account id in the expected format. Raw
    account ids in hex form are rejected here because they carry no SS58 format; use
    `decode_account_id` for those.

    Args:
        blockchain_address (str): SS58 address.
        ss58_format (int): SS58 format (network prefix) the address must use.

    Returns:
        BlockchainAccountInfo: an object with info about the address.

    Raises:
        TypeError: If blockchain_address is not a str, or ss58_format is not an int.
        Ss58Error: If the address is not a valid SS58 string of a 32-byte account
            id; its ``kind`` carries the failure classification
            (``Ss58FormatMismatchError`` if the address is valid but uses a different
            format than expected).
        ValueError: If ss58_format is outside [0, 16383].
    """
    account_id_bytes = _decode_ss58_account_id_bytes(blockchain_address, ss58_format)
    return _classify_account_id(blockchain_address, account_id_bytes)


def decode_account_id(account_id: BlockchainAccountId) -> BlockchainAccountInfo:
    """
    Decode a raw 32-byte account id and classify it.

    Mirrors `decode_account_info` of the Rust implementation, which takes the raw
    `AccountId32` bytes. Raw account bytes carry no SS58 format, so unlike
    `decode_address` this function has no `ss58_format` parameter.

    Args:
        account_id (str): The account id: `0x` followed by exactly 64 hex characters.

    Returns:
        BlockchainAccountInfo: an object with info about the account id. The `address`
            field is `None` (raw account bytes carry no address; render one with
            `encode_address`).

    Raises:
        TypeError: If account_id is not a str.
        ValueError: If the input is not `0x` followed by exactly 64 hex characters.
    """
    _require_str(account_id, "account_id")
    if _HEX_ACCOUNT_ID.fullmatch(account_id) is None:
        msg = "Invalid hex account id: expected 0x followed by 64 hex characters"
        raise ValueError(msg)
    return _classify_account_id(None, bytes.fromhex(account_id[2:]))


def encode_address(
    account_id: BlockchainAccountId, ss58_format: SS58Format = SS58_FORMAT_TREVO_ASSET_HUB
) -> BlockchainAddress:
    """
    Encode a raw 32-byte account id as an SS58 address.

    The counterpart of `decode_address`: any account id obtained from
    `BlockchainAccountInfo.account_id` (or from chain state) renders as its canonical
    address string. Mirrors `AccountId32::to_ss58check_with_version` of the Rust
    implementation. The input contract is `decode_account_id`'s: `0x` followed by
    exactly 64 hex characters (both digit cases accepted).

    Args:
        account_id (str): The account id: `0x` followed by exactly 64 hex characters.
        ss58_format (int): SS58 format (network prefix) to use for the address.

    Returns:
        str: The SS58 address.

    Raises:
        TypeError: If account_id is not a str, or ss58_format is not an int.
        ValueError: If the input is not `0x` followed by exactly 64 hex characters, or
            ss58_format is outside [0, 16383].
        Ss58Error: kind ``"InvalidFormat"`` if ss58_format is reserved (46, 47).
    """
    _require_str(account_id, "account_id")
    if _HEX_ACCOUNT_ID.fullmatch(account_id) is None:
        msg = "Invalid hex account id: expected 0x followed by 64 hex characters"
        raise ValueError(msg)
    _validate_ss58_format(ss58_format)
    return ss58_encode(bytes.fromhex(account_id[2:]), ss58_format)


def _classify_account_id(
    blockchain_address: BlockchainAddress | None, account_id_bytes: bytes
) -> BlockchainAccountInfo:
    """
    Classifies a 32-byte account id and extracts any embedded keyless identifiers.

    Args:
        blockchain_address (str | None): The SS58 address the account id was decoded
            from, stored in the `address` field of the result; None when the input was
            a raw account id.
        account_id_bytes (bytes): The 32-byte account id.

    Returns:
        BlockchainAccountInfo: an object with info about the account id.
    """
    account_id = "0x" + account_id_bytes.hex()

    # Read the byte that encodes type of the address
    account_type_byte = account_id_bytes[4]

    if account_type_byte == AccountType.AppAgent.value:
        # Verify checksum
        open_part_size = 5
        open_part = account_id_bytes[:open_part_size]
        checksum = account_id_bytes[open_part_size:]
        checksum_calculated = _blake2_256(open_part)[open_part_size:]
        if checksum == checksum_calculated:
            # Extract and decode app_agent_id
            app_agent_id_bytes = account_id_bytes[:4]
            app_agent_id = int.from_bytes(app_agent_id_bytes, byteorder="little")

            return BlockchainAccountInfo(
                address=blockchain_address,
                account_id=account_id,
                account_type=AccountType.AppAgent,
                app_agent_id=app_agent_id,
                ta_id=None,
                account_name=None,
            )

    if account_type_byte == AccountType.Transactional.value:
        # Verify checksum
        open_part_size = 9
        open_part = account_id_bytes[:open_part_size]
        checksum = account_id_bytes[open_part_size:]
        checksum_calculated = _blake2_256(open_part)[open_part_size:]
        if checksum == checksum_calculated:
            # Extract and decode app_agent_id and ta_id
            app_agent_id_bytes = account_id_bytes[:4]
            ta_id_bytes = account_id_bytes[5:9]
            app_agent_id = int.from_bytes(app_agent_id_bytes, byteorder="little")
            ta_id = int.from_bytes(ta_id_bytes, byteorder="little")

            return BlockchainAccountInfo(
                address=blockchain_address,
                account_id=account_id,
                account_type=AccountType.Transactional,
                app_agent_id=app_agent_id,
                ta_id=ta_id,
                account_name=None,
            )

    if account_type_byte == AccountType.Named.value:
        # Verify checksum
        open_part_size = 15
        open_part = account_id_bytes[:open_part_size]
        checksum = account_id_bytes[open_part_size:]
        checksum_calculated = _blake2_256(open_part)[open_part_size:]
        if checksum == checksum_calculated:
            # Extract and decode app_agent_id and account_name.
            # The name bytes must additionally pass charset validation: an address with a
            # valid checksum over invalid name bytes is classified as Regular, in line with
            # the Rust implementation.
            app_agent_id_bytes = account_id_bytes[:4]
            name_bytes = account_id_bytes[5:15]
            app_agent_id = int.from_bytes(app_agent_id_bytes, byteorder="little")
            account_name = _decode_account_name(name_bytes)

            if account_name is not None:
                return BlockchainAccountInfo(
                    address=blockchain_address,
                    account_id=account_id,
                    account_type=AccountType.Named,
                    app_agent_id=app_agent_id,
                    ta_id=None,
                    account_name=account_name,
                )

    return BlockchainAccountInfo(
        address=blockchain_address,
        account_id=account_id,
        account_type=AccountType.Regular,
        app_agent_id=None,
        ta_id=None,
        account_name=None,
    )


def is_keyless_address(
    blockchain_address: BlockchainAddress, ss58_format: SS58Format = SS58_FORMAT_TREVO_ASSET_HUB
) -> bool:
    """
    Returns True if the address passes the keyless checksum verification (AppAgent,
    Transactional, or Named), False for any other (Regular) address - including a Named
    layout whose checksum is valid but whose name bytes fail charset validation.

    Mirrors `is_keyless_account` of the Rust implementation. Accepts SS58 addresses
    only and raises (rather than returning False) when the input cannot be decoded at
    all; classify raw account ids with `decode_account_id` instead.

    Args:
        blockchain_address (str): SS58 address.
        ss58_format (int): SS58 format (network prefix) the address must use.

    Returns:
        bool: True if the address is a keyless address.

    Raises:
        ValueError: If the address cannot be decoded at all.
    """
    return decode_address(blockchain_address, ss58_format).account_type is not AccountType.Regular
