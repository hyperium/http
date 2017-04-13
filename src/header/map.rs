use super::HeaderValue;
use super::fast_hash::FastHash;
use super::name::{HeaderName, HdrName};

use std::{cmp, mem, ptr, u16};
use std::cell::Cell;
use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::iter::FromIterator;
use std::marker::PhantomData;

/// A set of HTTP headers
///
/// `HeaderMap` is an map of `HeaderName` to `HeaderValue`.
pub struct HeaderMap {
    // Used to mask values to get an index
    mask: Size,
    indices: Vec<Pos>,
    entries: Vec<Bucket>,
    extra_values: Vec<ExtraValue>,
    danger: Danger,
}

// # Implementation notes
//
// Below, you will find a fairly large amount of code. Most of this is to
// provide the necessary functions to efficiently manipulate the header
// multimap. The core hashing table is based on robin hood hashing [1]. While
// this is the same hashing algorithm used as part of Rust's `HashMap` in
// stdlib, many implementation details are different. The two primary reasons
// for this divergence are that `HeaderMap` is a multimap and the structure has
// been optimized to take advantage of the characteristics of HTTP headers.
//
// ## Structure Layout
//
// Most of the data contained by `HeaderMap` is *not* stored in the hash table.
// Instead, pairs of header name and *first* associated header value are stored
// in the `entries` vector. If the header name has more than one associated
// header value, then additional values are stored in `extra_values`. The actual
// hash table (`indices`) only maps hash codes to indices in `entries`. This
// means that, when an eviction happens, the actual header name and value stay
// put and only a tiny amount of memory has to be copied.
//
// Extra values associated with a header name are tracked using a linked list.
// Links are formed with offsets into `ValueSlab` and not pointers.
//
// [1]: ???

/// A `HeaderMap` iterator.
///
/// Yields `(HeaderName, value)` tuples. The same header name may be yielded
/// more than once if it has more than one associated value.
pub struct Iter<'a> {
    map: &'a HeaderMap,
    entry: Size,
    cursor: Option<Cursor>,
}

/// An iterator over `HeaderMap` keys.
///
/// Each header name is yielded only once, even if it has more than one
/// associated value.
pub struct Names<'a> {
    inner: Iter<'a>,
}

/// An iterator over `HeaderMap` values.
pub struct Values<'a> {
    inner: Iter<'a>,
}

/// A drain iterator for `HeaderMap`.
pub struct Drain<'a> {
    idx: usize,
    map: *mut HeaderMap,
    lt: PhantomData<&'a ()>,
}

/// A view to all values associated with a single header name.
pub struct ValueSet<'a> {
    map: &'a HeaderMap,
    index: Size,
}

/// A mutable view to all values associated with a single header name.
pub struct ValueSetMut<'a> {
    map: &'a mut HeaderMap,
    index: Size,
}

/// A view into a single location in a `HeaderMap`, which may be vaccant or occupied.
pub enum Entry<'a> {
    Occupied(OccupiedEntry<'a>),
    Vacant(VacantEntry<'a>),
}

/// A view into a single empty location in a `HeaderMap`.
///
/// This struct is returned as part of the `Entry` enum.
pub struct VacantEntry<'a> {
    map: &'a mut HeaderMap,
    key: HeaderName,
    hash: HashValue,
    probe: Size,
    danger: bool,
}

/// A view into a single occupied location in a `HeaderMap`.
///
/// This struct is returned as part of the `Entry` enum.
pub struct OccupiedEntry<'a> {
    inner: ValueSetMut<'a>,
    probe: Size,
}

/// An iterator of all values associated with a single header name.
pub struct EntryIter<'a> {
    map: &'a HeaderMap,
    index: Size,
    front: Option<Cursor>,
    back: Option<Cursor>,
}

/// An drain iterator of all values associated with a single header name.
pub struct DrainEntry<'a> {
    map: *mut HeaderMap,
    first: Option<HeaderValue>,
    next: Option<Size>,
    lt: PhantomData<&'a ()>,
}

/// Tracks the value iterator state
#[derive(Copy, Clone, Eq, PartialEq)]
enum Cursor {
    Head,
    Values(Size),
}

/// Type used for representing the size of a HeaderMap value.
///
/// 32,768 is more than enough entries for a single header map. Setting this
/// limit enables using `u16` to represent all offsets, which takes 2 bytes
/// instead of 8 on 64 bit processors.
///
/// Setting this limit is especially benificial for `indices`, making it more
/// cache friendly. More hash codes can fit in a cache line.
///
/// You may notice that `u16` may represent more than 32,768 values. This is
/// true, but 32,768 should be plenty and it allows us to reserve the top bit
/// for future usage.
type Size = u16;

/// This limit falls out from above.
const MAX_SIZE: usize = (1 << 15);

/// An entry in the hash table. This represents the full hash code for an entry
/// as well as the position of the entry in the `entries` vector.
#[derive(Copy, Clone)]
struct Pos {
    // Index in the `entries` vec
    index: Size,
    // Hash value
    hash: HashValue,
}

/// Hash values are limited to u16 as well. While `fast_hash` and `Hasher`
/// return `usize` hash codes, limiting the effective hash code to the lower 16
/// bits is fine since we know that the `indices` vector will never grow beyond
/// that size.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
struct HashValue(Size);

/// Stores the data associated with a `HeaderMap` entry. Only the first value is
/// included in this struct. If a header name has more than one associated
/// value, all extra values are stored in the `extra_values` vector. A doubly
/// linked list of entries is maintained. The doubly linked list is used so that
/// removing a value is constant time. This also has the nice property of
/// enabling double ended iteration.
struct Bucket {
    hash: Cell<HashValue>,
    key: HeaderName,
    value: HeaderValue,
    links: Option<Links>,
}

/// The head and tail of the value linked list.
#[derive(Debug, Copy, Clone)]
struct Links {
    next: Size,
    tail: Size,
}

/// Node in doubly-linked list of header value entries
struct ExtraValue {
    value: HeaderValue,
    prev: Cell<Link>,
    next: Cell<Link>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum Link {
    Entry(Size),
    Extra(Size),
}

enum Danger {
    Green,
    Yellow,
    Red(RandomState),
}

// The HeaderMap will use a sequential search strategy until the size of the map
// exceeds this threshold.
const SEQ_SEARCH_THRESHOLD: usize = 8;

// Beyond this displacement, we switch to safe hashing or grow the table.
const DISPLACEMENT_THRESHOLD: usize = 128;
const FORWARD_SHIFT_THRESHOLD: usize = 512;

// When the map's load factor is below this threshold, we switch to safe hashing.
// Otherwise, we grow the table.
const LOAD_FACTOR_THRESHOLD: f32 = 0.2;

macro_rules! probe_loop {
    ($label:tt: $probe_var: ident < $len: expr, $body: expr) => {
        $label:
        loop {
            if $probe_var < $len {
                $body
                $probe_var += 1;
            } else {
                $probe_var = 0;
            }
        }
    };
    ($probe_var: ident < $len: expr, $body: expr) => {
        loop {
            if $probe_var < $len {
                $body
                $probe_var += 1;
            } else {
                $probe_var = 0;
            }
        }
    };
}

macro_rules! insert_phase_one {
    ($map:ident,
     $key:expr,
     $probe:ident,
     $pos:ident,
     $hash:ident,
     $danger:ident,
     $vacant:expr,
     $occupied:expr,
     $robinhood:expr) =>
    {{
        let $hash = hash_elem_using(&$map.danger, &$key);
        let mut $probe = desired_pos($map.mask as usize, $hash);
        let mut dist = 0;
        let len = $map.indices.len();
        let ret;
        // This is the probe loop
        probe_loop!('probe: $probe < len, {
            if let Some(($pos, entry_hash)) = $map.indices[$probe].resolve() {
                // if existing element probed less than us, swap
                let their_dist = probe_distance($map.mask as usize, entry_hash, $probe);

                if their_dist < dist {
                    let $danger =
                        dist >= FORWARD_SHIFT_THRESHOLD && !$map.danger.is_red();

                    ret = $robinhood;
                    break 'probe;
                } else if entry_hash == $hash && $map.entries[$pos].key == $key {
                    ret = $occupied;
                    break 'probe;
                }
            } else {
                let $danger =
                    dist >= FORWARD_SHIFT_THRESHOLD && !$map.danger.is_red();

                ret = $vacant;
                break 'probe;
            }

            dist += 1;
        });
        ret
    }}
}

