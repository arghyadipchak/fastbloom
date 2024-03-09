#![allow(rustdoc::bare_urls)]
#![doc = include_str!("../README.md")]

use std::hash::{BuildHasher, Hash, Hasher};
mod hasher;
pub use hasher::DefaultHasher;
mod builder;
pub use builder::Builder;
mod bit_vector;
use bit_vector::BlockedBitVec;
mod signature;

/// A space efficient approximate membership set data structure.
/// False positives from [`contains`](Self::contains) are possible, but false negatives
/// are not, i.e. [`contains`](Self::contains) for all items in the set is guaranteed to return
/// true, while [`contains`](Self::contains) for all items not in the set probably return false.
///
/// [`BloomFilter`] is supported by an underlying bit vector, chunked into 512, 256, 128, or 64 bit "blocks", to track item membership.
/// To insert, a number of bits are set at positions based on the item's hash in one of the underlying bit vector's block.
/// To check membership, a number of bits are checked at positions based on the item's hash in one of the underlying bit vector's block.
///
/// Once constructed, neither the bloom filter's underlying memory usage nor number of bits per item change.
///
/// # Examples
/// Basic usage:
/// ```rust
/// use fastbloom::BloomFilter;
///
/// let num_bits = 1024;
///
/// let mut filter = BloomFilter::builder(num_bits).expected_items(2);
/// filter.insert("42");
/// filter.insert("🦀");
/// ```
/// Instantiate from a collection of items:
/// ```rust
/// use fastbloom::BloomFilter;
///
/// let num_bits = 1024;
///
/// let filter = BloomFilter::builder(num_bits).items(["42", "🦀"]);
/// assert!(filter.contains("42"));
/// assert!(filter.contains("🦀"));
/// ```
/// Use any hasher:
/// ```rust
/// use fastbloom::BloomFilter;
/// use ahash::RandomState;
///
/// let num_bits = 1024;
///
/// let filter = BloomFilter::builder(num_bits)
///     .hasher(RandomState::default())
///     .items(["42", "🦀"]);
/// ```
#[derive(Debug, Clone)]
pub struct BloomFilter<const BLOCK_SIZE_BITS: usize = 512, S = DefaultHasher> {
    bits: BlockedBitVec<BLOCK_SIZE_BITS>,
    /// The total target hashes per item that is specified by user or optimized to maximize accuracy
    target_hashes: u64,
    /// The target number of bits to set/check per u64 per item when inserting/checking an item.
    num_rounds: Option<u64>,
    /// The number of hashes per item in addition to `num_rounds`. These hashes can be applied across many `u64`s in a block.
    /// These hashes are in addition to `num_rounds` to make up for rounding errors.
    num_hashes: u64,
    hasher: S,
}

impl BloomFilter {
    fn new_builder<const BLOCK_SIZE_BITS: usize>(num_bits: usize) -> Builder<BLOCK_SIZE_BITS> {
        assert!(num_bits > 0);
        let num_blocks = num_bits.div_ceil(BLOCK_SIZE_BITS);
        Builder::<BLOCK_SIZE_BITS> {
            data: BlockedBitVec::<BLOCK_SIZE_BITS>::new(num_blocks).unwrap(),
            hasher: Default::default(),
        }
    }

    fn new_builder_from_vec<const BLOCK_SIZE_BITS: usize>(
        vec: Vec<u64>,
    ) -> Builder<BLOCK_SIZE_BITS> {
        assert!(!vec.is_empty());
        Builder::<BLOCK_SIZE_BITS> {
            data: vec.into(),
            hasher: Default::default(),
        }
    }

    /// Creates a new instance of [`Builder`] to construct a `BloomFilter`
    /// with `num_bits` number of bits for tracking item membership.
    ///
    /// The `BloomFilter` built from the returned builder will have a block size of 512 bits.
    ///
    /// Use either
    /// - [`BloomFilter::<256>::builder_from_bits`]
    /// - [`BloomFilter::<128>::builder_from_bits`]
    /// - [`BloomFilter::<64>::builder_from_bits`]
    ///
    /// for more speed but slightly higher false positive rates.
    ///
    /// # Examples
    ///
    /// ```
    /// use fastbloom::BloomFilter;
    ///
    /// let bloom = BloomFilter::builder(1024).hashes(4);
    /// ```
    pub fn builder(num_bits: usize) -> Builder<512> {
        BloomFilter::<512>::builder_from_bits(num_bits)
    }
}

