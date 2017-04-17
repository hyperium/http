use super::fast_hash::FastHash;
use super::name::{HeaderName, HdrName};

use std::{cmp, fmt, mem, ops, ptr, u16};
use std::cell::Cell;
use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::iter::FromIterator;
use std::marker::PhantomData;

/// A set of HTTP headers
///
/// `HeaderMap` is an multimap of `HeaderName` to values.
///
/// # Examples
///
/// Basic usage
///
/// ```
/// # use http::HeaderMap;
/// let mut headers = HeaderMap::new();
///
/// headers.insert("Host", "example.com");
/// headers.insert("Content-Length", "123");
///
/// assert!(headers.contains_key("host"));
/// assert!(!headers.contains_key("Location"));
///
/// assert_eq!(headers["host"], "example.com");
///
/// headers.remove("host");
///
/// assert!(!headers.contains_key("host"));
/// ```
#[derive(Clone)]
pub struct HeaderMap<T> {
    // Used to mask values to get an index
    mask: Size,
    indices: Vec<Pos>,
    entries: Vec<Bucket<T>>,
    extra_values: Vec<ExtraValue<T>>,
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
// Links are formed with offsets into `extra_values` and not pointers.
//
// [1]: https://en.wikipedia.org/wiki/Hash_table#Robin_Hood_hashing

/// `HeaderMap` entry iterator.
///
/// Yields `(&HeaderName, &value)` tuples. The same header name may be yielded
/// more than once if it has more than one associated value.
pub struct Iter<'a, T: 'a> {
    inner: IterMut<'a, T>,
}

/// `HeaderMap` mutable entry iterator
///
/// Yields `(&HeaderName, &mut value)` tuples. The same header name may be
/// yielded more than once if it has more than one associated value.
pub struct IterMut<'a, T: 'a> {
    map: *mut HeaderMap<T>,
    entry: Size,
    cursor: Option<Cursor>,
    lt: PhantomData<&'a mut ()>,
}

/// An iterator over `HeaderMap` keys.
///
/// Each header name is yielded only once, even if it has more than one
/// associated value.
pub struct Keys<'a, T: 'a> {
    inner: Iter<'a, T>,
}

/// `HeaderMap` value iterator.
pub struct Values<'a, T: 'a> {
    inner: Iter<'a, T>,
}

/// `HeaderMap` mutable value iterator
pub struct ValuesMut<'a, T: 'a> {
    inner: IterMut<'a, T>,
}

/// A drain iterator for `HeaderMap`.
pub struct Drain<'a, T> {
    idx: usize,
    map: *mut HeaderMap<T>,
    lt: PhantomData<&'a ()>,
}

/// A view to all values associated with a single header name.
pub struct ValueSet<'a, T: 'a> {
    map: &'a HeaderMap<T>,
    index: Size,
}

/// A mutable view to all values associated with a single header name.
pub struct ValueSetMut<'a, T: 'a> {
    map: &'a mut HeaderMap<T>,
    index: Size,
}

/// A view into a single location in a `HeaderMap`, which may be vaccant or occupied.
pub enum Entry<'a, T: 'a> {
    Occupied(OccupiedEntry<'a, T>),
    Vacant(VacantEntry<'a, T>),
}

/// A view into a single empty location in a `HeaderMap`.
///
/// This struct is returned as part of the `Entry` enum.
pub struct VacantEntry<'a, T: 'a> {
    map: &'a mut HeaderMap<T>,
    key: HeaderName,
    hash: HashValue,
    probe: Size,
    danger: bool,
}

/// A view into a single occupied location in a `HeaderMap`.
///
/// This struct is returned as part of the `Entry` enum.
pub struct OccupiedEntry<'a, T: 'a> {
    inner: ValueSetMut<'a, T>,
    probe: Size,
}

/// An iterator of all values associated with a single header name.
pub struct EntryIter<'a, T: 'a> {
    map: &'a HeaderMap<T>,
    index: Size,
    front: Option<Cursor>,
    back: Option<Cursor>,
}

/// A mutable iterator of all values associated with a single header name.
pub struct EntryIterMut<'a, T: 'a> {
    map: *mut HeaderMap<T>,
    index: Size,
    front: Option<Cursor>,
    back: Option<Cursor>,
    lt: PhantomData<&'a ()>,
}

/// An drain iterator of all values associated with a single header name.
pub struct DrainEntry<'a, T> {
    map: *mut HeaderMap<T>,
    first: Option<T>,
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
    // Full hash value for the entry.
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
#[derive(Clone)]
struct Bucket<T> {
    hash: Cell<HashValue>,
    key: HeaderName,
    value: T,
    links: Option<Links>,
}

/// The head and tail of the value linked list.
#[derive(Debug, Copy, Clone)]
struct Links {
    next: Size,
    tail: Size,
}

/// Node in doubly-linked list of header value entries
#[derive(Clone)]
struct ExtraValue<T> {
    value: T,
    prev: Cell<Link>,
    next: Cell<Link>,
}

/// A header value node is either linked to another node in the `extra_values`
/// list or it points to an entry in `entries`. The entry in `entries` is the
/// start of the list and holds the associated header name.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum Link {
    Entry(Size),
    Extra(Size),
}

/// Tracks the header map danger level! This relates to the adaptive hashing
/// algorithm. A HeaderMap starts in the "green" state, when a large number of
/// collisions are detected, it transitions to the yellow state. At this point,
/// the header map will either grow and switch back to the green state OR it
/// will transition to the red state.
///
/// When in the red state, a safe hashing algorithm is used and all values in
/// the header map have to be rehashed.
#[derive(Clone)]
enum Danger {
    Green,
    Yellow,
    Red(RandomState),
}

// The HeaderMap will use a sequential search strategy until the size of the map
// exceeds this threshold. This tends to be faster for very small header maps.
// This way all hashing logic can be skipped.
const SEQ_SEARCH_THRESHOLD: usize = 8;

// Constants related to detecting DOS attacks.
//
// Displacement is the number of entries that get shifted when inserting a new
// value. Forward shift is how far the entry gets stored from the ideal
// position.
//
// The current constant values were picked from another implementation. It could
// be that there are different values better suited to the header map case.
const DISPLACEMENT_THRESHOLD: usize = 128;
const FORWARD_SHIFT_THRESHOLD: usize = 512;

// The default strategy for handling the yellow danger state is to increase the
// header map capacity in order to (hopefully) reduce the number of collisions.
// If growing the hash map would cause the load factor to drop bellow this
// threshold, then instead of growing, the headermap is switched to the red
// danger state and safe hashing is used instead.
const LOAD_FACTOR_THRESHOLD: f32 = 0.2;

// Macro used to iterate the hash table starting at a given point, looping when
// the end is hit.
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

// First part of the robinhood algorithm. Given a key, find the slot in which it
// will be inserted. This is done by starting at the "ideal" spot. Then scanning
// until the destination slot is found. A destination slot is either the next
// empty slot or the next slot that is occupied by an entry that has a lower
// displacement (displacement is the distance from the ideal spot).
//
// This is implemented as a macro instead of a function that takes a closure in
// order to guarantee that it is "inlined". There is no way to annotate closures
// to guarantee inlining.
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

