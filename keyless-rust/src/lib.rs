//! Encoding, decoding, and classification of Trevo keyless accounts.
//!
//! ## What is a keyless account?
//!
//! A **keyless account** is a 32-byte on-chain account identifier whose private key is
//! deliberately unknown. Its bytes are deterministically derived from a set of logical
//! identifiers (see byte layout below), so anyone can compute the address from the identifiers
//! and verify that an address belongs to a specific App Agent simply by reading its bytes.
//!
//! Because no private key exists, keyless accounts cannot sign transactions. Instead, an App
//! Agent submits **Clearing Transactions** to act on behalf of its keyless accounts.
//!
//! There are three keyless account types:
//!
//! | Type | Constant | Controlled by |
//! |------|----------|---------------|
//! | App Agent root | `APP_AGENT_ACCOUNT_IDENTIFIER` (1) | the App Agent |
//! | Transactional  | `TRANSACTIONAL_ACCOUNT_IDENTIFIER` (2) | the App Agent |
//! | Named          | `NAMED_ACCOUNT_IDENTIFIER` (3) | the App Agent |
//!
//! ## Byte layout
//!
//! Every keyless account is 32 bytes. The first N bytes form the **open part** (plain-text
//! identifiers); the remaining bytes are a truncated Blake2-256 checksum of the open part.
//! Decoding reads the open part, recomputes the checksum, and compares it — an account that
//! fails this check is treated as a regular (non-keyless) account.
//!
//! ```text
//! AppAgent Account (open part: 5 bytes, checksum: 27 bytes):
//!   [0..=3]  AppAgentId (little-endian u32)
//!   [4]      Account type identifier = 1
//!   [5..=31] blake2_256(open_part)[5..=31]
//!
//! Transactional Account (open part: 9 bytes, checksum: 23 bytes):
//!   [0..=3]  AppAgentId (little-endian u32)
//!   [4]      Account type identifier = 2
//!   [5..=8]  TransactionalId (little-endian u32)
//!   [9..=31] blake2_256(open_part)[9..=31]
//!
//! Named Account (open part: 15 bytes, checksum: 17 bytes):
//!   [0..=3]  AppAgentId (little-endian u32)
//!   [4]      Account type identifier = 3
//!   [5..=14] AccountName (10 bytes, restricted charset: 0-9 a-z A-Z - #)
//!   [15..=31] blake2_256(open_part)[15..=31]
//! ```
//!
//! ## Public interface
//!
//! - [`encode_app_agent_account`] — derive the root account for an App Agent.
//! - [`encode_transactional_account`] — derive a transactional sub-account.
//! - [`encode_named_account`] — derive a named sub-account.
//! - [`decode_account_info`] — classify any account and extract its embedded identifiers.
//! - [`decode_app_agent_account`] — decode and return only the `AppAgentId` (errors if not an
//!   App Agent account).
//! - [`decode_transactional_account`] — decode and return `(AppAgentId, TransactionalId)`.
//! - [`decode_named_account`] — decode and return `(AppAgentId, AccountName)`.
//! - [`is_keyless_account`] — quick check for whether an account follows keyless encoding.
//!
//! # Examples
//!
//! ```rust
//! use trevo_keyless::*;
//!
//! // Encode an AppAgent root account
//! let app_agent_id: AppAgentId = 123;
//! let account = encode_app_agent_account(&app_agent_id);
//!
//! // Decode it back
//! let decoded_id = decode_app_agent_account(account).unwrap();
//! assert_eq!(decoded_id, app_agent_id);
//! ```
#![cfg_attr(not(feature = "std"), no_std)]
mod keyless;
mod types;

mod tests_binary;
mod tests_encoding;

pub use keyless::*;
pub use types::*;