// ===== impl HeaderMap =====

impl HeaderMap {
    pub fn new() -> HeaderMap {
        HeaderMap::with_capacity(0)
    }

    pub fn with_capacity(n: usize) -> HeaderMap {
        assert!(n <= MAX_SIZE, "requested capacity too large");

        if n == 0 {
            HeaderMap {
                mask: 0,
                indices: Vec::new(),
                entries: Vec::new(),
                extra_values: Vec::new(),
                danger: Danger::Green,
            }
        } else {
            let entries_cap = to_raw_capacity(n).next_power_of_two();
            let indices_cap = if entries_cap > SEQ_SEARCH_THRESHOLD {
                entries_cap
            } else {
                0
            };

            HeaderMap {
                mask: entries_cap.wrapping_sub(1) as Size,
                indices: vec![Pos::none(); indices_cap],
                entries: Vec::with_capacity(entries_cap),
                extra_values: Vec::new(),
                danger: Danger::Green,
            }
        }
    }

    /// Returns the number of headers stored in the map.
    #[inline]
    pub fn len(&self) -> usize {
        self.entries.len() + self.extra_values.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.entries.len() == 0
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.extra_values.clear();
        self.danger = Danger::Green;

        for e in self.indices.iter_mut() {
            *e = Pos::none();
        }
    }

    /// Returns the number of headers the map can hold without reallocating.
    #[inline]
    pub fn capacity(&self) -> usize {
        if self.is_scan() {
            self.capacity_scan()
        } else {
            self.capacity_hashed()
        }
    }

    #[inline]
    fn capacity_scan(&self) -> usize {
        self.entries.capacity()
    }

    #[inline]
    fn capacity_hashed(&self) -> usize {
        usable_capacity(self.indices.len())
    }

    /// Reserves capacity for at least `additional` more headers to be inserted
    /// into the `HeaderMap`.
    ///
    /// The header map may reserve more space to avoid frequent reallocations.
    ///
    /// # Panics
    ///
    /// Panics if the new allocation size overflows `usize`.
    ///
    /// # Examples
    ///
    /// ```
    /// use http::HeaderMap;
    ///
    /// let mut map = HeaderMap::new();
    /// map.reserve(10);
    /// ```
    pub fn reserve(&mut self, additional: usize) {
        if self.is_scan() {
            self.entries.reserve(additional);
        } else {
            let cap = self.entries.len()
                .checked_add(additional)
                .expect("reserve overflow");

            if cap > self.indices.len() {
                self.grow_hashed(cap.next_power_of_two());
            }
        }
    }

    /// Returns a reference to the value associated with the key.
    ///
    /// If there are multiple values associated with the key, then the first one
    /// is returned.
    pub fn get<K: ?Sized>(&self, key: &K) -> Option<&HeaderValue>
        where K: IntoHeaderName
    {
        let res = if self.is_scan() {
            key.find_scan(self).map(|i| (0, i))
        } else {
            key.find_hashed(self)
        };

        match res {
            Some((_, found)) => {
                let entry = &self.entries[found];
                Some(&entry.value)
            }
            None => None,
        }
    }

    pub fn get_all<K: ?Sized>(&self, key: &K) -> Option<ValueSet>
        where K: IntoHeaderName
    {
        let res = if self.is_scan() {
            key.find_scan(self).map(|i| (0, i))
        } else {
            key.find_hashed(self)
        };

        match res {
            Some((_, found)) => {
                Some(ValueSet {
                    map: self,
                    index: found as Size,
                })
            }
            None => None,
        }
    }

    pub fn get_all_mut<K: ?Sized>(&mut self, key: &K) -> Option<ValueSetMut>
        where K: IntoHeaderName
    {
        let res = if self.is_scan() {
            key.find_scan(self).map(|i| (0, i))
        } else {
            key.find_hashed(self)
        };

        match res {
            Some((_, found)) => {
                Some(ValueSetMut {
                    map: self,
                    index: found as Size,
                })
            }
            None => None,
        }
    }

    pub fn iter(&self) -> Iter {
        Iter {
            map: self,
            entry: 0,
            cursor: self.entries.first().map(|_| Cursor::Head),
        }
    }

    pub fn names(&self) -> Names {
        Names { inner: self.iter() }
    }

    pub fn values(&self) -> Values {
        Values { inner: self.iter() }
    }

    pub fn drain(&mut self) -> Drain {
        Drain {
            idx: 0,
            map: self as *mut _,
            lt: PhantomData,
        }
    }

    fn entry_iter(&self, idx: Size) -> EntryIter {
        use self::Cursor::*;

        let back = {
            let entry = &self.entries[idx as usize];

            entry.links
                .map(|l| Values(l.tail))
                .unwrap_or(Head)
        };

        EntryIter {
            map: self,
            index: idx,
            front: Some(Head),
            back: Some(back),
        }
    }

    #[inline]
    pub fn entry<K>(&mut self, key: K) -> Entry
        where K: IntoHeaderName,
    {
        key.entry(self)
    }

    fn entry2<K>(&mut self, key: K) -> Entry
        where K: FastHash + Into<HeaderName>,
              HeaderName: PartialEq<K>,
    {
        if self.is_scan() {
            self.entry_scan(key)
        } else {
            self.entry_hashed(key)
        }
    }

    fn entry_scan<K>(&mut self, key: K) -> Entry
        where K: FastHash + Into<HeaderName>,
              HeaderName: PartialEq<K>,
    {
        match self.find_scan(&key) {
            Some(index) => {
                Entry::Occupied(OccupiedEntry {
                    inner: ValueSetMut {
                        map: self,
                        index: index as Size,
                    },
                    probe: 0,
                })
            }
            None => {
                Entry::Vacant(VacantEntry {
                    map: self,
                    hash: HashValue(0),
                    key: key.into(),
                    probe: 0,
                    danger: false,
                })
            }
        }
    }

    fn entry_hashed<K>(&mut self, key: K) -> Entry
        where K: FastHash + Into<HeaderName>,
              HeaderName: PartialEq<K>,
    {
        insert_phase_one!(
            self,
            key,
            probe,
            pos,
            hash,
            danger,
            Entry::Vacant(VacantEntry {
                map: self,
                hash: hash,
                key: key.into(),
                probe: probe as Size,
                danger: danger,
            }),
            Entry::Occupied(OccupiedEntry {
                inner: ValueSetMut {
                    map: self,
                    index: pos as Size,
                },
                probe: probe as Size,
            }),
            Entry::Vacant(VacantEntry {
                map: self,
                hash: hash,
                key: key.into(),
                probe: probe as Size,
                danger: danger,
            }))
    }

