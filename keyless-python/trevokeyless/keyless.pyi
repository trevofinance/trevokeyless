from dataclasses import dataclass
from enum import Enum
from typing import TypeAlias

from .ss58 import ss58_decode as ss58_decode
from .ss58 import ss58_encode as ss58_encode

NAMED_ADDRESS_LENGTH: int
SS58_FORMAT__TREVO_ASSET_HUB: int
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
    def __init__(
        self,
        address: BlockchainAddress,
        account_id: BlockchainAccountId,
        address_type: AddressType,
        app_agent_id: AppAgentId | None,
        ta_id: TransactionalAddressId | None,
        address_name: AddressName | None,
    ) -> None: ...

def encode_app_agent_address(app_agent_id: AppAgentId, ss58_format: SS58Format = ...) -> BlockchainAddress: ...
def decode_app_agent_address(encoded_address: BlockchainAddress, ss58_format: SS58Format = ...) -> AppAgentId: ...
def encode_transactional_address(
    app_agent_id: AppAgentId, ta_id: TransactionalAddressId, ss58_format: SS58Format = ...
) -> BlockchainAddress: ...
def decode_transactional_address(
    encoded_address: BlockchainAddress, ss58_format: SS58Format = ...
) -> tuple[AppAgentId, TransactionalAddressId]: ...
def encode_named_address(
    app_agent_id: AppAgentId, name: AddressName, ss58_format: SS58Format = ...
) -> BlockchainAddress: ...
def decode_named_address(
    encoded_address: BlockchainAddress, ss58_format: SS58Format = ...
) -> tuple[AppAgentId, AddressName]: ...
def decode_address(blockchain_address: BlockchainAddress, ss58_format: SS58Format = ...) -> BlockchainAddressInfo: ...
