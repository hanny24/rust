// Copyright 2012-2013 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#[allow(missing_doc)];


use std::cmp;
use std::ops;
use std::uint;
use std::vec;

struct SmallBitv {
    /// only the lowest nbits of this value are used. the rest is undefined.
    bits: uint
}

/// a mask that has a 1 for each defined bit in a small_bitv, assuming n bits
#[inline]
fn small_mask(nbits: uint) -> uint {
    (1 << nbits) - 1
}

impl SmallBitv {
    pub fn new(bits: uint) -> SmallBitv {
        SmallBitv {bits: bits}
    }

    #[inline]
    pub fn bits_op(&mut self,
                   right_bits: uint,
                   nbits: uint,
                   f: &fn(uint, uint) -> uint)
                   -> bool {
        let mask = small_mask(nbits);
        let old_b: uint = self.bits;
        let new_b = f(old_b, right_bits);
        self.bits = new_b;
        mask & old_b != mask & new_b
    }

    #[inline]
    pub fn union(&mut self, s: &SmallBitv, nbits: uint) -> bool {
        self.bits_op(s.bits, nbits, |u1, u2| u1 | u2)
    }

    #[inline]
    pub fn intersect(&mut self, s: &SmallBitv, nbits: uint) -> bool {
        self.bits_op(s.bits, nbits, |u1, u2| u1 & u2)
    }

    #[inline]
    pub fn become(&mut self, s: &SmallBitv, nbits: uint) -> bool {
        self.bits_op(s.bits, nbits, |_u1, u2| u2)
    }

    #[inline]
    pub fn difference(&mut self, s: &SmallBitv, nbits: uint) -> bool {
        self.bits_op(s.bits, nbits, |u1, u2| u1 & !u2)
    }

    #[inline]
    pub fn get(&self, i: uint) -> bool {
        (self.bits & (1 << i)) != 0
    }

    #[inline]
    pub fn set(&mut self, i: uint, x: bool) {
        if x {
            self.bits |= 1<<i;
        }
        else {
            self.bits &= !(1<<i as uint);
        }
    }

    #[inline]
    pub fn equals(&self, b: &SmallBitv, nbits: uint) -> bool {
        let mask = small_mask(nbits);
        mask & self.bits == mask & b.bits
    }

    #[inline]
    pub fn clear(&mut self) { self.bits = 0; }

    #[inline]
    pub fn set_all(&mut self) { self.bits = !0; }

    #[inline]
    pub fn is_true(&self, nbits: uint) -> bool {
        small_mask(nbits) & !self.bits == 0
    }

    #[inline]
    pub fn is_false(&self, nbits: uint) -> bool {
        small_mask(nbits) & self.bits == 0
    }

    #[inline]
    pub fn invert(&mut self) { self.bits = !self.bits; }
}

struct BigBitv {
    storage: ~[uint]
}

/**
 * a mask that has a 1 for each defined bit in the nth element of a big_bitv,
 * assuming n bits.
 */
#[inline]
fn big_mask(nbits: uint, elem: uint) -> uint {
    let rmd = nbits % uint::bits;
    let nelems = nbits/uint::bits + if rmd == 0 {0} else {1};

    if elem < nelems - 1 || rmd == 0 {
        !0
    } else {
        (1 << rmd) - 1
    }
}

impl BigBitv {
    pub fn new(storage: ~[uint]) -> BigBitv {
        BigBitv {storage: storage}
    }

    #[inline]
    pub fn process(&mut self,
                   b: &BigBitv,
                   nbits: uint,
                   op: &fn(uint, uint) -> uint)
                   -> bool {
        let len = b.storage.len();
        assert_eq!(self.storage.len(), len);
        let mut changed = false;
        for uint::range(0, len) |i| {
            let mask = big_mask(nbits, i);
            let w0 = self.storage[i] & mask;
            let w1 = b.storage[i] & mask;
            let w = op(w0, w1) & mask;
            if w0 != w {
                changed = true;
                self.storage[i] = w;
            }
        }
        changed
    }

    #[inline]
    pub fn each_storage(&mut self, op: &fn(v: &mut uint) -> bool) -> bool {
        uint::range(0, self.storage.len(), |i| op(&mut self.storage[i]))
    }

    #[inline]
    pub fn invert(&mut self) { for self.each_storage |w| { *w = !*w } }

    #[inline]
    pub fn union(&mut self, b: &BigBitv, nbits: uint) -> bool {
        self.process(b, nbits, |w1, w2| w1 | w2)
    }

    #[inline]
    pub fn intersect(&mut self, b: &BigBitv, nbits: uint) -> bool {
        self.process(b, nbits, |w1, w2| w1 & w2)
    }

    #[inline]
    pub fn become(&mut self, b: &BigBitv, nbits: uint) -> bool {
        self.process(b, nbits, |_, w| w)
    }

    #[inline]
    pub fn difference(&mut self, b: &BigBitv, nbits: uint) -> bool {
        self.process(b, nbits, |w1, w2| w1 & !w2)
    }

    #[inline]
    pub fn get(&self, i: uint) -> bool {
        let w = i / uint::bits;
        let b = i % uint::bits;
        let x = 1 & self.storage[w] >> b;
        x == 1
    }

    #[inline]
    pub fn set(&mut self, i: uint, x: bool) {
        let w = i / uint::bits;
        let b = i % uint::bits;
        let flag = 1 << b;
        self.storage[w] = if x { self.storage[w] | flag }
                          else { self.storage[w] & !flag };
    }