    pub fn set<K, V>(&mut self, key: K, val: V) -> Option<DrainEntry>
        where K: IntoHeaderName,
              V: Into<HeaderValue>,
    {
        key.set(self, val.into())
    }

    fn set2<K>(&mut self, key: K, val: HeaderValue) -> Option<DrainEntry>
        where K: FastHash + Into<HeaderName>,
              HeaderName: PartialEq<K>,
    {
        if self.is_scan() {
            self.set_scan(key, val)
        } else {
            self.set_hashed(key, val)
        }
    }

    #[inline]
    fn set_scan<K>(&mut self, key: K, value: HeaderValue) -> Option<DrainEntry>
        where K: FastHash + Into<HeaderName>,
              HeaderName: PartialEq<K>,
    {
        self.reserve_one_scan();

        let old;
        let links;

        // A little misdirection to make the borrow checker happy.
        'outer:
        loop {
            // Try to find the slot for the requested key
            for (_, entry) in self.entries.iter_mut().enumerate() {
                if entry.key == key {
                    // Found the entry
                    old = mem::replace(&mut entry.value, value);
                    links = entry.links.take();

                    break 'outer;
                }
            }

            self.insert_entry(HashValue(0), key.into(), value);
            self.maybe_promote();

            return None;
        }

        Some(DrainEntry {
            map: self as *mut _,
            first: Some(old),
            next: links.map(|l| l.next),
            lt: PhantomData,
        })
    }

    /// Set an occupied bucket to the given value
    #[inline]
    fn set_occupied(&mut self, index: Size, value: HeaderValue) -> DrainEntry {
        // TODO: Looks like this is repeated code
        let old;
        let links;

        {
            let entry = &mut self.entries[index as usize];

            old = mem::replace(&mut entry.value, value);
            links = entry.links.take();
        }

        DrainEntry {
            map: self as *mut _,
            first: Some(old),
            next: links.map(|l| l.next),
            lt: PhantomData,
        }
    }

    #[inline]
    fn set_hashed<K>(&mut self, key: K, value: HeaderValue) -> Option<DrainEntry>
        where K: FastHash + Into<HeaderName>,
              HeaderName: PartialEq<K>,
    {
        self.reserve_one_hashed();

        insert_phase_one!(
            self, key, probe, pos, hash, danger,
            // Vacant
            {
                drop(danger); // Make lint happy
                let index = self.entries.len();
                self.insert_entry(hash, key.into(), value);
                self.indices[probe] = Pos::new(index as Size, hash);
                None
            },
            // Occupied
            Some(self.set_occupied(pos as Size, value)),
            // Robinhood
            {
                self.insert_phase_two(
                    key.into(),
                    value,
                    hash,
                    probe as Size,
                    danger);
                None
            })
    }

    /// Inserts a header into the map without removing any values
    ///
    /// Returns `true` if `key` has not previously been stored in the map
    pub fn insert<K, V>(&mut self, key: K, val: V) -> bool
        where K: IntoHeaderName,
              V: Into<HeaderValue>,
    {
        key.insert(self, val.into())
    }

    #[inline]
    fn insert2<K>(&mut self, key: K, val: HeaderValue) -> bool
        where K: FastHash + Into<HeaderName>,
              HeaderName: PartialEq<K>,
    {
        if self.is_scan() {
            self.insert_scan(key, val)
        } else {
            self.insert_hashed(key, val)
        }
    }

    #[inline]
    fn insert_scan<K>(&mut self, key: K, value: HeaderValue) -> bool
        where K: FastHash + Into<HeaderName>,
              HeaderName: PartialEq<K>,
    {
        self.reserve_one_scan();

        // Try to find the slot for the requested key
        for (idx, entry) in self.entries.iter_mut().enumerate() {
            if entry.key == key {
                insert_value(idx, entry, &mut self.extra_values, value);
                return true;
            }
        }

        self.insert_entry(HashValue(0), key.into(), value);
        self.maybe_promote();

        false
    }

    #[inline]
    fn insert_hashed<K>(&mut self, key: K, value: HeaderValue) -> bool
        where K: FastHash + Into<HeaderName>,
              HeaderName: PartialEq<K>,
    {
        self.reserve_one_hashed();

        insert_phase_one!(
            self, key, probe, pos, hash, danger,
            // Vacant
            {
                drop(danger);
                let index = self.entries.len();
                self.insert_entry(hash, key.into(), value);
                self.indices[probe] = Pos::new(index as Size, hash);
                false
            },
            // Occupied
            {
                insert_value(pos, &mut self.entries[pos], &mut self.extra_values, value);
                true
            },
            // Robinhood
            {
                self.insert_phase_two(
                    key.into(),
                    value,
                    hash,
                    probe as Size,
                    danger);

                false
            })
    }

    #[inline]
    fn find_scan<K: ?Sized>(&self, key: &K) -> Option<usize>
        where HeaderName: PartialEq<K>
    {
        for (i, entry) in self.entries.iter().enumerate() {
            if entry.key == *key {
                return Some(i);
            }
        }

        None
    }

    #[inline]
    fn find_hashed<K: ?Sized>(&self, key: &K) -> Option<(usize, usize)>
        where K: FastHash + Into<HeaderName>,
              HeaderName: PartialEq<K>,
    {
        let h = hash_elem_using(&self.danger, key);
        self.find_using(h, move |entry| {
            entry.key == *key
        })
    }

    #[inline]
    fn find_using<F>(&self, hash: HashValue, key_eq: F) -> Option<(usize, usize)>
        where F: Fn(&Bucket) -> bool,
    {
        debug_assert!(self.entries.len() > 0);

        let mask = self.mask as usize;
        let mut probe = desired_pos(mask, hash);
        let mut dist = 0;

        probe_loop!(probe < self.indices.len(), {
            if let Some((i, entry_hash)) = self.indices[probe].resolve() {
                if dist > probe_distance(mask, entry_hash, probe) {
                    // give up when probe distance is too long
                    return None;
                } else if entry_hash == hash && key_eq(&self.entries[i]) {
                    return Some((probe, i));
                }
            } else {
                return None;
            }

            dist += 1;
        });
    }

    /// phase 2 is post-insert where we forward-shift `Pos` in the indices.
    ///
    /// This phase only needs to happen if currently in hashed mode
    #[inline]
    fn insert_phase_two(&mut self,
                        key: HeaderName,
                        value: HeaderValue,
                        hash: HashValue,
                        probe: Size,
                        danger: bool) -> usize
    {
        debug_assert!(!self.is_scan());

        // Push the value and get the index
        let index = self.entries.len();
        self.insert_entry(hash, key, value);

        let num_displaced = do_insert_phase_two(
            &mut self.indices,
            probe,
            Pos::new(index as Size, hash));

        if danger || num_displaced >= DISPLACEMENT_THRESHOLD {
            // Increase danger level
            self.danger.to_yellow();
        }

        index
    }

    pub fn remove<K>(&mut self, key: &K) -> Option<DrainEntry>
        where K: IntoHeaderName
    {
        self.remove_entry(key).map(|e| e.1)
    }

