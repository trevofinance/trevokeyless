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
    Helper function to calculate a 32 bytes Blake2b hash for provided data, used as key for Substrate storage items

    Parameters
    ----------
    data

    Returns
    -------

    """
    return blake2b(data, digest_size=32).digest()


__allowed_chars = set("0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ-#")


def _validate_address_name(name: str) -> None:
    """
    Validates that a name contains only allowed ASCII characters.

    Args:
        name (str): The name to validate.

    Returns:
        bool: True if the name is valid, otherwise False.
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
    Encode an address using the given open part, open part size, and checksum size.

    Args:
        open_part (bytes): The open part of the address.
        open_part_size (int): Size of the open part.
        checksum_size (int): Size of the checksum.

    Returns:
        str: Encoded address.
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
    """
    decoding_result = decode_address(encoded_address, ss58_format)

    if decoding_result.address_type is not AddressType.AppAgent:
        msg = f"Provided address is not an AppAgent address but is instance of {decoding_result.address_type}."
        raise ValueError(msg)
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
    """
    decoding_result = decode_address(encoded_address, ss58_format)

    if decoding_result.address_type is not AddressType.Transactional:
        msg = f"Provided address is not a Transactional address but is instance of {decoding_result.address_type}."
        raise ValueError(msg)
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
    """
    decoding_result = decode_address(encoded_address, ss58_format)

    if decoding_result.address_type is not AddressType.Named:
        msg = f"Provided address is not a Named address but is instance of {decoding_result.address_type}."
        raise ValueError(msg)
    if decoding_result.app_agent_id is None:
        msg = "Internal error in trevokeyless - app_agent_id of Named address is None"
        raise ValueError(msg)
    if decoding_result.address_name is None:
        msg = "Internal error in trevokeyless - name of Named address is None"
        raise ValueError(msg)

    return decoding_result.app_agent_id, decoding_result.address_name


def decode_address(
    blockchain_address: BlockchainAddress, ss58_format: SS58Format = SS58_FORMAT__TREVO_ASSET_HUB
) -> BlockchainAddressInfo:
    """
    Decode an encoded blockchain address.

    Args:
        encoded_address (str): Encoded keyless address of any type.

    Returns:
        an object with info about the address
    """
    # Decode the encoded address
    account_id = ss58_decode(blockchain_address, ss58_format)
    account_id_bytes = bytes.fromhex(account_id[2:])

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