    #[inline]
    pub fn equals(&self, b: &BigBitv, nbits: uint) -> bool {
        let len = b.storage.len();
        for uint::iterate(0, len) |i| {
            let mask = big_mask(nbits, i);
            if mask & self.storage[i] != mask & b.storage[i] {
                return false;
            }
        }
        return true;
    }
}

enum BitvVariant { Big(~BigBitv), Small(~SmallBitv) }

enum Op {Union, Intersect, Assign, Difference}

/// The bitvector type
pub struct Bitv {
    /// Internal representation of the bit vector (small or large)
    rep: BitvVariant,
    /// The number of valid bits in the internal representation
    nbits: uint
}

fn die() -> ! {
    fail!("Tried to do operation on bit vectors with different sizes");
}

impl Bitv {
    #[inline]
    fn do_op(&mut self, op: Op, other: &Bitv) -> bool {
        if self.nbits != other.nbits {
            die();
        }
        match self.rep {
          Small(ref mut s) => match other.rep {
            Small(ref s1) => match op {
              Union      => s.union(*s1,      self.nbits),
              Intersect  => s.intersect(*s1,  self.nbits),
              Assign     => s.become(*s1,     self.nbits),
              Difference => s.difference(*s1, self.nbits)
            },
            Big(_) => die()
          },
          Big(ref mut s) => match other.rep {
            Small(_) => die(),
            Big(ref s1) => match op {
              Union      => s.union(*s1,      self.nbits),
              Intersect  => s.intersect(*s1,  self.nbits),
              Assign     => s.become(*s1,     self.nbits),
              Difference => s.difference(*s1, self.nbits)
            }
          }
        }
    }

}

impl Bitv {
    pub fn new(nbits: uint, init: bool) -> Bitv {
        let rep = if nbits <= uint::bits {
            Small(~SmallBitv::new(if init {!0} else {0}))
        }
        else {
            let nelems = nbits/uint::bits +
                         if nbits % uint::bits == 0 {0} else {1};
            let elem = if init {!0} else {0};
            let s = vec::from_elem(nelems, elem);
            Big(~BigBitv::new(s))
        };
        Bitv {rep: rep, nbits: nbits}
    }

    /**
     * Calculates the union of two bitvectors
     *
     * Sets `self` to the union of `self` and `v1`. Both bitvectors must be
     * the same length. Returns 'true' if `self` changed.
    */
    #[inline]
    pub fn union(&mut self, v1: &Bitv) -> bool { self.do_op(Union, v1) }

    /**
     * Calculates the intersection of two bitvectors
     *
     * Sets `self` to the intersection of `self` and `v1`. Both bitvectors
     * must be the same length. Returns 'true' if `self` changed.
    */
    #[inline]
    pub fn intersect(&mut self, v1: &Bitv) -> bool {
        self.do_op(Intersect, v1)
    }

    /**
     * Assigns the value of `v1` to `self`
     *
     * Both bitvectors must be the same length. Returns `true` if `self` was
     * changed
     */
    #[inline]
    pub fn assign(&mut self, v: &Bitv) -> bool { self.do_op(Assign, v) }

    /// Retrieve the value at index `i`
    #[inline]
    pub fn get(&self, i: uint) -> bool {
        assert!((i < self.nbits));
        match self.rep {
            Big(ref b)   => b.get(i),
            Small(ref s) => s.get(i)
        }
    }

    /**
     * Set the value of a bit at a given index
     *
     * `i` must be less than the length of the bitvector.
     */
    #[inline]
    pub fn set(&mut self, i: uint, x: bool) {
      assert!((i < self.nbits));
      match self.rep {
        Big(ref mut b)   => b.set(i, x),
        Small(ref mut s) => s.set(i, x)
      }
    }

    /**
     * Compares two bitvectors
     *
     * Both bitvectors must be the same length. Returns `true` if both
     * bitvectors contain identical elements.
     */
    #[inline]
    pub fn equal(&self, v1: &Bitv) -> bool {
      if self.nbits != v1.nbits { return false; }
      match self.rep {
        Small(ref b) => match v1.rep {
          Small(ref b1) => b.equals(*b1, self.nbits),
          _ => false
        },
        Big(ref s) => match v1.rep {
          Big(ref s1) => s.equals(*s1, self.nbits),
          Small(_) => return false
        }
      }
    }

    /// Set all bits to 0
    #[inline]
    pub fn clear(&mut self) {
        match self.rep {
          Small(ref mut b) => b.clear(),
          Big(ref mut s) => for s.each_storage() |w| { *w = 0u }
        }
    }

    /// Set all bits to 1
    #[inline]
    pub fn set_all(&mut self) {
      match self.rep {
        Small(ref mut b) => b.set_all(),
        Big(ref mut s) => for s.each_storage() |w| { *w = !0u } }
    }

    /// Invert all bits
    #[inline]
    pub fn invert(&mut self) {
      match self.rep {
        Small(ref mut b) => b.invert(),
        Big(ref mut s) => for s.each_storage() |w| { *w = !*w } }
    }

    /**
     * Calculate the difference between two bitvectors
     *
     * Sets each element of `v0` to the value of that element minus the
     * element of `v1` at the same index. Both bitvectors must be the same
     * length.
     *
     * Returns `true` if `v0` was changed.
     */
    #[inline]
    pub fn difference(&mut self, v: &Bitv) -> bool {
        self.do_op(Difference, v)
    }

