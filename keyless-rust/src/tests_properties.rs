//! Property-style randomized tests over the encode/decode round trips and the classification
//! of arbitrary byte patterns. A deterministic PRNG (fixed seeds, xorshift64*) is used instead
//! of a property-testing dependency, so failures are exactly reproducible and the crate keeps
//! its minimal dependency footprint.
//!
//! The PRNG construction and seeds are shared verbatim with the Python and TypeScript suites,
//! so the three exercise the *same* generated cases and together act as a cross-language
//! differential fuzzer (a reference-sequence test below pins the stream). Keep the construction
//! byte-for-byte identical across the three languages when refactoring — otherwise they
//! silently stop testing the same inputs.

#[cfg(test)]
mod tests {
    use crate::keyless::*;
    use crate::types::*;

    /// Deterministic xorshift64* PRNG. Seeds must be non-zero.
    struct Rng(u64);

    impl Rng {
        fn next_u64(&mut self) -> u64 {
            let mut x = self.0;
            x ^= x >> 12;
            x ^= x << 25;
            x ^= x >> 27;
            self.0 = x;
            x.wrapping_mul(0x2545_F491_4F6C_DD1D)
        }

        fn next_u32(&mut self) -> u32 {
            (self.next_u64() >> 32) as u32
        }

        fn fill(&mut self, buf: &mut [u8]) {
            for chunk in buf.chunks_mut(8) {
                let bytes = self.next_u64().to_le_bytes();
                chunk.copy_from_slice(&bytes[..chunk.len()]);
            }
        }
    }

    /// The 64 characters allowed in an [`AccountName`].
    const CHARSET: &[u8; 64] = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ-#";

    fn random_name(rng: &mut Rng) -> AccountName {
        let mut bytes = [0u8; 10];
        for byte in &mut bytes {
            *byte = CHARSET[(rng.next_u64() % 64) as usize];
        }
        AccountName::new(&bytes).expect("generated from the allowed charset")
    }

    #[test]
    fn prop_app_agent_roundtrip() {
        let mut rng = Rng(0x5EED_0001);
        let ids = (0..1000).map(|_| rng.next_u32()).chain([0, 1, u32::MAX]);
        for app_agent_id in ids {
            let account = encode_app_agent_account(app_agent_id);
            assert_eq!(
                decode_account_info(&account),
                BlockchainAccountInfo::AppAgent {
                    account_id: account,
                    app_agent_id
                }
            );
            assert_eq!(decode_app_agent_account(&account), Ok(app_agent_id));
            assert_eq!(
                decode_transactional_account(&account),
                Err(KeylessDecodeError::WrongAccountType {
                    expected: AccountType::Transactional,
                    found: AccountType::AppAgent
                })
            );
            assert_eq!(
                decode_named_account(&account),
                Err(KeylessDecodeError::WrongAccountType {
                    expected: AccountType::Named,
                    found: AccountType::AppAgent
                })
            );
            assert!(is_keyless_account(&account));
        }
    }

    #[test]
    fn prop_transactional_roundtrip() {
        let mut rng = Rng(0x5EED_0002);
        let ids = (0..1000).map(|_| (rng.next_u32(), rng.next_u32())).chain([
            (0, 0),
            (0, u32::MAX),
            (u32::MAX, 0),
            (u32::MAX, u32::MAX),
        ]);
        for (app_agent_id, ta_id) in ids {
            let account = encode_transactional_account(app_agent_id, ta_id);
            assert_eq!(
                decode_account_info(&account),
                BlockchainAccountInfo::Transactional {
                    account_id: account,
                    app_agent_id,
                    ta_id
                }
            );
            assert_eq!(
                decode_transactional_account(&account),
                Ok(DecodedTransactionalAccount {
                    app_agent_id,
                    ta_id
                })
            );
            assert_eq!(
                decode_app_agent_account(&account),
                Err(KeylessDecodeError::WrongAccountType {
                    expected: AccountType::AppAgent,
                    found: AccountType::Transactional
                })
            );
            assert!(is_keyless_account(&account));
        }
    }