    pub fn remove_entry<K>(&mut self, key: &K) -> Option<(HeaderName, DrainEntry)>
        where K: IntoHeaderName
    {
        if self.is_scan() {
            match key.find_scan(self) {
                Some(idx) => {
                    Some(self.remove_found_scan(idx))
                }
                None => None,
            }
        } else {
            match key.find_hashed(self) {
                Some((probe, idx)) => {
                    Some(self.remove_found_hashed(probe, idx))
                }
                None => None,
            }
        }
    }

    /// Remove an entry from the map while in sequential mode
    #[inline]
    fn remove_found_scan(&mut self, index: usize) -> (HeaderName, DrainEntry) {
        let entry = self.entries.swap_remove(index);

        let drain = DrainEntry {
            map: self as *mut _,
            first: Some(entry.value),
            next: entry.links.map(|l| l.next),
            lt: PhantomData,
        };

        (entry.key, drain)
    }

    /// Remove an entry from the map while in hashed mode
    #[inline]
    fn remove_found_hashed(&mut self, probe: usize, found: usize) -> (HeaderName, DrainEntry) {
        // index `probe` and entry `found` is to be removed
        // use swap_remove, but then we need to update the index that points
        // to the other entry that has to move
        self.indices[probe] = Pos::none();
        let entry = self.entries.swap_remove(found);

        // correct index that points to the entry that had to swap places
        if let Some(entry) = self.entries.get(found) {
            // was not last element
            // examine new element in `found` and find it in indices
            let mut probe = desired_pos(self.mask as usize, entry.hash.get());

            probe_loop!(probe < self.indices.len(), {
                if let Some((i, _)) = self.indices[probe].resolve() {
                    if i >= self.entries.len() {
                        // found it
                        self.indices[probe] = Pos::new(found as Size, entry.hash.get());
                        break;
                    }
                }
            });
        }

        // backward shift deletion in self.indices
        // after probe, shift all non-ideally placed indices backward
        if self.entries.len() > 0 {
            let mut last_probe = probe;
            let mut probe = probe + 1;

            probe_loop!(probe < self.indices.len(), {
                if let Some((_, entry_hash)) = self.indices[probe].resolve() {
                    if probe_distance(self.mask as usize, entry_hash, probe) > 0 {
                        self.indices[last_probe] = self.indices[probe];
                        self.indices[probe] = Pos::none();
                    } else {
                        break;
                    }
                } else {
                    break;
                }

                last_probe = probe;
            });
        }

        let drain = DrainEntry {
            map: self as *mut _,
            first: Some(entry.value),
            next: entry.links.map(|l| l.next),
            lt: PhantomData,
        };

        (entry.key, drain)
    }

    /// Removes the `ExtraValue` at the given index.
    #[inline]
    fn remove_extra_value(&mut self, idx: usize) -> ExtraValue {
        {
            let extra = &self.extra_values[idx];

            // First unlink the extra value
            match extra.prev.get() {
                Link::Entry(entry_idx) => {
                    // Set the link head to the next value
                    match extra.next.get() {
                        Link::Entry(_) => {
                            // This is the only extra value, so unset the entry
                            // links
                            self.entries[entry_idx as usize].links = None;
                        }
                        Link::Extra(extra_idx) => {
                            self.entries[entry_idx as usize].links.as_mut().unwrap()
                                .next = extra_idx;
                        }
                    }
                }
                Link::Extra(extra_idx) => {
                    self.extra_values[extra_idx as usize].next.set(extra.next.get());
                }
            }

            match extra.next.get() {
                Link::Entry(entry_idx) => {
                    match extra.prev.get() {
                        // Nothing to do, this was already handled above
                        Link::Entry(_) => {}
                        Link::Extra(extra_idx) => {
                            self.entries[entry_idx as usize].links.as_mut().unwrap()
                                .tail = extra_idx;
                        }
                    }
                }
                Link::Extra(extra_idx) => {
                    self.extra_values[extra_idx as usize].prev.set(extra.prev.get());
                }
            }
        }

        // Remove the extra value
        let extra = self.extra_values.swap_remove(idx);

        // This is the index of the value that was moved (possibly `extra`)
        let old_idx = self.extra_values.len() as Size;

        // Check if another entry was displaced. If it was, then the links
        // need to be fixed.
        if let Some(moved) = self.extra_values.get(idx) {
            // An entry was moved, we have to the links
            match moved.prev.get() {
                Link::Entry(entry_idx) => {
                    // It is critical that we do not attempt to read the
                    // header name or value as that memory may have been
                    // "released" already.
                    let links = self.entries[entry_idx as usize].links.as_mut().unwrap();
                    links.next = idx as Size;
                }
                Link::Extra(extra_idx) => {
                    self.extra_values[extra_idx as usize].next.set(Link::Extra(idx as Size));
                }
            }

            match moved.next.get() {
                Link::Entry(entry_idx) => {
                    let links = self.entries[entry_idx as usize].links.as_mut().unwrap();
                    links.tail = idx as Size;
                }
                Link::Extra(extra_idx) => {
                    self.extra_values[extra_idx as usize].prev.set(Link::Extra(idx as Size));
                }
            }
        }

        // Finally, update the links in `extra`
        if extra.prev.get() == Link::Extra(old_idx) {
            extra.prev.set(Link::Extra(idx as Size));
        }

        if extra.next.get() == Link::Extra(old_idx) {
            extra.next.set(Link::Extra(idx as Size));
        }

        extra
    }

    #[inline]
    fn insert_entry(&mut self, hash: HashValue, key: HeaderName, value: HeaderValue) {
        assert!(self.entries.len() < MAX_SIZE as usize, "header map at capacity");

        self.entries.push(Bucket {
            hash: Cell::new(hash),
            key: key,
            value: value,
            links: None,
        });
    }

    #[inline]
    fn maybe_promote(&mut self) {
        if self.entries.len() == (SEQ_SEARCH_THRESHOLD + 1) {
            let cap = cmp::max(
                SEQ_SEARCH_THRESHOLD << 1,
                self.entries.capacity().next_power_of_two());

            // Initialze the indices
            self.indices = vec![Pos::none(); cap];

            // Rebuild the table
            self.rebuild();
        }
    }

    fn rebuild(&mut self) {
        // This path should only be hit in hashed mode
        debug_assert!(!self.is_scan());

        // Loop over all entries and re-insert them into the map
        for entry in &self.entries {
            let hash = hash_elem_using(&self.danger, &entry.key);
            let mut probe = desired_pos(self.mask as usize, hash);
            let mut dist = 0;

            probe_loop!(probe < self.indices.len(), {
                if let Some((_, entry_hash)) = self.indices[probe].resolve() {
                    // if existing element probed less than us, swap
                    let their_dist = probe_distance(self.mask as usize, entry_hash, probe);

                    if their_dist < dist {
                        break;
                    }
                } else {
                    break;
                }

                dist += 1;
            });

            entry.hash.set(hash);

            do_insert_phase_two(
                &mut self.indices,
                probe as Size,
                Pos::new(probe as Size, hash));
        }
    }

    fn reinsert_entry_in_order(&mut self, pos: Pos) {
        // This path should only be hit in scan mode
        debug_assert!(!self.is_scan());

        if let Some((_, entry_hash)) = pos.resolve() {
            // Find first empty bucket and insert there
            let mut probe = desired_pos(self.mask as usize, entry_hash);

            probe_loop!(probe < self.indices.len(), {
                if self.indices[probe].resolve().is_none() {
                    // empty bucket, insert here
                    self.indices[probe] = pos;
                    return;
                }
            });
        }
    }