impl BloomFilter<64, DefaultHasher> {
    /// Creates a new instance of [`Builder`] to construct a `BloomFilter`
    /// with `num_bits` number of bits for tracking item membership.
    ///
    /// The `BloomFilter` built from the returned builder will have a block size of 64 bits.
    ///
    /// `BloomFilter<64>` is faster but less accurate than `BloomFilter<128>`
    ///
    /// # Examples
    ///
    /// ```
    /// use fastbloom::BloomFilter;
    ///
    /// let bloom = BloomFilter::<64>::builder_from_bits(1024).hashes(4);
    /// ```
    pub fn builder_from_bits(num_bits: usize) -> Builder<64> {
        BloomFilter::new_builder::<64>(num_bits)
    }

    /// Creates a new instance of [`Builder`] to construct a `BloomFilter` initialized with bit vector `bit_vec`.
    /// The `BloomFilter` built from the returned builder will have a block size of 64 bits.
    ///
    /// # Examples
    ///
    /// ```
    /// use fastbloom::BloomFilter;
    ///
    /// let bloom = BloomFilter::<64>::builder_from_vec(vec![0x517cc1b727220a95]).hashes(4);
    /// ```
    pub fn builder_from_vec(bit_vec: Vec<u64>) -> Builder<64> {
        BloomFilter::new_builder_from_vec::<64>(bit_vec)
    }
}

impl BloomFilter<128, DefaultHasher> {
    /// Creates a new instance of [`Builder`] to construct a `BloomFilter`
    /// with `num_bits` number of bits for tracking item membership.
    ///
    /// The `BloomFilter` built from the returned builder will have a block size of 128 bits.
    ///
    /// `BloomFilter<128>` is faster but less accurate than `BloomFilter<256>`
    ///
    /// # Examples
    ///
    /// ```
    /// use fastbloom::BloomFilter;
    ///
    /// let bloom = BloomFilter::<128>::builder_from_bits(1024).hashes(4);
    /// ```
    pub fn builder_from_bits(num_bits: usize) -> Builder<128> {
        BloomFilter::new_builder::<128>(num_bits)
    }

    /// Creates a new instance of [`Builder`] to construct a `BloomFilter` initialized with bit vector `bit_vec`.
    /// The `BloomFilter` built from the returned builder will have a block size of 128 bits.
    /// To fit a 128 bit block size, `bit_vec` will be padded with `0u64` to have a length multiple of 2.
    ///
    /// # Examples
    ///
    /// ```
    /// use fastbloom::BloomFilter;
    ///
    /// let bloom = BloomFilter::<128>::builder_from_vec(vec![0x517cc1b727220a95; 2]).hashes(4);
    /// ```
    pub fn builder_from_vec(bit_vec: Vec<u64>) -> Builder<128> {
        BloomFilter::new_builder_from_vec::<128>(bit_vec)
    }
}

impl BloomFilter<256, DefaultHasher> {
    /// Creates a new instance of [`Builder`] to construct a `BloomFilter`
    /// with `num_bits` number of bits for tracking item membership.
    ///
    /// The `BloomFilter` built from the returned builder will have a block size of 256 bits.
    ///
    /// `BloomFilter<256>` is faster but less accurate than `BloomFilter<512>`
    ///
    /// # Examples
    ///
    /// ```
    /// use fastbloom::BloomFilter;
    ///
    /// let bloom = BloomFilter::<256>::builder_from_bits(1024).hashes(4);
    /// ```
    pub fn builder_from_bits(num_bits: usize) -> Builder<256> {
        BloomFilter::new_builder::<256>(num_bits)
    }

