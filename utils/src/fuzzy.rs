//! Provides a collection that allows fuzzy text searching.
//!
//! Build a [`Search`] to be able to search for things by a text value,
//! and then [search](`Search::search`) it for [Matches](`Match`).
//!
//! Searches happen by normalized text fragments with sizes based on the `MIN`
//! and `MAX` parameters to the [`Search`]. It first searches for the larger
//! fragments, falling back to smaller ones if no matches are found.
//!
//! # Fragmenting
//!
//! The normalized text is fragmented as moving windows of a given size, similar
//! to the [`windows`](std::slice::Windows) method on slices.
//!
//! These fragments are compared to known fragments and the corresponding values
//! are considered as match candidates.
//!
//! # Match Score
//!
//! The [`Match::score`] is based on how many of these fragments matched the
//! original text. `1.0` indicates _every_ fragment of the input had a match,
//! but this doesn't indicate an exact match with the original text.
//!
//! The final set of matches will often contain vaguely similar texts, even if
//! there is an exact match. Furthermore, since the [`Match::score`] cannot be
//! used to check for exact matches, _multiple_ matches may have a score of
//! `1.0` for the same search.
//!
//! This could, for instance, happen if one were to search for `"egg"` when the
//! search contains `"Eggs and Bacon"` and `"Egg (raw)"`.
//!
//! Matches with the same score will be sorted by the length of their original,
//! normalized text, and, if that still ties, then sorted by their insertion
//! order.
//!
//! # Text Normalization
//!
//! The normalization lowercases the entire text, and non-alphanumeric sequences
//! are translated into "separators". A separator is added to the start and end
//! also.
//!
//! For instance, the following texts are equivalent after normalization:
//! - `"Hello World!"`
//! - `hello-world`
//! - `(hELLO)(wORLD)`

use std::cmp;
use std::collections::HashMap;
use std::ptr::{self, NonNull};
use std::vec::IntoIter as VecIntoIter;

use arrayvec::ArrayVec;
use smallvec::SmallVec;

use crate::private::ptr::RawRef;

// exists to save some memory.
// this only becomes an issue once more than 4 BILLION elements have been added
// to the Search. at that point, the current behavior is to panic.
// for 32- or 16-bit systems, allocating for the values Vec will panic first.
#[cfg(not(target_pointer_width = "16"))]
type MatchIndex = u32;
#[cfg(target_pointer_width = "16")]
type MatchIndex = u16;

// amount of MatchIndex values that can be stored within a SmallVec without
// increasing its size.
#[cfg(target_pointer_width = "64")]
const MATCH_INLINE: usize = 4;
#[cfg(not(target_pointer_width = "64"))]
const MATCH_INLINE: usize = 2;

/// Provides a fuzzy text searcher.
///
/// [`Search::insert`] new elements with associated, then [`Search::search`] for
/// the data by the key.
///
/// The `T` generic parameter defines the associated data to store.
/// You can use `()` (unit) to not store data and instead always just use
/// entry's index.
///
/// The `MIN` and `MAX` generic parameters can be used to customize the fragment
/// splitting.
#[derive(Debug, Clone)]
pub struct Search<T, const MIN: usize = 2, const MAX: usize = 4> {
    min_match_score: f64,

    // Safety invariant: Every value in the vectors within `match_map`
    // _must_ be a valid index into `values`. Unsafe code may rely on this.
    match_map: HashMap<Segment<MAX>, SmallVec<[MatchIndex; MATCH_INLINE]>>,
    values: Vec<ValueMetadata<T>>,
}

/// Struct to hold metadata about a value and the user's data.
///
/// The original string is never stored.
#[derive(Debug, Clone)]
struct ValueMetadata<T> {
    /// The length of the original, normalized input.
    ///
    /// Used as a tie-breaker when sorting matches with equal counts.
    len: MatchIndex,
    /// The attached userdata.
    userdata: T,
}

impl<T, const MIN: usize, const MAX: usize> Search<T, MIN, MAX> {
    /// Creates a new empty search instance.
    #[must_use]
    pub fn new() -> Self {
        const {
            assert!(MIN <= MAX, "MIN must be <= MAX");
            assert!(MIN > 0, "MIN must be > 0");
            assert!(MAX < MatchIndex::MAX as usize, "MAX must be < u32::MAX");
        }

        Self {
            min_match_score: 0.5,
            match_map: HashMap::new(),
            values: Vec::new(),
        }
    }

