//! This module provides functionality for encoding and decoding keyless accounts.
//!
//! A keyless account is a type of account that doesn't rely on cryptographic key pairs for identification.
//! Instead, it is derived from a combination of identifiers and checksums, making it suitable and
//! convenient for off-chain applications to use.
//!
//! # Examples
//!
//! ```rust
//! use trevo_keyless::*;
//!
//! // Encode an AppAgent keyless account
//! let app_agent_id = 123;
//! let app_agent_account = encode_app_agent_account(&app_agent_id);
//!
//! // Decode the keyless account
//! let decoded_app_agent_id = decode_app_agent_account(app_agent_account).unwrap();
//! assert_eq!(decoded_app_agent_id, app_agent_id);
//! ```
//!
//! # Keyless Account Structure
//!
//! A keyless account consists of two parts:
//!
//! - Open Part: Contains specific identifiers or data relevant to the account type.
//!
//! - Checksum: A hash-based checksum to ensure account integrity.
//!
//! Each keyless account belongs to one of the account types. It can be one of the following:
//!
//!   - `APP_AGENT_ACCOUNT_IDENTIFIER`: Identifies an AppAgent keyless account.
//!   - `TRANSACTIONAL_ACCOUNT_IDENTIFIER`: Identifies a Transactional keyless account.
//!   - `NAMED_ACCOUNT_IDENTIFIER`: Identifies a Named keyless account.
//!
//! Each account type has a different structure for the open part, as described below:
//!
//! - **AppAgent Account**:
//!   - [0..=3] -> AppAgentId
//!   - [4..4] -> Account type identifier `APP_AGENT_ACCOUNT_IDENTIFIER`
//!   - [5..=31] -> Checksum
//!
//! - **Transactional Account**:
//!   - [0..=3] -> AppAgentId
//!   - [4..4] -> Account type identifier `TRANSACTIONAL_ACCOUNT_IDENTIFIER`
//!   - [5..=8] -> TransactionalId
//!   - [9..=31] -> Checksum
//!
//! - **Named Account**:
//!   - [0..=3] -> AppAgentId
//!   - [4..4] -> Account type identifier `NAMED_ACCOUNT_IDENTIFIER`
//!   - [5..=14] -> Account name
//!   - [15..=31] -> Checksum
//!
//! # Public Interface
//!
//! This module exposes functions to encode, decode, and manipulate keyless accounts:
//!
//! - `encode_app_agent_account`: Builds a keyless account for the specified `AppAgentId`.
//! - `decode_app_agent_account`: Decodes the provided keyless account and retrieves the corresponding `AppAgentId`.
//! - `encode_transactional_account`: Builds a keyless account for the specified `AppAgentId` and `TransactionalId`.
//! - `decode_transactional_account`: Decodes the provided keyless account and retrieves the `AppAgentId` and `TransactionalId`.
//! - `encode_named_account`: Builds a keyless account for the specified `AppAgentId` and `Name`.
//! - `decode_named_account`: Decodes the provided keyless account and retrieves the `AppAgentId` and `AccountName`.
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
