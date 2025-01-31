use std::hint::unreachable_unchecked;

use bytes::{Buf as _, BufMut};
use crate::{copy_from_slice, error::{Error, Result}};

//SHORT STRING || SHORT_LIST | EMPTY_STRING
pub const EMPTY_STRING_CODE: u8 = 0x80;
pub const EMPTY_LIST_CODE: u8 = 0xC0;
pub const MAX_SHORT_LEN: usize = 55;
pub const LONG_STRING_OFFSET: u8 = 0xB7;
pub const LONG_LIST_OFFSET: u8 = 0xF7;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Header {
    // Combined list flag and payload length into a single field
    // MSB is the flag, this is done to avoid padding and additional memory wastage(might do nothing tbh but still valid)
    // allows optimium cache efficiency
    pub packed: usize, //(Length x Bool packeddddddddddddd)
}

impl Header {
    #[inline(always)]
    pub const fn new(list: bool, payload_length: usize) -> Self {
        Self {
            packed: payload_length | (if list { 1 << (usize::BITS - 1) } else { 0 }),
        }
    }

    #[inline(always)]
    pub const fn list(&self) -> bool {
        self.packed & (1 << (usize::BITS - 1)) != 0
    }

    #[inline(always)]
    pub const fn payload_length(&self) -> usize {
        self.packed & !(1 << (usize::BITS - 1))
    }

    pub fn decode(buf: &mut &[u8]) -> Result<Self> {
        let mut list = false;
        let payload_length;

        match get_next_byte(buf)? {
            0..=0x7F => payload_length = 1,

            b @ EMPTY_STRING_CODE..=LONG_STRING_OFFSET => {
                buf.advance(1);
                payload_length = (b - EMPTY_STRING_CODE) as usize;

                // Validate canonical encoding
                if payload_length == 1 && get_next_byte(buf)? < EMPTY_STRING_CODE {
                    return Err(Error::NonCanonicalSingleByte);
                }
            }

            b @ (0xB8..=0xBF | 0xF8..=0xFF) => {
                buf.advance(1);
                list = b >= 0xF8;
                let offset = if list {
                    LONG_LIST_OFFSET
                } else {
                    LONG_STRING_OFFSET
                };

                // SAFETY: b is in range that makes this subtraction valid
                let len_of_len = unsafe { b.checked_sub(offset).unwrap_unchecked() } as usize;

                if len_of_len == 0 || len_of_len > 8 {
                    unsafe { unreachable_unchecked() }
                }

                if buf.len() < len_of_len {
                    return Err(Error::InputTooShort);
                }

                // SAFETY: length checked above
                let len = unsafe { buf.get_unchecked(..len_of_len) };
                buf.advance(len_of_len);

                let len = u64::from_be_bytes(static_left_pad(len)?);
                payload_length = usize::try_from(len).map_err(|_| Error::UnexpectedLength)?;

                if payload_length <= MAX_SHORT_LEN {
                    return Err(Error::NonCanonicalSize);
                }
            }

            // Short list
            b @ EMPTY_LIST_CODE..=LONG_LIST_OFFSET => {
                buf.advance(1);
                list = true;
                payload_length = (b - EMPTY_LIST_CODE) as usize;
            }
        }

        if buf.remaining() < payload_length {
            return Err(Error::InputTooShort);
        }

        Ok(Self::new(list, payload_length))
    }

    #[inline(always)]
    pub fn encode(&self, out: &mut dyn BufMut) {
        // println!("Debug: packed={:b}, payload_length={}", self.packed, self.payload_length());
        let payload_length = self.payload_length();
        if payload_length <= MAX_SHORT_LEN {
            let offset = if self.list() { EMPTY_LIST_CODE } else { EMPTY_STRING_CODE };
            out.put_u8(offset + payload_length as u8);
        } else {
            let len_be = to_be_bytes_trimmed(payload_length);
            let offset = if self.list() { LONG_LIST_OFFSET } else { LONG_STRING_OFFSET };
            out.put_u8(offset + len_be.len() as u8);
            out.put_slice(&len_be);
        }
    }

    #[inline]
    pub fn decode_bytes<'a>(buf: &mut &'a [u8], list: bool) -> Result<&'a [u8]> {
        let header = Self::decode(buf)?;
        
        if header.list() != list {
            return Err(if list {
                Error::UnexpectedString
            } else {
                Error::UnexpectedList
            });
        }

        let payload_length = header.payload_length();
        if buf.len() < payload_length {
            return Err(Error::InputTooShort);
        }

        let bytes = &buf[..payload_length];
        *buf = &buf[payload_length..];
        
        Ok(bytes)
    }

    #[inline]
    pub fn decode_str<'a>(buf: &mut &'a [u8]) -> Result<&'a str> {
        let bytes = Self::decode_bytes(buf, false)?;
        core::str::from_utf8(bytes).map_err(|_| Error::UnexpectedString)
    }

    #[inline(always)]
    pub const fn length(&self) -> usize {
        length_of_length(self.payload_length())
    }
}

#[inline(always)]
fn get_next_byte(buf: &[u8]) -> Result<u8> {
    buf.first().copied().ok_or(Error::InputTooShort)
}

#[inline(always)]
pub unsafe fn advance_unchecked<'a>(buf: &mut &'a [u8], cnt: usize) -> &'a [u8] {
    debug_assert!(buf.len() >= cnt);
    let bytes = &buf[..cnt];
    buf.advance(cnt);
    bytes
}



#[inline(always)]
pub const fn length_of_length(payload_length: usize) -> usize {
    if payload_length <= MAX_SHORT_LEN {
        1
    } else {
        1 + (usize::BITS as usize / 8) - (payload_length.leading_zeros() as usize / 8)
    }
}

#[inline(always)]
fn to_be_bytes_trimmed(x: usize) -> Vec<u8> {
    let be = x.to_be_bytes();
    let skip = be.iter().take_while(|&&b| b == 0).count();
    be[skip..].to_vec()
}

#[inline(always)]
fn static_left_pad<const N: usize>(data: &[u8]) -> Result<[u8; N]> {
    if data.len() > N {
        return Err(Error::Overflow);
    }

    let mut v = [0; N];
    if !data.is_empty() {
        if data[0] == 0 {
            return Err(Error::LeadingZero);
        }
        copy_from_slice(&mut v[N - data.len()..], data);
    }
    Ok(v)
}