    /// Returns true if all bits are 1
    #[inline]
    pub fn is_true(&self) -> bool {
      match self.rep {
        Small(ref b) => b.is_true(self.nbits),
        _ => {
          for self.each() |i| { if !i { return false; } }
          true
        }
      }
    }

    #[inline]
    pub fn each(&self, f: &fn(bool) -> bool) -> bool {
        let mut i = 0;
        while i < self.nbits {
            if !f(self.get(i)) { return false; }
            i += 1;
        }
        return true;
    }

    /// Returns true if all bits are 0
    pub fn is_false(&self) -> bool {
      match self.rep {
        Small(ref b) => b.is_false(self.nbits),
        Big(_) => {
          for self.each() |i| { if i { return false; } }
          true
        }
      }
    }

    pub fn init_to_vec(&self, i: uint) -> uint {
      return if self.get(i) { 1 } else { 0 };
    }

    /**
     * Converts `self` to a vector of uint with the same length.
     *
     * Each uint in the resulting vector has either value 0u or 1u.
     */
    pub fn to_vec(&self) -> ~[uint] {
        vec::from_fn(self.nbits, |x| self.init_to_vec(x))
    }

    /**
     * Organise the bits into bytes, such that the first bit in the
     * bitv becomes the high-order bit of the first byte. If the
     * size of the bitv is not a multiple of 8 then trailing bits
     * will be filled-in with false/0
     */
    pub fn to_bytes(&self) -> ~[u8] {
        fn bit (bitv: &Bitv, byte: uint, bit: uint) -> u8 {
            let offset = byte * 8 + bit;
            if offset >= bitv.nbits {
                0
            } else {
                bitv[offset] as u8 << (7 - bit)
            }
        }

        let len = self.nbits/8 +
                  if self.nbits % 8 == 0 { 0 } else { 1 };
        vec::from_fn(len, |i|
            bit(self, i, 0) |
            bit(self, i, 1) |
            bit(self, i, 2) |
            bit(self, i, 3) |
            bit(self, i, 4) |
            bit(self, i, 5) |
            bit(self, i, 6) |
            bit(self, i, 7)
        )
    }

    /**
     * Transform self into a [bool] by turning each bit into a bool
     */
    pub fn to_bools(&self) -> ~[bool] {
        vec::from_fn(self.nbits, |i| self[i])
    }

    /**
     * Converts `self` to a string.
     *
     * The resulting string has the same length as `self`, and each
     * character is either '0' or '1'.
     */
     pub fn to_str(&self) -> ~str {
        let mut rs = ~"";
        for self.each() |i| {
            if i {
                rs.push_char('1');
            } else {
                rs.push_char('0');
            }
        };
        rs
     }


    /**
     * Compare a bitvector to a vector of uint
     *
     * The uint vector is expected to only contain the values 0u and 1u. Both
     * the bitvector and vector must have the same length
     */
    pub fn eq_vec(&self, v: ~[uint]) -> bool {
        assert_eq!(self.nbits, v.len());
        let mut i = 0;
        while i < self.nbits {
            let w0 = self.get(i);
            let w1 = v[i];
            if !w0 && w1 != 0u || w0 && w1 == 0u { return false; }
            i = i + 1;
        }
        true
    }

    pub fn ones(&self, f: &fn(uint) -> bool) -> bool {
        uint::range(0, self.nbits, |i| !self.get(i) || f(i))
    }

}

impl Clone for Bitv {
    /// Makes a copy of a bitvector
    #[inline]
    fn clone(&self) -> Bitv {
        match self.rep {
          Small(ref b) => {
            Bitv{nbits: self.nbits, rep: Small(~SmallBitv{bits: b.bits})}
          }
          Big(ref b) => {
            let mut st = vec::from_elem(self.nbits / uint::bits + 1, 0);
            let len = st.len();
            for uint::range(0, len) |i| { st[i] = b.storage[i]; };
            Bitv{nbits: self.nbits, rep: Big(~BigBitv{storage: st})}
          }
        }
    }
}

/**
 * Transform a byte-vector into a bitv. Each byte becomes 8 bits,
 * with the most significant bits of each byte coming first. Each
 * bit becomes true if equal to 1 or false if equal to 0.
 */
pub fn from_bytes(bytes: &[u8]) -> Bitv {
    from_fn(bytes.len() * 8, |i| {
        let b = bytes[i / 8] as uint;
        let offset = i % 8;
        b >> (7 - offset) & 1 == 1
    })
}

/**
 * Transform a [bool] into a bitv by converting each bool into a bit.
 */
pub fn from_bools(bools: &[bool]) -> Bitv {
    from_fn(bools.len(), |i| bools[i])
}

/**
 * Create a bitv of the specified length where the value at each
 * index is f(index).
 */
pub fn from_fn(len: uint, f: &fn(index: uint) -> bool) -> Bitv {
    let mut bitv = Bitv::new(len, false);
    for uint::range(0, len) |i| {
        bitv.set(i, f(i));
    }
    bitv
}

impl ops::Index<uint,bool> for Bitv {
    fn index(&self, i: &uint) -> bool {
        self.get(*i)
    }
}

#[inline]
fn iterate_bits(base: uint, bits: uint, f: &fn(uint) -> bool) -> bool {
    if bits == 0 {
        return true;
    }
    for uint::range(0, uint::bits) |i| {
        if bits & (1 << i) != 0 {
            if !f(base + i) {
                return false;
            }
        }
    }
    return true;
}