    #[inline]
    fn reserve_one_scan(&mut self) {
        debug_assert!(self.danger.is_green());

        if self.entries.len() == self.capacity_scan() {
            self.double_capacity_scan();
        }
    }

    #[inline]
    fn reserve_one_hashed(&mut self) {
        if self.danger.is_yellow() {
            debug_assert!(!self.is_scan());

            let load_factor = self.entries.len() as f32 / self.indices.len() as f32;

            if load_factor >= LOAD_FACTOR_THRESHOLD {
                self.danger.to_green();
                self.double_capacity_hashed();
            } else {
                self.danger.to_red();

                // Rebuild hash table
                for index in &mut self.indices {
                    *index = Pos::none();
                }

                self.rebuild();
            }
        } else if self.entries.len() == self.capacity_hashed() {
            self.double_capacity_hashed();
        }
    }


    /// Double the HeaderMap capacity while currently in scan mode
    #[inline]
    fn double_capacity_scan(&mut self) {
        let len = self.entries.len();

        if len == 0 {
            let capacity = 8usize;

            // Make sure the hash map stays in the sequential search threshold
            debug_assert!(capacity <= SEQ_SEARCH_THRESHOLD);

            self.entries = Vec::with_capacity(capacity);
        } else {
            // Double the capacity
            self.entries.reserve(len);
        }
    }

    #[inline]
    fn double_capacity_hashed(&mut self) {
        let cap = self.indices.len().checked_mul(2).expect("grow overflow");
        self.grow_hashed(cap);
    }

    #[inline]
    fn grow_hashed(&mut self, new_raw_cap: usize) {
        // This path can never be reached when handling the first allocation in
        // the map.
        debug_assert!(self.entries.len() > 0);

        // find first ideally placed element -- start of cluster
        let mut first_ideal = 0;

        for (i, pos) in self.indices.iter().enumerate() {
            if let Some((_, entry_hash)) = pos.resolve() {
                if 0 == probe_distance(self.mask as usize, entry_hash, i) {
                    first_ideal = i;
                    break;
                }
            }
        }

        // visit the entries in an order where we can simply reinsert them
        // into self.indices without any bucket stealing.
        let old_indices = mem::replace(&mut self.indices, vec![Pos::none(); new_raw_cap]);
        self.mask = new_raw_cap.wrapping_sub(1) as Size;

        for &pos in &old_indices[first_ideal..] {
            self.reinsert_entry_in_order(pos);
        }

        for &pos in &old_indices[..first_ideal] {
            self.reinsert_entry_in_order(pos);
        }

        // Reserve additional entry slots
        let more = self.capacity_hashed() - self.entries.len();
        self.entries.reserve(more);
    }

    #[inline]
    fn is_scan(&self) -> bool {
        self.indices.len() == 0
    }
}

impl<K, V> FromIterator<(K, V)> for HeaderMap
    where K: IntoHeaderName,
          V: Into<HeaderValue>,
{
    fn from_iter<T>(iter: T) -> Self
        where T: IntoIterator<Item = (K, V)>
    {
       let mut map = HeaderMap::new();
       map.extend(iter);
       map
    }
}

impl<'a, K, V> FromIterator<&'a (K, V)> for HeaderMap
    where K: 'a + IntoHeaderName,
          V: 'a,
          for<'b> &'b V: Into<HeaderValue>,
{
    fn from_iter<T>(iter: T) -> Self
        where T: IntoIterator<Item = &'a (K, V)>
    {
       let mut map = HeaderMap::new();
       map.extend(iter);
       map
    }
}

impl<K, V> Extend<(K, V)> for HeaderMap
    where K: IntoHeaderName,
          V: Into<HeaderValue>,
{
    fn extend<T: IntoIterator<Item = (K, V)>>(&mut self, iter: T) {
        // Keys may be already present or show multiple times in the iterator.
        // Reserve the entire hint lower bound if the map is empty.
        // Otherwise reserve half the hint (rounded up), so the map
        // will only resize twice in the worst case.
        let iter = iter.into_iter();

        let reserve = if self.is_empty() {
            iter.size_hint().0
        } else {
            (iter.size_hint().0 + 1) / 2
        };

        self.reserve(reserve);

        for (k, v) in iter {
            self.insert(k, v);
        }
    }
}

impl<'a, K, V> Extend<&'a (K, V)> for HeaderMap
    where K: 'a + IntoHeaderName,
          V: 'a,
          for<'b> &'b V: Into<HeaderValue>,
{
    fn extend<T: IntoIterator<Item = &'a (K, V)>>(&mut self, iter: T) {
        // Keys may be already present or show multiple times in the iterator.
        // Reserve the entire hint lower bound if the map is empty.
        // Otherwise reserve half the hint (rounded up), so the map
        // will only resize twice in the worst case.
        let iter = iter.into_iter();

        let reserve = if self.is_empty() {
            iter.size_hint().0
        } else {
            (iter.size_hint().0 + 1) / 2
        };

        self.reserve(reserve);

        for &(ref k, ref v) in iter {
            k.insert_ref(self, v.into());
        }
    }
}

/// phase 2 is post-insert where we forward-shift `Pos` in the indices.
///
/// returns the number of displaced elements
#[inline]
fn do_insert_phase_two(indices: &mut Vec<Pos>,
          probe: Size,
          mut old_pos: Pos)
    -> usize
{
    let mut probe = probe as usize;

    let mut num_displaced = 0;

    probe_loop!(probe < indices.len(), {
        let pos = &mut indices[probe];

        if pos.is_none() {
            *pos = old_pos;
            break;
        } else {
            num_displaced += 1;
            old_pos = mem::replace(pos, old_pos);
        }
    });

    num_displaced
}

#[inline]
fn insert_value(entry_idx: usize,
                entry: &mut Bucket,
                extra: &mut Vec<ExtraValue>,
                value: HeaderValue)
{
    match entry.links {
        Some(links) => {
            let idx = extra.len() as Size;
            extra.push(ExtraValue {
                value: value,
                prev: Cell::new(Link::Extra(links.tail)),
                next: Cell::new(Link::Entry(entry_idx as Size)),
            });

            entry.links = Some(Links {
                tail: idx,
                .. links
            });
        }
        None => {
            let idx = extra.len() as Size;
            extra.push(ExtraValue {
                value: value,
                prev: Cell::new(Link::Entry(entry_idx as Size)),
                next: Cell::new(Link::Entry(entry_idx as Size)),
            });

            entry.links = Some(Links {
                next: idx,
                tail: idx,
            });
        }
    }
}

// ===== impl Iter =====

impl<'a> Iterator for Iter<'a> {
    type Item = (&'a HeaderName, &'a HeaderValue);

    fn next(&mut self) -> Option<Self::Item> {
        use self::Cursor::*;

        if self.cursor.is_none() {
            if (self.entry + 1) as usize >= self.map.entries.len() {
                return None;
            }

            self.entry += 1;
            self.cursor = Some(Cursor::Head);
        }

        let entry = &self.map.entries[self.entry as usize];

        match self.cursor.unwrap() {
            Head => {
                self.cursor = entry.links.map(|l| Values(l.next));
                Some((&entry.key, &entry.value))
            }
            Values(idx) => {
                let idx = idx as usize;
                let extra = &self.map.extra_values[idx as usize];

                match extra.next.get() {
                    Link::Entry(_) => self.cursor = None,
                    Link::Extra(i) => self.cursor = Some(Values(i)),
                }

                Some((&entry.key, &extra.value))
            }
        }
    }
}

