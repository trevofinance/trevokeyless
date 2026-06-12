//! Fuzzes the SS58 decoder, the only parser of untrusted variable-length input in the
//! crate: it must never panic on arbitrary strings, and every accepted address must
//! round-trip through the encoder.
//!
//! The round trip compares the decoded `(account, format)` pair rather than the address
//! strings: non-canonical two-byte-prefix spellings of formats below 64 decode (like the
//! upstream Substrate codec) but re-encode to the canonical single-byte form.
#![no_main]

use libfuzzer_sys::fuzz_target;
use trevokeyless::AccountId32;

fuzz_target!(|data: &str| {
    if let Ok((account, format)) = AccountId32::from_ss58check_with_version(data) {
        let reencoded = account
            .to_ss58check_with_version(format)
            .expect("decoded formats are always encodable");
        let (account2, format2) = AccountId32::from_ss58check_with_version(&reencoded)
            .expect("re-encoded addresses must decode");
        assert_eq!((account2, format2), (account, format));
    }
});