/// An implementation of a set using a bit vector as an underlying
/// representation for holding numerical elements.
///
/// It should also be noted that the amount of storage necessary for holding a
/// set of objects is proportional to the maximum of the objects when viewed
/// as a uint.
pub struct BitvSet {
    priv size: uint,

    // In theory this is a Bitv instead of always a BigBitv, but knowing that
    // there's an array of storage makes our lives a whole lot easier when
    // performing union/intersection/etc operations
    priv bitv: BigBitv
}

impl BitvSet {
    /// Creates a new bit vector set with initially no contents
    pub fn new() -> BitvSet {
        BitvSet{ size: 0, bitv: BigBitv::new(~[0]) }
    }

    /// Creates a new bit vector set from the given bit vector
    pub fn from_bitv(bitv: Bitv) -> BitvSet {
        let mut size = 0;
        for bitv.ones |_| {
            size += 1;
        }
        let Bitv{rep, _} = bitv;
        match rep {
            Big(~b) => BitvSet{ size: size, bitv: b },
            Small(~SmallBitv{bits}) =>
                BitvSet{ size: size, bitv: BigBitv{ storage: ~[bits] } },
        }
    }

    /// Returns the capacity in bits for this bit vector. Inserting any
    /// element less than this amount will not trigger a resizing.
    pub fn capacity(&self) -> uint { self.bitv.storage.len() * uint::bits }

    /// Consumes this set to return the underlying bit vector
    pub fn unwrap(self) -> Bitv {
        let cap = self.capacity();
        let BitvSet{bitv, _} = self;
        return Bitv{ nbits:cap, rep: Big(~bitv) };
    }

    #[inline]
    fn other_op(&mut self, other: &BitvSet, f: &fn(uint, uint) -> uint) {
        fn nbits(mut w: uint) -> uint {
            let mut bits = 0;
            for uint::bits.times {
                if w == 0 {
                    break;
                }
                bits += w & 1;
                w >>= 1;
            }
            return bits;
        }
        if self.capacity() < other.capacity() {
            self.bitv.storage.grow(other.capacity() / uint::bits, &0);
        }
        for other.bitv.storage.iter().enumerate().advance |(i, &w)| {
            let old = self.bitv.storage[i];
            let new = f(old, w);
            self.bitv.storage[i] = new;
            self.size += nbits(new) - nbits(old);
        }
    }

    /// Union in-place with the specified other bit vector
    pub fn union_with(&mut self, other: &BitvSet) {
        self.other_op(other, |w1, w2| w1 | w2);
    }

    /// Intersect in-place with the specified other bit vector
    pub fn intersect_with(&mut self, other: &BitvSet) {
        self.other_op(other, |w1, w2| w1 & w2);
    }

    /// Difference in-place with the specified other bit vector
    pub fn difference_with(&mut self, other: &BitvSet) {
        self.other_op(other, |w1, w2| w1 & !w2);
    }

    /// Symmetric difference in-place with the specified other bit vector
    pub fn symmetric_difference_with(&mut self, other: &BitvSet) {
        self.other_op(other, |w1, w2| w1 ^ w2);
    }

    pub fn each(&self, blk: &fn(v: &uint) -> bool) -> bool {
        for self.bitv.storage.iter().enumerate().advance |(i, &w)| {
            if !iterate_bits(i * uint::bits, w, |b| blk(&b)) {
                return false;
            }
        }
        return true;
    }
}

impl cmp::Eq for BitvSet {
    fn eq(&self, other: &BitvSet) -> bool {
        if self.size != other.size {
            return false;
        }
        for self.each_common(other) |_, w1, w2| {
            if w1 != w2 {
                return false;
            }
        }
        for self.each_outlier(other) |_, _, w| {
            if w != 0 {
                return false;
            }
        }
        return true;
    }

    fn ne(&self, other: &BitvSet) -> bool { !self.eq(other) }
}

impl Container for BitvSet {
    fn len(&self) -> uint { self.size }
    fn is_empty(&self) -> bool { self.size == 0 }
}

impl Mutable for BitvSet {
    fn clear(&mut self) {
        for self.bitv.each_storage |w| { *w = 0; }
        self.size = 0;
    }
}

impl Set<uint> for BitvSet {
    fn contains(&self, value: &uint) -> bool {
        *value < self.bitv.storage.len() * uint::bits && self.bitv.get(*value)
    }

    fn insert(&mut self, value: uint) -> bool {
        if self.contains(&value) {
            return false;
        }
        let nbits = self.capacity();
        if value >= nbits {
            let newsize = uint::max(value, nbits * 2) / uint::bits + 1;
            assert!(newsize > self.bitv.storage.len());
            self.bitv.storage.grow(newsize, &0);
        }
        self.size += 1;
        self.bitv.set(value, true);
        return true;
    }

    fn remove(&mut self, value: &uint) -> bool {
        if !self.contains(value) {
            return false;
        }
        self.size -= 1;
        self.bitv.set(*value, false);

        // Attempt to truncate our storage
        let mut i = self.bitv.storage.len();
        while i > 1 && self.bitv.storage[i - 1] == 0 {
            i -= 1;
        }
        self.bitv.storage.truncate(i);

        return true;
    }

    fn is_disjoint(&self, other: &BitvSet) -> bool {
        for self.intersection(other) |_| {
            return false;
        }
        return true;
    }

