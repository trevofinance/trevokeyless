"""keyless.py

This module provides functionality for encoding and decoding keyless addresses.

A keyless address is a type of address that doesn't rely on cryptographic key pairs for identification.
Instead, it is derived from a combination of identifiers and checksums, making it suitable for various
use cases such as tracking and verification.

Examples:
    # Encode an AppAgent keyless address
    app_agent_id = AppAgentId(123)
    app_agent_address = encode_app_agent_address(app_agent_id)

    # Decode the keyless address
    decoded_app_agent_id = decode_app_agent_address(app_agent_address)
    assert decoded_app_agent_id == app_agent_id
"""

import re
from dataclasses import dataclass, field
from enum import IntEnum
from hashlib import blake2b
from typing import Final, Literal, NamedTuple, TypeAlias, TypeGuard, get_args

from ._ss58 import SS58_FORMAT_MAX as _SS58_FORMAT_MAX
from ._ss58 import Ss58Error, ss58_decode, ss58_encode
from ._ss58 import _text_encoder_utf8_length as _utf8_length

# Length in bytes of the name embedded in a Named account. Every allowed character is
# one-byte ASCII, so this is equivalently the number of allowed characters. Mirrors the
# Rust `ACCOUNT_NAME_LENGTH`.
ACCOUNT_NAME_LENGTH = 10

SS58_FORMAT_TREVO_ASSET_HUB = 5335

# Largest value of a u32 identifier (AppAgent ID, TA ID). Mirrors the `u32` types of the
# Rust implementation and the `U32_MAX` bound of the TypeScript implementation.
U32_MAX = 4294967295

# The byte-[4] account type discriminants of a keyless account id, under the same names as
# the Rust constants. They equal the corresponding `AccountType` values.
APP_AGENT_ACCOUNT_IDENTIFIER = 1
TRANSACTIONAL_ACCOUNT_IDENTIFIER = 2
NAMED_ACCOUNT_IDENTIFIER = 3

# A raw account id accepted by decode_account_id: "0x" followed by exactly 64 hex
# characters. bytes.fromhex is lenient about ASCII whitespace, so the shape must be
# checked explicitly to enforce the documented contract (the TypeScript implementation
# uses the same expression; Rust's AccountId32 FromStr accepts the same string form).
_HEX_ACCOUNT_ID = re.compile(r"0x[0-9a-fA-F]{64}")


# Types of address identifiers
BlockchainAddress: TypeAlias = str
BlockchainAccountId: TypeAlias = str
SS58Format: TypeAlias = int


# Echoing a rejected int back verbatim is only safe while the int stays small: CPython's
# int->str conversion limit (``sys.set_int_max_str_digits``, 4300 digits by default) turns
# the message f-string itself into the raised error for a huge adversarial value — the
# caller sees CPython's "Exceeds the limit ... for integer string conversion" instead of
# the documented message — and Pythons without the limit (< 3.10.7) would echo megabytes
# of adversarial input back at the caller. 64 bits keeps every plausible
# wrong-width-type mistake (2**40, 2**53 + 1, -(2**32)) echoed verbatim.
_MAX_ECHOED_INT: Final = 1 << 64


def _describe_rejected_int(value: int) -> str:
    """Renders a rejected out-of-range int for an error message, capping the echoed size."""
    if -_MAX_ECHOED_INT < value < _MAX_ECHOED_INT:
        return repr(value)
    sign = "negative " if value < 0 else ""
    return f"a {sign}{value.bit_length()}-bit int"


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
        msg = f"{label} must be an integer in [0, {U32_MAX}], got {_describe_rejected_int(value)}"
        raise ValueError(msg)


class AppAgentId(int):
    """Numeric identifier of an AppAgent: an ``int`` in the u32 range [0, 4294967295].

    A validating ``int`` subclass rather than a ``typing.NewType``: the constructor
    rejects out-of-range and wrong-typed values at the point of construction, exactly
    like the TypeScript ``appAgentId()`` factory, instead of deferring the failure to an
    encoder (or letting it through entirely at an untyped call site). Being an ``int``,
    it supports arithmetic, ``to_bytes``, and equality with plain ints, and is a distinct
    type from :class:`TransactionalId`, so a type checker catches transposed identifiers.

    Raises:
        TypeError: If the value is not an int (``bool`` included).
        ValueError: If the value is outside [0, 4294967295].
    """

    __slots__ = ()

    # PYI034 wants `Self`, which typing only gained in 3.11; the package supports 3.10.
    def __new__(cls, value: int) -> "AppAgentId":  # noqa: PYI034
        _validate_u32(value, "app_agent_id")
        return super().__new__(cls, value)


class TransactionalId(int):
    """Numeric identifier of a Transactional account: an ``int`` in the u32 range
    [0, 4294967295].

    The counterpart of :class:`AppAgentId` for Transactional identifiers; validating and
    distinct for the same reasons.

    Raises:
        TypeError: If the value is not an int (``bool`` included).
        ValueError: If the value is outside [0, 4294967295].
    """

    __slots__ = ()

    def __new__(cls, value: int) -> "TransactionalId":  # noqa: PYI034
        _validate_u32(value, "ta_id")
        return super().__new__(cls, value)


