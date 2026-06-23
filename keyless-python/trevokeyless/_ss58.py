"""Strict SS58 codec for 32-byte account ids (private module).

A port of the Rust reference implementation (``keyless-rust/src/ss58.rs``, mirrored by
the TypeScript ``src/ss58.ts``). Decoding accepts only the canonical structure: a 1- or
2-byte format prefix, a 32-byte account id, and a full two-byte checksum
(``blake2b_512(b"SS58PRE" ++ prefix ++ account_id)[0..2]``), with a non-reserved format.

The historically vendored generic codec additionally accepted account-index payloads
(1, 2, 4, 8 bytes), 33-byte public keys, and a ``"0x"`` hex passthrough; those inputs
are now rejected with the same error classification as the Rust decoder. The expected
SS58 format is deliberately *not* checked here: ``ss58_decode`` returns the format found
in the address, and the caller compares it only after the address has fully validated —
exactly like ``from_ss58check`` of the Rust implementation.

This module is private to the package; only the entry points of
``trevokeyless.keyless`` are public API. The error types (``Ss58Error``,
``Ss58ErrorKind``) are re-exported from the package root.
"""

from hashlib import blake2b
from typing import Literal, TypeAlias

# Largest valid SS58 format: the prefix encodes 14 bits. Mirrors the SS58_FORMAT_MAX
# bound of the Rust implementation.
SS58_FORMAT_MAX = 16383

# The reserved SS58 formats that can never be used (per the SS58 registry).
_RESERVED_FORMATS = frozenset((46, 47))

# First-byte ranges of the SS58 prefix encoding: 0..=63 is a single-byte format,
# 64..=127 starts a two-byte format, 128..=255 is invalid.
_SINGLE_BYTE_FORMAT_MAX = 0b0011_1111
_TWO_BYTE_PREFIX_FIRST_MAX = 0b0111_1111

# Length in bytes of the SS58 checksum of a 32-byte account id.
_SS58_CHECKSUM_LENGTH = 2

# Length in bytes of a blockchain account id.
_ACCOUNT_ID_LENGTH = 32

# Maximum length in characters of an SS58 address of a 32-byte account id: the payload
# is at most 2 (prefix) + 32 (account id) + 2 (checksum) = 36 bytes = 288 bits, and each
# base58 character carries log2(58) ~ 5.858 bits, so a valid address has at most
# ceil(288 / 5.858) = 50 characters. Longer input is rejected before base58 decoding,
# whose cost grows quadratically with the input length. The bound is applied to the
# UTF-8 byte length, exactly like the Rust reference decoder (bytes and characters
# coincide on the all-ASCII base58 alphabet). It is a safe over-approximation: the
# first payload byte of a valid address is at most 0x7F, so the longest achievable
# address is 49 characters (pinned by the all-0xFF/format-16383 conformance vector).
SS58_MAX_ADDRESS_LENGTH = 50

# The 58 characters of the Bitcoin base58 alphabet used by SS58, in value order.
_BASE58_ALPHABET_STR = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz"

# Value of each alphabet character, for decoding.
_BASE58_INDEX = {char: index for index, char in enumerate(_BASE58_ALPHABET_STR)}

# Charset of the alphabet, checked explicitly in ss58_decode before any base58
# arithmetic so out-of-alphabet input always classifies as InvalidBase58. The codec
# below is vendored: the historically used `base58` package right-stripped whitespace
# from its input (str.rstrip), which would have given every address additional accepted
# spellings such as "<address>\n"; the Rust (bs58) and TypeScript (@scure/base)
# decoders reject any character outside the alphabet, and so does this one.
_BASE58_ALPHABET = frozenset(_BASE58_ALPHABET_STR)

Ss58ErrorKind: TypeAlias = Literal[
    "InvalidBase58",
    "InvalidLength",
    "InvalidChecksum",
    "InvalidFormat",
    "UnexpectedFormat",
]
"""The failure classification of an ``Ss58Error``.

The names mirror the variants of the Rust ``Ss58Error`` enum; the TypeScript
``Ss58Error.kind`` uses the same set. ``"UnexpectedFormat"`` is raised only by
``trevokeyless.keyless`` (as ``Ss58FormatMismatchError``), never by this module.
"""