    fn is_subset(&self, other: &BitvSet) -> bool {
        for self.each_common(other) |_, w1, w2| {
            if w1 & w2 != w1 {
                return false;
            }
        }
        /* If anything is not ours, then everything is not ours so we're
           definitely a subset in that case. Otherwise if there's any stray
           ones that 'other' doesn't have, we're not a subset. */
        for self.each_outlier(other) |mine, _, w| {
            if !mine {
                return true;
            } else if w != 0 {
                return false;
            }
        }
        return true;
    }

    fn is_superset(&self, other: &BitvSet) -> bool {
        other.is_subset(self)
    }

    fn difference(&self, other: &BitvSet, f: &fn(&uint) -> bool) -> bool {
        for self.each_common(other) |i, w1, w2| {
            if !iterate_bits(i, w1 & !w2, |b| f(&b)) {
                return false;
            }
        }
        /* everything we have that they don't also shows up */
        self.each_outlier(other, |mine, i, w|
            !mine || iterate_bits(i, w, |b| f(&b))
        )
    }

    fn symmetric_difference(&self, other: &BitvSet,
                            f: &fn(&uint) -> bool) -> bool {
        for self.each_common(other) |i, w1, w2| {
            if !iterate_bits(i, w1 ^ w2, |b| f(&b)) {
                return false;
            }
        }
        self.each_outlier(other, |_, i, w| iterate_bits(i, w, |b| f(&b)))
    }

    fn intersection(&self, other: &BitvSet, f: &fn(&uint) -> bool) -> bool {
        self.each_common(other, |i, w1, w2| iterate_bits(i, w1 & w2, |b| f(&b)))
    }

    fn union(&self, other: &BitvSet, f: &fn(&uint) -> bool) -> bool {
        for self.each_common(other) |i, w1, w2| {
            if !iterate_bits(i, w1 | w2, |b| f(&b)) {
                return false;
            }
        }
        self.each_outlier(other, |_, i, w| iterate_bits(i, w, |b| f(&b)))
    }
}

impl BitvSet {
    /// Visits each of the words that the two bit vectors (self and other)
    /// both have in common. The three yielded arguments are (bit location,
    /// w1, w2) where the bit location is the number of bits offset so far,
    /// and w1/w2 are the words coming from the two vectors self, other.
    fn each_common(&self, other: &BitvSet,
                   f: &fn(uint, uint, uint) -> bool) -> bool {
        let min = uint::min(self.bitv.storage.len(),
                            other.bitv.storage.len());
        self.bitv.storage.slice(0, min).iter().enumerate().advance(|(i, &w)| {
            f(i * uint::bits, w, other.bitv.storage[i])
        })
    }

    /// Visits each word in self or other that extends beyond the other. This
    /// will only iterate through one of the vectors, and it only iterates
    /// over the portion that doesn't overlap with the other one.
    ///
    /// The yielded arguments are a bool, the bit offset, and a word. The bool
    /// is true if the word comes from 'self', and false if it comes from
    /// 'other'.
    fn each_outlier(&self, other: &BitvSet,
                    f: &fn(bool, uint, uint) -> bool) -> bool {
        let len1 = self.bitv.storage.len();
        let len2 = other.bitv.storage.len();
        let min = uint::min(len1, len2);

        /* only one of these loops will execute and that's the point */
        for self.bitv.storage.slice(min, len1).iter().enumerate().advance |(i, &w)| {
            if !f(true, (i + min) * uint::bits, w) {
                return false;
            }
        }
        for other.bitv.storage.slice(min, len2).iter().enumerate().advance |(i, &w)| {
            if !f(false, (i + min) * uint::bits, w) {
                return false;
            }
        }
        return true;
    }
}

#[cfg(test)]
mod tests {
    use extra::test::BenchHarness;

    use bitv::*;
    use bitv;

    use std::uint;
    use std::vec;
    use std::rand;
    use std::rand::Rng;

    static BENCH_BITS : uint = 1 << 14;

    #[test]
    fn test_to_str() {
        let zerolen = Bitv::new(0u, false);
        assert_eq!(zerolen.to_str(), ~"");

        let eightbits = Bitv::new(8u, false);
        assert_eq!(eightbits.to_str(), ~"00000000");
    }

    #[test]
    fn test_0_elements() {
        let act = Bitv::new(0u, false);
        let exp = vec::from_elem::<uint>(0u, 0u);
        assert!(act.eq_vec(exp));
    }

    #[test]
    fn test_1_element() {
        let mut act = Bitv::new(1u, false);
        assert!(act.eq_vec(~[0u]));
        act = Bitv::new(1u, true);
        assert!(act.eq_vec(~[1u]));
    }

    #[test]
    fn test_2_elements() {
        let mut b = bitv::Bitv::new(2, false);
        b.set(0, true);
        b.set(1, false);
        assert_eq!(b.to_str(), ~"10");
    }

    #[test]
    fn test_10_elements() {
        let mut act;
        // all 0

        act = Bitv::new(10u, false);
        assert!((act.eq_vec(~[0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u])));
        // all 1

        act = Bitv::new(10u, true);
        assert!((act.eq_vec(~[1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u])));
        // mixed

        act = Bitv::new(10u, false);
        act.set(0u, true);
        act.set(1u, true);
        act.set(2u, true);
        act.set(3u, true);
        act.set(4u, true);
        assert!((act.eq_vec(~[1u, 1u, 1u, 1u, 1u, 0u, 0u, 0u, 0u, 0u])));
        // mixed

        act = Bitv::new(10u, false);
        act.set(5u, true);
        act.set(6u, true);
        act.set(7u, true);
        act.set(8u, true);
        act.set(9u, true);
        assert!((act.eq_vec(~[0u, 0u, 0u, 0u, 0u, 1u, 1u, 1u, 1u, 1u])));
        // mixed

        act = Bitv::new(10u, false);
        act.set(0u, true);
        act.set(3u, true);
        act.set(6u, true);
        act.set(9u, true);
        assert!((act.eq_vec(~[1u, 0u, 0u, 1u, 0u, 0u, 1u, 0u, 0u, 1u])));
    }

