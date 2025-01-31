use crate::{copy_from_slice, error::{Error, Result}, header::Header};
use bytes::{Bytes, BytesMut};
use core::marker::{PhantomData, PhantomPinned};

pub trait Decodable: Sized {
    fn decode(buf: &mut &[u8]) -> Result<Self>;
}

pub struct Rlp<'a> {
    payload_view: &'a [u8],
}

impl<'a> Rlp<'a> {
    pub fn new(mut payload: &'a [u8]) -> Result<Self> {
        let payload_view = Header::decode_bytes(&mut payload, true)?;
        Ok(Self { payload_view })
    }

    #[inline]
    pub fn get_next<T: Decodable>(&mut self) -> Result<Option<T>> {
        if self.payload_view.is_empty() {
            Ok(None)
        } else {
            T::decode(&mut self.payload_view).map(Some)
        }
    }
}

impl<T: ?Sized> Decodable for PhantomData<T> {
    fn decode(_buf: &mut &[u8]) -> Result<Self> {
        Ok(Self)
    }
}

impl Decodable for PhantomPinned {
    fn decode(_buf: &mut &[u8]) -> Result<Self> {
        Ok(Self)
    }
}

impl Decodable for bool {
    #[inline]
    fn decode(buf: &mut &[u8]) -> Result<Self> {
        Ok(match u8::decode(buf)? {
            0 => false,
            1 => true,
            _ => return Err(Error::InputTooShort),
        })
    }
}

// Fixed-size array implementation
impl<const N: usize> Decodable for [u8; N] {
    #[inline]
    fn decode(from: &mut &[u8]) -> Result<Self> {
        let bytes = Header::decode_bytes(from, false)?;
        Self::try_from(bytes).map_err(|_| Error::UnexpectedLength)
    }
}


macro_rules! decode_integer {
    ($($t:ty),+ $(,)?) => {$(
        impl Decodable for $t {
            #[inline]
            fn decode(buf: &mut &[u8]) -> Result<Self> {
                let bytes = Header::decode_bytes(buf, false)?;
                static_left_pad(bytes).map(<$t>::from_be_bytes)
            }
        }
    )+};
}

decode_integer!(u8, u16, u32, u64, usize, u128);

impl Decodable for Bytes {
    #[inline]
    fn decode(buf: &mut &[u8]) -> Result<Self> {
        Header::decode_bytes(buf, false).map(|x| Self::from(x.to_vec()))
    }
}

impl Decodable for BytesMut {
    #[inline]
    fn decode(buf: &mut &[u8]) -> Result<Self> {
        Header::decode_bytes(buf, false).map(Self::from)
    }
}


impl Decodable for String {
    #[inline]
    fn decode(buf: &mut &[u8]) -> Result<Self> {
        Header::decode_str(buf).map(Into::into)
    }
}

impl<T: Decodable> Decodable for Vec<T> {
    #[inline]
    fn decode(buf: &mut &[u8]) -> Result<Self> {
        let mut bytes = Header::decode_bytes(buf, true)?;
        let mut vec = Self::new();
        while !bytes.is_empty() {
            vec.push(T::decode(&mut bytes)?);
        }
        Ok(vec)
    }
}


#[inline]
pub fn decode_exact<T: Decodable>(bytes: impl AsRef<[u8]>) -> Result<T> {
    let mut buf = bytes.as_ref();
    let out = T::decode(&mut buf)?;

    if !buf.is_empty() {
        return Err(Error::UnexpectedLength);
    }

    Ok(out)
}

#[inline]
pub(crate) fn static_left_pad<const N: usize>(data: &[u8]) -> Result<[u8; N]> {
    if data.len() > N {
        return Err(Error::Overflow);
    }

    let mut v = [0; N];
    if !data.is_empty() {
        if unsafe { *data.get_unchecked(0) } == 0 {
            return Err(Error::LeadingZero);
        }
        copy_from_slice(&mut v[N - data.len()..], data);
    }
    Ok(v)
}