# ruff: noqa

# Python SCALE Codec Library
#
# Copyright 2018-2021 Stichting Polkascan (Polkascan Foundation).
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
# http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.
#
#  ss58.py

""" SS58 is a simple address format designed for Substrate based chains.
    Encoding/decoding according to specification on
    https://github.com/paritytech/substrate/wiki/External-Address-Format-(SS58)

"""
from typing import Optional, Union

import base58
from hashlib import blake2b

# Longest base58 string any payload supported by this codec can produce: the largest
# payload in the checksum-length table of ss58_decode is 2 (prefix) + 33 (public key) +
# 2 (checksum) = 37 bytes = 296 bits, and each base58 character carries log2(58) ~ 5.858
# bits, so a valid address has at most ceil(296 / 5.858) = 51 characters. Longer input is
# rejected before base58 decoding, whose cost grows quadratically with the input length.
# (The trevokeyless layer applies its stricter 50-character bound for 32-byte account
# ids; this generic bound mirrors the same hardening inside the Rust SS58 decoder.)
_MAX_SS58_ADDRESS_LENGTH = 51

# The 58 characters of the Bitcoin base58 alphabet used by SS58. base58.b58decode
# right-strips whitespace from its input (str.rstrip) before decoding, which would give
# every address additional accepted spellings such as "<address>\n"; the Rust (bs58) and
# TypeScript (@polkadot) decoders reject any character outside the alphabet. The charset
# is therefore checked explicitly before decoding.
_BASE58_ALPHABET = frozenset("123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz")

# The hex digits accepted in a "0x"-prefixed account id string.
_HEX_DIGITS = frozenset("0123456789abcdefABCDEF")


def _validate_base58_charset(address: str) -> None:
    if not all(c in _BASE58_ALPHABET for c in address):
        raise ValueError("Invalid base58 character")


def _decode_hex_strict(hex_digits: str) -> bytes:
    """bytes.fromhex with a strict charset: fromhex tolerates ASCII whitespace between
    digit pairs, which would let malformed strings through. The trevokeyless layer and
    the Rust and TypeScript implementations all reject such input."""
    if not all(c in _HEX_DIGITS for c in hex_digits):
        raise ValueError("Invalid hex account id")
    try:
        return bytes.fromhex(hex_digits)
    except ValueError as error:
        # Odd number of hex digits.
        raise ValueError("Invalid hex account id") from error


def ss58_decode(address: str, valid_ss58_format: Optional[int] = None) -> str:
    """
    Decodes given SS58 encoded address to an account ID
    Parameters
    ----------
    address: e.g. EaG2CRhJWPb7qmdcJvy3LiWdh26Jreu9Dx6R1rXxPmYXoDk
    valid_ss58_format

    Returns
    -------
    Decoded string AccountId
    """

    # Check if address is already decoded. The hex digits are still validated, so that
    # malformed input (e.g. "0xzz") raises ValueError instead of being returned verbatim.
    if address.startswith('0x'):
        _decode_hex_strict(address[2:])
        return address

    if address == '':
        raise ValueError("Empty address provided")

    if len(address) > _MAX_SS58_ADDRESS_LENGTH:
        # No valid address of any supported payload is longer; rejecting early keeps the
        # cost of base58-decoding untrusted input bounded.
        raise ValueError("Invalid address length")

    _validate_base58_charset(address)

    checksum_prefix = b'SS58PRE'

    address_decoded = base58.b58decode(address)

    if len(address_decoded) < 2:
        raise ValueError("Invalid address length")

    if address_decoded[0] & 0b1000_0000:
        # Per the SS58 specification the first byte must be in 0..=63 (single-byte format)
        # or 64..=127 (two-byte format); first bytes 128..=255 are invalid.
        raise ValueError("Invalid SS58 format prefix")

    if address_decoded[0] & 0b0100_0000:
        ss58_format_length = 2
        ss58_format = ((address_decoded[0] & 0b0011_1111) << 2) | (address_decoded[1] >> 6) | \
                      ((address_decoded[1] & 0b0011_1111) << 8)
    else:
        ss58_format_length = 1
        ss58_format = address_decoded[0]

    if ss58_format in [46, 47]:
        raise ValueError(f"{ss58_format} is a reserved SS58 format")

    if valid_ss58_format is not None and ss58_format != valid_ss58_format:
        raise ValueError("Invalid SS58 format")

    # Determine checksum length according to length of address string
    if len(address_decoded) in [3, 4, 6, 10]:
        checksum_length = 1
    elif len(address_decoded) in [5, 7, 11, 34 + ss58_format_length, 35 + ss58_format_length]:
        checksum_length = 2
    elif len(address_decoded) in [8, 12]:
        checksum_length = 3
    elif len(address_decoded) in [9, 13]:
        checksum_length = 4
    elif len(address_decoded) in [14]:
        checksum_length = 5
    elif len(address_decoded) in [15]:
        checksum_length = 6
    elif len(address_decoded) in [16]:
        checksum_length = 7
    elif len(address_decoded) in [17]:
        checksum_length = 8
    else:
        raise ValueError("Invalid address length")

    checksum = blake2b(checksum_prefix + address_decoded[0:-checksum_length]).digest()

    if checksum[0:checksum_length] != address_decoded[-checksum_length:]:
        raise ValueError("Invalid checksum")

    return "0x" + address_decoded[ss58_format_length:len(address_decoded)-checksum_length].hex()