    #[test]
    fn test_31_elements() {
        let mut act;
        // all 0

        act = Bitv::new(31u, false);
        assert!(act.eq_vec(
                       ~[0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u,
                        0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u,
                        0u, 0u, 0u, 0u, 0u]));
        // all 1

        act = Bitv::new(31u, true);
        assert!(act.eq_vec(
                       ~[1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u,
                        1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u,
                        1u, 1u, 1u, 1u, 1u]));
        // mixed

        act = Bitv::new(31u, false);
        act.set(0u, true);
        act.set(1u, true);
        act.set(2u, true);
        act.set(3u, true);
        act.set(4u, true);
        act.set(5u, true);
        act.set(6u, true);
        act.set(7u, true);
        assert!(act.eq_vec(
                       ~[1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 0u, 0u, 0u, 0u, 0u,
                        0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u,
                        0u, 0u, 0u, 0u, 0u]));
        // mixed

        act = Bitv::new(31u, false);
        act.set(16u, true);
        act.set(17u, true);
        act.set(18u, true);
        act.set(19u, true);
        act.set(20u, true);
        act.set(21u, true);
        act.set(22u, true);
        act.set(23u, true);
        assert!(act.eq_vec(
                       ~[0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u,
                        0u, 0u, 0u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 0u, 0u,
                        0u, 0u, 0u, 0u, 0u]));
        // mixed

        act = Bitv::new(31u, false);
        act.set(24u, true);
        act.set(25u, true);
        act.set(26u, true);
        act.set(27u, true);
        act.set(28u, true);
        act.set(29u, true);
        act.set(30u, true);
        assert!(act.eq_vec(
                       ~[0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u,
                        0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 1u, 1u,
                        1u, 1u, 1u, 1u, 1u]));
        // mixed

        act = Bitv::new(31u, false);
        act.set(3u, true);
        act.set(17u, true);
        act.set(30u, true);
        assert!(act.eq_vec(
                       ~[0u, 0u, 0u, 1u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u,
                        0u, 0u, 0u, 0u, 1u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u,
                        0u, 0u, 0u, 0u, 1u]));
    }

    #[test]
    fn test_32_elements() {
        let mut act;
        // all 0

        act = Bitv::new(32u, false);
        assert!(act.eq_vec(
                       ~[0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u,
                        0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u,
                        0u, 0u, 0u, 0u, 0u, 0u]));
        // all 1

        act = Bitv::new(32u, true);
        assert!(act.eq_vec(
                       ~[1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u,
                        1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u,
                        1u, 1u, 1u, 1u, 1u, 1u]));
        // mixed

        act = Bitv::new(32u, false);
        act.set(0u, true);
        act.set(1u, true);
        act.set(2u, true);
        act.set(3u, true);
        act.set(4u, true);
        act.set(5u, true);
        act.set(6u, true);
        act.set(7u, true);
        assert!(act.eq_vec(
                       ~[1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 0u, 0u, 0u, 0u, 0u,
                        0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u,
                        0u, 0u, 0u, 0u, 0u, 0u]));
        // mixed

        act = Bitv::new(32u, false);
        act.set(16u, true);
        act.set(17u, true);
        act.set(18u, true);
        act.set(19u, true);
        act.set(20u, true);
        act.set(21u, true);
        act.set(22u, true);
        act.set(23u, true);
        assert!(act.eq_vec(
                       ~[0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u,
                        0u, 0u, 0u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 0u, 0u,
                        0u, 0u, 0u, 0u, 0u, 0u]));
        // mixed

        act = Bitv::new(32u, false);
        act.set(24u, true);
        act.set(25u, true);
        act.set(26u, true);
        act.set(27u, true);
        act.set(28u, true);
        act.set(29u, true);
        act.set(30u, true);
        act.set(31u, true);
        assert!(act.eq_vec(
                       ~[0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u,
                        0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 1u, 1u,
                        1u, 1u, 1u, 1u, 1u, 1u]));
        // mixed

        act = Bitv::new(32u, false);
        act.set(3u, true);
        act.set(17u, true);
        act.set(30u, true);
        act.set(31u, true);
        assert!(act.eq_vec(
                       ~[0u, 0u, 0u, 1u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u,
                        0u, 0u, 0u, 0u, 1u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u,
                        0u, 0u, 0u, 0u, 1u, 1u]));
    }

