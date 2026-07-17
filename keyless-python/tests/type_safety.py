"""Compile-only assertions for the public identifier types.

This module is checked by mypy but deliberately does not match unittest's ``test_*``
discovery pattern. The ignored calls must remain type errors: strict mypy enables
``warn_unused_ignores``, so weakening either identifier type makes CI fail.
"""

from trevokeyless import (
    AppAgentId,
    TransactionalId,
    decode_app_agent_account_id,
    decode_transactional_account_id,
    encode_app_agent_account_id,
    encode_transactional_account_id,
)


def identifier_types_are_not_interchangeable() -> None:
    app_agent_id = AppAgentId(1)
    transactional_id = TransactionalId(2)

    encode_app_agent_account_id(app_agent_id)
    encode_transactional_account_id(app_agent_id=app_agent_id, ta_id=transactional_id)

    encode_app_agent_account_id(transactional_id)  # type: ignore[arg-type]
    encode_transactional_account_id(
        app_agent_id=transactional_id,  # type: ignore[arg-type]
        ta_id=app_agent_id,  # type: ignore[arg-type]
    )

    # A plain int is not an identifier either: the nominal types must be constructed
    # (which is where the u32 range is enforced), not implied.
    encode_app_agent_account_id(1)  # type: ignore[arg-type]


def identifier_types_are_usable_as_ints() -> None:
    # The identifiers are int subclasses, so they stay usable in integer contexts.
    app_agent_id = AppAgentId(1)
    plain: int = app_agent_id
    _ = plain + 1
    _ = app_agent_id.to_bytes(4, byteorder="little")


def decoders_return_the_identifier_types() -> None:
    account_id = encode_app_agent_account_id(AppAgentId(1))
    app_agent_id: AppAgentId = decode_app_agent_account_id(account_id)
    encode_app_agent_account_id(app_agent_id)

    transactional_account_id = encode_transactional_account_id(app_agent_id=AppAgentId(1), ta_id=TransactionalId(2))
    decoded = decode_transactional_account_id(transactional_account_id)
    ta_id: TransactionalId = decoded.ta_id
    encode_transactional_account_id(app_agent_id=decoded.app_agent_id, ta_id=ta_id)