class Ss58Error(ValueError):
    """Raised by the SS58 decoding (and encoding, for invalid formats) functions.

    Mirrors the ``Ss58Error`` enum of the Rust implementation: ``kind`` is the variant
    name and the message matches the Rust ``Display`` output (the TypeScript
    implementation reports the same pair). Subclasses ``ValueError`` so existing
    ``except ValueError`` handlers treat it like any other decoding failure.

    ``trevokeyless.Ss58FormatMismatchError`` is the ``"UnexpectedFormat"`` case; it
    additionally carries the expected and the found format.

    Attributes:
        kind (Ss58ErrorKind): The failure classification.
    """

    def __init__(self, kind: Ss58ErrorKind, message: str) -> None:
        self.kind: Ss58ErrorKind = kind
        super().__init__(message)

    @classmethod
    def invalid_base58(cls) -> "Ss58Error":
        """The string contains a character outside the base58 alphabet."""
        return cls("InvalidBase58", "Address is not valid base58")

    @classmethod
    def invalid_length(cls) -> "Ss58Error":
        """The address is longer than any valid SS58-encoded 32-byte account id, or the
        decoded payload does not have the expected length."""
        return cls("InvalidLength", "Address has an invalid length")

    @classmethod
    def invalid_checksum(cls) -> "Ss58Error":
        """The checksum does not match the payload."""
        return cls("InvalidChecksum", "Address has an invalid checksum")

    @classmethod
    def invalid_format(cls) -> "Ss58Error":
        """The SS58 format is reserved (46, 47), out of range (> 16383), or its prefix
        byte cannot be parsed."""
        return cls("InvalidFormat", "SS58 format is invalid or reserved")


def _b58encode(data: bytes) -> str:
    """Base58-encodes ``data`` with the Bitcoin alphabet.

    Vendored so the package has no third-party runtime dependencies; the SS58 payloads
    handled here are at most 36 bytes, so the big-int arithmetic is trivial. Each
    leading zero byte encodes as one leading ``"1"`` character.
    """
    num = int.from_bytes(data, byteorder="big")
    encoded: list[str] = []
    while num:
        num, digit = divmod(num, 58)
        encoded.append(_BASE58_ALPHABET_STR[digit])
    zeros = len(data) - len(data.lstrip(b"\x00"))
    return "1" * zeros + "".join(reversed(encoded))