def ss58_encode(address: Union[str, bytes], ss58_format: int = 42) -> str:
    """
    Encodes an account ID to an Substrate address according to provided address_type

    Parameters
    ----------
    address
    ss58_format

    Returns
    -------
    str
    """
    checksum_prefix = b'SS58PRE'

    if ss58_format < 0 or ss58_format > 16383 or ss58_format in [46, 47]:
        raise ValueError("Invalid value for ss58_format")

    if type(address) is bytes or type(address) is bytearray:
        address_bytes = address
    else:
        # Strip only the optional "0x" prefix: str.replace would drop "0x" occurrences
        # anywhere in the string, silently accepting malformed input such as
        # "00...0x...00". The strict decode also rejects the whitespace that
        # bytes.fromhex would otherwise tolerate.
        address_bytes = _decode_hex_strict(address.removeprefix('0x'))  # type: ignore[arg-type]

    if len(address_bytes) in [32, 33]:
        # Checksum size is 2 bytes for public key
        checksum_length = 2
    elif len(address_bytes) in [1, 2, 4, 8]:
        # Checksum size is 1 byte for account index
        checksum_length = 1
    else:
        raise ValueError("Invalid length for address")

    if ss58_format < 64:
        ss58_format_bytes = bytes([ss58_format])
    else:
        ss58_format_bytes = bytes([
            ((ss58_format & 0b0000_0000_1111_1100) >> 2) | 0b0100_0000,
            (ss58_format >> 8) | ((ss58_format & 0b0000_0000_0000_0011) << 6)
        ])

    input_bytes = ss58_format_bytes + address_bytes
    checksum = blake2b(checksum_prefix + input_bytes).digest()

    return base58.b58encode(input_bytes + checksum[:checksum_length]).decode()


def is_valid_ss58_address(value: str, valid_ss58_format: Optional[int] = None) -> bool:
    """
    Checks if given value is a valid SS58 formatted address, optionally check if address is valid for specified
    ss58_format

    Parameters
    ----------
    value: value to checked
    valid_ss58_format: if valid_ss58_format is provided the address must be valid for specified ss58_format (network) as well

    Returns
    -------
    bool
    """

    # Return False in case a public key is provided
    if value.startswith('0x'):
        return False

    try:
        ss58_decode(value, valid_ss58_format=valid_ss58_format)
    except ValueError:
        return False

    return True


def get_ss58_format(ss58_address: str) -> int:
    """
    Returns the SS58 format for given SS58 address

    Parameters
    ----------
    ss58_address

    Returns
    -------
    int
    """
    if len(ss58_address) > _MAX_SS58_ADDRESS_LENGTH:
        # Same early bound as ss58_decode: keeps base58 decoding of untrusted input cheap.
        raise ValueError("Invalid address length")

    _validate_base58_charset(ss58_address)

    address_decoded = base58.b58decode(ss58_address)

    if len(address_decoded) < 2:
        raise ValueError("Invalid address length")

    if address_decoded[0] & 0b1000_0000:
        # Per the SS58 specification the first byte must be in 0..=63 (single-byte format)
        # or 64..=127 (two-byte format); first bytes 128..=255 are invalid.
        raise ValueError("Invalid SS58 format prefix")

    if address_decoded[0] & 0b0100_0000:
        ss58_format = ((address_decoded[0] & 0b0011_1111) << 2) | (address_decoded[1] >> 6) | \
                      ((address_decoded[1] & 0b0011_1111) << 8)
    else:
        ss58_format = address_decoded[0]

    if ss58_format in [46, 47]:
        raise ValueError(f"{ss58_format} is a reserved SS58 format")

    return ss58_format