// ===== impl Names =====

impl<'a> Iterator for Names<'a> {
    type Item = &'a HeaderName;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(n, _)| n)
    }
}

// ===== impl Values ====

impl<'a> Iterator for Values<'a> {
    type Item = &'a HeaderValue;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(_, v)| v)
    }
}

// ===== impl Drain =====

impl<'a> Iterator for Drain<'a> {
    type Item = (HeaderName, DrainEntry<'a>);

    fn next(&mut self) -> Option<Self::Item> {
        let idx = self.idx;

        if idx == unsafe { (*self.map).entries.len() } {
            return None;
        }

        self.idx += 1;

        let name;
        let value;
        let next;

        unsafe {
            let entry = &(*self.map).entries[idx];

            // Read the header name
            name = ptr::read(&entry.key as *const _);
            value = ptr::read(&entry.value as *const _);
            next = entry.links.map(|l| l.next);
        };

        let values = DrainEntry {
            map: self.map,
            first: Some(value),
            next: next,
            lt: PhantomData,
        };

        Some((name, values))
    }
}

impl<'a> Drop for Drain<'a> {
    fn drop(&mut self) {
        unsafe {
            let map = &mut *self.map;
            debug_assert!(map.extra_values.is_empty());
            map.entries.set_len(0);
        }
    }
}

// ===== impl VacantEntry =====

impl<'a> VacantEntry<'a> {
    #[inline]
    pub fn name(&self) -> &HeaderName {
        &self.key
    }

    #[inline]
    pub fn into_name(self) -> HeaderName {
        self.key
    }

    pub fn set<T: Into<HeaderValue>>(self, value: T) -> &'a mut HeaderValue {
        let index = if self.map.is_scan() {
            let index = self.map.entries.len();
            self.map.insert_entry(self.hash, self.key, value.into());

            self.map.maybe_promote();
            index
        } else {
            self.map.insert_phase_two(
                self.key,
                value.into(),
                self.hash,
                self.probe,
                self.danger)
        };

        &mut self.map.entries[index].value
    }
}


// ===== impl ValueSet =====

impl<'a> ValueSet<'a> {
    /// Get a reference to the header name.
    #[inline]
    pub fn name(&self) -> &HeaderName {
        &self.map.entries[self.index as usize].key
    }

    /// Get a reference to the first value in the set.
    #[inline]
    pub fn first(&self) -> &HeaderValue {
        &self.map.entries[self.index as usize].value
    }

    /// Get a reference to the last value in the set.
    #[inline]
    pub fn last(&self) -> &HeaderValue {
        let entry = &self.map.entries[self.index as usize];

        match entry.links {
            Some(links) => {
                let extra = &self.map.extra_values[links.tail as usize];
                &extra.value
            }
            None => &entry.value
        }
    }

    #[inline]
    pub fn iter(&self) -> EntryIter {
        self.into_iter()
    }
}

impl<'a> IntoIterator for ValueSet<'a> {
    type Item = &'a HeaderValue;
    type IntoIter = EntryIter<'a>;

    #[inline]
    fn into_iter(self) -> EntryIter<'a> {
        self.map.entry_iter(self.index)
    }
}

impl<'a, 'b: 'a> IntoIterator for &'b ValueSet<'a> {
    type Item = &'a HeaderValue;
    type IntoIter = EntryIter<'a>;

    #[inline]
    fn into_iter(self) -> EntryIter<'a> {
        self.map.entry_iter(self.index)
    }
}

// ===== impl EntryIter =====

impl<'a> Iterator for EntryIter<'a> {
    type Item = &'a HeaderValue;

    fn next(&mut self) -> Option<Self::Item> {
        use self::Cursor::*;

        let entry = &self.map.entries[self.index as usize];

        match self.front {
            Some(Head) => {
                if self.back == Some(Head) {
                    self.front = None;
                    self.back = None;
                } else {
                    // Update the iterator state
                    match entry.links {
                        Some(links) => {
                            self.front = Some(Values(links.next));
                        }
                        None => unreachable!(),
                    }
                }

                Some(&entry.value)
            }
            Some(Values(idx)) => {
                let extra = &self.map.extra_values[idx as usize];

                if self.front == self.back {
                    self.front = None;
                    self.back = None;
                } else {
                    match extra.next.get() {
                        Link::Entry(_) => self.front = None,
                        Link::Extra(i) => self.front = Some(Values(i)),
                    }
                }

                Some(&extra.value)
            }
            None => None,
        }
    }
}

impl<'a> DoubleEndedIterator for EntryIter<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        use self::Cursor::*;

        let entry = &self.map.entries[self.index as usize];

        match self.back {
            Some(Head) => {
                self.front = None;
                self.back = None;
                Some(&entry.value)
            }
            Some(Values(idx)) => {
                let extra = &self.map.extra_values[idx as usize];

                if self.front == self.back {
                    self.front = None;
                    self.back = None;
                } else {
                    match extra.prev.get() {
                        Link::Entry(_) => self.back = Some(Head),
                        Link::Extra(idx) => self.back = Some(Values(idx)),
                    }
                }

                Some(&extra.value)
            }
            None => None,
        }
    }
}

// ===== impl ValueSetMut =====

impl<'a> ValueSetMut<'a> {
    /// Get a reference to the header name.
    #[inline]
    pub fn name(&self) -> &HeaderName {
        &self.map.entries[self.index as usize].key
    }

    /// Get a reference to the first header value in the entry.
    ///
    /// # Panics
    ///
    /// Panics if there are no values for the entry.
    #[inline]
    pub fn first(&self) -> &HeaderValue {
        &self.map.entries[self.index as usize].value
    }

    /// Get a mutable reference to the first header value in the entry.
    #[inline]
    pub fn first_mut(&mut self) -> &mut HeaderValue {
        &mut self.map.entries[self.index as usize].value
    }

    /// Get a reference to the last header value in this entry.
    #[inline]
    pub fn last(&self) -> &HeaderValue {
        let entry = &self.map.entries[self.index as usize];

        match entry.links {
            Some(links) => {
                let extra = &self.map.extra_values[links.tail as usize];
                &extra.value
            }
            None => &entry.value
        }
    }

    /// Get a mutable reference to the last header value in this entry.
    #[inline]
    pub fn last_mut(&mut self) -> &mut HeaderValue {
        let entry = &mut self.map.entries[self.index as usize];

        match entry.links {
            Some(links) => {
                let extra = &mut self.map.extra_values[links.tail as usize];
                &mut extra.value
            }
            None => &mut entry.value
        }
    }

    /// Replaces all values for this entry with the provided value.
    #[inline]
    pub fn set<T: Into<HeaderValue>>(&mut self, value: T) -> DrainEntry {
        self.map.set_occupied(self.index, value.into())
    }

    pub fn insert<T: Into<HeaderValue>>(&mut self, value: T) {
        let idx = self.index as usize;
        let entry = &mut self.map.entries[idx];
        insert_value(idx, entry, &mut self.map.extra_values, value.into());
    }

    #[inline]
    pub fn iter(&self) -> EntryIter {
        self.into_iter()
    }
}

impl<'a> IntoIterator for ValueSetMut<'a> {
    type Item = &'a HeaderValue;
    type IntoIter = EntryIter<'a>;

