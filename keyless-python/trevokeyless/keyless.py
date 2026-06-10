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

from dataclasses import dataclass
from enum import Enum
from hashlib import blake2b
from typing import TypeAlias

from .ss58 import ss58_decode, ss58_encode

# Constants
NAMED_ADDRESS_LENGTH = 10
SS58_FORMAT__TREVO_ASSET_HUB = 5335

# Length in bytes of a blockchain account id.
ACCOUNT_ID_LENGTH = 32

# Maximum length in characters of an SS58 address of a 32-byte account id: the payload is
# at most 2 (prefix) + 32 (account id) + 2 (checksum) = 36 bytes = 288 bits, and each
# base58 character carries log2(58) ~ 5.858 bits, so a valid address has at most
# ceil(288 / 5.858) = 50 characters. Longer input is rejected before base58 decoding,
# whose cost grows quadratically with the input length. Mirrors the Rust implementation.
SS58_MAX_ADDRESS_LENGTH = 50


# Types of address identifiers
BlockchainAddress: TypeAlias = str
BlockchainAccountId: TypeAlias = str
AppAgentId: TypeAlias = int
TransactionalAddressId: TypeAlias = int
AddressName: TypeAlias = str
SS58Format: TypeAlias = int


class AddressType(Enum):
    Regular = 0
    AppAgent = 1
    Transactional = 2
    Named = 3


class AddressTypeMismatchError(ValueError):
    """Raised by the typed decode_* functions when the address is of a different type.

    Carries the expected and the actual (found) classification of the address, mirroring
    the ``KeylessDecodeError`` of the Rust implementation. Subclasses ``ValueError`` for
    backwards compatibility with previous releases.

    Attributes:
        expected (AddressType): The address type required by the caller.
        found (AddressType): The actual classification of the address.
    """

    def __init__(self, expected: AddressType, found: AddressType) -> None:
        self.expected = expected
        self.found = found
        article = "an" if expected is AddressType.AppAgent else "a"
        super().__init__(f"Provided address is not {article} {expected.name} address but is instance of {found}.")


@dataclass
class BlockchainAddressInfo:
    address: BlockchainAddress
    account_id: BlockchainAccountId
    address_type: AddressType
    app_agent_id: AppAgentId | None
    ta_id: TransactionalAddressId | None
    address_name: AddressName | None


def _blake2_256(data: bytes) -> bytes:
    """
    Calculates the 32-byte Blake2b hash of the provided data.

    Args:
        data (bytes): The data to hash.

    Returns:
        bytes: The 32-byte Blake2b digest.
    """
    return blake2b(data, digest_size=32).digest()


__allowed_chars = set("0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ-#")


def _validate_address_name(name: str) -> None:
    """
    Validates that a name is exactly 10 characters long and contains only allowed ASCII
    characters (`0-9 a-z A-Z - #`).

    Args:
        name (str): The name to validate.

    Raises:
        ValueError: If the name has a wrong length or contains invalid characters.
    """
    if len(name) != NAMED_ADDRESS_LENGTH:
        msg = "Named keyless address must be of 10 chars length"
        raise ValueError(msg)

    if not all(c in __allowed_chars for c in name):
        msg = "Address name contains invalid characters"
        raise ValueError(msg)


