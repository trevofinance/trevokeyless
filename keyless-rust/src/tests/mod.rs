//! Test suite for the crate.
//!
//! The whole module is compiled only under `#[cfg(all(test, feature = "std"))]`, applied to
//! the `mod tests;` declaration in `lib.rs` (the suite needs `std` for `format!`, `Vec`, and
//! the dev-dependencies; without it only the doc tests run). That shared gate is therefore
//! not repeated on the submodules below — only the additional per-feature gates are.

mod tests_binary;
mod tests_conformance;
mod tests_encoding;
mod tests_properties;
mod types;

#[cfg(feature = "serde")]
mod tests_serde;
#[cfg(feature = "ss58")]
mod tests_ss58;