    #[inline]
    fn into_iter(self) -> EntryIter<'a> {
        self.map.entry_iter(self.index)
    }
}

impl<'a, 'b: 'a> IntoIterator for &'b ValueSetMut<'a> {
    type Item = &'a HeaderValue;
    type IntoIter = EntryIter<'a>;

    #[inline]
    fn into_iter(self) -> EntryIter<'a> {
        self.map.entry_iter(self.index)
    }
}

impl<'a, 'b: 'a> IntoIterator for &'b mut ValueSetMut<'a> {
    type Item = &'a HeaderValue;
    type IntoIter = EntryIter<'a>;

    #[inline]
    fn into_iter(self) -> EntryIter<'a> {
        self.map.entry_iter(self.index)
    }
}

// ===== impl OccupiedEntry =====

impl<'a> OccupiedEntry<'a> {
    /// Get a reference to the header name in the entry.
    #[inline]
    pub fn name(&self) -> &HeaderName {
        self.inner.name()
    }

    /// Get a reference to the first header value in the entry.
    ///
    /// # Panics
    ///
    /// Panics if there are no values for the entry.
    #[inline]
    pub fn first(&self) -> &HeaderValue {
        self.inner.first()
    }

    /// Get a mutable reference to the first header value in the entry.
    #[inline]
    pub fn first_mut(&mut self) -> &mut HeaderValue {
        self.inner.first_mut()
    }

    /// Get a reference to the last header value in this entry.
    #[inline]
    pub fn last(&self) -> &HeaderValue {
        self.inner.last()
    }

    /// Get a mutable reference to the last header value in this entry.
    #[inline]
    pub fn last_mut(&mut self) -> &mut HeaderValue {
        self.inner.last_mut()
    }

    /// Replaces all values for this entry with the provided value.
    #[inline]
    pub fn set<T: Into<HeaderValue>>(&mut self, value: T) -> DrainEntry {
        self.inner.set(value)
    }

    pub fn insert<T: Into<HeaderValue>>(&mut self, value: T) {
        self.inner.insert(value)
    }

    pub fn remove(self) -> DrainEntry<'a> {
        self.remove_entry().1
    }

    pub fn remove_entry(self) -> (HeaderName, DrainEntry<'a>) {
        if self.inner.map.is_scan() {
            self.inner.map.remove_found_scan(
                self.inner.index as usize)
        } else {
            self.inner.map.remove_found_hashed(
                self.probe as usize,
                self.inner.index as usize)
        }
    }
}

// ===== impl DrainEntry =====

impl<'a> Iterator for DrainEntry<'a> {
    type Item = HeaderValue;

    #[inline]
    fn next(&mut self) -> Option<HeaderValue> {
        if self.first.is_some() {
            self.first.take()
        } else if let Some(next) = self.next {
            // Remove the extra value
            let extra = unsafe { &mut (*self.map) }.remove_extra_value(next as usize);

            match extra.next.get() {
                Link::Extra(idx) => self.next = Some(idx),
                Link::Entry(_) => self.next = None,
            }

            Some(extra.value)
        } else {
            None
        }
    }
}

impl<'a> Drop for DrainEntry<'a> {
    fn drop(&mut self) {
        while let Some(_) = self.next() {
        }
    }
}

// ===== impl Pos =====

impl Pos {
    #[inline]
    fn new(index: Size, hash: HashValue) -> Self {
        Pos {
            index: index,
            hash: hash,
        }
    }

    #[inline]
    fn none() -> Self {
        Pos {
            index: !0,
            hash: HashValue(0),
        }
    }

    #[inline]
    fn is_some(&self) -> bool {
        !self.is_none()
    }

    #[inline]
    fn is_none(&self) -> bool {
        self.index == !0
    }

    #[inline]
    fn resolve(&self) -> Option<(usize, HashValue)> {
        if self.is_some() {
            Some((self.index as usize, self.hash))
        } else {
            None
        }
    }
}

impl Danger {
    fn is_red(&self) -> bool {
        match *self {
            Danger::Red(_) => true,
            _ => false,
        }
    }

    fn to_red(&mut self) {
        debug_assert!(self.is_yellow());
        *self = Danger::Red(RandomState::new());
    }

    fn is_yellow(&self) -> bool {
        match *self {
            Danger::Yellow => true,
            _ => false,
        }
    }

    fn to_yellow(&mut self) {
        match *self {
            Danger::Green => {
                *self = Danger::Yellow;
            }
            _ => {}
        }
    }

    fn is_green(&self) -> bool {
        match *self {
            Danger::Green => true,
            _ => false,
        }
    }

    fn to_green(&mut self) {
        debug_assert!(self.is_yellow());
        *self = Danger::Green;
    }
}

// ===== impl Utils =====

#[inline]
fn usable_capacity(cap: usize) -> usize {
    cap - cap / 4
}

#[inline]
fn to_raw_capacity(n: usize) -> usize {
    n + n / 3
}

#[inline]
fn desired_pos(mask: usize, hash: HashValue) -> usize {
    hash.0 as usize & mask
}

/// The number of steps that `current` is forward of the desired position for hash
#[inline]
fn probe_distance(mask: usize, hash: HashValue, current: usize) -> usize {
    current.wrapping_sub(desired_pos(mask, hash)) & mask
}

#[inline]
fn hash_elem_using<K: ?Sized>(danger: &Danger, k: &K) -> HashValue
    where K: FastHash
{
    const MASK: u64 = MAX_SIZE as u64;

    let hash = match *danger {
        // Safe hash
        Danger::Red(ref hasher) => {
            let mut h = hasher.build_hasher();
            k.hash(&mut h);
            h.finish()
        }
        // Fast hash
        _ => {
            k.fast_hash()
        }
    };

    HashValue((hash & MASK) as Size)
}

/*
 *
 * ===== impl IntoHeaderName =====
 *
 */

/// A marker trait used to identify values that can be used as keys to a
/// `HeaderMap`.
pub trait IntoHeaderName: Sealed {
    #[doc(hidden)]
    fn set(self, map: &mut HeaderMap, val: HeaderValue) -> Option<DrainEntry>
        where Self: Sized
    {
        drop(map);
        drop(val);
        unimplemented!();
    }

    #[doc(hidden)]
    fn insert(self, map: &mut HeaderMap, val: HeaderValue) -> bool
        where Self: Sized
    {
        drop(map);
        drop(val);
        unimplemented!();
    }

    #[doc(hidden)]
    fn insert_ref(&self, map: &mut HeaderMap, val: HeaderValue);

    #[doc(hidden)]
    fn entry(self, map: &mut HeaderMap) -> Entry where Self: Sized {
        drop(map);
        unimplemented!();
    }

    #[doc(hidden)]
    fn find_scan(&self, map: &HeaderMap) -> Option<usize>;

    #[doc(hidden)]
    fn find_hashed(&self, map: &HeaderMap) -> Option<(usize, usize)>;
}

// Prevent users from implementing the `IntoHeaderName` trait.
pub trait Sealed {}

impl IntoHeaderName for HeaderName {
    #[doc(hidden)]
    #[inline]
    fn set(self, map: &mut HeaderMap, val: HeaderValue) -> Option<DrainEntry> {
        map.set2(self, val)
    }

    #[doc(hidden)]
    #[inline]
    fn insert(self, map: &mut HeaderMap, val: HeaderValue) -> bool {
        map.insert2(self, val)
    }

