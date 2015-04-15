use num::bigint::ToBigUint;
use num::{BigUint, Zero, One};
use std::fmt;

pub use self::FromBase58Error::*;

static BTC_ALPHA: &'static[u8] = b"123456789\
                                   ABCDEFGHJKLMNPQRSTUVWXYZ\
                                   abcdefghijkmnopqrstuvwxyz";

static FLICKR_ALPHA: &'static[u8] = b"123456789\
                                      abcdefghijkmnopqrstuvwxyz\
                                      ABCDEFGHJKLMNPQRSTUVWXYZ";

/// A trait for converting base58-encoded values
// TODO: This should incorporate the alphabet used as an associated constant. 
// However, associated constants are not implemented in Rust yet. There is a
// PR though: https://github.com/rust-lang/rust/pull/23606
pub trait FromBase58 {
    /// Converts the value of `self`, interpreted as base58 encoded data,
    /// into an owned vector of bytes, returning the vector.
    fn from_base58(&self) -> Result<Vec<u8>, FromBase58Error>;
}


/// Errors that can occur when decoding a base58-encoded string
#[derive(Clone, Copy)]
pub enum FromBase58Error {
    /// The input contained a character not part of the base58 alphabet
    InvalidBase58Byte(u8, usize),
}

impl fmt::Debug for FromBase58Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            InvalidBase58Byte(ch, idx) =>
                write!(f, "Invalid character '{}' at position {}", ch, idx),
        }
    }
}

impl fmt::Display for FromBase58Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&self, f)
    }
}



impl FromBase58 for str {
    fn from_base58(&self) -> Result<Vec<u8>, FromBase58Error> {
        self.as_bytes().from_base58()
    }
}

impl FromBase58 for [u8] {
    // TODO: fix some of the below when the binary assignment operators +=, *=
    // are overloadable
    fn from_base58(&self) -> Result<Vec<u8>, FromBase58Error> {
        let radix = 58.to_biguint().unwrap();
        let mut x: BigUint = Zero::zero();
        let mut rad_mult: BigUint = One::one();

        // Convert the base58 string to a BigUint `x`
        for (idx, &byte) in self.iter().enumerate().rev() {
            let first_idx = BTC_ALPHA.iter()
                                     .enumerate()
                                     .find(|x| *x.1 == byte)
                                     .map(|x| x.0);
            match first_idx {
                Some(i) => { x = x + i.to_biguint().unwrap() * &rad_mult; },
                None => return Err(InvalidBase58Byte(self[idx], idx))
            }

            rad_mult = &rad_mult * &radix;
        }

        let mut r = Vec::with_capacity(self.len());
        for _ in self.iter().take_while(|&x| *x == BTC_ALPHA[0]) {
            r.push(0);
        }
        if x > Zero::zero() {
            r.append(&mut x.to_bytes_be());
        }
        Ok(r)
    }
}


#[cfg(test)]
mod tests {
    use base58::FromBase58;

    #[test]
    fn test_from_base58_basic() {
        assert_eq!("".from_base58().unwrap(), b"");
        assert_eq!("z".from_base58().unwrap(), b"9");
        assert_eq!("21".from_base58().unwrap(), b":");
        assert_eq!("2g".from_base58().unwrap(), b"a");
        assert_eq!("8Qq".from_base58().unwrap(), b"ab");
        assert_eq!("ZiCa".from_base58().unwrap(), b"abc");
    }

    #[test]
    fn test_from_base58_bytes() {
        assert_eq!(b"ZiCa".from_base58().unwrap(), b"abc");
    }

    #[test]
    fn test_from_base58_invalid_char() {
        assert!("AC0".from_base58().is_err());
        assert!("s5<".from_base58().is_err());
    }

    #[test]
    fn test_from_base58_initial_zeros() {
        assert_eq!("1ZiCa".from_base58().unwrap(), b"\0abc");
        assert_eq!("11ZiCa".from_base58().unwrap(), b"\0\0abc");
        assert_eq!("111ZiCa".from_base58().unwrap(), b"\0\0\0abc");
        assert_eq!("1111ZiCa".from_base58().unwrap(), b"\0\0\0\0abc");
    }
}
