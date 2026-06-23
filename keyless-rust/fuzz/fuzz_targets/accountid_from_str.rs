//! Fuzzes the `AccountId32` hex parser (`FromStr`), the crate's other parser of untrusted
//! variable-length input: it must never panic on arbitrary strings, and any string it
//! accepts must round-trip through `Display` back to the same account id.
//!
//! `Display` always renders the canonical lowercase `0x` + 64-hex form, so a re-parse of
//! its output must reproduce the original account exactly.
#![no_main]

use libfuzzer_sys::fuzz_target;
use trevokeyless::AccountId32;

fuzz_target!(|data: &str| {
    if let Ok(account) = data.parse::<AccountId32>() {
        let rendered = account.to_string();
        let reparsed: AccountId32 = rendered.parse().expect("Display output must re-parse");
        assert_eq!(reparsed, account);
    }
});