    #[test]
    fn test_33_elements() {
        let mut act;
        // all 0

        act = Bitv::new(33u, false);
        assert!(act.eq_vec(
                       ~[0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u,
                        0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u,
                        0u, 0u, 0u, 0u, 0u, 0u, 0u]));
        // all 1

        act = Bitv::new(33u, true);
        assert!(act.eq_vec(
                       ~[1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u,
                        1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u,
                        1u, 1u, 1u, 1u, 1u, 1u, 1u]));
        // mixed

        act = Bitv::new(33u, false);
        act.set(0u, true);
        act.set(1u, true);
        act.set(2u, true);
        act.set(3u, true);
        act.set(4u, true);
        act.set(5u, true);
        act.set(6u, true);
        act.set(7u, true);
        assert!(act.eq_vec(
                       ~[1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 0u, 0u, 0u, 0u, 0u,
                        0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u,
                        0u, 0u, 0u, 0u, 0u, 0u, 0u]));
        // mixed

        act = Bitv::new(33u, false);
        act.set(16u, true);
        act.set(17u, true);
        act.set(18u, true);
        act.set(19u, true);
        act.set(20u, true);
        act.set(21u, true);
        act.set(22u, true);
        act.set(23u, true);
        assert!(act.eq_vec(
                       ~[0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u,
                        0u, 0u, 0u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u, 0u, 0u,
                        0u, 0u, 0u, 0u, 0u, 0u, 0u]));
        // mixed

        act = Bitv::new(33u, false);
        act.set(24u, true);
        act.set(25u, true);
        act.set(26u, true);
        act.set(27u, true);
        act.set(28u, true);
        act.set(29u, true);
        act.set(30u, true);
        act.set(31u, true);
        assert!(act.eq_vec(
                       ~[0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u,
                        0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 1u, 1u,
                        1u, 1u, 1u, 1u, 1u, 1u, 0u]));
        // mixed

        act = Bitv::new(33u, false);
        act.set(3u, true);
        act.set(17u, true);
        act.set(30u, true);
        act.set(31u, true);
        act.set(32u, true);
        assert!(act.eq_vec(
                       ~[0u, 0u, 0u, 1u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u,
                        0u, 0u, 0u, 0u, 1u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u,
                        0u, 0u, 0u, 0u, 1u, 1u, 1u]));
    }

    #[test]
    fn test_equal_differing_sizes() {
        let v0 = Bitv::new(10u, false);
        let v1 = Bitv::new(11u, false);
        assert!(!v0.equal(&v1));
    }

    #[test]
    fn test_equal_greatly_differing_sizes() {
        let v0 = Bitv::new(10u, false);
        let v1 = Bitv::new(110u, false);
        assert!(!v0.equal(&v1));
    }

    #[test]
    fn test_equal_sneaky_small() {
        let mut a = bitv::Bitv::new(1, false);
        a.set(0, true);

        let mut b = bitv::Bitv::new(1, true);
        b.set(0, true);

        assert!(a.equal(&b));
    }

    #[test]
    fn test_equal_sneaky_big() {
        let mut a = bitv::Bitv::new(100, false);
        for uint::range(0, 100) |i| {
            a.set(i, true);
        }

        let mut b = bitv::Bitv::new(100, true);
        for uint::range(0, 100) |i| {
            b.set(i, true);
        }

        assert!(a.equal(&b));
    }

    #[test]
    fn test_from_bytes() {
        let bitv = from_bytes([0b10110110, 0b00000000, 0b11111111]);
        let str = ~"10110110" + "00000000" + "11111111";
        assert_eq!(bitv.to_str(), str);
    }

    #[test]
    fn test_to_bytes() {
        let mut bv = Bitv::new(3, true);
        bv.set(1, false);
        assert_eq!(bv.to_bytes(), ~[0b10100000]);

        let mut bv = Bitv::new(9, false);
        bv.set(2, true);
        bv.set(8, true);
        assert_eq!(bv.to_bytes(), ~[0b00100000, 0b10000000]);
    }

    #[test]
    fn test_from_bools() {
        assert!(from_bools([true, false, true, true]).to_str() ==
            ~"1011");
    }

    #[test]
    fn test_to_bools() {
        let bools = ~[false, false, true, false, false, true, true, false];
        assert_eq!(from_bytes([0b00100110]).to_bools(), bools);
    }

    #[test]
    fn test_small_difference() {
        let mut b1 = Bitv::new(3, false);
        let mut b2 = Bitv::new(3, false);
        b1.set(0, true);
        b1.set(1, true);
        b2.set(1, true);
        b2.set(2, true);
        assert!(b1.difference(&b2));
        assert!(b1[0]);
        assert!(!b1[1]);
        assert!(!b1[2]);
    }

    #[test]
    fn test_big_difference() {
        let mut b1 = Bitv::new(100, false);
        let mut b2 = Bitv::new(100, false);
        b1.set(0, true);
        b1.set(40, true);
        b2.set(40, true);
        b2.set(80, true);
        assert!(b1.difference(&b2));
        assert!(b1[0]);
        assert!(!b1[40]);
        assert!(!b1[80]);
    }

    #[test]
    fn test_small_clear() {
        let mut b = Bitv::new(14, true);
        b.clear();
        for b.ones |i| {
            fail!("found 1 at %?", i);
        }
    }

    #[test]
    fn test_big_clear() {
        let mut b = Bitv::new(140, true);
        b.clear();
        for b.ones |i| {
            fail!("found 1 at %?", i);
        }
    }

    #[test]
    fn test_bitv_set_basic() {
        let mut b = BitvSet::new();
        assert!(b.insert(3));
        assert!(!b.insert(3));
        assert!(b.contains(&3));
        assert!(b.insert(400));
        assert!(!b.insert(400));
        assert!(b.contains(&400));
        assert_eq!(b.len(), 2);
    }