    /// Creates a new instance of [`Builder`] to construct a `BloomFilter` initialized with bit vector `bit_vec`.
    /// The `BloomFilter` built from the returned builder will have a block size of 256 bits.
    /// To fit a 256 bit block size, `bit_vec` will be padded with `0u64` to have a length multiple of 4.
    ///
    /// # Examples
    ///
    /// ```
    /// use fastbloom::BloomFilter;
    ///
    /// let bloom = BloomFilter::<256>::builder_from_vec(vec![0x517cc1b727220a95; 4]).hashes(4);
    /// ```
    pub fn builder_from_vec(bit_vec: Vec<u64>) -> Builder<256> {
        BloomFilter::new_builder_from_vec::<256>(bit_vec)
    }
}

impl BloomFilter<512, DefaultHasher> {
    /// Creates a new instance of [`Builder`] to construct a `BloomFilter`
    /// with `num_bits` number of bits for tracking item membership.
    ///
    /// The returned `BloomFilter` has a block size of 512 bits.
    ///
    /// Use either
    /// - [`BloomFilter::<256>::builder_from_bits`]
    /// - [`BloomFilter::<128>::builder_from_bits`]
    /// - [`BloomFilter::<64>::builder_from_bits`]
    ///
    /// for more speed but slightly higher false positive rates.
    ///
    /// # Examples
    ///
    /// ```
    /// use fastbloom::BloomFilter;
    ///
    /// let bloom = BloomFilter::<512>::builder_from_bits(1024).hashes(4);
    /// ```
    pub fn builder_from_bits(num_bits: usize) -> Builder<512> {
        BloomFilter::new_builder::<512>(num_bits)
    }

    /// Creates a new instance of [`Builder`] to construct a `BloomFilter` initialized with bit vector `bit_vec`.
    /// The `BloomFilter` built from the returned builder will have a block size of 512 bits.
    /// To fit a 512 bit block size, `bit_vec` will be padded with `0u64` to have a length multiple of 8.
    ///
    /// # Examples
    ///
    /// ```
    /// use fastbloom::BloomFilter;
    ///
    /// let bloom = BloomFilter::<256>::builder_from_vec(vec![0x517cc1b727220a95; 8]).hashes(4);
    /// ```
    pub fn builder_from_vec(bit_vec: Vec<u64>) -> Builder<512> {
        BloomFilter::new_builder_from_vec::<512>(bit_vec)
    }
}

impl<const BLOCK_SIZE_BITS: usize, S: BuildHasher> BloomFilter<BLOCK_SIZE_BITS, S> {
    /// Used to grab the last N bits from a hash.
    const BIT_INDEX_MASK: u64 = (BLOCK_SIZE_BITS - 1) as u64;

    /// The optimal number of hashes to perform for an item given the expected number of items to be contained in one block.
    /// Proof under "False Positives Analysis": <https://brilliant.org/wiki/bloom-filter/>
    #[inline]
    fn optimal_hashes_f(items_per_block: f64) -> f64 {
        let block_size = BLOCK_SIZE_BITS as f64;

        // `items_per_block` is an average. When block sizes decrease
        // the variance in the actual item per block decrease,
        // meaning we are more likely to have a "crowded" block, with
        // way too many bits set. So we decrease the max hashes
        // to decrease this "crowding" effect.
        // TODO: a more precise formula for this
        let min_hashes_mult = (BLOCK_SIZE_BITS as f64) / (512f64);

        let max_hashes = block_size / 64.0f64 * signature::hashes_for_bits(32) * min_hashes_mult;
        let hashes_per_block = block_size / items_per_block * f64::ln(2.0f64);
        if hashes_per_block > max_hashes {
            max_hashes
        } else {
            hashes_per_block
        }
    }

    fn bit_index(hash1: &mut u64, hash2: u64) -> usize {
        let h = next_hash(hash1, hash2);
        (h & Self::BIT_INDEX_MASK) as usize
    }