    #[doc(hidden)]
    #[inline]
    fn insert_ref(&self, map: &mut HeaderMap, val: HeaderValue) {
        map.insert2(self, val);
    }

    #[doc(hidden)]
    #[inline]
    fn entry(self, map: &mut HeaderMap) -> Entry {
        map.entry2(self)
    }

    #[doc(hidden)]
    #[inline]
    fn find_scan(&self, map: &HeaderMap) -> Option<usize> {
        map.find_scan(self)
    }

    #[doc(hidden)]
    #[inline]
    fn find_hashed(&self, map: &HeaderMap) -> Option<(usize, usize)> {
        map.find_hashed(self)
    }
}

impl Sealed for HeaderName {}

impl<'a> IntoHeaderName for &'a HeaderName {
    #[doc(hidden)]
    #[inline]
    fn set(self, map: &mut HeaderMap, val: HeaderValue) -> Option<DrainEntry> {
        map.set2(self, val)
    }

    #[doc(hidden)]
    #[inline]
    fn insert(self, map: &mut HeaderMap, val: HeaderValue) -> bool {
        map.insert2(self, val)
    }

    #[doc(hidden)]
    #[inline]
    fn insert_ref(&self, map: &mut HeaderMap, val: HeaderValue) {
        map.insert2(*self, val);
    }

    #[doc(hidden)]
    #[inline]
    fn entry(self, map: &mut HeaderMap) -> Entry {
        map.entry2(self)
    }

    #[doc(hidden)]
    #[inline]
    fn find_scan(&self, map: &HeaderMap) -> Option<usize> {
        map.find_scan(*self)
    }

    #[doc(hidden)]
    #[inline]
    fn find_hashed(&self, map: &HeaderMap) -> Option<(usize, usize)> {
        map.find_hashed(*self)
    }
}

impl<'a> Sealed for &'a HeaderName {}

impl IntoHeaderName for str {
    #[doc(hidden)]
    #[inline]
    fn insert_ref(&self, map: &mut HeaderMap, val: HeaderValue) {
        HdrName::from_bytes(self.as_bytes(), move |hdr| map.insert2(hdr, val)).unwrap();
    }

    #[doc(hidden)]
    #[inline]
    fn find_scan(&self, map: &HeaderMap) -> Option<usize> {
        HdrName::from_bytes(self.as_bytes(), |hdr| map.find_scan(&hdr)).unwrap()
    }

    #[doc(hidden)]
    #[inline]
    fn find_hashed(&self, map: &HeaderMap) -> Option<(usize, usize)> {
        HdrName::from_bytes(self.as_bytes(), |hdr| map.find_hashed(&hdr)).unwrap()
    }
}

impl Sealed for str {}

impl<'a> IntoHeaderName for &'a str {
    #[doc(hidden)]
    #[inline]
    fn set(self, map: &mut HeaderMap, val: HeaderValue) -> Option<DrainEntry> {
        HdrName::from_bytes(self.as_bytes(), move |hdr| map.set2(hdr, val)).unwrap()
    }

    #[doc(hidden)]
    #[inline]
    fn insert(self, map: &mut HeaderMap, val: HeaderValue) -> bool {
        HdrName::from_bytes(self.as_bytes(), move |hdr| map.insert2(hdr, val)).unwrap()
    }

    #[doc(hidden)]
    #[inline]
    fn insert_ref(&self, map: &mut HeaderMap, val: HeaderValue) {
        HdrName::from_bytes(self.as_bytes(), move |hdr| map.insert2(hdr, val)).unwrap();
    }

    #[doc(hidden)]
    #[inline]
    fn entry(self, map: &mut HeaderMap) -> Entry {
        HdrName::from_bytes(self.as_bytes(), move |hdr| map.entry2(hdr)).unwrap()
    }

    #[doc(hidden)]
    #[inline]
    fn find_scan(&self, map: &HeaderMap) -> Option<usize> {
        HdrName::from_bytes(self.as_bytes(), |hdr| map.find_scan(&hdr)).unwrap()
    }

    #[doc(hidden)]
    #[inline]
    fn find_hashed(&self, map: &HeaderMap) -> Option<(usize, usize)> {
        HdrName::from_bytes(self.as_bytes(), |hdr| map.find_hashed(&hdr)).unwrap()
    }
}

impl<'a> Sealed for &'a str {}

impl IntoHeaderName for String {
    #[doc(hidden)]
    #[inline]
    fn set(self, map: &mut HeaderMap, val: HeaderValue) -> Option<DrainEntry> {
        HdrName::from_bytes(self.as_bytes(), move |hdr| map.set2(hdr, val)).unwrap()
    }

    #[doc(hidden)]
    #[inline]
    fn insert(self, map: &mut HeaderMap, val: HeaderValue) -> bool {
        HdrName::from_bytes(self.as_bytes(), move |hdr| map.insert2(hdr, val)).unwrap()
    }

    #[doc(hidden)]
    #[inline]
    fn insert_ref(&self, map: &mut HeaderMap, val: HeaderValue) {
        HdrName::from_bytes(self.as_bytes(), move |hdr| map.insert2(hdr, val)).unwrap();
    }

    #[doc(hidden)]
    #[inline]
    fn entry(self, map: &mut HeaderMap) -> Entry {
        HdrName::from_bytes(self.as_bytes(), move |hdr| map.entry2(hdr)).unwrap()
    }

    #[doc(hidden)]
    #[inline]
    fn find_scan(&self, map: &HeaderMap) -> Option<usize> {
        HdrName::from_bytes(self.as_bytes(), |hdr| map.find_scan(&hdr)).unwrap()
    }

    #[doc(hidden)]
    #[inline]
    fn find_hashed(&self, map: &HeaderMap) -> Option<(usize, usize)> {
        HdrName::from_bytes(self.as_bytes(), |hdr| map.find_hashed(&hdr)).unwrap()
    }
}

impl Sealed for String {}

impl<'a> IntoHeaderName for &'a String {
    #[doc(hidden)]
    #[inline]
    fn set(self, map: &mut HeaderMap, val: HeaderValue) -> Option<DrainEntry> {
        HdrName::from_bytes(self.as_bytes(), move |hdr| map.set2(hdr, val)).unwrap()
    }

    #[doc(hidden)]
    #[inline]
    fn insert(self, map: &mut HeaderMap, val: HeaderValue) -> bool {
        HdrName::from_bytes(self.as_bytes(), move |hdr| map.insert2(hdr, val)).unwrap()
    }

    #[doc(hidden)]
    #[inline]
    fn insert_ref(&self, map: &mut HeaderMap, val: HeaderValue) {
        HdrName::from_bytes(self.as_bytes(), move |hdr| map.insert2(hdr, val)).unwrap();
    }

    #[doc(hidden)]
    #[inline]
    fn entry(self, map: &mut HeaderMap) -> Entry {
        HdrName::from_bytes(self.as_bytes(), move |hdr| map.entry2(hdr)).unwrap()
    }

    #[doc(hidden)]
    #[inline]
    fn find_scan(&self, map: &HeaderMap) -> Option<usize> {
        HdrName::from_bytes(self.as_bytes(), |hdr| map.find_scan(&hdr)).unwrap()
    }

    #[doc(hidden)]
    #[inline]
    fn find_hashed(&self, map: &HeaderMap) -> Option<(usize, usize)> {
        HdrName::from_bytes(self.as_bytes(), |hdr| map.find_hashed(&hdr)).unwrap()
    }
}

impl<'a> Sealed for &'a String {}