class AccountType(IntEnum):
    Regular = 0
    AppAgent = APP_AGENT_ACCOUNT_IDENTIFIER
    Transactional = TRANSACTIONAL_ACCOUNT_IDENTIFIER
    Named = NAMED_ACCOUNT_IDENTIFIER


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
        # bool is rejected explicitly first: it is an int subclass, so True would
        # otherwise silently coerce to AccountType.AppAgent below — the package-wide
        # convention is that bool never stands in for an int.
        if isinstance(expected, bool) or isinstance(found, bool):
            msg = "expected and found must be AccountType members (or their int values), not bool"
            raise TypeError(msg)
        # Coerced through the enum so an invalid argument fails here with the enum's own
        # ValueError, the way every sibling error constructor validates its inputs,
        # instead of leaking an AttributeError from the message f-string below. An
        # untyped caller may pass the numeric values (0-3); they coerce to the members.
        self.expected = AccountType(expected)
        self.found = AccountType(found)
        super().__init__(
            f"Provided address is not {_account_type_article(self.expected)} {self.expected.name} "
            f"address but is {_account_type_article(self.found)} {self.found.name} address"
        )

    def __reduce__(self) -> tuple[type, tuple[object, ...]]:
        # Rebuild from the (expected, found) pair — the cross-language contract — rather
        # than from the rendered message BaseException.__reduce__ would replay.
        return (self.__class__, (self.expected, self.found))


AccountNameErrorKind: TypeAlias = Literal["InvalidLength", "InvalidCharacter"]

# The kinds as runtime data, so AccountNameError can reject an out-of-taxonomy kind instead
# of silently mapping it to the InvalidCharacter message.
_ACCOUNT_NAME_ERROR_KINDS: Final[tuple[AccountNameErrorKind, ...]] = get_args(AccountNameErrorKind)


class AccountNameError(ValueError):
    """Raised when a Named account name violates the shared wire contract.

    ``kind`` is stable machine-readable data; the human-readable messages remain the
    exact messages shared by all three implementations.
    """

    def __init__(self, kind: AccountNameErrorKind) -> None:
        if kind not in _ACCOUNT_NAME_ERROR_KINDS:
            msg = f"kind must be one of {', '.join(_ACCOUNT_NAME_ERROR_KINDS)}, got {kind!r}"
            raise ValueError(msg)
        self.kind = kind
        message = (
            "Account name must be exactly 10 characters long"
            if kind == "InvalidLength"
            else "Account name contains invalid characters"
        )
        super().__init__(message)

    def __reduce__(self) -> tuple[type, tuple[object, ...]]:
        # BaseException.__reduce__ rebuilds the exception as ``cls(*self.args)`` — and args
        # holds the *message*, not the kind. Unpickling therefore reconstructed
        # AccountNameError("Account name must be exactly 10 characters long"), whose kind is
        # not "InvalidLength", silently flipping the message to the InvalidCharacter one.
        return (self.__class__, (self.kind,))


class AccountName(str):
    """The name embedded in a Named account: exactly 10 UTF-8 bytes from `0-9 a-z A-Z - #`.

    A validating ``str`` subclass rather than a bare alias, for the same reason
    :class:`AppAgentId` is a validating ``int`` subclass: the constructor rejects an invalid
    name at the point of construction — where the caller can see it — instead of deferring the
    failure to an encoder. It is the counterpart of the Rust ``AccountName`` newtype and of the
    TypeScript ``accountName()`` factory.

    Being a ``str``, it compares equal to the plain string and works anywhere a ``str`` is
    accepted. The reverse does not hold: the encoders are annotated to take an ``AccountName``,
    so a raw ``str`` is a type error, and a typed caller is told to construct the name — and
    see it rejected — before reaching the encoder. (The encoders still re-validate at runtime,
    for callers with no type checker; that is a safety net, not an alternative calling
    convention.)

    Names are case-sensitive, exact byte strings: ``Treasury-1`` and ``treasury-1`` are two
    different accounts of the same App Agent. Never case-fold or trim a name.

    Raises:
        TypeError: If the value is not a str.
        AccountNameError: If the name is not 10 UTF-8 bytes long (kind ``"InvalidLength"``) or
            contains characters outside the allowed set (kind ``"InvalidCharacter"``).
    """

    __slots__ = ()

    def __new__(cls, value: str) -> "AccountName":  # noqa: PYI034
        _validate_account_name(value)
        return super().__new__(cls, value)


# Non-throwing counterparts of the validating constructors above, mirroring the
# TypeScript ``isAppAgentId`` / ``isTransactionalId`` / ``isAccountName`` type guards.
# Each reuses the constructor's exact validation rules rather than restating them, and
# narrows for a type checker without blessing the value at runtime — the encoders
# re-validate regardless, exactly as they do for the constructed types.


def is_app_agent_id(value: object) -> TypeGuard[AppAgentId]:
    """Non-throwing counterpart of :class:`AppAgentId` — True when the constructor
    would accept the value.

    Args:
        value (object): The value to test.

    Returns:
        bool: True if the value is an ``int`` (``bool`` excluded) in the u32 range
        [0, 4294967295]; narrows the value to :class:`AppAgentId` for a type checker.
    """
    if isinstance(value, bool) or not isinstance(value, int):
        return False
    try:
        _validate_u32(value, "value")
    except ValueError:
        return False
    return True


def is_transactional_id(value: object) -> TypeGuard[TransactionalId]:
    """Non-throwing counterpart of :class:`TransactionalId` — True when the constructor
    would accept the value.

    Args:
        value (object): The value to test.

    Returns:
        bool: True if the value is an ``int`` (``bool`` excluded) in the u32 range
        [0, 4294967295]; narrows the value to :class:`TransactionalId` for a type
        checker.
    """
    if isinstance(value, bool) or not isinstance(value, int):
        return False
    try:
        _validate_u32(value, "value")
    except ValueError:
        return False
    return True


def is_account_name(value: object) -> TypeGuard[AccountName]:
    """Non-throwing counterpart of :class:`AccountName` — True when the constructor
    would accept the value.

    Args:
        value (object): The value to test.

    Returns:
        bool: True if the value is a ``str`` of exactly 10 UTF-8 bytes from
        ``0-9 a-z A-Z - #``; narrows the value to :class:`AccountName` for a type
        checker.
    """
    if not isinstance(value, str):
        return False
    try:
        _validate_account_name(value)
    except AccountNameError:
        return False
    return True