    /// Adds a value to the bloom filter.
    ///
    /// # Examples
    /// ```
    /// use fastbloom::BloomFilter;
    ///
    /// let mut bloom = BloomFilter::builder(1024).hashes(4);
    /// bloom.insert(&2);
    /// assert!(bloom.contains(&2));
    /// ```
    #[inline]
    pub fn insert(&mut self, val: &(impl Hash + ?Sized)) {
        let [mut h1, h2] = get_orginal_hashes(&self.hasher, val);
        let block_index = block_index(self.num_blocks(), h1);
        let block = &mut self.bits.get_block_mut(block_index);
        for _ in 0..self.num_hashes {
            BlockedBitVec::<BLOCK_SIZE_BITS>::set_for_block(block, Self::bit_index(&mut h1, h2));
        }
        if let Some(num_rounds) = self.num_rounds {
            for i in 0..self.bits.get_block(block_index).len() {
                let data = signature::signature(&mut h1, h2, num_rounds);
                let block = &mut self.bits.get_block_mut(block_index);
                block[i] |= data;
            }
        }
    }

    /// Returns `false` if the bloom filter definitely does not contain a value.
    /// Returns `true` if the bloom filter may contain a value, with a degree of certainty.
    ///
    /// # Examples
    ///
    /// ```
    /// use fastbloom::BloomFilter;
    ///
    /// let bloom = BloomFilter::builder(1024).items([1, 2, 3]);
    /// assert!(bloom.contains(&1));
    /// ```
    #[inline]
    pub fn contains(&self, val: &(impl Hash + ?Sized)) -> bool {
        let [mut h1, h2] = get_orginal_hashes(&self.hasher, val);
        let block_index = block_index(self.num_blocks(), h1);
        let block = &self.bits.get_block(block_index);
        (0..self.num_hashes).into_iter().all(|_| {
            BlockedBitVec::<BLOCK_SIZE_BITS>::check_for_block(block, Self::bit_index(&mut h1, h2))
        }) && (if let Some(num_rounds) = self.num_rounds {
            (0..block.len()).all(|i| {
                let data = signature::signature(&mut h1, h2, num_rounds);
                (block[i] & data) == data
            })
        } else {
            true
        })
    }

    /// Returns the number of hashes per item.
    #[inline]
    pub fn num_hashes(&self) -> u32 {
        self.target_hashes as u32
    }

    /// Returns the total number of in-memory bits supporting the bloom filter.
    pub fn num_bits(&self) -> usize {
        self.num_blocks() * BLOCK_SIZE_BITS
    }

    /// Returns the total number of in-memory blocks supporting the bloom filter.
    /// Each block is `BLOCK_SIZE_BITS` bits.
    pub fn num_blocks(&self) -> usize {
        self.bits.num_blocks()
    }

    /// Returns a `u64` slice of this `BloomFilter`’s contents.
    ///
    /// # Examples
    ///
    /// ```
    /// use fastbloom::BloomFilter;
    ///
    /// let data = vec![0x517cc1b727220a95; 8];
    /// let bloom = BloomFilter::<512>::builder_from_vec(data.clone()).hashes(4);
    /// assert_eq!(bloom.as_slice().to_vec(), data);
    /// ```
    #[inline]
    pub fn as_slice(&self) -> &[u64] {
        self.bits.as_slice()
    }
}

impl<T, const BLOCK_SIZE_BITS: usize, S: BuildHasher> Extend<T> for BloomFilter<BLOCK_SIZE_BITS, S>
where
    T: Hash,
{
    #[inline]
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        for val in iter {
            self.insert(&val);
        }
    }
}

impl<const BLOCK_SIZE_BITS: usize, S: BuildHasher> PartialEq for BloomFilter<BLOCK_SIZE_BITS, S> {
    fn eq(&self, other: &Self) -> bool {
        self.bits == other.bits && self.num_hashes == other.num_hashes
    }
}
impl Eq for BloomFilter {}

/// Produces a new hash efficiently from two orignal hashes and a seed.
///
/// Modified from <https://www.eecs.harvard.edu/~michaelm/postscripts/rsa2008.pdf>.
#[inline]
fn next_hash(h1: &mut u64, h2: u64) -> u64 {
    *h1 = h1.wrapping_add(h2).rotate_left(5);
    *h1
}

