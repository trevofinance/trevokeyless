//! Encoding, decoding, and classification of Trevo keyless accounts.
//!
//! ## What is a keyless account?
//!
//! A **keyless account** is a 32-byte on-chain account identifier whose private key is
//! deliberately unknown. Its bytes are deterministically derived from a set of logical
//! identifiers (see byte layout below), so anyone can compute the address from the identifiers
//! and read back, by inspecting the bytes alone, which App Agent controls the account under
//! the Trevo chain's rules.
//!
//! Because no private key is known, keyless accounts cannot sign transactions. Instead, an App
//! Agent submits **App Transactions** to act on behalf of its keyless accounts.
//!
//! **Classification is not authorization.** The keyless checksum involves no key material and
//! no secret of any kind: *anyone* can construct a valid keyless account embedding *any*
//! AppAgentId, for free. A successful decode therefore proves only that the bytes follow the
//! keyless layout — never that the embedded App Agent created, requested, or authorized the
//! account. Never grant privileges (allow-listing, fee waivers, webhook trust, ...) to an
//! account merely because it decodes to a particular AppAgentId. See the
//! [security model](https://github.com/trevo-finance/trevo-keyless#security-model) for the
//! full contract.
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
//! identifiers); the remaining bytes are a truncated Blake2b-256 checksum of the open part.
//! Decoding reads the open part, recomputes the checksum, and compares it — an account that
//! fails this check is treated as a regular (non-keyless) account. For Named accounts the
//! name bytes must additionally pass charset validation (`0-9 a-z A-Z - #`); an account
//! carrying a valid checksum over out-of-charset name bytes is also treated as regular.
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
//! - [`decode_transactional_account`] — decode and return a [`DecodedTransactionalAccount`].
//! - [`decode_named_account`] — decode and return a [`DecodedNamedAccount`].
//! - [`is_keyless_account`] — returns `true` if the account passes keyless checksum verification
//!   (performs up to one Blake2b-256 hash internally).
//! - [`AccountId32`] renders as, and parses from, the raw interchange form `0x` + 64 hex
//!   characters (`Display` / `FromStr`, the latter returning [`ParseAccountIdError`] on
//!   any deviation) — the same strings the Python `decode_account_id` and TypeScript
//!   `decodeAccountId` entry points accept.
//!
//! ## Cargo features
//!
//! - `std` (default) — implements `std::error::Error` for the error types and enables the
//!   `std` features of the dependencies. Disable for `no_std` builds.
//! - `ss58` — SS58 address encoding/decoding for [`AccountId32`]
//!   (`to_ss58check*` / `from_ss58check*`, [`SS58_FORMAT_TREVO_ASSET_HUB`]). Works in
//!   `no_std` environments but requires an allocator.
//! - `scale` — SCALE codec and metadata support: `Encode`/`Decode`/`DecodeWithMemTracking`/
//!   `TypeInfo`/`MaxEncodedLen` implementations for every public type, for direct use inside
//!   a Substrate runtime. SCALE `Decode` of [`BlockchainAccountInfo`] performs no
//!   consistency validation (see the type documentation).
//! - `serde` — serde support (implies `ss58`): [`AccountId32`] (de)serializes as an SS58
//!   address string in the Trevo format, [`AccountName`] as a string, and the remaining
//!   public types via derived implementations. Deserializing [`BlockchainAccountInfo`]
//!   validates that the embedded identifiers match `account_id`; its serialized shape is the
//!   Rust enum representation, which differs from the info objects of the Python and
//!   TypeScript libraries (see the type documentation).
//!
//! # Examples
//!
//! ```rust
//! use trevokeyless::*;
//!
//! // Encode an AppAgent root account
//! let app_agent_id: AppAgentId = 123;
//! let account = encode_app_agent_account(app_agent_id);
//!
//! // Decode it back
//! let decoded_id = decode_app_agent_account(&account).unwrap();
//! assert_eq!(decoded_id, app_agent_id);
//!
//! // SS58 address encoding (feature `ss58`)
//! # #[cfg(feature = "ss58")]
//! # {
//! let address = account.to_ss58check();
//! assert_eq!(address, "ttowKp8AmQuGfbBGikG2pbdYNnErhHRaLrdktJeZEfJVeVnTp");
//! assert_eq!(AccountId32::from_ss58check(&address).unwrap(), account);
//! # }
//! ```
#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(feature = "ss58")]
extern crate alloc;

mod keyless;
#[cfg(feature = "ss58")]
mod ss58;
mod types;

// The test suite requires `std` (`format!`, `Vec`, dev-dependencies); without it only the
// doc tests run. The individual test modules live in `src/tests/`; the additional
// per-feature gates (`serde`, `ss58`, `scale`) are applied there.
#[cfg(all(test, feature = "std"))]
mod tests;

pub use keyless::*;
#[cfg(feature = "ss58")]
pub use ss58::*;
pub use types::*;

// Compiles and runs the README example as a doctest, so the README cannot drift from the
// API. The example exercises `to_ss58check`, so it is only collected when the `ss58`
// feature is enabled (`cargo test --no-default-features` must keep passing).
#[cfg(all(doctest, feature = "ss58"))]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;