class ParseAccountIdError(ValueError):
    """Raised when a string is not a well-formed raw account id.

    A raw account id is ``0x`` followed by exactly 64 hex characters (both digit cases
    accepted). Raised by :func:`decode_account_id`, :func:`encode_address`, and
    :func:`is_keyless_account_id`, and by the typed account-id decoders. Mirrors the Rust
    ``ParseAccountIdError`` (the message is its ``Display`` output). Subclasses
    ``ValueError`` so ``except ValueError`` handlers treat it like any other decoding
    failure.
    """

    def __init__(self) -> None:
        super().__init__("Invalid hex account id: expected 0x followed by 64 hex characters")

    def __reduce__(self) -> tuple[type, tuple[object, ...]]:
        # Takes no constructor arguments, but BaseException.__reduce__ would replay the
        # message as one.
        return (self.__class__, ())


class NotAnAddressError(ValueError):
    """Raised when a raw ``0x`` account id is passed to an SS58 address entry point.

    SS58 addresses and raw account ids are separate input classes: a raw account id
    carries no SS58 format, so accepting one here would silently bypass the
    expected-format check. Raised by :func:`decode_address`,
    :func:`decode_address_with_format`, :func:`is_keyless_address`, and the typed address
    decoders; :func:`decode_account_id` is the entry point for raw account ids.
    Subclasses ``ValueError``.
    """

    def __init__(self) -> None:
        super().__init__("Raw account id hex is not an SS58 address; use decode_account_id")

    def __reduce__(self) -> tuple[type, tuple[object, ...]]:
        # Takes no constructor arguments, but BaseException.__reduce__ would replay the
        # message as one.
        return (self.__class__, ())


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
        # Both fields are SS58 formats; validate them the way every sibling error
        # constructor validates its inputs. The decode paths always pass pre-validated
        # values, so this guards direct construction only — where a huge adversarial
        # int would otherwise surface as CPython's int->str digit-limit error from the
        # message f-string below, and a wrong-typed value would mint a nonsense error.
        # Reserved formats stay constructible: 46/47 are well-formed *expected* values.
        _validate_ss58_format(expected)
        _validate_ss58_format(found)
        self.expected = expected
        self.found = found
        # Bypasses Ss58Error.__init__, which derives the message from the kind and has no
        # fixed string for "UnexpectedFormat" (this message embeds the pair): set the kind
        # directly and hand ValueError the formatted message. The wording is shared
        # verbatim with the Rust Display output and the TypeScript implementation.
        self.kind = "UnexpectedFormat"
        ValueError.__init__(self, f"Expected SS58 format {expected} but found {found}")

    def __reduce__(self) -> tuple[type, tuple[object, ...]]:
        # Overrides Ss58Error.__reduce__: this subclass takes (expected, found), not
        # (kind, message).
        return (self.__class__, (self.expected, self.found))


@dataclass(frozen=True, slots=True, kw_only=True)
class RegularAccountInfo:
    """Decoded identity of a regular (non-keyless) account."""

    address: BlockchainAddress | None
    account_id: BlockchainAccountId
    account_type: Literal[AccountType.Regular] = field(init=False, default=AccountType.Regular)


@dataclass(frozen=True, slots=True, kw_only=True)
class AppAgentAccountInfo:
    """Decoded identity of an AppAgent account."""

    address: BlockchainAddress | None
    account_id: BlockchainAccountId
    app_agent_id: AppAgentId
    account_type: Literal[AccountType.AppAgent] = field(init=False, default=AccountType.AppAgent)


@dataclass(frozen=True, slots=True, kw_only=True)
class TransactionalAccountInfo:
    """Decoded identity of a Transactional account."""

    address: BlockchainAddress | None
    account_id: BlockchainAccountId
    app_agent_id: AppAgentId
    ta_id: TransactionalId
    account_type: Literal[AccountType.Transactional] = field(init=False, default=AccountType.Transactional)


@dataclass(frozen=True, slots=True, kw_only=True)
class NamedAccountInfo:
    """Decoded identity of a Named account."""

    address: BlockchainAddress | None
    account_id: BlockchainAccountId
    app_agent_id: AppAgentId
    account_name: AccountName
    account_type: Literal[AccountType.Named] = field(init=False, default=AccountType.Named)


BlockchainAccountInfo: TypeAlias = (
    RegularAccountInfo | AppAgentAccountInfo | TransactionalAccountInfo | NamedAccountInfo
)
"""Immutable, per-variant decoded account identity.

Every variant contains only the identifiers valid for that account type, so callers
cannot construct contradictory states such as a Regular account with an AppAgent ID.
``address`` is the input SS58 spelling, or ``None`` for ``decode_account_id``;
``account_id`` is always normalized lowercase ``0x`` hex.
"""


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


