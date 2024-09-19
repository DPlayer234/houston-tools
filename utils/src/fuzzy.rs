//! Provides a collection that allows fuzzy text searching.
//!
//! Build a [`Search`] to be able to search for things by a text value,
//! and then [search](`Search::search`) it for [Matches](`Match`).
//!
//! Searches happen by normalized text fragments with sizes based on the `MIN` and `MAX`
//! parameters to the [`Search`]. It first searches for the larger fragments, falling
//! back to smaller ones if no matches are found.
//!
//! # Fragmenting
//!
//! The normalized text is fragmented as moving windows of a given size, similar to the
//! [`windows`](std::slice::Windows) method on slices.
//!
//! These fragments are compared to known fragments and the corresponding values are
//! considered as match candidates.
//!
//! # Match Score
//!
//! The [`Match::score`] is based on how many of these fragments matched the original text.
//! `1.0` indicates _every_ fragment of the input had a match, but this doesn't indicate
//! an exact match with the original text.
//!
//! The final set of matches will often contain vaguely similar texts, even if there is an
//! exact match. Furthermore, since the [`Match::score`] cannot be used to check for exact
//! matches, _multiple_ matches may have a score of `1.0` for the same search.
//!
//! This could, for instance, happen if one were to search for `"egg"` when the search
//! contains `"Eggs and Bacon"` and `"Egg (raw)"`.
//!
//! # Text Normalization
//!
//! The normalization lowercases the entire text, and non-alphanumeric sequences are
//! translated into "separators". A separator is added to the start and end also.
//!
//! For instance, the following texts are equivalent after normalization:
//! - `"Hello World!"`
//! - `hello-world`
//! - `(hELLO)(wORLD)`

use std::collections::HashMap;

use arrayvec::ArrayVec;
use smallvec::SmallVec;

// exists to save some memory.
// this only becomes an issue once more than 4 BILLION elements have been added to the Search.
// at that point, the current behavior is to panic
#[cfg(not(target_pointer_width = "16"))]
type MatchIndex = u32;
#[cfg(target_pointer_width = "16")]
type MatchIndex = u16;

// amount of MatchIndex values that can be stored within a SmallVec without increasing its size.
#[cfg(target_pointer_width = "64")]
const MATCH_INLINE: usize = 4;
#[cfg(target_pointer_width = "32")]
const MATCH_INLINE: usize = 2;
#[cfg(target_pointer_width = "16")]
const MATCH_INLINE: usize = 2;

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
    match_map: HashMap<Segment<MAX>, SmallVec<[MatchIndex; MATCH_INLINE]>>,
    values: Vec<T>,
}

impl<T, const MIN: usize, const MAX: usize> Search<T, MIN, MAX> {
    /// Creates a new empty search instance.
    pub fn new() -> Self {
        const { assert!(MIN <= MAX); }

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
    pub fn with_min_match_score(mut self, score: f64) -> Self {
        assert!((0.0..=1.0).contains(&score));

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
    /// The returned entries are sorted by their score.
    /// The first match will have the highest score.
    ///
    /// Check [`Match::score`] for more details.
    pub fn search<'st>(&'st self, value: &str) -> Vec<Match<&'st T>> {
        let norm = norm_str(value);
        let mut results = Vec::new();

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

        // println!("seg: {}, mem: ~{}", self.match_map.len(), self.match_map.len() * 60 + self.match_map.values().map(|v| v.len()).sum::<usize>() * size_of::<MatchIndex>());
    }

    #[inline]
    fn add_segments_of(&mut self, norm: &[u16], size: usize) {
        let index: MatchIndex = self.values.len()
            .try_into()
            .expect("cannot add more than u32::MAX elements to Search");

        for segment in iter_segments(norm, size) {
            self.match_map
                .entry(segment)
                .or_default()
                .push(index);
        }
    }

    fn find_with_segment_size<'st>(&'st self, norm: &[u16], size: usize) -> Vec<Match<&'st T>> {
        const MAX_MATCHES: usize = 32;

        struct MatchInfo {
            count: MatchIndex,
            index: MatchIndex,
        }

        let mut results = <ArrayVec<MatchInfo, MAX_MATCHES>>::new();
        let mut total = 0usize;

        for segment in iter_segments(norm, size) {
            total += 1;
            let Some(match_entry) = self.match_map.get(&segment) else { continue };

            for &index in match_entry {
                let res = results
                    .iter_mut()
                    .find(|m| m.index == index);

                if let Some(res) = res {
                    res.count += 1;
                } else {
                    // discard results past the max capacity
                    _ = results.try_push(MatchInfo { count: 1, index });
                }
            }
        }

        let total = total as f64;
        let match_count = total * self.min_match_score;

        results.retain(|r| f64::from(r.count) >= match_count);
        results.sort_by_key(|r| !r.count);

        results.into_iter()
            .map(|r| Match {
                score: f64::from(r.count) / total,
                index: r.index as usize,
                data: &self.values[r.index as usize],
            })
            .collect()
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
type Segment<const N: usize> = [u16; N];

unsafe fn new_segment<const N: usize>(pts: &[u16]) -> Segment<N> {
    let mut res = [0u16; N];

    // SAFETY: Caller passes segments with size N.
    unsafe { res.get_unchecked_mut(..pts.len()) }.copy_from_slice(pts);
    res
}

fn iter_segments<const N: usize>(slice: &[u16], size: usize) -> impl Iterator<Item = Segment<N>> + '_ {
    assert!((1..=N).contains(&size));

    slice.windows(size)
        .map(|w| unsafe { new_segment(w) })
}

fn norm_str(str: &str) -> SmallVec<[u16; 20]> {
    let mut out = SmallVec::new();
    let mut whitespace = true;

    out.push(1u16);

    for c in str.chars() {
        if c.is_alphanumeric() {
            let lowercase = c.to_lowercase()
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
    use super::{Search, Match};

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

        assert_eq!(&just_data(search.search("ello")), &[1, 2]);
        assert_eq!(&just_data(search.search("world")), &[1, 3]);
        assert_eq!(&just_data(search.search("el e")), &[1, 2, 3]);
        assert_eq!(&just_data(search.search("non")), &[4]);
    }

    fn just_data(v: Vec<Match<&u8>>) -> Vec<u8> {
        let mut v: Vec<u8> = v.into_iter().map(|p| *p.data).collect();
        v.sort();
        v
    }
}
