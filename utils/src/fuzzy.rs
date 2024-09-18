use std::collections::HashMap;

// exists to save some memory.
// this only becomes an issue once more than 4 BILLION elements have been added to the Search.
// at that point, the current behavior is to panic
#[cfg(not(target_pointer_width = "16"))]
type MatchIndex = u32;
#[cfg(target_pointer_width = "16")]
type MatchIndex = u16;

/// Provides a fuzzy text searcher.
///
/// [`Search::insert`] new elements with associated, then [`Search::search`] for the data by the key.
///
/// The `T` generic parameter defines the associated data to store.
/// You can use `()` (unit) to not store data and instead always just use entry's index.
///
/// The `MIN` and `MAX` generic parameters can be used to customize the fragment splitting.
#[derive(Debug, Clone)]
pub struct Search<T, const MIN: usize = 2, const MAX: usize = 4> {
    min_match_score: f64,
    match_map: HashMap<Segment<MAX>, Vec<MatchIndex>>,
    values: Vec<T>,
}

impl<T, const MIN: usize, const MAX: usize> Search<T, MIN, MAX> {
    /// Creates a new empty search instance.
    pub fn new() -> Self {
        const { assert!(MIN <= MAX); }

        Self {
            min_match_score: 0.3,
            match_map: HashMap::new(),
            values: Vec::new(),
        }
    }

    /// Changes the minimum matching score for returned values.
    /// The default is `0.3`.
    ///
    /// Check [`Match::score`] for more details.
    pub fn with_min_match_score(mut self, score: f64) -> Self {
        self.min_match_score = score;
        self
    }

    /// Inserts a new value with associated data.
    ///
    /// The return is the entry's index. This index is also returned on a search [`Match`]
    /// and can be used in place of associated data if you wish to store the data elsewhere.
    ///
    /// The indices are created ascendingly, with `0` being the first item.
    /// The second item would be `1`, the third `2`, and so on.
    pub fn insert(&mut self, value: &str, data: T) -> usize {
        let norm = norm_str(value);
        let index = self.values.len();

        if norm.len() >= MIN {
            let upper = MAX.min(norm.len());

            for s in (MIN..=upper).rev() {
                self.add_segments_of(&norm, s);
            }
        }

        self.values.push(data);
        index
    }

    /// Searches for a given text.
    ///
    /// The returned entries are sortede by their score.
    ///
    /// Check [`Match::score`] for more details.
    pub fn search<'st>(&'st self, value: &str) -> Vec<Match<&'st T>> {
        let norm = norm_str(value);
        let mut results = Vec::new();

        if norm.len() >= MIN {
            let upper = MAX.min(norm.len());

            for s in (MIN..=upper).rev() {
                results = self.find_with_segment_size(&norm, s);

                if !results.is_empty() {
                    break;
                }
            }
        }

        results
    }

    /// Shrinks the internal capacity as much as possible.
    pub fn shrink_to_fit(&mut self) {
        self.match_map.shrink_to_fit();
        for value in self.match_map.values_mut() {
            value.shrink_to_fit();
        }

        self.values.shrink_to_fit();

        // println!("seg: {}, mem: ~{}", self.match_map.len(), self.match_map.len() * 60 + self.match_map.values().map(|v| v.len()).sum::<usize>() * size_of::<MatchIndex>());
    }

    #[inline]
    fn add_segments_of(&mut self, norm: &[u16], gram_size: usize) {
        let index: MatchIndex = self.values.len()
            .try_into()
            .expect("cannot add more than u32::MAX elements to Search");

        for s in Segment::iterate(norm, gram_size) {
            self.match_map
                .entry(s)
                .or_default()
                .push(index);
        }
    }

    fn find_with_segment_size<'st>(&'st self, norm: &[u16], size: usize) -> Vec<Match<&'st T>> {
        use std::mem::MaybeUninit;

        #[derive(Clone, Copy)]
        struct Pair {
            key: MatchIndex,
            value: usize,
        }

        let mut match_set = [<MaybeUninit<Pair>>::uninit(); 32];
        let mut match_count: usize = 0;

        let mut results = Vec::new();

        for s in Segment::iterate(norm, size) {
            let Some(match_entry) = self.match_map.get(&s) else { continue };

            for &match_index in match_entry {
                // SAFETY: the memory region we take must have been initialized by now
                let real_set = unsafe { crate::mem::assume_init_slice_mut(&mut match_set[..match_count]) };
                let real_count = real_set.iter_mut().find(|p| p.key == match_index);

                if let Some(pair) = real_count {
                    pair.value += 1
                } else if match_count < match_set.len() {
                    results.push(Match {
                        score: 0.0,
                        index: match_index as usize,
                        data: &self.values[match_index as usize],
                    });

                    match_set[match_count].write(Pair {
                        key: match_index,
                        value: 1,
                    });

                    match_count += 1;
                }
            }
        }

        let total = (norm.len() + 1 - size) as f64;
        for (index, r#match) in results.iter_mut().enumerate() {
            let count = unsafe { match_set[index].assume_init_ref() }.value as f64;
            r#match.score = count / total;
        }

        results.retain(|r| r.score >= self.min_match_score);

        #[allow(clippy::cast_possible_wrap)]
        #[allow(clippy::cast_possible_truncation)]
        results.sort_by_key(|r| -(r.score.to_bits() as isize));
        results
    }
}

impl<T, const MIN: usize, const MAX: usize> Default for Search<T, MIN, MAX> {
    fn default() -> Self {
        Self::new()
    }
}

/// A matched value from a [`Search`].
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct Match<T> {
    /// The match score.
    ///
    /// The score is calculated based on how many segments of the input matched the found value.
    /// `1.0` means _every_ input segment matched for this value. This doesn't necessarily indicate an exact match.
    pub score: f64,

    /// The search entry's index.
    pub index: usize,

    /// The associated data.
    pub data: T,
}

/// A search segment. Used as a key.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
struct Segment<const N: usize>([u16; N]);

impl<const N: usize> Segment<N> {
    unsafe fn new_unchecked(pts: &[u16]) -> Self {
        let mut res = Segment([0; N]);

        // SAFETY: Caller never passes segments larger than the MAX_SIZE.
        unsafe { res.0.get_unchecked_mut(..pts.len()) }.copy_from_slice(pts);
        res
    }

    fn iterate(slice: &[u16], size: usize) -> impl Iterator<Item = Self> + '_ {
        assert!((1..=N).contains(&size));
        slice.windows(size)
            .map(|w| unsafe { Segment::new_unchecked(w) })
    }
}

fn norm_str(str: &str) -> Vec<u16> {
    let del = std::iter::once(1u16);
    let main = str
        .chars()
        .flat_map(|c| c.to_lowercase())
        .filter(|c| c.is_alphanumeric())
        .map(|c| c as u16);

    del.clone().chain(main).chain(del).collect()
}
