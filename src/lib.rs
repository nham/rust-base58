extern crate num;
extern crate sha2;
#[cfg(test)] extern crate rand;

use num::bigint::ToBigUint;
use num::{BigUint, Zero, One};
use num::traits::ToPrimitive;
use sha2::{Sha256, Digest};
use std::fmt;

pub use self::FromBase58Error::*;

const BTC_ALPHA: &'static[u8] = b"123456789\
                                  ABCDEFGHJKLMNPQRSTUVWXYZ\
                                  abcdefghijkmnopqrstuvwxyz";

const FLICKR_ALPHA: &'static[u8] = b"123456789\
                                     abcdefghijkmnopqrstuvwxyz\
                                     ABCDEFGHJKLMNPQRSTUVWXYZ";

/// A trait for converting base58-encoded values
pub trait FromBase58 {
    /// Converts the value of `self`, interpreted as base58 encoded data,
    /// into an owned vector of bytes, returning the vector.
    fn from_base58(&self) -> Result<Vec<u8>, FromBase58Error>;

    /// Converts the value of `self`, interpreted as base58check encoded data,
    /// into an owned vector of bytes, returning the vector.
    fn from_base58_check(&self) -> Result<Vec<u8>, FromBase58Error>;
}


/// Errors that can occur when decoding a base58-encoded string or when decoding a base58check-encoded string
#[derive(Clone, Copy)]
pub enum FromBase58Error {
    /// The input contained a character not part of the base58 alphabet
    InvalidBase58Byte(u8, usize),
    /// The checksum was not correct
    InvalidBase58Checksum([u8; 4], [u8; 4]),
    /// The checksum was not present
    NoBase58Checksum
}

impl fmt::Debug for FromBase58Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            InvalidBase58Byte(ch, idx) =>
                write!(f, "Invalid character '{}' at position {}", ch, idx),
            InvalidBase58Checksum(chk, expected) =>
                write!(f, "Invalid checksum '{:?}', expected {:?}", &chk, &expected),
            NoBase58Checksum =>
                write!(f, "No checksum present")
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

    fn from_base58_check(&self) -> Result<Vec<u8>, FromBase58Error> {
        self.as_bytes().from_base58_check()
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
            // TODO: use append when it becomes stable
            r.extend(x.to_bytes_be());
        }
        Ok(r)
    }

    fn from_base58_check(&self) -> Result<Vec<u8>, FromBase58Error> {
        let decoded = self.from_base58()?;
        let length = decoded.len();
        if length < 4 {
            return Err(NoBase58Checksum)
        }
        let (content, check) = decoded.split_at(length-4);

        let first_hash = Sha256::digest(&content);
        let second_hash = Sha256::digest(&first_hash);
        let (expected_hash, _) = second_hash.split_at(4);

        if check != expected_hash {
            let mut a: [u8; 4] = Default::default();
            a.copy_from_slice(&check[..]);
            let mut b: [u8; 4] = Default::default();
            b.copy_from_slice(&expected_hash[..]);
            return Err(InvalidBase58Checksum(a, b))
        } else {
            return Ok(content.to_vec())
        }
    }
}


/// A trait for converting a value to base58 encoding.
pub trait ToBase58 {
    /// Converts the value of `self` to a base-58 value, returning the owned
    /// string.
    fn to_base58(&self) -> String;

    /// Converts the value of `self` to a base-58 check value, returning the owned
    /// string.
    fn to_base58_check(&self) -> String;
}

impl ToBase58 for [u8] {
    // This function has to read in the entire byte slice and convert it to a
    // (big) int before creating the string. There's no way to incrementally read
    // the slice and create parts of the base58 string. Example:
    //   [1, 33] should be "5z"
    //   [1, 34] should be "61"
    // so by reading "1", no way to know if first character should be 5 or 6
    // without reading the rest
    fn to_base58(&self) -> String {
        let radix = 58.to_biguint().unwrap();
        let mut x = BigUint::from_bytes_be(&self);
        let mut ans = vec![];
        while x > Zero::zero() {
            let rem = (&x % &radix).to_usize().unwrap();
            ans.push(BTC_ALPHA[rem]);
            x = &x / &radix;
        }

        // take care of leading zeros
        for _ in self.iter().take_while(|&x| *x == 0) {
            ans.push(BTC_ALPHA[0]);
        }
        ans.reverse();
        String::from_utf8(ans).unwrap()
    }

    fn to_base58_check(&self) -> String {
        let first_hash = Sha256::digest(&self);
        let second_hash = Sha256::digest(&first_hash);
        let mut with_check = self.iter().cloned().collect::<Vec<u8>>();
        with_check.extend(second_hash.iter().cloned().take(4));
        with_check.to_base58()
    }
}


#[cfg(test)]
mod tests {
    use super::{FromBase58, ToBase58};

    const INVALID_BASE58: [&'static str; 10] = [
        "0",
        "O",
        "I",
        "l",
        "3mJr0",
        "O3yxU",
        "3sNI",
        "4kl8",
        "s!5<",
        "t$@mX<*"
    ];

