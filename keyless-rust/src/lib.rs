//! This module provides functionality for encoding and decoding keyless addresses.
//!
//! A keyless address is a type of address that doesn't rely on cryptographic key pairs for identification.
//! Instead, it is derived from a combination of identifiers and checksums, making it suitable and
//! convenient for off-chain applications to use.
//!
//! # Examples
//!
//! ```rust
//! use trevo_keyless::*;
//!
//! // Encode an AppAgent keyless address
//! let app_agent_id = 123;
//! let app_agent_address = encode_app_agent_account(&app_agent_id);
//!
//! // Decode the keyless address
//! let decoded_app_agent_id = decode_app_agent_account(&app_agent_address).unwrap();
//! assert_eq!(decoded_app_agent_id, app_agent_id);
//! ```
//!
//! # Keyless Address Structure
//!
//! A keyless address consists of two parts:
//!
//! - Open Part: Contains specific identifiers or data relevant to the address type.
//!
//! - Checksum: A hash-based checksum to ensure address integrity.
//!
//! Each keyless address belongs to one of the address types. It can be one of the following:
//!
//!   - `APP_AGENT_ADDRESS_IDENTIFIER`: Identifies an AppAgent keyless address.
//!   - `TRANSACTIONAL_ADDRESS_IDENTIFIER`: Identifies a Transactional keyless address.
//!   - `NAMED_ADDRESS_IDENTIFIER`: Identifies a Named keyless address.
//!
//! Each address type has a different structure for the open part, as described below:
//!
//! - **AppAgent Address**:
//!   - [0..=3] -> AppAgentId
//!   - [4..4] -> Address type identifier `APP_AGENT_ADDRESS_IDENTIFIER`
//!   - [5..=31] -> Checksum
//!
//! - **Transactional Address**:
//!   - [0..=3] -> AppAgentId
//!   - [4..4] -> Address type identifier `TRANSACTIONAL_ADDRESS_IDENTIFIER`
//!   - [5..=8] -> TransactionalId
//!   - [9..=31] -> Checksum
//!
//! - **Named Address**:
//!   - [0..=3] -> AppAgentId
//!   - [4..4] -> Address type identifier `NAMED_ADDRESS_IDENTIFIER`
//!   - [5..=14] -> Address name
//!   - [15..=31] -> Checksum
//!
//! # Public Interface
//!
//! This module exposes functions to encode, decode, and manipulate keyless addresses:
//!
//! - `encode_app_agent_account`: Builds a keyless address for the specified `AppAgentId`.
//! - `decode_app_agent_account`: Decodes the provided keyless address and retrieves the corresponding `AppAgentId`.
//! - `encode_transactional_account`: Builds a keyless address for the specified `AppAgentId` and `TransactionalId`.
//! - `decode_transactional_account`: Decodes the provided keyless address and retrieves the `AppAgentId` and `TransactionalId`.
//! - `encode_named_account`: Builds a keyless address for the specified `AppAgentId` and `Name`.
//! - `decode_named_account`: Decodes the provided keyless address and retrieves the `AppAgentId` and `AddressName`.
//! - `decode_account`: Decodes any type of account, including regular accounts, and provides full information about the account.
//!
//! # Errors
//!
//! Errors can occur during the encoding or decoding process, such as invalid input or checksum failure.
//! In such cases, functions return a `Result` containing an error message.
//!
//! # Safety
//!
//! This module assumes that the provided data types are used correctly and safely.
//! It's essential to ensure that the input values are valid and within the expected ranges.
//!
#![cfg_attr(not(feature = "std"), no_std)]
mod keyless;
mod types;

mod tests_binary;
mod tests_encoding;

pub use keyless::*;
pub use types::*;