        // Start at the ideal position, checking all slots
        probe_loop!('probe: $probe < len, {
            if let Some(($pos, entry_hash)) = $map.indices[$probe].resolve() {
                // The slot is already occupied, but check if it has a lower
                // displacement.
                let their_dist = probe_distance($map.mask as usize, entry_hash, $probe);

                if their_dist < dist {
                    // The new key's distance is larger, so claim this spot and
                    // displace the current entry.
                    //
                    // Check if this insertion is above the danger threshold.
                    let $danger =
                        dist >= FORWARD_SHIFT_THRESHOLD && !$map.danger.is_red();

                    ret = $robinhood;
                    break 'probe;
                } else if entry_hash == $hash && $map.entries[$pos].key == $key {
                    // There already is an entry with the same key.
                    ret = $occupied;
                    break 'probe;
                }
            } else {
                // The entry is vacant, use it for this key.
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

impl<T> HeaderMap<T> {
    /// Create an empty `HeaderMap`.
    ///
    /// The map will be created without any capacity. This function will not
    /// allocate.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::HeaderMap;
    /// let map: HeaderMap<u32> = HeaderMap::new();
    ///
    /// assert!(map.is_empty());
    /// assert_eq!(0, map.capacity());
    /// ```
    pub fn new() -> HeaderMap<T> {
        HeaderMap::with_capacity(0)
    }

    /// Create an empty `HeaderMap` with the specified capacity.
    ///
    /// The returned map will allocate internal storage in order to hold about
    /// `capacity` elements without reallocating. However, this is a "best
    /// effort" as there are usage patterns that could cause additional
    /// allocations before `capacity` headers are stored in the map.
    ///
    /// More capacity than requested may be allocated.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::HeaderMap;
    /// let map: HeaderMap<u32> = HeaderMap::with_capacity(10);
    ///
    /// assert!(map.is_empty());
    /// assert_eq!(12, map.capacity());
    /// ```
    pub fn with_capacity(capacity: usize) -> HeaderMap<T> {
        assert!(capacity <= MAX_SIZE, "requested capacity too large");

        if capacity == 0 {
            HeaderMap {
                mask: 0,
                indices: Vec::new(),
                entries: Vec::new(),
                extra_values: Vec::new(),
                danger: Danger::Green,
            }
        } else {
            // Avoid allocating storage for the hash table if the requested
            // capacity is below the threshold at which the hash map algorithm
            // is used.
            let entries_cap = to_raw_capacity(capacity).next_power_of_two();
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
    ///
    /// This number represents the total number of **values** stored in the map.
    /// This number can be greater than or equal to the number of **keys**
    /// stored given that a single key may have more than one associated value.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::HeaderMap;
    /// let mut map = HeaderMap::new();
    ///
    /// assert_eq!(0, map.len());
    ///
    /// map.insert("x-header-one", "1");
    /// map.insert("x-header-two", "2");
    ///
    /// assert_eq!(2, map.len());
    ///
    /// map.insert("x-header-two", "deux");
    ///
    /// assert_eq!(3, map.len());
    /// ```
    #[inline]
    pub fn len(&self) -> usize {
        self.entries.len() + self.extra_values.len()
    }

    /// Returns the number of keys stored in the map.
    ///
    /// This number will be less than or equal to `len()` as each key may have
    /// more than one associated value.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::HeaderMap;
    /// let mut map = HeaderMap::new();
    ///
    /// assert_eq!(0, map.keys_len());
    ///
    /// map.insert("x-header-one", "1");
    /// map.insert("x-header-two", "2");
    ///
    /// assert_eq!(2, map.keys_len());
    ///
    /// map.insert("x-header-two", "deux");
    ///
    /// assert_eq!(2, map.keys_len());
    /// ```
    #[inline]
    pub fn keys_len(&self) -> usize {
        self.entries.len()
    }

    /// Returns true if the map contains no elements.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::HeaderMap;
    /// let mut map = HeaderMap::new();
    ///
    /// assert!(map.is_empty());
    ///
    /// map.insert("x-hello", "world");
    ///
    /// assert!(!map.is_empty());
    /// ```
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.entries.len() == 0
    }

    /// Clears the map, removing all key-value pairs. Keeps the allocated memory
    /// for reuse.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::HeaderMap;
    /// let mut map = HeaderMap::new();
    /// map.insert("x-hello", "world");
    ///
    /// map.clear();
    /// assert!(map.is_empty());
    /// assert!(map.capacity() > 0);
    /// ```
    pub fn clear(&mut self) {
        self.entries.clear();
        self.extra_values.clear();
        self.danger = Danger::Green;

        for e in self.indices.iter_mut() {
            *e = Pos::none();
        }
    }

    /// Returns the number of headers the map can hold without reallocating.
    ///
    /// This number is an approximation as certain usage patterns could cause
    /// additional allocations before the returned capacity is filled.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::HeaderMap;
    /// let mut map = HeaderMap::new();
    ///
    /// assert_eq!(0, map.capacity());
    ///
    /// map.insert("x-hello", "world");
    /// assert_eq!(8, map.capacity());
    /// ```
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
    /// Like with `with_capacity`, this will be a "best effort" to avoid
    /// allocations until `additional` more headers are inserted. Certain usage
    /// patterns could cause additional allocations before the number is
    /// reached.
    ///
    /// # Panics
    ///
    /// Panics if the new allocation size overflows `usize`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::HeaderMap;
    /// let mut map = HeaderMap::new();
    /// map.reserve(10);
    /// # map.insert("foo", "bar");
    /// ```
    pub fn reserve(&mut self, additional: usize) {
        if self.is_scan() {
            // If in "scan" mode, then the hash table is unallocated. All we
            // have to do is grow the entries table.
            //
            // The new size of the entries table *may* be above the sequential
            // scan threshold, but we don't transition hashing until the number
            // of inserted elements passes the threshold.
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
    /// is returned. Use `get_all` to get all values associated with a given
    /// key. Returns `None` if there are no values associated with the key.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::HeaderMap;
    /// let mut map = HeaderMap::new();
    /// assert!(map.get("x-hello").is_none());
    ///
    /// map.insert("x-hello", "hello");
    /// assert_eq!(map.get("x-hello").unwrap(), &"hello");
    ///
    /// map.insert("x-hello", "world");
    /// assert_eq!(map.get("x-hello").unwrap(), &"hello");
    /// ```
    pub fn get<K: ?Sized>(&self, key: &K) -> Option<&T>
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

    /// Returns a view of all values associated with a key.
    ///
    /// The returned view does not incur any allocations and allows iterating
    /// the values associated with the key.  See [`ValueSet`] for more details.
    /// Returns `None` if there are no values associated with the key.
    ///
    /// [`ValueSet`]: struct.ValueSet.html
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::HeaderMap;
    /// let mut map = HeaderMap::new();
    ///
    /// map.insert("x-hello", "hello");
    /// map.insert("x-hello", "goodbye");
    ///
    /// let view = map.get_all("x-hello").unwrap();
    /// assert_eq!(view.first(), &"hello");
    ///
    /// let mut iter = view.iter();
    /// assert_eq!(&"hello", iter.next().unwrap());
    /// assert_eq!(&"goodbye", iter.next().unwrap());
    /// assert!(iter.next().is_none());
    /// ```
    pub fn get_all<K: ?Sized>(&self, key: &K) -> Option<ValueSet<T>>
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

    /// Returns a mutable view of all values associated with a key.
    ///
    /// The returned view does not incur any allocations and allows iterating
    /// the values associated with the key. See [`ValueSetMut`] for more
    /// details.
    ///
    /// [`ValueSetMut`]: struct.ValueSetMut.html
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::HeaderMap;
    /// let mut map = HeaderMap::new();
    ///
    /// map.insert("x-hello", "hello".to_string());
    /// map.insert("x-hello", "goodbye".to_string());
    ///
    /// {
    ///     let mut view = map.get_all_mut("x-hello").unwrap();
    ///     assert_eq!(view.first(), &"hello");
    ///
    ///     let mut iter = view.iter_mut();
    ///     iter.next().unwrap().push_str("-hello");
    ///     iter.next().unwrap().push_str("-goodbye");
    ///     assert!(iter.next().is_none());
    /// }
    ///
    /// assert_eq!(map["x-hello"], "hello-hello");
    /// ```
    pub fn get_all_mut<K: ?Sized>(&mut self, key: &K) -> Option<ValueSetMut<T>>
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

    /// Returns true if the map contains a value for the specified key.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::HeaderMap;
    /// let mut map = HeaderMap::new();
    /// assert!(!map.contains_key("x-hello"));
    ///
    /// map.insert("x-hello", "world");
    /// assert!(map.contains_key("x-hello"));
    /// ```
    pub fn contains_key<K: ?Sized>(&self, key: &K) -> bool
        where K: IntoHeaderName
    {
        if self.is_scan() {
            key.find_scan(self).is_some()
        } else {
            key.find_hashed(self).is_some()
        }
    }

    /// An iterator visiting all key-value pairs.
    ///
    /// The iteration order is arbitrary, but consistent across platforms for
    /// the same crate version. Each key will be yielded once per associated
    /// value. So, if a key has 3 associated values, it will be yielded 3 times.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::HeaderMap;
    /// let mut map = HeaderMap::new();
    ///
    /// map.insert("x-hello", "hello");
    /// map.insert("x-hello", "goodbye");
    /// map.insert("Content-Length", "123");
    ///
    /// for (key, value) in map.iter() {
    ///     println!("{:?}: {}", key, value);
    /// }
    /// ```
    pub fn iter(&self) -> Iter<T> {
        Iter {
            inner: IterMut {
                map: self as *const _ as *mut _,
                entry: 0,
                cursor: self.entries.first().map(|_| Cursor::Head),
                lt: PhantomData,
            }
        }
    }

    /// An iterator visiting all key-value pairs, with mutable value references.
    ///
    /// The iterator order is arbitrary, but consistent across platforms for the
    /// same crate version. Each key will be yielded once per associated value,
    /// so if a key has 3 associated values, it will be yielded 3 times.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::HeaderMap;
    /// let mut map = HeaderMap::new();
    ///
    /// map.insert("x-hello", "hello".to_string());
    /// map.insert("x-hello", "goodbye".to_string());
    /// map.insert("Content-Length", "123".to_string());
    ///
    /// for (key, value) in map.iter_mut() {
    ///     value.push_str("-boop");
    /// }
    /// ```
    pub fn iter_mut(&mut self) -> IterMut<T> {
        IterMut {
            map: self as *mut _,
            entry: 0,
            cursor: self.entries.first().map(|_| Cursor::Head),
            lt: PhantomData,
        }
    }

    /// An iterator visiting all keys.
    ///
    /// The iteration order is arbitrary, but consistent across platforms for
    /// the same crate version. Each key will be yielded only once even if it
    /// has multiple associated values.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::HeaderMap;
    /// let mut map = HeaderMap::new();
    ///
    /// map.insert("x-hello", "hello");
    /// map.insert("x-hello", "goodbye");
    /// map.insert("Content-Length", "123");
    ///
    /// for key in map.keys() {
    ///     println!("{:?}", key);
    /// }
    /// ```
    pub fn keys(&self) -> Keys<T> {
        Keys { inner: self.iter() }
    }

    /// An iterator visiting all values.
    ///
    /// The iteration order is arbitrary, but consistent across platforms for
    /// the same crate version.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::HeaderMap;
    /// let mut map = HeaderMap::new();
    ///
    /// map.insert("x-hello", "hello");
    /// map.insert("x-hello", "goodbye");
    /// map.insert("Content-Length", "123");
    ///
    /// for value in map.values() {
    ///     println!("{}", value);
    /// }
    /// ```
    pub fn values(&self) -> Values<T> {
        Values { inner: self.iter() }
    }

    /// An iterator visiting all values mutably.
    ///
    /// The iteration order is arbitrary, but consistent across platforms for
    /// the same crate version.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::HeaderMap;
    /// let mut map = HeaderMap::new();
    ///
    /// map.insert("x-hello", "hello".to_string());
    /// map.insert("x-hello", "goodbye".to_string());
    /// map.insert("Content-Length", "123".to_string());
    ///
    /// for value in map.values_mut() {
    ///     value.push_str("-boop");
    /// }
    /// ```
    pub fn values_mut(&mut self) -> ValuesMut<T> {
        ValuesMut { inner: self.iter_mut() }
    }

    /// Clears the map, returning all entries as an iterator.
    ///
    /// The internal memory is kept for reuse.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http::HeaderMap;
    /// let mut map = HeaderMap::new();
    ///
    /// map.insert("x-hello", "hello");
    /// map.insert("x-hello", "goodbye");
    /// map.insert("Content-Length", "123");
    ///
    /// let mut drain = map.drain();
    ///
    /// let (key, mut vals) = drain.next().unwrap();
    ///
    /// assert_eq!("x-hello", key.as_str());
    /// assert_eq!("hello", vals.next().unwrap());
    /// assert_eq!("goodbye", vals.next().unwrap());
    /// assert!(vals.next().is_none());
    ///
    /// let (key, mut vals) = drain.next().unwrap();
    ///
    /// assert_eq!("content-length", key.as_str());
    /// assert_eq!("123", vals.next().unwrap());
    /// assert!(vals.next().is_none());
    /// ```
    pub fn drain(&mut self) -> Drain<T> {
        Drain {
            idx: 0,
            map: self as *mut _,
            lt: PhantomData,
        }
    }

    fn entry_iter(&self, idx: Size) -> EntryIter<T> {
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

    fn entry_iter_mut(&mut self, idx: Size) -> EntryIterMut<T> {
        use self::Cursor::*;

        let back = {
            let entry = &self.entries[idx as usize];

            entry.links
                .map(|l| Values(l.tail))
                .unwrap_or(Head)
        };

        EntryIterMut {
            map: self as *mut _,
            index: idx,
            front: Some(Head),
            back: Some(back),
            lt: PhantomData,
        }
    }

    #[inline]
    pub fn entry<K>(&mut self, key: K) -> Entry<T>
        where K: IntoHeaderName,
    {
        key.entry(self)
    }

    fn entry2<K>(&mut self, key: K) -> Entry<T>
        where K: FastHash + Into<HeaderName>,
              HeaderName: PartialEq<K>,
    {
        if self.is_scan() {
            self.entry_scan(key)
        } else {
            self.entry_hashed(key)
        }
    }

    fn entry_scan<K>(&mut self, key: K) -> Entry<T>
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

    fn entry_hashed<K>(&mut self, key: K) -> Entry<T>
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

    pub fn set<K>(&mut self, key: K, val: T) -> Option<DrainEntry<T>>
        where K: IntoHeaderName,
    {
        key.set(self, val.into())
    }

    fn set2<K>(&mut self, key: K, val: T) -> Option<DrainEntry<T>>
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
    fn set_scan<K>(&mut self, key: K, value: T) -> Option<DrainEntry<T>>
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
    fn set_occupied(&mut self, index: Size, value: T) -> DrainEntry<T> {
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
    fn set_hashed<K>(&mut self, key: K, value: T) -> Option<DrainEntry<T>>
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
    pub fn insert<K>(&mut self, key: K, val: T) -> bool
        where K: IntoHeaderName,
    {
        key.insert(self, val.into())
    }

    #[inline]
    fn insert2<K>(&mut self, key: K, val: T) -> bool
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
    fn insert_scan<K>(&mut self, key: K, value: T) -> bool
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
    fn insert_hashed<K>(&mut self, key: K, value: T) -> bool
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
        where F: Fn(&Bucket<T>) -> bool,
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
                        value: T,
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

    pub fn remove<K: ?Sized>(&mut self, key: &K) -> Option<DrainEntry<T>>
        where K: IntoHeaderName
    {
        self.remove_entry(key).map(|e| e.1)
    }

    pub fn remove_entry<K: ?Sized>(&mut self, key: &K) -> Option<(HeaderName, DrainEntry<T>)>
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
    fn remove_found_scan(&mut self, index: usize) -> (HeaderName, DrainEntry<T>) {
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
    fn remove_found_hashed(&mut self,
                           probe: usize,
                           found: usize) -> (HeaderName, DrainEntry<T>)
    {
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
    fn remove_extra_value(&mut self, idx: usize) -> ExtraValue<T> {
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
    fn insert_entry(&mut self, hash: HashValue, key: HeaderName, value: T) {
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

impl<'a, T> IntoIterator for &'a HeaderMap<T> {
    type Item = (&'a HeaderName, &'a T);
    type IntoIter = Iter<'a, T>;

    fn into_iter(self) -> Iter<'a, T> {
        self.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut HeaderMap<T> {
    type Item = (&'a HeaderName, &'a mut T);
    type IntoIter = IterMut<'a, T>;

    fn into_iter(self) -> IterMut<'a, T> {
        self.iter_mut()
    }
}

impl<K, T> FromIterator<(K, T)> for HeaderMap<T>
    where K: IntoHeaderName,
{
    fn from_iter<I>(iter: I) -> Self
        where I: IntoIterator<Item = (K, T)>
    {
       let mut map = HeaderMap::new();
       map.extend(iter);
       map
    }
}

impl<K, T> Extend<(K, T)> for HeaderMap<T>
    where K: IntoHeaderName,
{
    fn extend<I: IntoIterator<Item = (K, T)>>(&mut self, iter: I) {
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

impl<T: PartialEq> PartialEq for HeaderMap<T> {
    fn eq(&self, other: &HeaderMap<T>) -> bool {
        if self.len() != other.len() {
            return false;
        }

        self.keys().all(|key| {
            self.get_all(key) == other.get_all(key)
        })
    }
}

impl<T: Eq> Eq for HeaderMap<T> {}

impl<T: fmt::Debug> fmt::Debug for HeaderMap<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

impl<T: Default> Default for HeaderMap<T> {
    fn default() -> Self {
        HeaderMap::new()
    }
}

impl<'a, K: ?Sized, T> ops::Index<&'a K> for HeaderMap<T>
    where K: IntoHeaderName,
{
    type Output = T;

    #[inline]
    fn index(&self, index: &K) -> &T {
        self.get(index).expect("no entry found for key")
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
fn insert_value<T>(entry_idx: usize,
                   entry: &mut Bucket<T>,
                   extra: &mut Vec<ExtraValue<T>>,
                   value: T)
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

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = (&'a HeaderName, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next_unsafe().map(|(key, ptr)| {
            (key, unsafe { &*ptr })
        })
    }
}

// ===== impl IterMut =====

impl<'a, T> IterMut<'a, T> {
    fn next_unsafe(&mut self) -> Option<(&'a HeaderName, *mut T)> {
        use self::Cursor::*;

        if self.cursor.is_none() {
            if (self.entry + 1) as usize >= unsafe { &*self.map }.entries.len() {
                return None;
            }

            self.entry += 1;
            self.cursor = Some(Cursor::Head);
        }

        let entry = unsafe { &(*self.map).entries[self.entry as usize] };

        match self.cursor.unwrap() {
            Head => {
                self.cursor = entry.links.map(|l| Values(l.next));
                Some((&entry.key, &entry.value as *const _ as *mut _))
            }
            Values(idx) => {
                let idx = idx as usize;
                let extra = unsafe { &(*self.map).extra_values[idx as usize] };

                match extra.next.get() {
                    Link::Entry(_) => self.cursor = None,
                    Link::Extra(i) => self.cursor = Some(Values(i)),
                }

                Some((&entry.key, &extra.value as *const _ as *mut _))
            }
        }
    }
}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = (&'a HeaderName, &'a mut T);

    fn next(&mut self) -> Option<Self::Item> {
        self.next_unsafe().map(|(key, ptr)| {
            (key, unsafe { &mut *ptr })
        })
    }
}

// ===== impl Keys =====

impl<'a, T> Iterator for Keys<'a, T> {
    type Item = &'a HeaderName;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(n, _)| n)
    }
}

// ===== impl Values ====

impl<'a, T> Iterator for Values<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(_, v)| v)
    }
}

// ===== impl ValuesMut ====

impl<'a, T> Iterator for ValuesMut<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(_, v)| v)
    }
}

// ===== impl Drain =====

impl<'a, T> Iterator for Drain<'a, T> {
    type Item = (HeaderName, DrainEntry<'a, T>);

    fn next(&mut self) -> Option<Self::Item> {
        let idx = self.idx;

        if idx == unsafe { (*self.map).entries.len() } {
            return None;
        }

        self.idx += 1;

        let key;
        let value;
        let next;

        unsafe {
            let entry = &(*self.map).entries[idx];

            // Read the header name
            key = ptr::read(&entry.key as *const _);
            value = ptr::read(&entry.value as *const _);
            next = entry.links.map(|l| l.next);
        };

        let values = DrainEntry {
            map: self.map,
            first: Some(value),
            next: next,
            lt: PhantomData,
        };

        Some((key, values))
    }
}

impl<'a, T> Drop for Drain<'a, T> {
    fn drop(&mut self) {
        unsafe {
            let map = &mut *self.map;
            debug_assert!(map.extra_values.is_empty());
            map.entries.set_len(0);
        }
    }
}

// ===== impl VacantEntry =====

impl<'a, T> VacantEntry<'a, T> {
    #[inline]
    pub fn key(&self) -> &HeaderName {
        &self.key
    }

    #[inline]
    pub fn into_key(self) -> HeaderName {
        self.key
    }

    pub fn set(self, value: T) -> &'a mut T {
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

impl<'a, T> ValueSet<'a, T> {
    /// Get a reference to the header name.
    #[inline]
    pub fn key(&self) -> &HeaderName {
        &self.map.entries[self.index as usize].key
    }

    /// Get a reference to the first value in the set.
    #[inline]
    pub fn first(&self) -> &T {
        &self.map.entries[self.index as usize].value
    }

    /// Get a reference to the last value in the set.
    #[inline]
    pub fn last(&self) -> &T {
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
    pub fn iter(&self) -> EntryIter<T> {
        self.into_iter()
    }
}

impl<'a, T: PartialEq> PartialEq for ValueSet<'a, T> {
    fn eq(&self, other: &Self) -> bool {
        self.iter().eq(other.iter())
    }
}

impl<'a, T> IntoIterator for ValueSet<'a, T> {
    type Item = &'a T;
    type IntoIter = EntryIter<'a, T>;

    #[inline]
    fn into_iter(self) -> EntryIter<'a, T> {
        self.map.entry_iter(self.index)
    }
}

impl<'a, 'b: 'a, T> IntoIterator for &'b ValueSet<'a, T> {
    type Item = &'a T;
    type IntoIter = EntryIter<'a, T>;

    #[inline]
    fn into_iter(self) -> EntryIter<'a, T> {
        self.map.entry_iter(self.index)
    }
}

// ===== impl EntryIter =====

impl<'a, T: 'a> Iterator for EntryIter<'a, T> {
    type Item = &'a T;

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

impl<'a, T: 'a> DoubleEndedIterator for EntryIter<'a, T> {
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

// ===== impl EntryIterMut =====

impl<'a, T: 'a> Iterator for EntryIterMut<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        use self::Cursor::*;

        let entry = unsafe { &mut (*self.map).entries[self.index as usize] };

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

                Some(&mut entry.value)
            }
            Some(Values(idx)) => {
                let extra = unsafe { &mut (*self.map).extra_values[idx as usize] };

                if self.front == self.back {
                    self.front = None;
                    self.back = None;
                } else {
                    match extra.next.get() {
                        Link::Entry(_) => self.front = None,
                        Link::Extra(i) => self.front = Some(Values(i)),
                    }
                }

                Some(&mut extra.value)
            }
            None => None,
        }
    }
}

impl<'a, T: 'a> DoubleEndedIterator for EntryIterMut<'a, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        use self::Cursor::*;

        let entry = unsafe { &mut (*self.map).entries[self.index as usize] };

        match self.back {
            Some(Head) => {
                self.front = None;
                self.back = None;
                Some(&mut entry.value)
            }
            Some(Values(idx)) => {
                let extra = unsafe { &mut (*self.map).extra_values[idx as usize] };

                if self.front == self.back {
                    self.front = None;
                    self.back = None;
                } else {
                    match extra.prev.get() {
                        Link::Entry(_) => self.back = Some(Head),
                        Link::Extra(idx) => self.back = Some(Values(idx)),
                    }
                }

                Some(&mut extra.value)
            }
            None => None,
        }
    }
}

// ===== impl ValueSetMut =====

impl<'a, T: 'a> ValueSetMut<'a, T> {
    /// Get a reference to the header name.
    #[inline]
    pub fn key(&self) -> &HeaderName {
        &self.map.entries[self.index as usize].key
    }

    /// Get a reference to the first header value in the entry.
    ///
    /// # Panics
    ///
    /// Panics if there are no values for the entry.
    #[inline]
    pub fn first(&self) -> &T {
        &self.map.entries[self.index as usize].value
    }

    /// Get a mutable reference to the first header value in the entry.
    #[inline]
    pub fn first_mut(&mut self) -> &mut T {
        &mut self.map.entries[self.index as usize].value
    }

    /// Get a reference to the last header value in this entry.
    #[inline]
    pub fn last(&self) -> &T {
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
    pub fn last_mut(&mut self) -> &mut T {
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
    pub fn set(&mut self, value: T) -> DrainEntry<T> {
        self.map.set_occupied(self.index, value.into())
    }

    pub fn insert(&mut self, value: T) {
        let idx = self.index as usize;
        let entry = &mut self.map.entries[idx];
        insert_value(idx, entry, &mut self.map.extra_values, value.into());
    }

    #[inline]
    pub fn iter(&self) -> EntryIter<T> {
        self.map.entry_iter(self.index)
    }

    #[inline]
    pub fn iter_mut(&mut self) -> EntryIterMut<T> {
        self.map.entry_iter_mut(self.index)
    }
}

impl<'a, T> IntoIterator for ValueSetMut<'a, T> {
    type Item = &'a mut T;
    type IntoIter = EntryIterMut<'a, T>;

    #[inline]
    fn into_iter(self) -> EntryIterMut<'a, T> {
        self.map.entry_iter_mut(self.index)
    }
}

impl<'a, 'b: 'a, T> IntoIterator for &'b ValueSetMut<'a, T> {
    type Item = &'a T;
    type IntoIter = EntryIter<'a, T>;

    #[inline]
    fn into_iter(self) -> EntryIter<'a, T> {
        self.iter()
    }
}