    #[test]
    fn test_bitv_set_intersection() {
        let mut a = BitvSet::new();
        let mut b = BitvSet::new();

        assert!(a.insert(11));
        assert!(a.insert(1));
        assert!(a.insert(3));
        assert!(a.insert(77));
        assert!(a.insert(103));
        assert!(a.insert(5));

        assert!(b.insert(2));
        assert!(b.insert(11));
        assert!(b.insert(77));
        assert!(b.insert(5));
        assert!(b.insert(3));

        let mut i = 0;
        let expected = [3, 5, 11, 77];
        for a.intersection(&b) |x| {
            assert_eq!(*x, expected[i]);
            i += 1
        }
        assert_eq!(i, expected.len());
    }

    #[test]
    fn test_bitv_set_difference() {
        let mut a = BitvSet::new();
        let mut b = BitvSet::new();

        assert!(a.insert(1));
        assert!(a.insert(3));
        assert!(a.insert(5));
        assert!(a.insert(200));
        assert!(a.insert(500));

        assert!(b.insert(3));
        assert!(b.insert(200));

        let mut i = 0;
        let expected = [1, 5, 500];
        for a.difference(&b) |x| {
            assert_eq!(*x, expected[i]);
            i += 1
        }
        assert_eq!(i, expected.len());
    }

    #[test]
    fn test_bitv_set_symmetric_difference() {
        let mut a = BitvSet::new();
        let mut b = BitvSet::new();

        assert!(a.insert(1));
        assert!(a.insert(3));
        assert!(a.insert(5));
        assert!(a.insert(9));
        assert!(a.insert(11));

        assert!(b.insert(3));
        assert!(b.insert(9));
        assert!(b.insert(14));
        assert!(b.insert(220));

        let mut i = 0;
        let expected = [1, 5, 11, 14, 220];
        for a.symmetric_difference(&b) |x| {
            assert_eq!(*x, expected[i]);
            i += 1
        }
        assert_eq!(i, expected.len());
    }

    #[test]
    fn test_bitv_set_union() {
        let mut a = BitvSet::new();
        let mut b = BitvSet::new();
        assert!(a.insert(1));
        assert!(a.insert(3));
        assert!(a.insert(5));
        assert!(a.insert(9));
        assert!(a.insert(11));
        assert!(a.insert(160));
        assert!(a.insert(19));
        assert!(a.insert(24));

        assert!(b.insert(1));
        assert!(b.insert(5));
        assert!(b.insert(9));
        assert!(b.insert(13));
        assert!(b.insert(19));

        let mut i = 0;
        let expected = [1, 3, 5, 9, 11, 13, 19, 24, 160];
        for a.union(&b) |x| {
            assert_eq!(*x, expected[i]);
            i += 1
        }
        assert_eq!(i, expected.len());
    }

    #[test]
    fn test_bitv_remove() {
        let mut a = BitvSet::new();

        assert!(a.insert(1));
        assert!(a.remove(&1));

        assert!(a.insert(100));
        assert!(a.remove(&100));

        assert!(a.insert(1000));
        assert!(a.remove(&1000));
        assert_eq!(a.capacity(), uint::bits);
    }

    fn rng() -> rand::IsaacRng {
        let seed = [1, 2, 3, 4, 5, 6, 7, 8, 9, 0];
        rand::IsaacRng::new_seeded(seed)
    }

    #[bench]
    fn bench_uint_small(b: &mut BenchHarness) {
        let mut r = rng();
        let mut bitv = 0 as uint;
        do b.iter {
            bitv |= (1 << ((r.next() as uint) % uint::bits));
        }
    }

    #[bench]
    fn bench_small_bitv_small(b: &mut BenchHarness) {
        let mut r = rng();
        let mut bitv = SmallBitv::new(uint::bits);
        do b.iter {
            bitv.set((r.next() as uint) % uint::bits, true);
        }
    }

    #[bench]
    fn bench_big_bitv_small(b: &mut BenchHarness) {
        let mut r = rng();
        let mut bitv = BigBitv::new(~[0]);
        do b.iter {
            bitv.set((r.next() as uint) % uint::bits, true);
        }
    }

    #[bench]
    fn bench_big_bitv_big(b: &mut BenchHarness) {
        let mut r = rng();
        let mut storage = ~[];
        storage.grow(BENCH_BITS / uint::bits, &0);
        let mut bitv = BigBitv::new(storage);
        do b.iter {
            bitv.set((r.next() as uint) % BENCH_BITS, true);
        }
    }

    #[bench]
    fn bench_bitv_big(b: &mut BenchHarness) {
        let mut r = rng();
        let mut bitv = Bitv::new(BENCH_BITS, false);
        do b.iter {
            bitv.set((r.next() as uint) % BENCH_BITS, true);
        }
    }

    #[bench]
    fn bench_bitv_small(b: &mut BenchHarness) {
        let mut r = rng();
        let mut bitv = Bitv::new(uint::bits, false);
        do b.iter {
            bitv.set((r.next() as uint) % uint::bits, true);
        }
    }

    #[bench]
    fn bench_bitv_set_small(b: &mut BenchHarness) {
        let mut r = rng();
        let mut bitv = BitvSet::new();
        do b.iter {
            bitv.insert((r.next() as uint) % uint::bits);
        }
    }

    #[bench]
    fn bench_bitv_set_big(b: &mut BenchHarness) {
        let mut r = rng();
        let mut bitv = BitvSet::new();
        do b.iter {
            bitv.insert((r.next() as uint) % BENCH_BITS);
        }
    }

    #[bench]
    fn bench_bitv_big_union(b: &mut BenchHarness) {
        let mut b1 = Bitv::new(BENCH_BITS, false);
        let b2 = Bitv::new(BENCH_BITS, false);
        do b.iter {
            b1.union(&b2);
        }
    }
}
