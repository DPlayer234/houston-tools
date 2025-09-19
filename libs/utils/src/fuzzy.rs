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

use std::cmp::Reverse;
use std::collections::HashMap;
use std::mem::take;
use std::ptr;
use std::vec::IntoIter as VecIntoIter;

use smallvec::SmallVec;

use crate::private::ptr::RawRef;

// exists to save some memory.
// this only becomes an issue once more than 4 BILLION elements have been added
// to the Search. at that point, the current behavior is to panic.
// for 32- or 16-bit systems, allocating the values Vec is guaranteed to panic
// before then. for 64-bit systems, this panic would occur at over 16 GB
// of allocated memory, and will likely run into an OOM.
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

#[inline(always)]
const fn to_usize(index: MatchIndex) -> usize {
    const {
        assert!(
            size_of::<MatchIndex>() <= size_of::<usize>(),
            "MatchIndex as usize cast must be lossless"
        );
    }

    index as usize
}

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
            assert!(MAX < to_usize(MatchIndex::MAX), "MAX must be < u32::MAX");
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

        to_usize(index)
    }

    /// Searches for a given text.
    ///
    /// The returned entries are sorted by their score.
    /// The first match will have the highest score.
    ///
    /// Check [`Match::score`] for more details.
    pub fn search<'st>(&'st self, value: &str) -> MatchIter<'st, T> {
        let norm = norm_str(value);
        let norm = norm.as_slice();

        if norm.len() >= MIN {
            // reused buffer for all searches.
            // on success, will also be used in the return value.
            let mut buf = Vec::new();

            let upper = MAX.min(norm.len());
            for size in (MIN..=upper).rev() {
                // `find_with_segment_size` will ensure `buf` is empty when it returns
                if let Some(matches) = self.find_with_segment_size(norm, size, &mut buf) {
                    return matches;
                }
            }
        }

        MatchIter::default()
    }

    /// Shrinks the internal capacity as much as possible.
    pub fn shrink_to_fit(&mut self) {
        self.match_map.shrink_to_fit();
        for value in self.match_map.values_mut() {
            value.shrink_to_fit();
        }

        self.values.shrink_to_fit();
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

    fn find_with_segment_size<'st>(
        &'st self,
        norm: &[u16],
        size: usize,
        // assumed to be empty, and will be empty on return
        results: &mut Vec<MatchInfoLen>,
    ) -> Option<MatchIter<'st, T>> {
        let segments = iter_segments(norm, size);
        let total = segments.len();

        for segment in segments {
            let Some(match_entry) = self.match_map.get(&segment) else {
                continue;
            };

            for &index in match_entry {
                debug_assert!(
                    to_usize(index) < self.values.len(),
                    "search safety invariant not met"
                );

                // find & modify, or insert
                match results.iter_mut().find(|m| m.index == index) {
                    Some(res) => res.count += 1,
                    None => results.push(MatchInfoLen {
                        count: 1,
                        index,
                        // SAFETY: entry index must be valid into `self.values`
                        len: unsafe { self.values.get_unchecked(to_usize(index)).len },
                    }),
                }
            }
        }

        let total = total as f64;
        let match_count = total * self.min_match_score;

        // remove insufficiently accurate matches
        results.retain(|r| f64::from(r.count) >= match_count);
        if !results.is_empty() {
            // sort by count desc
            // then by len asc
            // then by index asc
            results.sort_unstable_by_key(|r| (Reverse(r.count), r.len, r.index));

            // copy as MatchInfo; TrustedLen should avoid redundant allocations
            // original code here already allocated, and it's fine perf-wise
            // don't use `into_iter`, that's not TrustedLen!
            let results = take(results).into_iter().map(MatchInfoLen::info).collect();

            // SAFETY: every index in `results` is a valid index into `values`
            // as guaranteed by the type invariants; indices come from `match_map`.
            Some(unsafe { MatchIter::new(total, results, &self.values) })
        } else {
            None
        }
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
#[derive(Debug, Clone, Copy)]
struct MatchInfoLen {
    count: MatchIndex,
    index: MatchIndex,
    len: MatchIndex,
}