def _decode_address_name(name_bytes: bytes) -> AddressName | None:
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
    """
    # Calculate checksum using blake2_256
    checksum = _blake2_256(open_part)

    # Construct address_encoded
    address_encoded = open_part + checksum[len(open_part) :]

    return ss58_encode(address_encoded, ss58_format)


def encode_app_agent_address(
    app_agent_id: AppAgentId, ss58_format: SS58Format = SS58_FORMAT__TREVO_ASSET_HUB
) -> BlockchainAddress:
    """
    Encode an AppAgent address.

    Args:
        app_agent_id (int): AppAgent ID.

    Returns:
        str: Encoded AppAgent address.
    """
    # Convert app_agent_id to little-endian bytes
    app_agent_id_bytes = app_agent_id.to_bytes(4, byteorder="little")

    # Construct open_part
    open_part = app_agent_id_bytes + bytes([AddressType.AppAgent.value])

    return _encode_address(open_part, ss58_format)


def decode_app_agent_address(
    encoded_address: BlockchainAddress, ss58_format: SS58Format = SS58_FORMAT__TREVO_ASSET_HUB
) -> AppAgentId:
    """
    Decode an encoded AppAgent address.

    Args:
        encoded_address (str): Encoded AppAgent address.

    Returns:
        int: Decoded AppAgent ID.

    Raises:
        AddressTypeMismatchError: If the address is not an AppAgent address.
        ValueError: If the address cannot be decoded at all.
    """
    decoding_result = decode_address(encoded_address, ss58_format)

    if decoding_result.address_type is not AddressType.AppAgent:
        raise AddressTypeMismatchError(AddressType.AppAgent, decoding_result.address_type)
    if decoding_result.app_agent_id is None:
        msg = "Internal error in trevokeyless - app_agent_id of AppAgent address is None"
        raise ValueError(msg)

    return decoding_result.app_agent_id


def encode_transactional_address(
    app_agent_id: AppAgentId,
    ta_id: TransactionalAddressId,
    ss58_format: SS58Format = SS58_FORMAT__TREVO_ASSET_HUB,
) -> BlockchainAddress:
    """
    Encode a Transactional address.

    Args:
        app_agent_id (int): AppAgent ID.
        ta_id (int): Transactional address ID.

    Returns:
        str: Encoded Transactional address.
    """
    # Convert app_agent_id and ta_id to little-endian bytes
    app_agent_id_bytes = app_agent_id.to_bytes(4, byteorder="little")
    ta_id_bytes = ta_id.to_bytes(4, byteorder="little")

    # Construct open_part
    open_part = app_agent_id_bytes + bytes([AddressType.Transactional.value]) + ta_id_bytes

    return _encode_address(open_part, ss58_format)


def decode_transactional_address(
    encoded_address: BlockchainAddress, ss58_format: SS58Format = SS58_FORMAT__TREVO_ASSET_HUB
) -> tuple[AppAgentId, TransactionalAddressId]:
    """
    Decode an encoded Transactional address.

    Args:
        encoded_address (str): Encoded Transactional address.

    Returns:
        Tuple[int, int]: Decoded AppAgent ID and Transactional address ID.

    Raises:
        AddressTypeMismatchError: If the address is not a Transactional address.
        ValueError: If the address cannot be decoded at all.
    """
    decoding_result = decode_address(encoded_address, ss58_format)

    if decoding_result.address_type is not AddressType.Transactional:
        raise AddressTypeMismatchError(AddressType.Transactional, decoding_result.address_type)
    if decoding_result.app_agent_id is None:
        msg = "Internal error in trevokeyless - app_agent_id of Transactional address is None"
        raise ValueError(msg)
    if decoding_result.ta_id is None:
        msg = "Internal error in trevokeyless - ta_id of Transactional address is None"
        raise ValueError(msg)

    return decoding_result.app_agent_id, decoding_result.ta_id


def encode_named_address(
    app_agent_id: AppAgentId, name: AddressName, ss58_format: SS58Format = SS58_FORMAT__TREVO_ASSET_HUB
) -> BlockchainAddress:
    """
    Encode a Named address.

    Args:
        app_agent_id (int): AppAgent ID.
        name (str): Address name.

    Returns:
        str: Encoded Named address.
    """
    _validate_address_name(name)

    # Convert app_agent_id to little-endian bytes
    app_agent_id_bytes = app_agent_id.to_bytes(4, byteorder="little")

    # Construct open_part
    open_part = app_agent_id_bytes + bytes([AddressType.Named.value]) + name.encode()

    return _encode_address(open_part, ss58_format)


def decode_named_address(
    encoded_address: BlockchainAddress, ss58_format: SS58Format = SS58_FORMAT__TREVO_ASSET_HUB
) -> tuple[AppAgentId, AddressName]:
    """
    Decode an encoded Named address.

    Args:
        encoded_address (str): Encoded Named address.

    Returns:
        Tuple[int, str]: Decoded AppAgent ID and address name.

    Raises:
        AddressTypeMismatchError: If the address is not a Named address.
        ValueError: If the address cannot be decoded at all.
    """
    decoding_result = decode_address(encoded_address, ss58_format)

    if decoding_result.address_type is not AddressType.Named:
        raise AddressTypeMismatchError(AddressType.Named, decoding_result.address_type)
    if decoding_result.app_agent_id is None:
        msg = "Internal error in trevokeyless - app_agent_id of Named address is None"
        raise ValueError(msg)
    if decoding_result.address_name is None:
        msg = "Internal error in trevokeyless - name of Named address is None"
        raise ValueError(msg)

    return decoding_result.app_agent_id, decoding_result.address_name


def _decode_account_id_bytes(blockchain_address: BlockchainAddress, ss58_format: SS58Format) -> bytes:
    """
    Resolves an address string to its raw 32-byte account id.

    Args:
        blockchain_address (str): SS58 address (or `0x`-prefixed 32-byte account id hex).
        ss58_format (int): SS58 format (network prefix) the address must use; not applied
            to raw hex input, which carries no format.

    Returns:
        bytes: The 32-byte account id.

    Raises:
        ValueError: If the input is not a valid SS58 address of a 32-byte account id in
            the expected format, or not a valid 32-byte hex account id.
    """
    if blockchain_address.startswith("0x"):
        # Raw account id passthrough; the ss58_format argument does not apply.
        try:
            account_id_bytes = bytes.fromhex(blockchain_address[2:])
        except ValueError as error:
            msg = "Invalid hex account id"
            raise ValueError(msg) from error
    else:
        if len(blockchain_address) > SS58_MAX_ADDRESS_LENGTH:
            # No valid address of a 32-byte account id is longer; rejecting early keeps
            # the cost of base58-decoding untrusted input bounded.
            msg = "Invalid address length"
            raise ValueError(msg)
        account_id = ss58_decode(blockchain_address, ss58_format)
        account_id_bytes = bytes.fromhex(account_id[2:])

    if len(account_id_bytes) != ACCOUNT_ID_LENGTH:
        msg = f"Invalid account id length: expected {ACCOUNT_ID_LENGTH} bytes, got {len(account_id_bytes)}"
        raise ValueError(msg)
    return account_id_bytes


def decode_address(
    blockchain_address: BlockchainAddress, ss58_format: SS58Format = SS58_FORMAT__TREVO_ASSET_HUB
) -> BlockchainAddressInfo:
    """
    Decode a blockchain address and classify it.

    Accepts an SS58 address string. For backwards compatibility a raw account id in hex
    form (`0x` followed by exactly 64 hex characters) is also accepted; such input
    bypasses SS58 decoding — including the `ss58_format` check, which does not apply to
    raw account bytes.

    Args:
        blockchain_address (str): SS58 address (or `0x`-prefixed 32-byte account id hex).
        ss58_format (int): SS58 format (network prefix) the address must use.

    Returns:
        BlockchainAddressInfo: an object with info about the address.

    Raises:
        ValueError: If the address is not a valid SS58 string of a 32-byte account id in
            the expected format, or not a valid 32-byte hex account id.
    """
    account_id_bytes = _decode_account_id_bytes(blockchain_address, ss58_format)
    account_id = "0x" + account_id_bytes.hex()

    # Read the byte that encodes type of the address
    address_type_byte = account_id_bytes[4]

    if address_type_byte == AddressType.AppAgent.value:
        # Verify checksum
        open_part_size = 5
        open_part = account_id_bytes[:open_part_size]
        checksum = account_id_bytes[open_part_size:]
        checksum_calculated = _blake2_256(open_part)[open_part_size:]
        if checksum == checksum_calculated:
            # Extract and decode app_agent_id
            app_agent_id_bytes = account_id_bytes[:4]
            app_agent_id = int.from_bytes(app_agent_id_bytes, byteorder="little")

            return BlockchainAddressInfo(
                address=blockchain_address,
                account_id=account_id,
                address_type=AddressType.AppAgent,
                app_agent_id=app_agent_id,
                ta_id=None,
                address_name=None,
            )

    if address_type_byte == AddressType.Transactional.value:
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

            return BlockchainAddressInfo(
                address=blockchain_address,
                account_id=account_id,
                address_type=AddressType.Transactional,
                app_agent_id=app_agent_id,
                ta_id=ta_id,
                address_name=None,
            )

    if address_type_byte == AddressType.Named.value:
        # Verify checksum
        open_part_size = 15
        open_part = account_id_bytes[:open_part_size]
        checksum = account_id_bytes[open_part_size:]
        checksum_calculated = _blake2_256(open_part)[open_part_size:]
        if checksum == checksum_calculated:
            # Extract and decode app_agent_id and address_name.
            # The name bytes must additionally pass charset validation: an address with a
            # valid checksum over invalid name bytes is classified as Regular, in line with
            # the Rust implementation.
            app_agent_id_bytes = account_id_bytes[:4]
            name_bytes = account_id_bytes[5:15]
            app_agent_id = int.from_bytes(app_agent_id_bytes, byteorder="little")
            address_name = _decode_address_name(name_bytes)

            if address_name is not None:
                return BlockchainAddressInfo(
                    address=blockchain_address,
                    account_id=account_id,
                    address_type=AddressType.Named,
                    app_agent_id=app_agent_id,
                    ta_id=None,
                    address_name=address_name,
                )

    return BlockchainAddressInfo(
        address=blockchain_address,
        account_id=account_id,
        address_type=AddressType.Regular,
        app_agent_id=None,
        ta_id=None,
        address_name=None,
    )


def is_keyless_address(
    blockchain_address: BlockchainAddress, ss58_format: SS58Format = SS58_FORMAT__TREVO_ASSET_HUB
) -> bool:
    """
    Returns True if the address passes the keyless checksum verification (AppAgent,
    Transactional, or Named), False for any other (Regular) address.

    Mirrors `is_keyless_account` of the Rust implementation.

    Args:
        blockchain_address (str): SS58 address (or `0x`-prefixed 32-byte account id hex).
        ss58_format (int): SS58 format (network prefix) the address must use.

    Returns:
        bool: True if the address is a keyless address.

    Raises:
        ValueError: If the address cannot be decoded at all.
    """
    return decode_address(blockchain_address, ss58_format).address_type is not AddressType.Regular
