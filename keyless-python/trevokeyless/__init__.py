# ruff: noqa: I001

# Hardcoded rather than resolved from importlib.metadata: the lookup raises
# PackageNotFoundError when the package is imported from a source tree, a vendored
# copy, or a frozen build without installed dist-info. tests/test_version.py pins this
# string to the pyproject.toml version so the two cannot drift.
__version__ = "1.0.0"

# The SS58 error types and the address length bound live in trevokeyless._ss58 (the
# module that raises/applies them); they are deliberately part of the public API,
# unlike the codec helpers of that module.
from trevokeyless._ss58 import (
    SS58_MAX_ADDRESS_LENGTH,
    Ss58Error,
    Ss58ErrorKind,
)
from trevokeyless.keyless import (
    ACCOUNT_ID_LENGTH,
    NAMED_ADDRESS_LENGTH,
    SS58_FORMAT_TREVO_ASSET_HUB,
    U32_MAX,
    BlockchainAddress,
    BlockchainAccountId,
    AccountType,
    AccountTypeMismatchError,
    Ss58FormatMismatchError,
    AppAgentId,
    TransactionalId,
    AccountName,
    SS58Format,
    BlockchainAccountInfo,
    DecodedTransactionalAddress,
    DecodedNamedAddress,
    encode_app_agent_address,
    decode_app_agent_address,
    encode_transactional_address,
    decode_transactional_address,
    encode_named_address,
    decode_named_address,
    decode_address,
    decode_account_id,
    encode_address,
    is_keyless_address,
    encode_app_agent_account_id,
    encode_transactional_account_id,
    encode_named_account_id,
    is_keyless_account_id,
)

__all__ = [
    "ACCOUNT_ID_LENGTH",
    "NAMED_ADDRESS_LENGTH",
    "SS58_FORMAT_TREVO_ASSET_HUB",
    "SS58_MAX_ADDRESS_LENGTH",
    "U32_MAX",
    "AccountName",
    "AccountType",
    "AccountTypeMismatchError",
    "AppAgentId",
    "BlockchainAccountId",
    "BlockchainAccountInfo",
    "BlockchainAddress",
    "DecodedNamedAddress",
    "DecodedTransactionalAddress",
    "SS58Format",
    "Ss58Error",
    "Ss58ErrorKind",
    "Ss58FormatMismatchError",
    "TransactionalId",
    "__version__",
    "decode_account_id",
    "decode_address",
    "decode_app_agent_address",
    "decode_named_address",
    "decode_transactional_address",
    "encode_address",
    "encode_app_agent_account_id",
    "encode_app_agent_address",
    "encode_named_account_id",
    "encode_named_address",
    "encode_transactional_account_id",
    "encode_transactional_address",
    "is_keyless_account_id",
    "is_keyless_address",
]
