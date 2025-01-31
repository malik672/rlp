use crate::header::{Header, EMPTY_STRING_CODE, length_of_length};
use bytes::{BufMut, Bytes, BytesMut};
use core::{
     borrow::Borrow, marker::{PhantomData, PhantomPinned}
};

use std::{borrow::Cow, rc::Rc, sync::Arc};
#[allow(unused_imports)]
use std::vec::Vec;

pub trait Encodable {
    fn encode(&self, out: &mut dyn BufMut);

    /// Returns the length of the encoding in bytes.
    #[inline]
    fn length(&self) -> usize {
        let mut out = Vec::new();
        self.encode(&mut out);
        out.len()
    }
}

// This function ensures Encodable is object-safe
fn _assert_trait_object(_b: &dyn Encodable) {}

/// Defines the max length of an Encodable type as a const generic.
/// # Safety: An invalid value can cause encoder panics.
pub unsafe trait MaxEncodedLen<const LEN: usize>: Encodable {}

/// Defines the max length of an Encodable type as an associated constant.
/// # Safety: An invalid value can cause encoder panics.
pub unsafe trait MaxEncodedLenAssoc: Encodable {
    const LEN: usize;
}

// Implement core primitive types
macro_rules! impl_uint {
    ($($t:ty),+ $(,)?) => {$(
        impl Encodable for $t {
            #[inline]
            fn length(&self) -> usize {
                let x = *self;
                if x < EMPTY_STRING_CODE as $t {
                    1
                } else {
                    1 + (<$t>::BITS as usize / 8) - (x.leading_zeros() as usize / 8)
                }
            }

            #[inline]
            fn encode(&self, out: &mut dyn BufMut) {
                let x = *self;
                if x == 0 {
                    out.put_u8(EMPTY_STRING_CODE);
                } else if x < EMPTY_STRING_CODE as $t {
                    out.put_u8(x as u8);
                } else {
                    let be;
                    let be = to_be_bytes_trimmed!(be, x);
                    out.put_u8(EMPTY_STRING_CODE + be.len() as u8);
                    out.put_slice(be);
                }
            }
        }

        unsafe impl MaxEncodedLenAssoc for $t {
            const LEN: usize = {
                let bytes = <$t>::BITS as usize / 8;
                bytes + length_of_length(bytes)
            };
        }
    )+};
}

macro_rules! to_be_bytes_trimmed {
    ($be:ident, $x:expr) => {{
        $be = $x.to_be_bytes();
        &$be[($x.leading_zeros() / 8) as usize..]
    }};
}

impl_uint!(u8, u16, u32, u64, usize, u128);

// Implement for slices and basic types
impl Encodable for [u8] {
    #[inline]
    fn length(&self) -> usize {
        let mut len = self.len();
        if len != 1 || self[0] >= EMPTY_STRING_CODE {
            len += length_of_length(len);
        }
        len
    }

    #[inline]
    fn encode(&self, out: &mut dyn BufMut) {
        if self.len() != 1 || self[0] >= EMPTY_STRING_CODE {
            Header::new(false, self.len()).encode(out);
        }
        out.put_slice(self);
    }
}

impl Encodable for str {
    #[inline]
    fn length(&self) -> usize {
        self.as_bytes().length()
    }

    #[inline]
    fn encode(&self, out: &mut dyn BufMut) {
        self.as_bytes().encode(out)
    }
}

impl<T: Encodable> Encodable for Vec<T> {
    #[inline]
    fn length(&self) -> usize {
        list_length(self)
    }

    #[inline]
    fn encode(&self, out: &mut dyn BufMut) {
        encode_list(self, out)
    }
}

// Implement for wrapper types
macro_rules! impl_wrapper {
    ($($(#[$attr:meta])* [$($gen:tt)*] $t:ty),+ $(,)?) => {$(
        $(#[$attr])*
        impl<$($gen)*> Encodable for $t {
            #[inline]
            fn length(&self) -> usize {
                (**self).length()
            }

            #[inline]
            fn encode(&self, out: &mut dyn BufMut) {
                (**self).encode(out)
            }
        }
    )+};
}

impl_wrapper! {
    [] String,
    [] Bytes,
    [] BytesMut,
    [T: ?Sized + Encodable] &T,
    [T: ?Sized + Encodable] &mut T,
    [T: ?Sized + Encodable] Box<T>,
    [T: ?Sized + Encodable] Rc<T>,
    [T: ?Sized + Encodable] Arc<T>,
    [T: ?Sized + ToOwned + Encodable] Cow<'_, T>,
}

// Zero-sized type implementations
impl<T: ?Sized> Encodable for PhantomData<T> {
    #[inline]
    fn length(&self) -> usize { 0 }
    #[inline]
    fn encode(&self, _out: &mut dyn BufMut) {}
}

impl Encodable for PhantomPinned {
    #[inline]
    fn length(&self) -> usize { 0 }
    #[inline]
    fn encode(&self, _out: &mut dyn BufMut) {}
}

#[inline]
pub fn encode<T: Encodable>(value: T) -> Vec<u8> {
    let mut out = Vec::with_capacity(value.length());
    value.encode(&mut out);
    out
}

#[inline]
pub fn encode_list<B, T>(values: &[B], out: &mut dyn BufMut)
where
    B: Borrow<T>,
    T: ?Sized + Encodable,
{
    let h = list_header(values);
    h.encode(out);
    for value in values {
        value.borrow().encode(out);
    }
}

#[inline]
pub fn encode_iter<I, B, T>(values: I, out: &mut dyn BufMut)
where
    I: Iterator<Item = B> + Clone,
    B: Borrow<T>,
    T: ?Sized + Encodable,
{
    let mut h = Header { packed: 0 };
    for t in values.clone() {
        h = Header::new(true, h.payload_length() + t.borrow().length());
    }
    h.encode(out);
    for value in values {
        value.borrow().encode(out);
    }
}

#[inline]
pub fn list_length<B, T>(list: &[B]) -> usize
where
    B: Borrow<T>,
    T: ?Sized + Encodable,
{
    
    let h = list_header(list);
    h.payload_length() + h.length()
}

#[inline]
fn list_header<B, T>(values: &[B]) -> Header
where
    B: Borrow<T>,
    T: ?Sized + Encodable,
{
    let mut len = 0;
    for value in values {
        len += value.borrow().length();
    }

    Header::new(true, len)
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex_literal::hex;

    #[test]
    fn encode_integers() {
        assert_eq!(encode(0u8), hex!("80"));
        assert_eq!(encode(1u8), hex!("01"));
        assert_eq!(encode(0x7Fu8), hex!("7F"));
        assert_eq!(encode(0x80u8), hex!("8180"));
    }

    #[test]
    fn encode_strings() {
        assert_eq!(encode(""), hex!("80"));
        assert_eq!(encode("test"), hex!("8474657374"));
    }

    #[test]
    fn encode_lists() {
        assert_eq!(encode(Vec::<u8>::new()), hex!("c0"));
        assert_eq!(encode(vec![0xFFu8, 0xFFu8]), hex!("c481ff81ff"));
    }
}