/// The first two hashes of the value, h1 and h2.
///
/// Subsequent hashes, h, are efficiently derived from these two using `next_hash`.
///
/// This strategy is adapted from <https://www.eecs.harvard.edu/~michaelm/postscripts/rsa2008.pdf>,
/// in which a keyed hash function is used to generate two real hashes, h1 and h2, which are then used to produce
/// many more "fake hahes" h, using h = h1 + i * h2.
///
/// However, here we only use 1 real hash, for performance, and derive h1 and h2:
/// First, we'll think of the 64 bit real hash as two seperate 32 bit hashes, h1 and h2.
///     - Using h = h1 + i * h2 generates entropy in at least the lower 32 bits
/// Second, for more entropy in the upper 32 bits, we'll populate the upper 32 bits for both h1 and h2:
/// For h1, we'll use the original upper bits 32 of the real hash.
///     - h1 is the same as the real hash
/// For h2 we'll use lower 32 bits of h, and multiply by a large constant (same constant as FxHash)
///     - h2 is basically a "weak hash" of h1
#[inline]
pub(crate) fn get_orginal_hashes(
    hasher: &impl BuildHasher,
    val: &(impl Hash + ?Sized),
) -> [u64; 2] {
    let mut state = hasher.build_hasher();
    val.hash(&mut state);
    let h1 = state.finish();
    let h2 = h1.wrapping_shr(32).wrapping_mul(0x51_7c_c1_b7_27_22_0a_95); // 0xffff_ffff_ffff_ffff / 0x517c_c1b7_2722_0a95 = π
    [h1, h2]
}