impl<'a, 'b: 'a, T> IntoIterator for &'b mut ValueSetMut<'a, T> {
    type Item = &'a mut T;
    type IntoIter = EntryIterMut<'a, T>;

    #[inline]
    fn into_iter(self) -> EntryIterMut<'a, T> {
        self.iter_mut()
    }
}

// ===== impl OccupiedEntry =====

impl<'a, T> OccupiedEntry<'a, T> {
    /// Get a reference to the header name in the entry.
    #[inline]
    pub fn key(&self) -> &HeaderName {
        self.inner.key()
    }

    /// Get a reference to the first header value in the entry.
    ///
    /// # Panics
    ///
    /// Panics if there are no values for the entry.
    #[inline]
    pub fn first(&self) -> &T {
        self.inner.first()
    }

    /// Get a mutable reference to the first header value in the entry.
    #[inline]
    pub fn first_mut(&mut self) -> &mut T {
        self.inner.first_mut()
    }

    /// Get a reference to the last header value in this entry.
    #[inline]
    pub fn last(&self) -> &T {
        self.inner.last()
    }

    /// Get a mutable reference to the last header value in this entry.
    #[inline]
    pub fn last_mut(&mut self) -> &mut T {
        self.inner.last_mut()
    }

    /// Replaces all values for this entry with the provided value.
    #[inline]
    pub fn set(&mut self, value: T) -> DrainEntry<T> {
        self.inner.set(value)
    }