def _validate_ss58_format(ss58_format: int) -> None:
    """
    Validates that an SS58 format is an integer in [0, 16383] (the prefix encodes
    14 bits).

    The Rust implementation enforces this through the `u16` type plus a range check; the
    TypeScript implementation performs the equivalent runtime check. The failure modes
    split along Python convention: a non-int (``bool`` included, for the same reason as
    in `_validate_u32`) raises a deliberate TypeError - not one leaked from the prefix
    byte arithmetic - and an out-of-range int raises ValueError. The reserved formats
    46 and 47 remain valid expected-format inputs for decoders; the SS58 layer rejects
    them only when an encoder tries to put one on the wire.

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
        msg = f"ss58_format must be an integer in [0, {_SS58_FORMAT_MAX}], got {_describe_rejected_int(ss58_format)}"
        raise ValueError(msg)


# The SS58 formats that can never appear in an address (per the SS58 registry).
_RESERVED_SS58_FORMATS: Final[frozenset[int]] = frozenset((46, 47))
_RESERVED_LIST = ", ".join(str(value) for value in sorted(_RESERVED_SS58_FORMATS))


def _validate_ss58_format_for_encoding(ss58_format: int) -> None:
    """
    Validates an SS58 format that is about to be put on the wire.

    Everything `_validate_ss58_format` rejects, plus the reserved formats 46 and 47: no
    address may carry them, so an encoder cannot honour the request. Decoders deliberately
    do *not* apply this check - a reserved expected-format is a well-formed question whose
    answer is "no address matches", i.e. an ``Ss58FormatMismatchError``.

    An unusable format is a rejected *argument* however it is unusable, so it raises the
    same TypeError/ValueError as an out-of-range one rather than an `Ss58Error`, which is a
    report about an *address*. The Rust encoder makes the same distinction with its
    dedicated `InvalidSs58Format` type, and the TypeScript one raises `RangeError` here.

    Args:
        ss58_format (int): The SS58 format to validate.

    Raises:
        TypeError: If the format is not an int (``bool`` included).
        ValueError: If the format is outside [0, 16383], or is reserved (46, 47).
    """
    _validate_ss58_format(ss58_format)
    if ss58_format in _RESERVED_SS58_FORMATS:
        msg = f"ss58_format must not be a reserved format ({_RESERVED_LIST}), got {ss58_format!r}"
        raise ValueError(msg)


# The account-name charset, frozen. A mutable `set` here would be a module-global that
# gates the wire format: mutating it in-process makes the encoder emit an account id no
# other implementation can produce or classify. The sibling base58 alphabet in _ss58 is a
# frozenset for the same reason. (Also not name-mangled: a leading double underscore
# would be rewritten inside any class body that referenced it.)
_ALLOWED_NAME_CHARS: Final[frozenset[str]] = frozenset(
    "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ-#"
)


def _validate_account_name(name: str) -> None:
    """
    Validates that a name is exactly 10 bytes long in UTF-8 and contains only allowed
    ASCII characters (`0-9 a-z A-Z - #`).

    The length is the UTF-8 **byte** length, not the number of code points: the name
    occupies exactly ``ACCOUNT_NAME_LENGTH`` bytes on the wire, and the Rust
    implementation (the source of truth) validates the byte length of the name bytes.
    The two rules accept and reject exactly the same names — every allowed character is
    one-byte ASCII — but they disagree on the *classification* of a non-ASCII name, and
    the classification is pinned per vector by the shared conformance fixture. So
    ``"héllo12345"`` (10 code points, 11 UTF-8 bytes) is an ``InvalidLength``, while
    ``"café12345"`` (9 code points, 10 UTF-8 bytes) is an ``InvalidCharacter``.

    The byte length is counted with JavaScript ``TextEncoder`` semantics (see
    ``_ss58._text_encoder_utf8_length``), so a lone surrogate — which ``str.encode``
    cannot represent at all — counts as the three-byte replacement character instead of
    raising ``UnicodeEncodeError``, keeping hostile host-language strings in the same
    error category as the TypeScript implementation.

    Args:
        name (str): The name to validate.

    Raises:
        TypeError: If the name is not a str.
        AccountNameError: If the name is not 10 UTF-8 bytes long (kind
            ``"InvalidLength"``) or contains invalid characters (kind
            ``"InvalidCharacter"``).
    """
    _require_str(name, "name")
    # Short-circuit on the code-point count before walking the string: UTF-8 encodes every
    # code point in at least one byte, so a name longer than ACCOUNT_NAME_LENGTH characters
    # cannot be ACCOUNT_NAME_LENGTH bytes. Without this, a caller-supplied name (the natural
    # input of encode_named_address) is scanned in full — a linear amplification vector, and
    # the only entry point that did not bound its input up front.
    if len(name) > ACCOUNT_NAME_LENGTH or _utf8_length(name) != ACCOUNT_NAME_LENGTH:
        kind: AccountNameErrorKind = "InvalidLength"
        raise AccountNameError(kind)

    if not all(c in _ALLOWED_NAME_CHARS for c in name):
        kind = "InvalidCharacter"
        raise AccountNameError(kind)


def _decode_account_name(name_bytes: bytes) -> AccountName | None:
    """
    Decodes the raw name bytes of a Named account, validating the character set.

    Anyone can construct an account id that carries a valid checksum over arbitrary name
    bytes (no private key is needed), so the name bytes must be validated during decoding.

    The sole caller (``_classify_account_id``) always passes ``account_id_bytes[5:15]``,
    exactly ``ACCOUNT_NAME_LENGTH`` bytes sliced from a validated 32-byte account id, so no
    length check is needed here.

    Args:
        name_bytes (bytes): The raw 10 name bytes of the Named account.

    Returns:
        AccountName | None: The decoded name, or None if the bytes are not a valid account
        name.
    """
    if not all(chr(b) in _ALLOWED_NAME_CHARS for b in name_bytes):
        return None
    # The bytes have just passed the length and charset checks, so this cannot raise.
    return AccountName(name_bytes.decode("ascii"))


def _account_id_from_open_part(open_part: bytes) -> bytes:
    """
    Builds the 32-byte keyless account id from its open part.

    The account id is `open_part + blake2_256(open_part)[len(open_part):]` — the trailing
    bytes of the hash serve as the checksum.

    Args:
        open_part (bytes): The open (plain-text) part of the account id.

    Returns:
        bytes: The 32-byte keyless account id.
    """
    checksum = _blake2_256(open_part)
    return open_part + checksum[len(open_part) :]


def _hex_account_id(open_part: bytes) -> BlockchainAccountId:
    """The normalized `0x` + lowercase hex account id for the given open part."""
    return "0x" + _account_id_from_open_part(open_part).hex()


def _app_agent_open_part(app_agent_id: AppAgentId) -> bytes:
    """The open part of an AppAgent account: AppAgent ID (LE) + the type identifier.

    The identifiers are validated here rather than relying on the AppAgentId /
    TransactionalId constructors: untyped callers can pass a plain int straight to a
    public encoder, and it must be checked exactly like a constructed identifier.
    """
    _validate_u32(app_agent_id, "app_agent_id")
    return app_agent_id.to_bytes(4, byteorder="little") + bytes([APP_AGENT_ACCOUNT_IDENTIFIER])


def _transactional_open_part(app_agent_id: AppAgentId, ta_id: TransactionalId) -> bytes:
    """The open part of a Transactional account: AppAgent ID, type identifier, TA ID."""
    _validate_u32(app_agent_id, "app_agent_id")
    _validate_u32(ta_id, "ta_id")
    return (
        app_agent_id.to_bytes(4, byteorder="little")
        + bytes([TRANSACTIONAL_ACCOUNT_IDENTIFIER])
        + ta_id.to_bytes(4, byteorder="little")
    )


def _named_open_part(app_agent_id: AppAgentId, name: AccountName) -> bytes:
    """The open part of a Named account: AppAgent ID, type identifier, the 10 name bytes."""
    _validate_u32(app_agent_id, "app_agent_id")
    _validate_account_name(name)
    return app_agent_id.to_bytes(4, byteorder="little") + bytes([NAMED_ACCOUNT_IDENTIFIER]) + name.encode("ascii")


def _encode_address(open_part: bytes, ss58_format: SS58Format) -> BlockchainAddress:
    """
    Wraps a keyless account id (built from its open part) in an SS58 address.

    Args:
        open_part (bytes): The open (plain-text) part of the account id.
        ss58_format (int): SS58 format (network prefix) to use for the address.

    Returns:
        str: Encoded SS58 address.

    Raises:
        TypeError: If ss58_format is not an int.
        ValueError: If ss58_format is outside [0, 16383], or is reserved (46, 47).
    """
    _validate_ss58_format_for_encoding(ss58_format)
    return ss58_encode(_account_id_from_open_part(open_part), ss58_format)


def encode_app_agent_address(
    app_agent_id: AppAgentId, *, ss58_format: SS58Format = SS58_FORMAT_TREVO_ASSET_HUB
) -> BlockchainAddress:
    """
    Encode an AppAgent address.

    Args:
        app_agent_id (AppAgentId): AppAgent ID.
        ss58_format (SS58Format, optional): SS58 format (network prefix) for the
            address. Keyword-only; defaults to SS58_FORMAT_TREVO_ASSET_HUB (5335).

    Returns:
        str: Encoded AppAgent address.

    Raises:
        TypeError: If app_agent_id or ss58_format is not an int.
        ValueError: If app_agent_id is outside [0, 4294967295], or ss58_format is
            not a valid SS58 format.
    """
    return _encode_address(_app_agent_open_part(app_agent_id), ss58_format)


def encode_app_agent_account_id(app_agent_id: AppAgentId) -> BlockchainAccountId:
    """
    Encode an AppAgent account id (the raw `0x` + 64 hex interchange form).

    The account-id counterpart of `encode_app_agent_address`: it returns the raw account
    id (which carries no SS58 format) instead of an SS58 address - the form accepted by
    `decode_account_id` and `encode_address` and exposed as the `account_id` field of a
    decode result. Mirrors `encode_app_agent_account` of the Rust implementation.

    Args:
        app_agent_id (AppAgentId): AppAgent ID.

    Returns:
        str: The account id: `0x` followed by 64 lowercase hex characters.

    Raises:
        TypeError: If app_agent_id is not an int.
        ValueError: If app_agent_id is outside [0, 4294967295].
    """
    return _hex_account_id(_app_agent_open_part(app_agent_id))


def decode_app_agent_address(
    encoded_address: BlockchainAddress, *, ss58_format: SS58Format = SS58_FORMAT_TREVO_ASSET_HUB
) -> AppAgentId:
    """
    Decode an encoded AppAgent address.

    Args:
        encoded_address (str): Encoded AppAgent address.
        ss58_format (int): SS58 format (network prefix) the address must use.

    Returns:
        AppAgentId: Decoded AppAgent ID.

    Raises:
        TypeError: If the address is not a str, or ss58_format is not an int.
        AccountTypeMismatchError: If the address is not an AppAgent address.
        NotAnAddressError: If the input is a raw `0x` account id (use
            `decode_app_agent_account_id`).
        ValueError: If the address cannot be decoded at all.
    """
    decoding_result = decode_address(encoded_address, ss58_format=ss58_format)

    if not isinstance(decoding_result, AppAgentAccountInfo):
        raise AccountTypeMismatchError(AccountType.AppAgent, decoding_result.account_type)

    return decoding_result.app_agent_id


def decode_app_agent_account_id(account_id: BlockchainAccountId) -> AppAgentId:
    """
    Decode a raw AppAgent account id (the `0x` + 64 hex interchange form).

    The account-id counterpart of `decode_app_agent_address`, mirroring
    `decode_app_agent_account` of the Rust implementation: it classifies the raw account
    id (which carries no SS58 format) instead of an SS58 address.

    Args:
        account_id (str): The account id: `0x` followed by exactly 64 hex characters.

    Returns:
        AppAgentId: Decoded AppAgent ID.

    Raises:
        TypeError: If account_id is not a str.
        AccountTypeMismatchError: If the account id is not an AppAgent account.
        ParseAccountIdError: If the input is not `0x` followed by exactly 64 hex
            characters.
    """
    decoding_result = decode_account_id(account_id)

    if not isinstance(decoding_result, AppAgentAccountInfo):
        raise AccountTypeMismatchError(AccountType.AppAgent, decoding_result.account_type)

    return decoding_result.app_agent_id


def encode_transactional_address(
    *,
    app_agent_id: AppAgentId,
    ta_id: TransactionalId,
    ss58_format: SS58Format = SS58_FORMAT_TREVO_ASSET_HUB,
) -> BlockchainAddress:
    """
    Encode a Transactional address.

    Args:
        app_agent_id (AppAgentId): AppAgent ID.
        ta_id (TransactionalId): Transactional address ID.
        ss58_format (SS58Format, optional): SS58 format (network prefix) for the
            address. Keyword-only; defaults to SS58_FORMAT_TREVO_ASSET_HUB (5335).

    Returns:
        str: Encoded Transactional address.

    Raises:
        TypeError: If app_agent_id, ta_id, or ss58_format is not an int.
        ValueError: If app_agent_id or ta_id is outside [0, 4294967295], or
            ss58_format is not a valid SS58 format.
    """
    return _encode_address(_transactional_open_part(app_agent_id, ta_id), ss58_format)


def encode_transactional_account_id(*, app_agent_id: AppAgentId, ta_id: TransactionalId) -> BlockchainAccountId:
    """
    Encode a Transactional account id (the raw `0x` + 64 hex interchange form).

    The account-id counterpart of `encode_transactional_address`. Mirrors
    `encode_transactional_account` of the Rust implementation.

    Args:
        app_agent_id (AppAgentId): AppAgent ID.
        ta_id (TransactionalId): Transactional address ID.

    Returns:
        str: The account id: `0x` followed by 64 lowercase hex characters.

    Raises:
        TypeError: If app_agent_id or ta_id is not an int.
        ValueError: If app_agent_id or ta_id is outside [0, 4294967295].
    """
    return _hex_account_id(_transactional_open_part(app_agent_id, ta_id))


class DecodedTransactionalAddress(NamedTuple):
    """The identifiers embedded in a Transactional account, returned by
    :func:`decode_transactional_address` and :func:`decode_transactional_account_id`.

    A :class:`typing.NamedTuple` rather than a bare ``(app_agent_id, ta_id)`` tuple, so
    the two integer fields can be accessed by name and not silently transposed; it stays
    compatible with tuple unpacking and indexing. Mirrors the object the TypeScript
    ``decodeTransactionalAddress`` / ``decodeTransactionalAccountId`` return and the Rust
    ``TransactionalAccountIdentifiers`` struct.
    """

    app_agent_id: AppAgentId
    ta_id: TransactionalId


def decode_transactional_address(
    encoded_address: BlockchainAddress, *, ss58_format: SS58Format = SS58_FORMAT_TREVO_ASSET_HUB
) -> DecodedTransactionalAddress:
    """
    Decode an encoded Transactional address.

    Args:
        encoded_address (str): Encoded Transactional address.
        ss58_format (int): SS58 format (network prefix) the address must use.

    Returns:
        DecodedTransactionalAddress: The decoded AppAgent ID and Transactional address ID
            (a named tuple; unpacks as ``app_agent_id, ta_id``).

    Raises:
        TypeError: If the address is not a str, or ss58_format is not an int.
        AccountTypeMismatchError: If the address is not a Transactional address.
        NotAnAddressError: If the input is a raw `0x` account id (use
            `decode_transactional_account_id`).
        ValueError: If the address cannot be decoded at all.
    """
    decoding_result = decode_address(encoded_address, ss58_format=ss58_format)

    if not isinstance(decoding_result, TransactionalAccountInfo):
        raise AccountTypeMismatchError(AccountType.Transactional, decoding_result.account_type)

    return DecodedTransactionalAddress(decoding_result.app_agent_id, decoding_result.ta_id)


def decode_transactional_account_id(account_id: BlockchainAccountId) -> DecodedTransactionalAddress:
    """
    Decode a raw Transactional account id (the `0x` + 64 hex interchange form).

    The account-id counterpart of `decode_transactional_address`, mirroring
    `decode_transactional_account` of the Rust implementation.

    Args:
        account_id (str): The account id: `0x` followed by exactly 64 hex characters.

    Returns:
        DecodedTransactionalAddress: The decoded AppAgent ID and Transactional address ID
            (a named tuple; unpacks as ``app_agent_id, ta_id``).

    Raises:
        TypeError: If account_id is not a str.
        AccountTypeMismatchError: If the account id is not a Transactional account.
        ParseAccountIdError: If the input is not `0x` followed by exactly 64 hex
            characters.
    """
    decoding_result = decode_account_id(account_id)

    if not isinstance(decoding_result, TransactionalAccountInfo):
        raise AccountTypeMismatchError(AccountType.Transactional, decoding_result.account_type)

    return DecodedTransactionalAddress(decoding_result.app_agent_id, decoding_result.ta_id)


def encode_named_address(
    *,
    app_agent_id: AppAgentId,
    account_name: AccountName,
    ss58_format: SS58Format = SS58_FORMAT_TREVO_ASSET_HUB,
) -> BlockchainAddress:
    """
    Encode a Named address.

    Args:
        app_agent_id (AppAgentId): AppAgent ID.
        account_name (AccountName): Account name.
        ss58_format (SS58Format, optional): SS58 format (network prefix) for the
            address. Keyword-only; defaults to SS58_FORMAT_TREVO_ASSET_HUB (5335).

    Returns:
        str: Encoded Named address.

    Raises:
        TypeError: If app_agent_id or ss58_format is not an int, or account_name is not a str.
        AccountNameError: If the name has a wrong length or contains invalid characters.
        ValueError: If app_agent_id is outside [0, 4294967295], or ss58_format is not
            a valid SS58 format.
    """
    return _encode_address(_named_open_part(app_agent_id, account_name), ss58_format)


def encode_named_account_id(*, app_agent_id: AppAgentId, account_name: AccountName) -> BlockchainAccountId:
    """
    Encode a Named account id (the raw `0x` + 64 hex interchange form).

    The account-id counterpart of `encode_named_address`. Mirrors `encode_named_account`
    of the Rust implementation.

    Args:
        app_agent_id (AppAgentId): AppAgent ID.
        account_name (AccountName): Account name.

    Returns:
        str: The account id: `0x` followed by 64 lowercase hex characters.

    Raises:
        TypeError: If app_agent_id is not an int, or account_name is not a str.
        AccountNameError: If the name has a wrong length or contains invalid characters.
        ValueError: If app_agent_id is outside [0, 4294967295].
    """
    return _hex_account_id(_named_open_part(app_agent_id, account_name))


class DecodedNamedAddress(NamedTuple):
    """The identifiers embedded in a Named account, returned by
    :func:`decode_named_address` and :func:`decode_named_account_id`.

    A :class:`typing.NamedTuple` rather than a bare ``(app_agent_id, account_name)``
    tuple; stays compatible with tuple unpacking and indexing. Mirrors the object the
    TypeScript ``decodeNamedAddress`` / ``decodeNamedAccountId`` return and the Rust
    ``NamedAccountIdentifiers`` struct.
    """

    app_agent_id: AppAgentId
    account_name: AccountName


def decode_named_address(
    encoded_address: BlockchainAddress, *, ss58_format: SS58Format = SS58_FORMAT_TREVO_ASSET_HUB
) -> DecodedNamedAddress:
    """
    Decode an encoded Named address.

    Args:
        encoded_address (str): Encoded Named address.
        ss58_format (int): SS58 format (network prefix) the address must use.

    Returns:
        DecodedNamedAddress: The decoded AppAgent ID and account name (a named tuple;
            unpacks as ``app_agent_id, account_name``).

    Raises:
        TypeError: If the address is not a str, or ss58_format is not an int.
        AccountTypeMismatchError: If the address is not a Named address.
        NotAnAddressError: If the input is a raw `0x` account id (use
            `decode_named_account_id`).
        ValueError: If the address cannot be decoded at all.
    """
    decoding_result = decode_address(encoded_address, ss58_format=ss58_format)

    if not isinstance(decoding_result, NamedAccountInfo):
        raise AccountTypeMismatchError(AccountType.Named, decoding_result.account_type)

    return DecodedNamedAddress(decoding_result.app_agent_id, decoding_result.account_name)


def decode_named_account_id(account_id: BlockchainAccountId) -> DecodedNamedAddress:
    """
    Decode a raw Named account id (the `0x` + 64 hex interchange form).

    The account-id counterpart of `decode_named_address`, mirroring
    `decode_named_account` of the Rust implementation.

    Args:
        account_id (str): The account id: `0x` followed by exactly 64 hex characters.

    Returns:
        DecodedNamedAddress: The decoded AppAgent ID and account name (a named tuple;
            unpacks as ``app_agent_id, account_name``).

    Raises:
        TypeError: If account_id is not a str.
        AccountTypeMismatchError: If the account id is not a Named account.
        ParseAccountIdError: If the input is not `0x` followed by exactly 64 hex
            characters.
    """
    decoding_result = decode_account_id(account_id)

    if not isinstance(decoding_result, NamedAccountInfo):
        raise AccountTypeMismatchError(AccountType.Named, decoding_result.account_type)

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
        NotAnAddressError: If the input is a raw `0x` account id.
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
        raise NotAnAddressError

    account_id_bytes, found_format = ss58_decode(blockchain_address)
    # The expected format is compared only after the address has fully validated
    # (structure, length, checksum), exactly like the Rust from_ss58check: an invalid
    # address must raise its validation error, not a misleading format mismatch.
    if found_format != ss58_format:
        raise Ss58FormatMismatchError(ss58_format, found_format)
    return account_id_bytes


def decode_address(
    blockchain_address: BlockchainAddress, *, ss58_format: SS58Format = SS58_FORMAT_TREVO_ASSET_HUB
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
        NotAnAddressError: If the input is a raw `0x` account id (use
            `decode_account_id`).
        Ss58Error: If the address is not a valid SS58 string of a 32-byte account
            id; its ``kind`` carries the failure classification
            (``Ss58FormatMismatchError`` if the address is valid but uses a different
            format than expected).
        ValueError: If ss58_format is outside [0, 16383].
    """
    account_id_bytes = _decode_ss58_account_id_bytes(blockchain_address, ss58_format)
    return _classify_account_id(blockchain_address, account_id_bytes)