/// Returns a the block index for an item's hash.
/// The block index must be in the range `0..self.bits.num_blocks()`.
/// This implementation is a more performant alternative to `hash % self.bits.num_blocks()`:
/// <https://lemire.me/blog/2016/06/27/a-fast-alternative-to-the-modulo-reduction/>
#[inline]
pub(crate) fn block_index(num_blocks: usize, hash: u64) -> usize {
    (((hash >> 32) as usize * num_blocks) >> 32) as usize
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{rngs::StdRng, Rng, SeedableRng};
    use std::{collections::HashSet, iter::repeat};

    trait Seeded: BuildHasher {
        fn seeded(seed: &[u8; 16]) -> Self;
    }
    impl Seeded for DefaultHasher {
        fn seeded(seed: &[u8; 16]) -> Self {
            Self::seeded(seed)
        }
    }
    impl Seeded for ahash::RandomState {
        fn seeded(seed: &[u8; 16]) -> Self {
            ahash::RandomState::with_seed(seed[0] as usize)
        }
    }

    fn random_strings(num: usize, min_repeat: u32, max_repeat: u32, seed: u64) -> Vec<String> {
        let mut rng = StdRng::seed_from_u64(seed);
        let gen = rand_regex::Regex::compile(r"[a-zA-Z]+", max_repeat).unwrap();
        (&mut rng)
            .sample_iter(&gen)
            .filter(|s: &String| s.len() >= min_repeat as usize)
            .take(num)
            .collect()
    }

    fn random_numbers(num: usize, seed: u64) -> Vec<u64> {
        let mut rng = StdRng::seed_from_u64(seed);
        repeat(()).take(num).map(|_| rng.gen()).collect()
    }

    trait Container {
        fn new<I: IntoIterator<IntoIter = impl ExactSizeIterator<Item = impl Hash>>>(
            num_bits: usize,
            items: I,
        ) -> Self;
        fn check<X: Hash>(&self, s: X) -> bool;
        fn num_hashes(&self) -> usize;
        fn block_counts(&self) -> Vec<u64>;
    }
    impl<const N: usize, H: Seeded> Container for BloomFilter<N, H> {
        fn new<I: IntoIterator<IntoIter = impl ExactSizeIterator<Item = impl Hash>>>(
            num_bits: usize,
            items: I,
        ) -> Self {
            BloomFilter::new_builder::<N>(num_bits)
                .hasher(H::seeded(&[42; 16]))
                .items(items)
        }
        fn check<X: Hash>(&self, s: X) -> bool {
            self.contains(&s)
        }
        fn num_hashes(&self) -> usize {
            self.num_hashes() as usize
        }
        fn block_counts(&self) -> Vec<u64> {
            (0..self.num_blocks())
                .map(|i| {
                    self.bits
                        .get_block(i)
                        .iter()
                        .map(|x| x.count_ones() as u64)
                        .sum()
                })
                .collect()
        }
    }

    #[test]
    fn test_to_from_vec() {
        fn to_from_<const N: usize>(size: usize) {
            let vals = random_numbers(100, size as u64);
            let mut b = BloomFilter::new_builder::<N>(size).seed(&1).hashes(3);
            b.extend(vals.clone());
            let x = b.as_slice();
            let b2 = BloomFilter::new_builder_from_vec::<N>(x.to_vec())
                .seed(&1)
                .hashes(3);
            assert_eq!(b, b2);
            assert_eq!(b.num_blocks() * N, b.as_slice().len() * 64);
            assert!(size <= b.as_slice().len() * 64);
            assert!((size + N) > b.as_slice().len() * 64);
        }
        for size in 1..=10009 {
            to_from_::<64>(size);
            to_from_::<128>(size);
            to_from_::<256>(size);
            to_from_::<512>(size);
        }
    }

    #[test]
    fn random_inserts_always_contained() {
        fn random_inserts_always_contained_<T: Container>() {
            for mag in 1..6 {
                let size = 10usize.pow(mag);
                for bloom_size_mag in 6..10 {
                    let num_blocks_bytes = 1 << bloom_size_mag;
                    let sample_vals = random_numbers(size, 42);
                    let num_bits = num_blocks_bytes * 8;
                    let filter: T = Container::new(num_bits, sample_vals.iter());
                    assert!(sample_vals.into_iter().all(|x| filter.check(x)));
                }
            }
        }
        random_inserts_always_contained_::<BloomFilter<512>>();
        random_inserts_always_contained_::<BloomFilter<256>>();
        random_inserts_always_contained_::<BloomFilter<128>>();
        random_inserts_always_contained_::<BloomFilter<64>>();
    }

    #[test]
    fn test_optimal_hashes_is_optimal() {
        fn test_optimal_hashes_is_optimal_<const BLOCK_SIZE_BITS: usize, H: Seeded>() {
            let sizes = [1000, 2000, 5000, 6000, 8000, 10000];
            let mut wins = 0;
            for num_items in sizes.clone() {
                let sample_vals = random_numbers(num_items, 42);
                let num_bits = 65000 * 8;
                let filter = BloomFilter::new_builder::<BLOCK_SIZE_BITS>(num_bits)
                    .hasher(H::seeded(&[42; 16]))
                    .items(sample_vals.clone().into_iter());
                let control: HashSet<u64> = sample_vals.clone().into_iter().collect();
                let anti_vals = random_numbers(100_000, 3);
                let fp_to_beat = false_pos_rate_with_vals(&filter, &control, &anti_vals);
                let optimal_hashes = filter.num_hashes();

                for num_hashes in [optimal_hashes - 1, optimal_hashes + 1] {
                    let mut test_filter = BloomFilter::new_builder::<BLOCK_SIZE_BITS>(num_bits)
                        .hasher(H::seeded(&[42; 16]))
                        .hashes(num_hashes);
                    test_filter.extend(sample_vals.clone().into_iter());
                    let fp = false_pos_rate_with_vals(&test_filter, &control, &anti_vals);
                    wins += (fp_to_beat <= fp) as usize;
                }
            }
            assert!(wins > sizes.len() / 2);
        }
        test_optimal_hashes_is_optimal_::<512, DefaultHasher>();
        test_optimal_hashes_is_optimal_::<256, DefaultHasher>();
        test_optimal_hashes_is_optimal_::<128, DefaultHasher>();
        test_optimal_hashes_is_optimal_::<64, DefaultHasher>();
    }

    #[test]
    fn seeded_is_same() {
        let num_bits = 1 << 13;
        let sample_vals = random_strings(1000, 16, 32, 53226);
        for x in 0u8..10 {
            let seed = x as u128;
            assert_eq!(
                BloomFilter::builder(num_bits)
                    .seed(&seed)
                    .items(sample_vals.iter()),
                BloomFilter::builder(num_bits)
                    .seed(&seed)
                    .items(sample_vals.iter())
            );
            assert!(
                !(BloomFilter::builder(num_bits)
                    .seed(&(seed + 1))
                    .items(sample_vals.iter())
                    == BloomFilter::builder(num_bits)
                        .seed(&seed)
                        .items(sample_vals.iter()))
            );
        }
    }

    fn false_pos_rate_with_vals<'a, X: Hash + Eq + PartialEq + 'a>(
        filter: &impl Container,
        control: &HashSet<X>,
        anti_vals: impl IntoIterator<Item = &'a X>,
    ) -> f64 {
        let mut total = 0;
        let mut false_positives = 0;
        for x in anti_vals.into_iter() {
            if !control.contains(x) {
                total += 1;
                false_positives += filter.check(x) as usize;
            }
        }
        (false_positives as f64) / (total as f64)
    }

    #[test]
    fn false_pos_decrease_with_size() {
        fn false_pos_decrease_with_size_<T: Container>() {
            let anti_vals = random_numbers(1000, 2);
            for mag in 5..6 {
                let size = 10usize.pow(mag);
                let mut prev_fp = 1.0;
                let mut prev_prev_fp = 1.0;
                for num_bits_mag in 9..22 {
                    let num_bits = 1 << num_bits_mag;
                    let sample_vals = random_numbers(size, 1);
                    let filter: T = Container::new(num_bits, sample_vals.iter());
                    let control: HashSet<u64> = sample_vals.into_iter().collect();
                    let fp = false_pos_rate_with_vals(&filter, &control, &anti_vals);

                    let err = format!(
                        "size: {size:}, num_bits: {num_bits:}, {:.6}, {:?}",
                        fp,
                        filter.num_hashes(),
                    );
                    assert!(
                        fp <= prev_fp || prev_fp <= prev_prev_fp || fp < 0.01,
                        "{}",
                        err
                    ); // allows 1 data point to be higher
                    prev_prev_fp = prev_fp;
                    prev_fp = fp;
                }
            }
        }
        false_pos_decrease_with_size_::<BloomFilter<512>>();
        false_pos_decrease_with_size_::<BloomFilter<256>>();
        false_pos_decrease_with_size_::<BloomFilter<128>>();
        false_pos_decrease_with_size_::<BloomFilter<64>>();
    }

    fn assert_even_distribution(distr: &[u64], err: f64) {
        assert!(err > 0.0 && err < 1.0);
        let expected: i64 = (distr.iter().sum::<u64>() / (distr.len() as u64)) as i64;
        let thresh = (expected as f64 * err) as i64;
        for x in distr {
            let diff = (*x as i64 - expected).abs();
            assert!(diff <= thresh, "{x:?} deviates from {expected:?}");
        }
    }

    #[test]
    fn block_distribution() {
        fn block_distribution_<T: Container>() {
            let filter: T = Container::new(1000, random_numbers(1000, 1));
            assert_even_distribution(&filter.block_counts(), 0.4);
        }
        block_distribution_::<BloomFilter<512>>();
        block_distribution_::<BloomFilter<256>>();
        block_distribution_::<BloomFilter<128>>();
        block_distribution_::<BloomFilter<64>>();
    }
    #[test]
    fn block_hash_distribution() {
        fn block_hash_distribution_<H: BuildHasher + Seeded>(num_blocks: usize) {
            let mut buckets = vec![0; num_blocks];
            let hasher = H::seeded(&[42; 16]);
            for x in random_numbers(num_blocks * 10000, 42) {
                let [h1, _] = get_orginal_hashes(&hasher, &x);
                buckets[block_index(num_blocks, h1)] += 1;
            }
            assert_even_distribution(&buckets, 0.05);
        }
        for size in [2, 7, 10, 100] {
            block_hash_distribution_::<DefaultHasher>(size);
            block_hash_distribution_::<ahash::RandomState>(size);
        }
    }

    #[test]
    fn test_seeded_hash_from_hashes_depth() {
        for size in [1, 10, 100, 1000] {
            let mut rng = StdRng::seed_from_u64(524323);
            let mut h1 = (&mut rng).gen_range(0..u64::MAX);
            let h2 = (&mut rng).gen_range(0..u64::MAX);
            let mut seeded_hash_counts = vec![0; size];
            for _ in 0..(size * 10_000) {
                let hi = next_hash(&mut h1, h2);
                seeded_hash_counts[(hi as usize) % size] += 1;
            }
            assert_even_distribution(&seeded_hash_counts, 0.05);
        }
    }

    #[test]
    fn index_hash_distribution() {
        fn index_hash_distribution_<const N: usize>(thresh_pct: f64) {
            let filter: BloomFilter<N> = BloomFilter::new_builder(1).seed(&0).hashes(1);
            let [mut h1, h2] = get_orginal_hashes(&filter.hasher, "qwerty");
            let mut counts = vec![0; N];
            let iterations = 10000 * N as u64;
            for _ in 0..iterations {
                let bit_index = BloomFilter::<N>::bit_index(&mut h1, h2);
                let index = bit_index % N;
                counts[index] += 1;
            }
            assert_even_distribution(&counts, thresh_pct);
        }
        index_hash_distribution_::<512>(0.05);
        index_hash_distribution_::<256>(0.05);
        index_hash_distribution_::<128>(0.05);
        index_hash_distribution_::<64>(0.05);
    }

    #[test]
    fn test_hash_integration() {
        fn test_hash_integration_<const N: usize, H: BuildHasher + Seeded>(thresh_pct: f64) {
            fn test_with_distr_fn<
                const N: usize,
                H: BuildHasher + Seeded,
                F: FnMut(usize) -> usize,
            >(
                mut f: F,
                filter: &BloomFilter<N, H>,
                thresh_pct: f64,
            ) {
                let num = 2000 * N;
                let mut counts = vec![0; N * filter.num_blocks()];
                for val in (0..num).map(|i| f(i)) {
                    let [mut h1, h2] = get_orginal_hashes(&filter.hasher, &val);
                    let block_index = block_index(filter.num_blocks(), h1);
                    for _ in 0..filter.num_hashes() {
                        let j = BloomFilter::<N>::bit_index(&mut h1, h2);
                        let global = block_index * N + j;
                        counts[global] += 1;
                    }
                }
                assert_even_distribution(&counts, thresh_pct);
            }
            for num_hashes in [1, 4, 8] {
                let clone_me = BloomFilter::new_builder::<N>(4)
                    .hasher(H::seeded(&[42; 16]))
                    .hashes(num_hashes);
                let mut rng = StdRng::seed_from_u64(42);
                test_with_distr_fn(
                    |_| (&mut rng).gen_range(0..usize::MAX),
                    &clone_me,
                    thresh_pct,
                );
                test_with_distr_fn(|x| x * 2, &clone_me, thresh_pct);
                test_with_distr_fn(|x| x * 3, &clone_me, thresh_pct);
                test_with_distr_fn(
                    |x| x * clone_me.num_hashes() as usize,
                    &clone_me,
                    thresh_pct,
                );
                test_with_distr_fn(
                    |x| x * clone_me.num_blocks() as usize,
                    &clone_me,
                    thresh_pct,
                );
                test_with_distr_fn(|x| x * N, &clone_me, thresh_pct);
            }
        }
        let pct = 0.1;
        test_hash_integration_::<512, DefaultHasher>(pct);
        test_hash_integration_::<256, DefaultHasher>(pct);
        test_hash_integration_::<128, DefaultHasher>(pct);
        test_hash_integration_::<64, DefaultHasher>(pct);
    }

    #[test]
    fn test_debug() {
        let filter = BloomFilter::builder(1).hashes(1);
        assert!(!format!("{:?}", filter).is_empty());
    }

    #[test]
    fn test_clone() {
        let filter = BloomFilter::builder(4).hashes(4);
        assert_eq!(filter, filter.clone());
    }
}
