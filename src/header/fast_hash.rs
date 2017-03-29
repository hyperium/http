use std::hash::Hash;

/// A fast hashable type
///
/// Implementations of `FastHash` provie an optimized hash function that offers
/// no security guarantees and an accaptable distribution of hash values for
/// simple cases.
pub trait FastHash: Hash {
    fn fast_hash(&self) -> u64;
}