class DecodedAddressWithFormat(NamedTuple):
    """The result of :func:`decode_address_with_format`: the classification of the
    address and the SS58 format found in it.

    A :class:`typing.NamedTuple`, so the two fields can be accessed by name while
    staying compatible with tuple unpacking (``info, ss58_format = ...``). Mirrors the
    ``(AccountId32, u16)`` pair the Rust ``from_ss58check_with_version`` returns and
    the object the TypeScript ``decodeAddressWithFormat`` returns.
    """

    info: BlockchainAccountInfo
    ss58_format: SS58Format


def decode_address_with_format(blockchain_address: BlockchainAddress) -> DecodedAddressWithFormat:
    """
    Decode an SS58 blockchain address of *any* (non-reserved) format and classify it.

    The counterpart of `decode_address` for addresses whose SS58 format is not known up
    front: instead of requiring an expected format, it returns the format found in the
    address alongside the classification. Mirrors
    `AccountId32::from_ss58check_with_version` of the Rust implementation (the
    TypeScript implementation exposes the same entry point as
    `decodeAddressWithFormat`).

    The address is validated exactly like `decode_address` (canonical structure,
    length, checksum; raw `0x` account ids are rejected outright) — only the
    expected-format comparison is skipped, so `Ss58FormatMismatchError` is never
    raised. Prefer `decode_address` whenever the expected format is known: accepting
    any format silently admits addresses of other networks.

    Args:
        blockchain_address (str): SS58 address.

    Returns:
        DecodedAddressWithFormat: the classification of the address and the SS58 format
            found in it (a named tuple; unpacks as ``info, ss58_format``).

    Raises:
        TypeError: If blockchain_address is not a str.
        NotAnAddressError: If the input is a raw `0x` account id (use
            `decode_account_id`).
        Ss58Error: If the address is not a valid SS58 string of a 32-byte account id;
            its ``kind`` carries the failure classification.
    """
    _require_str(blockchain_address, "blockchain_address")

    if blockchain_address.startswith("0x"):
        # Raw account ids are deliberately not accepted here: they carry no SS58 format
        # for this function to report. decode_account_id is the entry point for raw
        # account ids.
        raise NotAnAddressError

    account_id_bytes, found_format = ss58_decode(blockchain_address)
    return DecodedAddressWithFormat(
        _classify_account_id(blockchain_address, account_id_bytes),
        found_format,
    )


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
        ParseAccountIdError: If the input is not `0x` followed by exactly 64 hex
            characters.
    """
    _require_str(account_id, "account_id")
    if _HEX_ACCOUNT_ID.fullmatch(account_id) is None:
        raise ParseAccountIdError
    return _classify_account_id(None, bytes.fromhex(account_id[2:]))


def encode_address(
    account_id: BlockchainAccountId, *, ss58_format: SS58Format = SS58_FORMAT_TREVO_ASSET_HUB
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
        ss58_format (SS58Format, optional): SS58 format (network prefix) for the
            address. Keyword-only; defaults to SS58_FORMAT_TREVO_ASSET_HUB (5335).

    Returns:
        str: The SS58 address.

    Raises:
        TypeError: If account_id is not a str, or ss58_format is not an int.
        ParseAccountIdError: If the input is not `0x` followed by exactly 64 hex
            characters.
        ValueError: If ss58_format is outside [0, 16383], or is reserved (46, 47).
    """
    # ss58_format first, then the payload - the same order as the decode path. The format is
    # a caller *parameter* (a programmer error) while the account id is *data* (frequently
    # untrusted), and a caller who got both wrong should hear about their own bug first.
    # Validating the payload first meant encode_address and decode_address disagreed about
    # which of two bad arguments to report.
    _validate_ss58_format_for_encoding(ss58_format)
    _require_str(account_id, "account_id")
    if _HEX_ACCOUNT_ID.fullmatch(account_id) is None:
        raise ParseAccountIdError
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

    if account_type_byte == APP_AGENT_ACCOUNT_IDENTIFIER:
        # Verify checksum
        open_part_size = 5
        open_part = account_id_bytes[:open_part_size]
        checksum = account_id_bytes[open_part_size:]
        checksum_calculated = _blake2_256(open_part)[open_part_size:]
        if checksum == checksum_calculated:
            # Extract and decode app_agent_id
            app_agent_id_bytes = account_id_bytes[:4]
            app_agent_id = AppAgentId(int.from_bytes(app_agent_id_bytes, byteorder="little"))

            return AppAgentAccountInfo(
                address=blockchain_address,
                account_id=account_id,
                app_agent_id=app_agent_id,
            )

    if account_type_byte == TRANSACTIONAL_ACCOUNT_IDENTIFIER:
        # Verify checksum
        open_part_size = 9
        open_part = account_id_bytes[:open_part_size]
        checksum = account_id_bytes[open_part_size:]
        checksum_calculated = _blake2_256(open_part)[open_part_size:]
        if checksum == checksum_calculated:
            # Extract and decode app_agent_id and ta_id
            app_agent_id_bytes = account_id_bytes[:4]
            ta_id_bytes = account_id_bytes[5:9]
            app_agent_id = AppAgentId(int.from_bytes(app_agent_id_bytes, byteorder="little"))
            ta_id = TransactionalId(int.from_bytes(ta_id_bytes, byteorder="little"))

            return TransactionalAccountInfo(
                address=blockchain_address,
                account_id=account_id,
                app_agent_id=app_agent_id,
                ta_id=ta_id,
            )

    if account_type_byte == NAMED_ACCOUNT_IDENTIFIER:
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
            app_agent_id = AppAgentId(int.from_bytes(app_agent_id_bytes, byteorder="little"))
            account_name = _decode_account_name(name_bytes)

            if account_name is not None:
                return NamedAccountInfo(
                    address=blockchain_address,
                    account_id=account_id,
                    app_agent_id=app_agent_id,
                    account_name=account_name,
                )

    return RegularAccountInfo(
        address=blockchain_address,
        account_id=account_id,
    )