def _b58decode(encoded: str) -> bytes:
    """Base58-decodes ``encoded``, rejecting any character outside the alphabet.

    Strict standalone: no whitespace tolerance of any kind (``ss58_decode``
    additionally pre-checks the charset, so the classification never depends on this
    decoder's rejection). Each leading ``"1"`` character decodes to one leading zero
    byte.

    Raises:
        Ss58Error: kind ``"InvalidBase58"`` on out-of-alphabet characters.
    """
    num = 0
    for char in encoded:
        digit = _BASE58_INDEX.get(char)
        if digit is None:
            raise Ss58Error.invalid_base58()
        num = num * 58 + digit
    body = num.to_bytes((num.bit_length() + 7) // 8, byteorder="big")
    zeros = len(encoded) - len(encoded.lstrip("1"))
    return b"\x00" * zeros + body


def _format_invalid(ss58_format: int) -> bool:
    """True for SS58 formats that can never be used: reserved (46, 47), out of the
    14-bit range, or not an integer at all.

    Mirrors the TypeScript ``formatInvalid`` check; the Rust implementation enforces
    integrality through the ``u16`` type. ``bool`` is rejected explicitly: it is an
    ``int`` subclass, but silently encoding ``True`` as format 1 would hide a caller
    bug.
    """
    return (
        isinstance(ss58_format, bool)
        or not isinstance(ss58_format, int)
        or ss58_format < 0
        or ss58_format > SS58_FORMAT_MAX
        or ss58_format in _RESERVED_FORMATS
    )


def _ss58_hash(data: bytes) -> bytes:
    """The 64-byte blake2b hash of ``b"SS58PRE" + data``, of which the first
    ``_SS58_CHECKSUM_LENGTH`` bytes are the SS58 checksum."""
    return blake2b(b"SS58PRE" + data, digest_size=64).digest()


def ss58_encode(account_id: bytes, ss58_format: int) -> str:
    """Encodes a 32-byte account id as an SS58 address with the given format.

    Args:
        account_id (bytes): The raw 32-byte account id.
        ss58_format (int): SS58 format (network prefix) to encode.

    Returns:
        str: The SS58 address.

    Raises:
        Ss58Error: kind ``"InvalidFormat"`` if the format is reserved (46, 47) or
            not an integer in [0, 16383]; kind ``"InvalidLength"`` if the account id is
            not exactly 32 bytes.
    """
    if _format_invalid(ss58_format):
        raise Ss58Error.invalid_format()
    if len(account_id) != _ACCOUNT_ID_LENGTH:
        raise Ss58Error.invalid_length()

    if ss58_format <= _SINGLE_BYTE_FORMAT_MAX:
        prefix = bytes([ss58_format])
    else:
        # Two-byte prefix: the first byte carries bits 2..=7 of the format (tagged with
        # 0b01 in its upper two bits), the second byte carries bits 8..=13 in its lower
        # six bits and bits 0..=1 in its upper two bits.
        prefix = bytes(
            [
                ((ss58_format & 0b0000_0000_1111_1100) >> 2) | 0b0100_0000,
                (ss58_format >> 8) | ((ss58_format & 0b0000_0000_0000_0011) << 6),
            ]
        )
    payload = prefix + account_id
    return _b58encode(payload + _ss58_hash(payload)[:_SS58_CHECKSUM_LENGTH])


def ss58_decode(address: str) -> tuple[bytes, int]:
    """Decodes an SS58 address of any (non-reserved) format.

    Input longer than the maximum length of a valid address (50 characters) is rejected
    before base58 decoding, so the cost of decoding untrusted input stays bounded.

    Args:
        address (str): The SS58 address string.

    Returns:
        tuple[bytes, int]: The 32-byte account id and the format found in the address.

    Raises:
        Ss58Error: with the ``kind`` describing the failure, exactly like the Rust
            ``from_ss58check_with_version``.
    """
    # The bound is defined in UTF-8 bytes, like the Rust reference decoder (which checks
    # the byte length of the &str): non-ASCII input can stay within 50 characters yet
    # exceed 50 bytes and must classify as a length failure in every implementation.
    # The code-point comparison short-circuits first, keeping the cost O(1) for huge
    # input; the byte encoding only ever sees at most 50 characters. Unpaired
    # surrogates - a str can hold them, e.g. from json.loads of a "\ud800" escape - are
    # counted via surrogatepass (3 bytes each, the byte count TextEncoder gives the
    # TypeScript implementation), so they classify through this error taxonomy (length
    # here, charset below) instead of leaking UnicodeEncodeError. Rust cannot represent
    # such strings at all (&str is always valid UTF-8). Best-effort caveat: a *paired*
    # surrogate stays two code points in Python (6 bytes) where JavaScript sees one
    # astral character (4 bytes); such strings cannot reach this code from UTF-8 or
    # JSON input.
    if (
        len(address) > SS58_MAX_ADDRESS_LENGTH
        or len(address.encode("utf-8", errors="surrogatepass")) > SS58_MAX_ADDRESS_LENGTH
    ):
        raise Ss58Error.invalid_length()
    if not all(c in _BASE58_ALPHABET for c in address):
        raise Ss58Error.invalid_base58()
    data = _b58decode(address)
    if not data:
        raise Ss58Error.invalid_length()

    if data[0] <= _SINGLE_BYTE_FORMAT_MAX:
        prefix_length = 1
        ss58_format = data[0]
    elif data[0] <= _TWO_BYTE_PREFIX_FIRST_MAX:
        if len(data) < 2:  # noqa: PLR2004 - the two-byte prefix needs a second byte
            raise Ss58Error.invalid_length()
        prefix_length = 2
        ss58_format = ((data[0] & 0b0011_1111) << 2) | (data[1] >> 6) | ((data[1] & 0b0011_1111) << 8)
    else:
        # First bytes 128..=255 are not a valid SS58 prefix encoding.
        raise Ss58Error.invalid_format()
    if _format_invalid(ss58_format):
        raise Ss58Error.invalid_format()
    if len(data) != prefix_length + _ACCOUNT_ID_LENGTH + _SS58_CHECKSUM_LENGTH:
        raise Ss58Error.invalid_length()

    checksum_start = len(data) - _SS58_CHECKSUM_LENGTH
    if data[checksum_start:] != _ss58_hash(data[:checksum_start])[:_SS58_CHECKSUM_LENGTH]:
        raise Ss58Error.invalid_checksum()

    return bytes(data[prefix_length:checksum_start]), ss58_format