    /// Changes the minimum matching score for returned values.
    /// The default is `0.5`.
    ///
    /// Check [`Match::score`] for more details.
    ///
    /// # Panics
    ///
    /// Panics if the provided score is less than `0.0` or greater than `1.0`.
    #[must_use]
    pub fn with_min_match_score(mut self, score: f64) -> Self {
        assert!(
            (0.0..=1.0).contains(&score),
            "score must be within 0.0..=1.0, but was {score}"
        );

        self.min_match_score = score;
        self
    }

    /// Inserts a new value with associated data.
    ///
    /// The return is the entry's index. This index is also returned on a search
    /// [`Match`] and can be used in place of associated data if you wish to
    /// store the data elsewhere.
    ///
    /// The indices are created ascendingly, with `0` being the first item.
    /// The second item would be `1`, the third `2`, and so on.
    pub fn insert(&mut self, value: &str, data: T) -> usize {
        let norm = norm_str(value);
        let index: MatchIndex = self
            .values
            .len()
            .try_into()
            .expect("cannot add more than u32::MAX elements to Search");

        // add the data first so safety invariants aren't violated if a panic occurs.
        self.values.push(ValueMetadata {
            len: norm.len().try_into().unwrap_or(MatchIndex::MAX),
            userdata: data,
        });

        if norm.len() >= MIN {
            let upper = MAX.min(norm.len());

            for s in (MIN..=upper).rev() {
                // SAFETY: index is a valid index into values
                unsafe {
                    self.add_segments_of(index, &norm, s);
                }
            }
        }

        index as usize
    }

    /// Searches for a given text.
    ///
    /// The returned entries are sorted by their score.
    /// The first match will have the highest score.
    ///
    /// Check [`Match::score`] for more details.
    pub fn search<'st>(&'st self, value: &str) -> MatchIter<'st, T> {
        let norm = norm_str(value);
        let mut results = MatchIter::default();

        if norm.len() >= MIN {
            let upper = MAX.min(norm.len());

            for size in (MIN..=upper).rev() {
                results = self.find_with_segment_size(&norm, size);
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

        // println!("seg: {}, mem: ~{}", self.match_map.len(),
        // self.match_map.len() * 60 + self.match_map.values().map(|v|
        // v.len()).sum::<usize>() * size_of::<MatchIndex>());
    }

    /// Adds the segments of the `norm` slice to [`Self::match_map`].
    ///
    /// # Safety
    ///
    /// `index as usize` must be a valid index into [`Self::values`].
    #[inline]
    unsafe fn add_segments_of(&mut self, index: MatchIndex, norm: &[u16], size: usize) {
        for segment in iter_segments(norm, size) {
            self.match_map.entry(segment).or_default().push(index);
        }
    }

    fn find_with_segment_size<'st>(&'st self, norm: &[u16], size: usize) -> MatchIter<'st, T> {
        const MAX_MATCHES: usize = 32;

        let mut results = <ArrayVec<MatchInfoLen, MAX_MATCHES>>::new();
        let mut total = 0usize;

        for segment in iter_segments(norm, size) {
            total += 1;
            let Some(match_entry) = self.match_map.get(&segment) else {
                continue;
            };

            for &index in match_entry {
                debug_assert!(
                    (index as usize) < self.values.len(),
                    "Search safety invariant not met"
                );

                // find & modify, or insert
                match results.iter_mut().find(|m| m.index == index) {
                    Some(res) => res.count += 1,
                    // discard results past the max capacity
                    None => {
                        _ = results.try_push(MatchInfoLen {
                            count: 1,
                            index,
                            // SAFETY: entry index must be valid into `self.values`
                            len: unsafe { self.values.get_unchecked(index as usize).len },
                        })
                    },
                }
            }
        }

        let total = total as f64;
        let match_count = total * self.min_match_score;

        results.retain(|r| f64::from(r.count) >= match_count);
        results.sort_unstable();

        // copy as MatchInfo; TrustedLen should avoid redundant allocations
        // original code here already allocated, and it's fine perf-wise
        let results = results.iter().map(|m| m.discard()).collect();

        // SAFETY: every index in `results` is a valid index into `values`
        // as guaranteed by the type invariants; indices come from `match_map`.
        unsafe { MatchIter::new(total, results, &self.values) }
    }
}

impl<T, const MIN: usize, const MAX: usize> Default for Search<T, MIN, MAX> {
    fn default() -> Self {
        Self::new()
    }
}

/// A matched value from a [`Search`].
#[derive(Debug)]
#[non_exhaustive]
pub struct Match<'st, T> {
    /// The match score.
    ///
    /// The score is calculated based on how many segments of the input matched
    /// the found value. `1.0` means _every_ input segment matched for this
    /// value. This doesn't necessarily indicate an exact match.
    pub score: f64,

    /// The search entry's index.
    ///
    /// This index is returned by [`Search::insert`] and represents the insert
    /// position.
    pub index: usize,

    /// The associated data.
    pub data: &'st T,
}