    #[test]
    fn prop_named_roundtrip() {
        let mut rng = Rng(0x5EED_0003);
        for _ in 0..1000 {
            let app_agent_id = rng.next_u32();
            let name = random_name(&mut rng);
            let account = encode_named_account(app_agent_id, name);
            assert_eq!(
                decode_account_info(&account),
                BlockchainAccountInfo::Named {
                    account_id: account,
                    app_agent_id,
                    account_name: name
                }
            );
            assert_eq!(
                decode_named_account(&account),
                Ok(DecodedNamedAccount {
                    app_agent_id,
                    account_name: name
                })
            );
            assert_eq!(
                decode_app_agent_account(&account),
                Err(KeylessDecodeError::WrongAccountType {
                    expected: AccountType::AppAgent,
                    found: AccountType::Named
                })
            );
            assert!(is_keyless_account(&account));
        }
    }

    #[test]
    fn prop_decode_arbitrary_bytes_never_panics_and_is_regular() {
        let mut rng = Rng(0x5EED_0004);
        for i in 0..50_000u64 {
            let mut bytes = [0u8; 32];
            rng.fill(&mut bytes);
            // Cycle the discriminant byte through 0..=5 so every match arm of
            // decode_account_info (including the checksum verifications) is exercised.
            bytes[4] = (i % 6) as u8;
            let account = AccountId32::from(bytes);
            assert_eq!(
                decode_account_info(&account),
                BlockchainAccountInfo::Regular {
                    account_id: account
                },
                "random bytes must classify as Regular \
                 (a checksum match has probability ~2^-136 at best)"
            );
        }
    }

    #[test]
    fn prop_single_byte_corruption_demotes_to_regular() {
        // Corrupting any single byte of a keyless account — open part or checksum — must
        // invalidate it: either the stored checksum no longer matches the open part, or the
        // recomputed checksum no longer matches the stored bytes.
        let mut rng = Rng(0x5EED_0005);
        let accounts = [
            encode_app_agent_account(rng.next_u32()),
            encode_transactional_account(rng.next_u32(), rng.next_u32()),
            encode_named_account(rng.next_u32(), random_name(&mut rng)),
        ];
        for account in accounts {
            let original: [u8; 32] = account.into();
            for position in 0..32 {
                let xor = (rng.next_u64() % 255) as u8 + 1; // non-zero
                let mut corrupted = original;
                corrupted[position] ^= xor;
                let corrupted = AccountId32::from(corrupted);
                assert_eq!(
                    decode_account_info(&corrupted),
                    BlockchainAccountInfo::Regular {
                        account_id: corrupted
                    },
                    "corrupting byte {position} of {account} must demote it to Regular"
                );
            }
        }
    }

    #[test]
    fn prop_account_name_construction() {
        let mut rng = Rng(0x5EED_0006);
        for _ in 0..500 {
            let name = random_name(&mut rng);
            assert_eq!(AccountName::new(name.as_str().as_bytes()).unwrap(), name);
            assert_eq!(name.as_str().len(), 10);

            // Replacing any character with an out-of-charset byte must be rejected.
            let mut bad = *name.as_bytes();
            bad[(rng.next_u64() % 10) as usize] = b'@';
            assert_eq!(
                AccountName::new(&bad).unwrap_err(),
                AccountNameError::InvalidCharacter
            );
        }
    }

    #[cfg(feature = "ss58")]
    #[test]
    fn prop_ss58_roundtrip_random_accounts_and_formats() {
        let mut rng = Rng(0x5EED_0007);
        for _ in 0..500 {
            let mut bytes = [0u8; 32];
            rng.fill(&mut bytes);
            let account = AccountId32::from(bytes);
            let mut format = (rng.next_u64() % 16384) as u16;
            if format == 46 || format == 47 {
                format = 48; // skip the reserved formats
            }
            let address = account.to_ss58check_with_version(format).unwrap();
            assert_eq!(
                AccountId32::from_ss58check_with_version(&address).unwrap(),
                (account, format),
                "SS58 round trip must preserve the account and format {format}"
            );
        }
    }
}