impl MatchInfoLen {
    /// Discards the `len` and creates a [`MatchInfo`].
    fn info(self) -> MatchInfo {
        MatchInfo {
            index: self.index,
            count: self.count,
        }
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
                .all(|m| to_usize(m.index) < search_values.len()),
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
}

impl<'st, T> MatchIterState<'st, T> {
    /// Creates a `Fn` that maps a [`MatchInfo`] to a [`Match`] with `self`.
    ///
    /// # Safety
    ///
    /// The returned `Fn` must only be called with infos coming from the `inner`
    /// iterator associated with `self`.
    unsafe fn mapper(self) -> impl Fn(MatchInfo) -> Match<'st, T> {
        // share a single closure type to minimize the amount of types that
        // need to be generated for the iterator adapter delegation
        move |info| Match {
            score: f64::from(info.count) / self.total,
            index: to_usize(info.index),
            // SAFETY: caller guarantees the match info comes from the inner iterator,
            // `new` requires that the indices are valid for the search values
            data: unsafe {
                &self
                    .search_values
                    .add(to_usize(info.index))
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
        // SAFETY: this is equivalent to `Search::search` with an empty `Search`, except
        // it isn't tied to the lifetime of another variable.
        // empty `search_values` are also necessarily valid with empty `inner` matches.
        unsafe { Self::new(0.0, Vec::new(), &[]) }
    }
}

impl<'st, T> Iterator for MatchIter<'st, T> {
    type Item = Match<'st, T>;

    fn next(&mut self) -> Option<Self::Item> {
        // SAFETY: `mapper` is safe to call with a value coming from the inner iterator
        self.inner.next().map(unsafe { self.state.mapper() })
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        // SAFETY: `mapper` is safe to call with a value coming from the inner iterator
        self.inner.nth(n).map(unsafe { self.state.mapper() })
    }

    fn last(mut self) -> Option<Self::Item> {
        self.next_back()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }

    fn count(self) -> usize {
        self.inner.count()
    }

    fn fold<B, F>(self, init: B, f: F) -> B
    where
        F: FnMut(B, Self::Item) -> B,
    {
        // intermediate map should optimize better
        // SAFETY: `mapper` is safe to call with a value coming from the inner iterator
        self.inner.map(unsafe { self.state.mapper() }).fold(init, f)
    }

    fn collect<B: FromIterator<Self::Item>>(self) -> B {
        // intermediate map should optimize better
        // SAFETY: `mapper` is safe to call with a value coming from the inner iterator
        self.inner.map(unsafe { self.state.mapper() }).collect()
    }
}

impl<T> DoubleEndedIterator for MatchIter<'_, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        // SAFETY: `mapper` is safe to call with a value coming from the inner iterator
        self.inner.next_back().map(unsafe { self.state.mapper() })
    }

    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        // SAFETY: `mapper` is safe to call with a value coming from the inner iterator
        self.inner.nth_back(n).map(unsafe { self.state.mapper() })
    }

    fn rfold<B, F>(self, init: B, f: F) -> B
    where
        F: FnMut(B, Self::Item) -> B,
    {
        // intermediate map should optimize better
        // SAFETY: `mapper` is safe to call with a value coming from the inner iterator
        self.inner
            .map(unsafe { self.state.mapper() })
            .rfold(init, f)
    }
}

impl<T> ExactSizeIterator for MatchIter<'_, T> {
    fn len(&self) -> usize {
        self.inner.len()
    }
}

/// A search segment. Used as a key.
type Segment<const N: usize> = [u16; N];

/// # Safety
/// `pts.len()` must be less than or equal to `N`.
unsafe fn new_segment<const N: usize>(pts: &[u16]) -> Segment<N> {
    let mut res = [0xFFFFu16; N];
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
) -> impl ExactSizeIterator<Item = Segment<N>> + Clone {
    assert!(
        (1..=N).contains(&size),
        "size must be within 1..={N}, but is {size}"
    );

    // SAFETY: asserted that size is <= N
    slice.windows(size).map(|w| unsafe { new_segment(w) })
}

fn norm_str(str: &str) -> SmallVec<[u16; 20]> {
    let mut out = SmallVec::new();
    let mut whitespace = true;

    out.push(1u16);

    for c in str.chars() {
        if c.is_alphanumeric() {
            // only 1 unicode character turns into more than 1 char when lowercased, and
            // conveniently the extra code isn't alphanumeric so we can skip it anyways
            let lowercase = c.to_lowercase().next().unwrap_or_default() as u16;

            out.push(lowercase);
            whitespace = false;
        } else if !whitespace {
            out.push(1u16);
            whitespace = true;
        }
    }

    if !whitespace {
        out.push(1u16);
    }

    out
}

#[cfg(test)]
mod tests {
    use super::{MatchIter, Search, norm_str};

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
            let mut search = TSearch::new().with_min_match_score(0.0);
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