impl<T> Copy for Match<'_, T> {}
impl<T> Clone for Match<'_, T> {
    fn clone(&self) -> Self {
        *self
    }
}

/// Info stored for each [`Match`] in the [`MatchIter`].
///
/// Constructed from [`MatchInfoLen`], usually.
#[derive(Debug, Clone, Copy)]
struct MatchInfo {
    count: MatchIndex,
    index: MatchIndex,
}

/// A sortable [`MatchInfo`], with an additional `len` field.
///
/// Used during search so the results can be
/// - sorted by `count` desc,
/// - then by `len` asc,
/// - then by `index` asc.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MatchInfoLen {
    count: MatchIndex,
    index: MatchIndex,
    len: MatchIndex,
}

impl MatchInfoLen {
    /// Discards the `len` and creates a [`MatchInfo`].
    fn discard(self) -> MatchInfo {
        MatchInfo {
            index: self.index,
            count: self.count,
        }
    }
}

impl Ord for MatchInfoLen {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        // sort by count desc
        // then by len asc
        // then by index asc
        self.count
            .cmp(&other.count)
            .reverse()
            .then_with(|| self.len.cmp(&other.len))
            .then_with(|| self.index.cmp(&other.index))
    }
}

impl PartialOrd for MatchInfoLen {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// An iterator over [`Matches`](Match) returned by [`Search::search`].
#[derive(Debug)]
#[must_use = "iterators are lazy and do nothing until iterated"]
pub struct MatchIter<'st, T> {
    // Safety invariant: Every index within here must be a valid index into
    // the memory pointed to by `state.search_values`.
    inner: VecIntoIter<MatchInfo>,
    state: MatchIterState<'st, T>,
}

/// Split data needed to construct [`Matches`](Match) from [`MatchIter`] to
/// allow disjointed borrows when the iterator is already mutably borrowed or
/// consumed.
#[derive(Debug)]
struct MatchIterState<'st, T> {
    total: f64,

    // This could be implemented with safe code but this saves a usize in memory.
    search_values: RawRef<'st, ValueMetadata<T>>,
}

impl<'st, T> MatchIter<'st, T> {
    /// Constructs a new [`MatchIter`].
    ///
    /// # Safety
    ///
    /// The `search_values` must come the same [`Search`] as the `inner`'s
    /// indices.
    unsafe fn new(
        total: f64,
        inner: Vec<MatchInfo>,
        search_values: &'st [ValueMetadata<T>],
    ) -> Self {
        debug_assert!(
            inner
                .iter()
                .all(|m| (m.index as usize) < search_values.len()),
            "MatchIter safety invariant not met"
        );

        Self {
            inner: inner.into_iter(),
            state: MatchIterState {
                total,
                search_values: RawRef::from(search_values).cast_element(),
            },
        }
    }

    fn is_empty(&self) -> bool {
        self.inner.len() == 0
    }
}

impl<'st, T> MatchIterState<'st, T> {
    /// Constructs a match.
    ///
    /// # Safety
    ///
    /// The `info` must come from the associated `inner` iterator.
    unsafe fn make_match(&self, info: MatchInfo) -> Match<'st, T> {
        Match {
            score: f64::from(info.count) / self.total,
            index: info.index as usize,
            // SAFETY: caller guarantees the match info comes from the inner iterator,
            // `new` requires that the indices are valid for the search values
            data: unsafe {
                &self
                    .search_values
                    .add(info.index as usize)
                    .as_ref()
                    .userdata
            },
        }
    }
}

impl<T> Clone for MatchIter<'_, T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            state: self.state,
        }
    }
}

impl<T> Copy for MatchIterState<'_, T> {}
impl<T> Clone for MatchIterState<'_, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Default for MatchIter<'_, T> {
    /// Creates an empty iterator with no matches.
    fn default() -> Self {
        Self {
            inner: VecIntoIter::default(),
            state: MatchIterState {
                total: 0.0,
                // no sound code would be able to index this anyways
                search_values: NonNull::dangling().into(),
            },
        }
    }
}

// to not repeat the same safety comment for every unsafe block wrapping
// make_match: SAFETY: make_match is safe to call when used with a value coming
// from the inner iterator
impl<'st, T> Iterator for MatchIter<'st, T> {
    type Item = Match<'st, T>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner
            .next()
            .map(|m| unsafe { self.state.make_match(m) })
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.inner
            .nth(n)
            .map(|m| unsafe { self.state.make_match(m) })
    }

    fn last(self) -> Option<Self::Item> {
        self.inner
            .last()
            .map(|m| unsafe { self.state.make_match(m) })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }

    fn collect<B: FromIterator<Self::Item>>(self) -> B {
        // this should optimize a bit better than a direct collect()
        self.inner
            .map(|m| unsafe { self.state.make_match(m) })
            .collect()
    }
}