    pub fn insert(&mut self, value: T) {
        self.inner.insert(value)
    }

    pub fn remove(self) -> DrainEntry<'a, T> {
        self.remove_entry().1
    }

    pub fn remove_entry(self) -> (HeaderName, DrainEntry<'a, T>) {
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

impl<'a, T> Iterator for DrainEntry<'a, T> {
    type Item = T;

    #[inline]
    fn next(&mut self) -> Option<T> {
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

impl<'a, T> Drop for DrainEntry<'a, T> {
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
    fn set<T>(self, map: &mut HeaderMap<T>, val: T) -> Option<DrainEntry<T>>
        where Self: Sized
    {
        drop(map);
        drop(val);
        unimplemented!();
    }

    #[doc(hidden)]
    fn insert<T>(self, map: &mut HeaderMap<T>, val: T) -> bool
        where Self: Sized
    {
        drop(map);
        drop(val);
        unimplemented!();
    }

    #[doc(hidden)]
    fn insert_ref<T>(&self, map: &mut HeaderMap<T>, val: T);

    #[doc(hidden)]
    fn entry<T>(self, map: &mut HeaderMap<T>) -> Entry<T> where Self: Sized {
        drop(map);
        unimplemented!();
    }

    #[doc(hidden)]
    fn find_scan<T>(&self, map: &HeaderMap<T>) -> Option<usize>;

    #[doc(hidden)]
    fn find_hashed<T>(&self, map: &HeaderMap<T>) -> Option<(usize, usize)>;
}

// Prevent users from implementing the `IntoHeaderName` trait.
pub trait Sealed {}

impl IntoHeaderName for HeaderName {
    #[doc(hidden)]
    #[inline]
    fn set<T>(self, map: &mut HeaderMap<T>, val: T) -> Option<DrainEntry<T>> {
        map.set2(self, val)
    }

    #[doc(hidden)]
    #[inline]
    fn insert<T>(self, map: &mut HeaderMap<T>, val: T) -> bool {
        map.insert2(self, val)
    }

    #[doc(hidden)]
    #[inline]
    fn insert_ref<T>(&self, map: &mut HeaderMap<T>, val: T) {
        map.insert2(self, val);
    }

    #[doc(hidden)]
    #[inline]
    fn entry<T>(self, map: &mut HeaderMap<T>) -> Entry<T> {
        map.entry2(self)
    }

    #[doc(hidden)]
    #[inline]
    fn find_scan<T>(&self, map: &HeaderMap<T>) -> Option<usize> {
        map.find_scan(self)
    }

    #[doc(hidden)]
    #[inline]
    fn find_hashed<T>(&self, map: &HeaderMap<T>) -> Option<(usize, usize)> {
        map.find_hashed(self)
    }
}

impl Sealed for HeaderName {}

impl<'a> IntoHeaderName for &'a HeaderName {
    #[doc(hidden)]
    #[inline]
    fn set<T>(self, map: &mut HeaderMap<T>, val: T) -> Option<DrainEntry<T>> {
        map.set2(self, val)
    }

    #[doc(hidden)]
    #[inline]
    fn insert<T>(self, map: &mut HeaderMap<T>, val: T) -> bool {
        map.insert2(self, val)
    }

    #[doc(hidden)]
    #[inline]
    fn insert_ref<T>(&self, map: &mut HeaderMap<T>, val: T) {
        map.insert2(*self, val);
    }

    #[doc(hidden)]
    #[inline]
    fn entry<T>(self, map: &mut HeaderMap<T>) -> Entry<T> {
        map.entry2(self)
    }

    #[doc(hidden)]
    #[inline]
    fn find_scan<T>(&self, map: &HeaderMap<T>) -> Option<usize> {
        map.find_scan(*self)
    }

    #[doc(hidden)]
    #[inline]
    fn find_hashed<T>(&self, map: &HeaderMap<T>) -> Option<(usize, usize)> {
        map.find_hashed(*self)
    }
}

impl<'a> Sealed for &'a HeaderName {}

impl IntoHeaderName for str {
    #[doc(hidden)]
    #[inline]
    fn insert_ref<T>(&self, map: &mut HeaderMap<T>, val: T) {
        HdrName::from_bytes(self.as_bytes(), move |hdr| map.insert2(hdr, val)).unwrap();
    }

    #[doc(hidden)]
    #[inline]
    fn find_scan<T>(&self, map: &HeaderMap<T>) -> Option<usize> {
        HdrName::from_bytes(self.as_bytes(), |hdr| map.find_scan(&hdr)).unwrap()
    }

    #[doc(hidden)]
    #[inline]
    fn find_hashed<T>(&self, map: &HeaderMap<T>) -> Option<(usize, usize)> {
        HdrName::from_bytes(self.as_bytes(), |hdr| map.find_hashed(&hdr)).unwrap()
    }
}

impl Sealed for str {}

impl<'a> IntoHeaderName for &'a str {
    #[doc(hidden)]
    #[inline]
    fn set<T>(self, map: &mut HeaderMap<T>, val: T) -> Option<DrainEntry<T>> {
        HdrName::from_bytes(self.as_bytes(), move |hdr| map.set2(hdr, val)).unwrap()
    }

    #[doc(hidden)]
    #[inline]
    fn insert<T>(self, map: &mut HeaderMap<T>, val: T) -> bool {
        HdrName::from_bytes(self.as_bytes(), move |hdr| map.insert2(hdr, val)).unwrap()
    }

    #[doc(hidden)]
    #[inline]
    fn insert_ref<T>(&self, map: &mut HeaderMap<T>, val: T) {
        HdrName::from_bytes(self.as_bytes(), move |hdr| map.insert2(hdr, val)).unwrap();
    }

    #[doc(hidden)]
    #[inline]
    fn entry<T>(self, map: &mut HeaderMap<T>) -> Entry<T> {
        HdrName::from_bytes(self.as_bytes(), move |hdr| map.entry2(hdr)).unwrap()
    }

    #[doc(hidden)]
    #[inline]
    fn find_scan<T>(&self, map: &HeaderMap<T>) -> Option<usize> {
        HdrName::from_bytes(self.as_bytes(), |hdr| map.find_scan(&hdr)).unwrap()
    }

    #[doc(hidden)]
    #[inline]
    fn find_hashed<T>(&self, map: &HeaderMap<T>) -> Option<(usize, usize)> {
        HdrName::from_bytes(self.as_bytes(), |hdr| map.find_hashed(&hdr)).unwrap()
    }
}

impl<'a> Sealed for &'a str {}

impl IntoHeaderName for String {
    #[doc(hidden)]
    #[inline]
    fn set<T>(self, map: &mut HeaderMap<T>, val: T) -> Option<DrainEntry<T>> {
        HdrName::from_bytes(self.as_bytes(), move |hdr| map.set2(hdr, val)).unwrap()
    }

    #[doc(hidden)]
    #[inline]
    fn insert<T>(self, map: &mut HeaderMap<T>, val: T) -> bool {
        HdrName::from_bytes(self.as_bytes(), move |hdr| map.insert2(hdr, val)).unwrap()
    }

    #[doc(hidden)]
    #[inline]
    fn insert_ref<T>(&self, map: &mut HeaderMap<T>, val: T) {
        HdrName::from_bytes(self.as_bytes(), move |hdr| map.insert2(hdr, val)).unwrap();
    }

    #[doc(hidden)]
    #[inline]
    fn entry<T>(self, map: &mut HeaderMap<T>) -> Entry<T> {
        HdrName::from_bytes(self.as_bytes(), move |hdr| map.entry2(hdr)).unwrap()
    }

    #[doc(hidden)]
    #[inline]
    fn find_scan<T>(&self, map: &HeaderMap<T>) -> Option<usize> {
        HdrName::from_bytes(self.as_bytes(), |hdr| map.find_scan(&hdr)).unwrap()
    }

    #[doc(hidden)]
    #[inline]
    fn find_hashed<T>(&self, map: &HeaderMap<T>) -> Option<(usize, usize)> {
        HdrName::from_bytes(self.as_bytes(), |hdr| map.find_hashed(&hdr)).unwrap()
    }
}

impl Sealed for String {}

impl<'a> IntoHeaderName for &'a String {
    #[doc(hidden)]
    #[inline]
    fn set<T>(self, map: &mut HeaderMap<T>, val: T) -> Option<DrainEntry<T>> {
        HdrName::from_bytes(self.as_bytes(), move |hdr| map.set2(hdr, val)).unwrap()
    }

    #[doc(hidden)]
    #[inline]
    fn insert<T>(self, map: &mut HeaderMap<T>, val: T) -> bool {
        HdrName::from_bytes(self.as_bytes(), move |hdr| map.insert2(hdr, val)).unwrap()
    }

    #[doc(hidden)]
    #[inline]
    fn insert_ref<T>(&self, map: &mut HeaderMap<T>, val: T) {
        HdrName::from_bytes(self.as_bytes(), move |hdr| map.insert2(hdr, val)).unwrap();
    }

    #[doc(hidden)]
    #[inline]
    fn entry<T>(self, map: &mut HeaderMap<T>) -> Entry<T> {
        HdrName::from_bytes(self.as_bytes(), move |hdr| map.entry2(hdr)).unwrap()
    }

    #[doc(hidden)]
    #[inline]
    fn find_scan<T>(&self, map: &HeaderMap<T>) -> Option<usize> {
        HdrName::from_bytes(self.as_bytes(), |hdr| map.find_scan(&hdr)).unwrap()
    }

    #[doc(hidden)]
    #[inline]
    fn find_hashed<T>(&self, map: &HeaderMap<T>) -> Option<(usize, usize)> {
        HdrName::from_bytes(self.as_bytes(), |hdr| map.find_hashed(&hdr)).unwrap()
    }
}

impl<'a> Sealed for &'a String {}