def is_keyless_address(
    blockchain_address: BlockchainAddress, *, ss58_format: SS58Format = SS58_FORMAT_TREVO_ASSET_HUB
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
        TypeError: If blockchain_address is not a str, or ss58_format is not an int.
        NotAnAddressError: If the input is a raw `0x` account id (use
            `is_keyless_account_id`).
        ValueError: If the address cannot be decoded at all.
    """
    return decode_address(blockchain_address, ss58_format=ss58_format).account_type is not AccountType.Regular


def is_keyless_account_id(account_id: BlockchainAccountId) -> bool:
    """
    Returns True if the raw account id passes the keyless checksum verification (AppAgent,
    Transactional, or Named), False for a Regular account id - including a Named layout
    whose checksum is valid but whose name bytes fail charset validation.

    The account-id counterpart of `is_keyless_address`: it classifies the raw `0x` + 64
    hex account id form (which carries no SS58 format) instead of an SS58 address, and is
    total over well-formed account ids. Mirrors `is_keyless_account` of the Rust
    implementation.

    Args:
        account_id (str): The account id: `0x` followed by exactly 64 hex characters.

    Returns:
        bool: True if the account id is keyless.

    Raises:
        TypeError: If account_id is not a str.
        ParseAccountIdError: If the input is not `0x` followed by exactly 64 hex
            characters.
    """
    return decode_account_id(account_id).account_type is not AccountType.Regular
