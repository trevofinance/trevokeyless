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
//! [security model](https://github.com/trevo-finance/trevokeyless#security-model) for the
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
//! - [`decode_transactional_account`] — decode and return
//!   [`TransactionalAccountIdentifiers`].
//! - [`decode_named_account`] — decode and return [`NamedAccountIdentifiers`].
//! - [`is_keyless_account`] — returns `true` if the account passes keyless checksum verification
//!   *and*, for a Named account, its name bytes pass charset validation (performs up to one
//!   Blake2b-256 hash internally).
//! - [`AccountId32`] renders as, and parses from, the raw interchange form `0x` + 64 hex
//!   characters (`Display` / `FromStr`, the latter returning [`ParseAccountIdError`] on
//!   any deviation) — the same strings the Python `decode_account_id` and TypeScript
//!   `decodeAccountId` entry points accept.
//! - Layout constants, shared by name with the Python and TypeScript libraries:
//!   [`ACCOUNT_ID_LENGTH`] (32), [`ACCOUNT_NAME_LENGTH`] (10), and the byte-`[4]`
//!   discriminants [`APP_AGENT_ACCOUNT_IDENTIFIER`], [`TRANSACTIONAL_ACCOUNT_IDENTIFIER`],
//!   [`NAMED_ACCOUNT_IDENTIFIER`].
//!
//! ## Cargo features
//!
//! - `std` (default) — enables the `std` features of the dependencies. Disable for
//!   `no_std` builds. The error types implement [`core::error::Error`] unconditionally
//!   (`std::error::Error` is the same trait), so error handling works identically with
//!   and without this feature.
//! - `ss58` — SS58 address encoding/decoding for [`AccountId32`]
//!   (`to_ss58check*` / `from_ss58check*`, [`SS58_FORMAT_TREVO_ASSET_HUB`]). Works in
//!   `no_std` environments but requires an allocator.
//! - `scale` — SCALE codec and metadata support: `Encode`/`Decode`/`DecodeWithMemTracking`/
//!   `TypeInfo`/`MaxEncodedLen` implementations for every public type, for direct use inside
//!   a Substrate runtime. [`BlockchainAccountInfo`] SCALE-encodes only its `AccountId32` and
//!   derives its classification and identifiers during decode, preserving its invariant.
//! - `serde` — serde support (implies `ss58`): [`AccountId32`] follows the serializer's
//!   `is_human_readable()` — an SS58 address string in the Trevo format into human-readable
//!   formats (JSON, YAML, TOML, ...), the raw 32 account-id bytes into binary formats
//!   (bincode, postcard, MessagePack, ...). [`AccountName`] (de)serializes as a string, and
//!   the remaining public types via derived implementations. Deserializing
//!   [`BlockchainAccountInfo`] validates that the embedded identifiers match `account_id`;
//!   its serialized shape is the Rust enum representation, which differs from the info
//!   objects of the Python and TypeScript libraries (see the type documentation).
//!   The `ss58` implication is permanent: the human-readable representation is *defined*
//!   as the SS58 address, and Cargo unifies features across a dependency graph — a
//!   `serde` without `ss58` falling back to another text form would let one crate's
//!   `ss58` choice silently change every other consumer's JSON wire format.
//! - `test-accounts` — fixed-width `AccountId64` / `AccountId128` byte containers for
//!   Substrate runtime tests that instantiate `frame_system::Config::AccountId` at widths
//!   other than 32 bytes. A keyless account is carried in the leading 32 bytes, right-padded
//!   with zeros; combine with `scale` (and `serde`) for the codec/metadata trait impls such a
//!   runtime requires. No SS58, no cross-language counterpart. Off by default.
//!
//! # Examples
//!
//! ```rust
//! use trevokeyless::*;
//!
//! // Encode an AppAgent root account
//! let app_agent_id = AppAgentId::new(123);
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
// Enabled by the `--cfg docsrs` rustdoc arg that [package.metadata.docs.rs] passes, so the
// feature-gated items below are labelled with the feature that provides them. Nightly-only,
// and inert in ordinary builds.
#![cfg_attr(docsrs, feature(doc_cfg))]

// `serde` is listed alongside `ss58` even though it implies it: the serde impls in `types.rs`
// and `test_accounts.rs` use `alloc::string::String` directly, so gating `alloc` on `ss58`
// alone made those modules compile only by way of that implication — one Cargo.toml edit away
// from an obscure no_std build failure.
#[cfg(any(feature = "ss58", feature = "serde"))]
extern crate alloc;

mod keyless;
#[cfg(feature = "ss58")]
mod ss58;
#[cfg(feature = "test-accounts")]
mod test_accounts;
mod types;

// The test suite requires `std` (`format!`, `Vec`, dev-dependencies); without it only the
// doc tests run. The individual test modules live in `src/tests/`; the additional
// per-feature gates (`serde`, `ss58`, `scale`) are applied there.
#[cfg(all(test, feature = "std"))]
mod tests;

pub use keyless::*;
#[cfg(feature = "ss58")]
#[cfg_attr(docsrs, doc(cfg(feature = "ss58")))]
pub use ss58::*;
#[cfg(feature = "test-accounts")]
#[cfg_attr(docsrs, doc(cfg(feature = "test-accounts")))]
pub use test_accounts::*;
pub use types::*;

// Compiles and runs the README example as a doctest, so the README cannot drift from the
// API. The example exercises `to_ss58check`, so it is only collected when the `ss58`
// feature is enabled (`cargo test --no-default-features` must keep passing).
#[cfg(all(doctest, feature = "ss58"))]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;
