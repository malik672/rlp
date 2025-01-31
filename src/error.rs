use core::fmt;

/// RLP result type alias
pub type Result<T, E = Error> = core::result::Result<T, E>;

/// Compact RLP error codes
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Error {
    Overflow = 1,
    LeadingZero = 2,
    InputTooShort = 3,
    NonCanonicalSingleByte = 4,
    NonCanonicalSize = 5,
    UnexpectedLength = 6,
    UnexpectedString = 7,
    UnexpectedList = 8,
    ListLengthMismatch(usize, usize) = 9,
}


impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Overflow => write!(f, "Numeric overflow occurred"),
            Error::LeadingZero => write!(f, "Leading zeros in input"),
            Error::InputTooShort => write!(f, "Input data is too short"),
            Error::NonCanonicalSingleByte => write!(f, "Non-canonical single-byte encoding"),
            Error::NonCanonicalSize => write!(f, "Non-canonical size encoding"),
            Error::UnexpectedLength => write!(f, "Unexpected length in input"),
            Error::UnexpectedString => write!(f, "Unexpected string in input"),
            Error::UnexpectedList => write!(f, "Unexpected list in input"),
            Error::ListLengthMismatch(expected, actual) => {
                write!(f, "List length mismatch: expected {}, got {}", expected, actual)
            }
        }
    }
}