    #[test]
    fn test_from_base58_basic() {
        assert_eq!("".from_base58().unwrap(), b"");
        assert_eq!("Z".from_base58().unwrap(), &[32]);
        assert_eq!("n".from_base58().unwrap(), &[45]);
        assert_eq!("q".from_base58().unwrap(), &[48]);
        assert_eq!("r".from_base58().unwrap(), &[49]);
        assert_eq!("z".from_base58().unwrap(), &[57]);
        assert_eq!("4SU".from_base58().unwrap(), &[45, 49]);
        assert_eq!("4k8".from_base58().unwrap(), &[49, 49]);
        assert_eq!("ZiCa".from_base58().unwrap(), &[97, 98, 99]);
        assert_eq!("3mJr7AoUXx2Wqd".from_base58().unwrap(), b"1234598760");
        assert_eq!("3yxU3u1igY8WkgtjK92fbJQCd4BZiiT1v25f".from_base58().unwrap(), b"abcdefghijklmnopqrstuvwxyz");
    }

    #[test]
    fn test_from_base58_bytes() {
        assert_eq!(b"ZiCa".from_base58().unwrap(), b"abc");
    }

    #[test]
    fn test_from_base58_invalid_char() {
        for s in INVALID_BASE58.iter() {
            assert!(s.from_base58().is_err());
        }
    }

    #[test]
    fn test_from_base58_check_basic() {
        assert_eq!("3QJmnh".from_base58_check().unwrap(), b"");
        assert_eq!("6bdbJ1U".from_base58_check().unwrap(), &[49]);
        assert_eq!("7VsrQCP".from_base58_check().unwrap(), &[57]);
        assert_eq!("PWEu9GGN".from_base58_check().unwrap(), &[45, 49]);
        assert_eq!("RVnPfpC2".from_base58_check().unwrap(), &[49, 49]);
        assert_eq!("K5zqBMZZTzUbAZQgrt4".from_base58_check().unwrap(), b"1234598760");
        assert_eq!("LWmP1W82eUos2HWzVn19rapmig4X5dqPWgGFLsUTJ".from_base58_check().unwrap(), b"abcdefghijklmnopqrstuvwxyz");
    }

    #[test]
    fn test_from_base58_check_invalid() {
        // Invalid base58
        for s in INVALID_BASE58.iter() {
            assert!(s.from_base58_check().is_err());
        }

        // Valid base58 but invalid base58check
        assert_eq!("4SU".from_base58().unwrap(), &[45, 49]);
        assert!("4SU".from_base58_check().is_err());
        assert_eq!("3mJr7AoUXx2Wqd".from_base58().unwrap(), b"1234598760");
        assert!("3mJr7AoUXx2Wqd".from_base58_check().is_err());
    }

    #[test]
    fn test_from_base58_initial_zeros() {
        assert_eq!("1ZiCa".from_base58().unwrap(), b"\0abc");
        assert_eq!("11ZiCa".from_base58().unwrap(), b"\0\0abc");
        assert_eq!("111ZiCa".from_base58().unwrap(), b"\0\0\0abc");
        assert_eq!("1111ZiCa".from_base58().unwrap(), b"\0\0\0\0abc");
    }

    #[test]
    fn test_to_base58_basic() {
        assert_eq!(b"".to_base58(), "");
        assert_eq!(&[32].to_base58(), "Z");
        assert_eq!(&[45].to_base58(), "n");
        assert_eq!(&[48].to_base58(), "q");
        assert_eq!(&[49].to_base58(), "r");
        assert_eq!(&[57].to_base58(), "z");
        assert_eq!(&[45, 49].to_base58(), "4SU");
        assert_eq!(&[49, 49].to_base58(), "4k8");
        assert_eq!(b"abc".to_base58(), "ZiCa");
        assert_eq!(b"1234598760".to_base58(), "3mJr7AoUXx2Wqd");
        assert_eq!(b"abcdefghijklmnopqrstuvwxyz".to_base58(), "3yxU3u1igY8WkgtjK92fbJQCd4BZiiT1v25f");
    }

    #[test]
    fn test_to_base58_initial_zeros() {
        assert_eq!(b"\0abc".to_base58(), "1ZiCa");
        assert_eq!(b"\0\0abc".to_base58(), "11ZiCa");
        assert_eq!(b"\0\0\0abc".to_base58(), "111ZiCa");
        assert_eq!(b"\0\0\0\0abc".to_base58(), "1111ZiCa");
    }

    #[test]
    fn test_to_base58_check_basic() {
        assert_eq!(b"".to_base58_check(), "3QJmnh");
        assert_eq!(&[49].to_base58_check(), "6bdbJ1U");
        assert_eq!(&[57].to_base58_check(), "7VsrQCP");
        assert_eq!(&[45, 49].to_base58_check(), "PWEu9GGN");
        assert_eq!(&[49, 49].to_base58_check(), "RVnPfpC2");
        assert_eq!(b"1234598760".to_base58_check(), "K5zqBMZZTzUbAZQgrt4");
        assert_eq!(b"abcdefghijklmnopqrstuvwxyz".to_base58_check(), "LWmP1W82eUos2HWzVn19rapmig4X5dqPWgGFLsUTJ");
    }

    #[test]
    fn test_base58_random() {
        use rand::{thread_rng, Rng};

        for _ in 0..200 {
            let times = thread_rng().gen_range(1, 100);
            let v = thread_rng().gen_iter::<u8>().take(times)
                                .collect::<Vec<_>>();
            assert_eq!(v.to_base58()
                        .from_base58()
                        .unwrap(),
                       v);
            assert_eq!(v.to_base58_check()
                           .from_base58_check()
                           .unwrap(),
                       v);
        }
    }
}