impl<T> DoubleEndedIterator for MatchIter<'_, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner
            .next_back()
            .map(|m| unsafe { self.state.make_match(m) })
    }

    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        self.inner
            .nth_back(n)
            .map(|m| unsafe { self.state.make_match(m) })
    }
}

impl<T> ExactSizeIterator for MatchIter<'_, T> {
    fn len(&self) -> usize {
        self.inner.len()
    }
}

/// A search segment. Used as a key.
type Segment<const N: usize> = [u16; N];

unsafe fn new_segment<const N: usize>(pts: &[u16]) -> Segment<N> {
    let mut res = [0u16; N];
    debug_assert!(
        pts.len() <= N,
        "safety: pts.len() must be at most {N} but is {}",
        pts.len()
    );

    // SAFETY: Caller passes segments with size N or less.
    unsafe {
        ptr::copy_nonoverlapping(pts.as_ptr(), res.as_mut_ptr(), pts.len());
    }
    res
}

fn iter_segments<const N: usize>(
    slice: &[u16],
    size: usize,
) -> impl Iterator<Item = Segment<N>> + '_ {
    assert!(
        (1..=N).contains(&size),
        "size must be within 1..={N}, but is {size}"
    );

    slice.windows(size).map(|w| unsafe { new_segment(w) })
}

fn norm_str(str: &str) -> SmallVec<[u16; 20]> {
    let mut out = SmallVec::new();
    let mut whitespace = true;

    out.push(1u16);

    for c in str.chars() {
        if c.is_alphanumeric() {
            let lowercase = c
                .to_lowercase()
                .filter(|c| c.is_alphanumeric())
                .map(|c| c as u16);

            out.extend(lowercase);
            whitespace = false;
        } else if !whitespace {
            out.push(1);
            whitespace = true;
        }
    }

    if !whitespace {
        out.push(1u16);
    }

    out
}

#[cfg(test)]
mod test {
    use super::{norm_str, MatchIter, Search};

    type TSearch = Search<u8>;

    #[test]
    fn search() {
        let search = {
            let mut search = TSearch::new().with_min_match_score(0.2);
            search.insert("Hello World!", 1u8);
            search.insert("Hello There.", 2);
            search.insert("World Welcome", 3);
            search.insert("Nonmatch", 4);
            search
        };

        assert_eq!(&sorted_data(search.search("ello")), &[1, 2]);
        assert_eq!(&sorted_data(search.search("world")), &[1, 3]);
        assert_eq!(&sorted_data(search.search("el e")), &[1, 2, 3]);
        assert_eq!(&sorted_data(search.search("non")), &[4]);

        fn sorted_data(v: MatchIter<'_, u8>) -> Vec<u8> {
            let mut v: Vec<u8> = v.map(|p| *p.data).collect();
            v.sort_unstable();
            v
        }
    }

    #[test]
    fn search_order() {
        let search = {
            let mut search = TSearch::new().with_min_match_score(f64::EPSILON);
            search.insert("Houston II", 4);
            search.insert("Ho", 1u8);
            search.insert("Houston", 3);
            search.insert("Hous", 2);
            search
        };

        // 1 isn't matched because the 4-segment check already succeeds
        assert_eq!(&just_data(search.search("Hous")), &[2, 3, 4]);
        assert_eq!(&just_data(search.search("Houston")), &[3, 4, 2]);

        fn just_data(v: MatchIter<'_, u8>) -> Vec<u8> {
            v.map(|p| *p.data).collect()
        }
    }

    #[test]
    fn norm_str_equality() {
        assert_eq!(norm_str("hello-world"), norm_str("Hello World!"));
        assert_eq!(norm_str("(hELLO)(wORLD)"), norm_str("Hello World!"));
        assert_eq!(norm_str(""), norm_str("----"));
        assert_eq!(norm_str("Hello123"), norm_str(" hELLO123 "));
    }

    #[test]
    fn norm_str_inequality() {
        assert_ne!(norm_str("hello-world"), norm_str("HelloWorld!"));
        assert_ne!(norm_str("(hELLOwORLD)"), norm_str("Hello World!"));
        assert_ne!(norm_str(""), norm_str("--a--"));
        assert_ne!(norm_str("Hello123"), norm_str("Hello 123"));
    }
}
