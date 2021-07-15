// Crates that have the "proc-macro" crate type are only allowed to export
// procedural macros. So we cannot have one crate that defines procedural macros
// alongside other types of public APIs like traits and structs.
//
// For this project we are going to need a #[bitfield] macro but also a trait
// and some structs. We solve this by defining the trait and structs in this
// crate, defining the attribute macro in a separate bitfield-impl crate, and
// then re-exporting the macro from this crate so that users only have one crate
// that they need to import.
//
// From the perspective of a user of this crate, they get all the necessary APIs
// (macro, trait, struct) through the one bitfield crate.
pub use bitfield_impl::bitfield;

use std::marker::PhantomData;

mod seal {
    pub trait Load<T> {
        fn load(off: usize, len: usize, data: &[u8]) -> T;
    }

    pub trait Store<T> {
        fn store(off: usize, len: usize, data: &mut [u8], val: T);
    }
}

impl seal::Load<u8> for u8 {
    fn load(off: usize, len: usize, data: &[u8]) -> u8 {
        if !(1..=u8::BITS as usize).contains(&len) {
            panic!();
        }

        let begin = off >> 3;
        let end = (off + len - 1) >> 3; // inclusive
        if data.is_empty() || end + 1 > data.len() {
            panic!();
        }

        let mask = u8::MAX >> (u8::BITS - len as u32);
        match &data[begin..=end] {
            &[b] => u8::from_le_bytes([b]) >> (off as u32 % u8::BITS) & mask,
            &[b, o] => (u16::from_le_bytes([b, o]) >> (off as u32 % u8::BITS)) as u8 & mask,
            _ => unreachable!(),
        }
    }
}

impl seal::Load<u16> for u16 {
    fn load(off: usize, len:usize, data: &[u8]) -> u16 {
        if !(u8::BITS as usize + 1..=Self::BITS as usize).contains(&len) {
            panic!();
        }

        let begin = off >> 3;
        let end = (off + len - 1) >> 3; // inclusive
        if data.is_empty() || end + 1 > data.len() {
            panic!();
        }

        let mask = Self::MAX >> (Self::BITS - len as u32);
        match &data[begin..=end] {
            &[b1, b2] => Self::from_le_bytes([b1, b2]) >> (off as u32 % Self::BITS) & mask,
            &[b1, b2, o] => (u32::from_le_bytes([b1, b2, o, 0]) >> (off as u32 % Self::BITS)) as Self & mask,
            z => unreachable!("{:?}", z),
        }
    }
}

impl seal::Load<u32> for u32 {
    fn load(off: usize, len: usize, data: &[u8]) -> u32 {
        if !(u16::BITS as usize + 1..=Self::BITS as usize).contains(&len) {
            panic!();
        }

        let begin = off >> 3;
        let end = (off + len - 1) >> 3; // inclusive
        if data.is_empty() || end + 1 > data.len() {
            panic!("{} {} {} {} {}", off, len, begin, end, data.len());
        }

        let mask = Self::MAX >> (Self::BITS - len as u32);
        match &data[begin..=end] {
            &[b1, b2, b3] => Self::from_le_bytes([b1, b2, b3, 0]) >> (off as u32 % Self::BITS) & mask,
            &[b1, b2, b3, b4] => Self::from_le_bytes([b1, b2, b3, b4]) >> (off as u32 % Self::BITS) & mask,
            &[b1, b2, b3, b4, o] => (u64::from_le_bytes([b1, b2, b3, b4, o, 0, 0, 0]) >> (off as u32 % Self::BITS)) as Self & mask,
            z => unreachable!("{:?}", z),
        }
    }
}

impl seal::Load<u64> for u64 {
    fn load(off: usize, len: usize, data: &[u8]) -> u64 {
        if !(u32::BITS as usize + 1..=Self::BITS as usize).contains(&len) {
            panic!();
        }

        let begin = off >> 3;
        let end = (off + len - 1) >> 3; // inclusive
        if data.is_empty() || end + 1 > data.len() {
            panic!();
        }

        let mask = Self::MAX >> (Self::BITS - len as u32);
        match &data[begin..=end] {
            &[b1, b2, b3, b4, b5] => Self::from_le_bytes([b1, b2, b3, b4, b5, 0, 0, 0]) >> (off as u32 % Self::BITS) & mask,
            &[b1, b2, b3, b4, b5, b6] => Self::from_le_bytes([b1, b2, b3, b4, b5, b6, 0, 0]) >> (off as u32 % Self::BITS) & mask,
            &[b1, b2, b3, b4, b5, b6, b7] => Self::from_le_bytes([b1, b2, b3, b4, b5, b6, b7, 0]) >> (off as u32 % Self::BITS) & mask,
            &[b1, b2, b3, b4, b5, b6, b7, b8] => Self::from_le_bytes([b1, b2, b3, b4, b5, b6, b7, b8]) >> (off as u32 % Self::BITS) & mask,
            &[b1, b2, b3, b4, b5, b6, b7, b8, o] => (u128::from_le_bytes([b1, b2, b3, b4, b5, b6, b7, b8, o, 0, 0, 0, 0, 0, 0, 0]) >> (off as u32 % Self::BITS)) as Self & mask,
            z => unreachable!("{:?}", z),
        }
    }
}

impl seal::Store<u8> for u8 {
    fn store(off: usize, len:usize, data: &mut [u8], val: u8) {
        if !(1..=u8::BITS as usize).contains(&len) {
            panic!();
        }

        let begin = off >> 3;
        let end = (off + len - 1) >> 3; // inclusive
        if data.is_empty() || end + 1 > data.len() {
            panic!();
        }

        let mask = u8::MAX >> (u8::BITS - len as u32);
        let val = val & mask;

        match &data[begin..=end] {
            &[b] => {
                let m = mask << off;
                data[begin] = b & !m | (val << off)
            }
            &[h, t] => {
                let mh = u8::MAX << (off as u32 % u8::BITS);
                let mt = u8::MAX << ((off + len) as u32 % u8::BITS);
                let buf = ((val as u16) << off).to_le_bytes();
                let head = buf[0];
                let tail = buf[1];

                data[begin] = h & !mh | head;
                data[end] = t & mt | tail;
            }
            _ => unreachable!(),
        }
    }
}

impl seal::Store<u16> for u16 {
    fn store(off: usize, len:usize, data: &mut [u8], val: u16) {
        if !(u8::BITS as usize + 1..=Self::BITS as usize).contains(&len) {
            panic!();
        }

        let begin = off >> 3;
        let end = (off + len - 1) >> 3; // inclusive
        if data.is_empty() || end + 1 > data.len() {
            panic!();
        }

        let mask = Self::MAX >> (Self::BITS - len as u32);
        let val = val & mask;

        match &data[begin..=end] {
            &[h, t] => {
                let mh = u8::MAX << (off as u32 % u8::BITS);
                let mt = u8::MAX << ((off + len) as u32 % u8::BITS);
                let buf = ((val as Self) << off).to_le_bytes();
                let head = buf[0];
                let tail = buf[1];

                data[begin] = h & !mh | head;
                data[end] = t & mt | tail;
            }
            &[h, _, t] => {
                let mh = u8::MAX << (off as u32 % u8::BITS);
                let mt = u8::MAX << ((off + len) as u32 % u8::BITS);
                let buf = ((val as u32) << off).to_le_bytes();
                let head = buf[0];
                let tail = buf[2];

                data[begin] = h & !mh | head;
                data[begin + 1 ..= end - 1].copy_from_slice(&buf[1 .. 2]);
                data[end] = t & mt | tail;
            }
            _ => unreachable!(),
        }
    }
}

impl seal::Store<u32> for u32 {
    fn store(off: usize, len:usize, data: &mut [u8], val: u32) {
        if !(u16::BITS as usize + 1..=Self::BITS as usize).contains(&len) {
            panic!();
        }

        let begin = off >> 3;
        let end = (off + len - 1) >> 3; // inclusive
        if data.is_empty() || end + 1 > data.len() {
            panic!();
        }

        let mask = Self::MAX >> (Self::BITS - len as u32);
        let val = val & mask;

        match &data[begin..=end] {
            &[h, _, t] => {
                let mh = u8::MAX << (off as u32 % u8::BITS);
                let mt = u8::MAX << ((off + len) as u32 % u8::BITS);
                let buf = ((val as Self) << off).to_le_bytes();
                let head = buf[0];
                let tail = buf[2];

                data[begin] = h & !mh | head;
                data[begin + 1 ..= end - 1].copy_from_slice(&buf[1 .. 2]);
                data[end] = t & mt | tail;
            }
            &[h, _, _, t] => {
                let mh = u8::MAX << (off as u32 % u8::BITS);
                let mt = u8::MAX << ((off + len) as u32 % u8::BITS);
                let buf = ((val as Self) << off).to_le_bytes();
                let head = buf[0];
                let tail = buf[3];

                data[begin] = h & !mh | head;
                data[begin + 1 ..= end - 1].copy_from_slice(&buf[1 .. 3]);
                data[end] = t & mt | tail;
            }
            &[h, _, _, _, t] => {
                let mh = u8::MAX << (off as u32 % u8::BITS);
                let mt = u8::MAX << ((off + len) as u32 % u8::BITS);
                let buf = ((val as u64) << off).to_le_bytes();
                let head = buf[0];
                let tail = buf[4];

                data[begin] = h & !mh | head;
                data[begin + 1 ..= end - 1].copy_from_slice(&buf[1 .. 4]);
                data[end] = t & mt | tail;
            }
            _ => unreachable!(),
        }
    }
}

impl seal::Store<u64> for u64 {
    fn store(off: usize, len:usize, data: &mut [u8], val: u64) {
        if !(u32::BITS as usize + 1..=Self::BITS as usize).contains(&len) {
            panic!();
        }

        let begin = off >> 3;
        let end = (off + len - 1) >> 3; // inclusive
        if data.is_empty() || end + 1 > data.len() {
            panic!();
        }

        let mask = Self::MAX >> (Self::BITS - len as u32);
        let val = val & mask;

        match &data[begin..=end] {
            &[h, _, _, _, t] => {
                let mh = u8::MAX << (off as u32 % u8::BITS);
                let mt = u8::MAX << ((off + len) as u32 % u8::BITS);
                let buf = ((val as Self) << off).to_le_bytes();
                let head = buf[0];
                let tail = buf[4];

                data[begin] = h & !mh | head;
                data[begin + 1 ..= end - 1].copy_from_slice(&buf[1 .. 4]);
                data[end] = t & mt | tail;
            }
            &[h, _, _, _, _, t] => {
                let mh = u8::MAX << (off as u32 % u8::BITS);
                let mt = u8::MAX << ((off + len) as u32 % u8::BITS);
                let buf = ((val as Self) << off).to_le_bytes();
                let head = buf[0];
                let tail = buf[5];

                data[begin] = h & !mh | head;
                data[begin + 1 ..= end - 1].copy_from_slice(&buf[1 .. 5]);
                data[end] = t & mt | tail;
            }
            &[h, _, _, _, _, _, t] => {
                let mh = u8::MAX << (off as u32 % u8::BITS);
                let mt = u8::MAX << ((off + len) as u32 % u8::BITS);
                let buf = ((val as Self) << off).to_le_bytes();
                let head = buf[0];
                let tail = buf[6];

                data[begin] = h & !mh | head;
                data[begin + 1 ..= end - 1].copy_from_slice(&buf[1 .. 6]);
                data[end] = t & mt | tail;
            }
            &[h, _, _, _, _, _, _, t] => {
                let mh = u8::MAX << (off as u32 % u8::BITS);
                let mt = u8::MAX << ((off + len) as u32 % u8::BITS);
                let buf = ((val as Self) << off).to_le_bytes();
                let head = buf[0];
                let tail = buf[7];

                data[begin] = h & !mh | head;
                data[begin + 1 ..= end - 1].copy_from_slice(&buf[1 .. 7]);
                data[end] = t & mt | tail;
            }
            &[h, _, _, _, _, _, _, _, t] => {
                let mh = u8::MAX << (off as u32 % u8::BITS);
                let mt = u8::MAX << ((off + len) as u32 % u8::BITS);
                let buf = ((val as u128) << off).to_le_bytes();
                let head = buf[0];
                let tail = buf[8];

                data[begin] = h & !mh | head;
                data[begin + 1 ..= end - 1].copy_from_slice(&buf[1 .. 8]);
                data[end] = t & mt | tail;
            }
            _ => unreachable!(),
        }
    }
}

pub trait Specifier {
    const BITS: usize;
    type Item: seal::Load<Self::Item> + seal::Store<Self::Item>;

    fn get(off: usize, data: &[u8]) -> Self::Item {
        <Self::Item as seal::Load<_>>::load(off, Self::BITS, data)
    }

    fn set(off: usize, data: &mut [u8], val: Self::Item) {
        <Self::Item as seal::Store<_>>::store(off, Self::BITS, data, val);
    }
}

pub struct Bn<I, const N: usize>(PhantomData<I>);

impl<I, const N: usize> Specifier for Bn<I, N> where I: seal::Load<I> + seal::Store<I> {
    const BITS: usize = N;
    type Item = I;
}

pub type B1 = Bn<u8, 1>;
pub type B2 = Bn<u8, 2>;
pub type B3 = Bn<u8, 3>;
pub type B4 = Bn<u8, 4>;
pub type B5 = Bn<u8, 5>;
pub type B6 = Bn<u8, 6>;
pub type B7 = Bn<u8, 7>;
pub type B8 = Bn<u8, 8>;
pub type B9 = Bn<u16, 9>;
pub type B10 = Bn<u16, 10>;
pub type B11 = Bn<u16, 11>;
pub type B12 = Bn<u16, 12>;
pub type B13 = Bn<u16, 13>;
pub type B14 = Bn<u16, 14>;
pub type B15 = Bn<u16, 15>;
pub type B16 = Bn<u16, 16>;
pub type B17 = Bn<u32, 17>;
pub type B18 = Bn<u32, 18>;
pub type B19 = Bn<u32, 19>;
pub type B20 = Bn<u32, 20>;
pub type B21 = Bn<u32, 21>;
pub type B22 = Bn<u32, 22>;
pub type B23 = Bn<u32, 23>;
pub type B24 = Bn<u32, 24>;
pub type B25 = Bn<u32, 25>;
pub type B26 = Bn<u32, 26>;
pub type B27 = Bn<u32, 27>;
pub type B28 = Bn<u32, 28>;
pub type B29 = Bn<u32, 29>;
pub type B30 = Bn<u32, 30>;
pub type B31 = Bn<u32, 31>;
pub type B32 = Bn<u32, 32>;
pub type B33 = Bn<u64, 33>;
pub type B34 = Bn<u64, 34>;
pub type B35 = Bn<u64, 35>;
pub type B36 = Bn<u64, 36>;
pub type B37 = Bn<u64, 37>;
pub type B38 = Bn<u64, 38>;
pub type B39 = Bn<u64, 39>;
pub type B40 = Bn<u64, 40>;
pub type B41 = Bn<u64, 41>;
pub type B42 = Bn<u64, 42>;
pub type B43 = Bn<u64, 43>;
pub type B44 = Bn<u64, 44>;
pub type B45 = Bn<u64, 45>;
pub type B46 = Bn<u64, 46>;
pub type B47 = Bn<u64, 47>;
pub type B48 = Bn<u64, 48>;
pub type B49 = Bn<u64, 49>;
pub type B50 = Bn<u64, 50>;
pub type B51 = Bn<u64, 51>;
pub type B52 = Bn<u64, 52>;
pub type B53 = Bn<u64, 53>;
pub type B54 = Bn<u64, 54>;
pub type B55 = Bn<u64, 55>;
pub type B56 = Bn<u64, 56>;
pub type B57 = Bn<u64, 57>;
pub type B58 = Bn<u64, 58>;
pub type B59 = Bn<u64, 59>;
pub type B60 = Bn<u64, 60>;
pub type B61 = Bn<u64, 61>;
pub type B62 = Bn<u64, 62>;
pub type B63 = Bn<u64, 63>;
pub type B64 = Bn<u64, 64>;

#[cfg(test)]
mod tests {
    use super::*;
    use super::seal::*;

    #[test]
    fn test_load8() {
        let data = [0b0000_0000];
        let r = u8::load(0, 8, &data);
        assert_eq!(0, r);

        let data = [0b0000_0001];
        let r = u8::load(0, 8, &data);
        assert_eq!(1, r);

        let data = [0b0000_0010];
        let r = u8::load(1, 7, &data);
        assert_eq!(1, r);

        let data = [0b0000_0010, 0b0000_0001];
        let r = u8::load(1, 8, &data);
        assert_eq!(0b1000_0001, r);
    }

    #[test]
    fn test_store8() {
        let mut data = [0b0000_0000];
        u8::store(0, 8, &mut data, 0b1111_1111);
        assert_eq!(&[0b1111_1111], &data);

        let mut data = [0b0000_0001];
        u8::store(1, 7, &mut data, 0b0111_1111);
        assert_eq!(&[0b1111_1111], &data);

        let mut data = [0b0000_0000, 0b0000_0000];
        u8::store(1, 8, &mut data, 0b1111_1111);
        assert_eq!(&[0b1111_1110, 0b0000_0001], &data);
    }

    #[test]
    fn test_get1() {
        let data = [0b0000_0000];
        let r = B1::get(0, &data);
        assert_eq!(0, r);

        let data = [0b0000_0001];
        let r = B1::get(0, &data);
        assert_eq!(1, r);

        let data = [0b0000_0010];
        let r = B1::get(1, &data);
        assert_eq!(1, r);
    }

    #[test]
    fn test_set1() {
        let mut data = [0b0000_0000];
        B1::set(0, &mut data, 1);
        assert_eq!(&[0b0000_0001][..], &data[..]);

        let mut data = [0b0000_0001];
        B1::set(0, &mut data, 0);
        assert_eq!(&[0b0000_0000][..], &data[..]);

        let mut data = [0b1111_1101];
        B1::set(1, &mut data, 1);
        assert_eq!(&[0b1111_1111][..], &data[..]);
    }

    #[test]
    fn test_load16() {
        let data = [0b1111_1111, 0b1111_1111];
        let r = u16::load(0, 16, &data);
        assert_eq!(0b1111_1111_1111_1111, r);

        let data = [0b1111_1110, 0b1111_1111];
        let r = u16::load(1, 15, &data);
        assert_eq!(0b0111_1111_1111_1111, r);

        let data = [0b1111_1110, 0b1111_1111, 0b0000_0001];
        let r = u16::load(1, 16, &data);
        assert_eq!(0b1111_1111_1111_1111, r);
    }

    #[test]
    fn test_store16() {
        let mut data = [0b0000_0000, 0b0000_0000];
        u16::store(0, 16, &mut data, 0b1111_1111_1111_1111);
        assert_eq!(&[0b1111_1111, 0b1111_1111], &data);

        let mut data = [0b0000_0001, 0b0000_0000];
        u16::store(1, 15, &mut data, 0b0111_1111_1111_1111);
        assert_eq!(&[0b1111_1111, 0b1111_1111], &data);

        let mut data = [0b0000_0000, 0b0000_0000, 0b0000_0000];
        u16::store(1, 16, &mut data, 0b1111_1111_1111_1111);
        assert_eq!(&[0b1111_1110, 0b1111_1111, 0b0000_0001], &data);
    }

    #[test]
    fn test_get9() {
        let data = [0b1111_1111, 0b1111_1111];
        let r = B9::get(0, &data);
        assert_eq!(0b0000_0001_1111_1111, r);

        let data = [0b0000_0000, 0b1111_1110];
        let r = B9::get(0, &data);
        assert_eq!(0, r);

        let data = [0b1111_1110, 0b0000_0011];
        let r = B9::get(1, &data);
        assert_eq!(0b0000_0001_1111_1111, r);
    }

    #[test]
    fn test_set9() {
        let mut data = [0b0000_0000, 0b0000_0000];
        B9::set(0, &mut data, 1);
        assert_eq!(&[0b0000_0001, 0b0000_0000][..], &data[..]);

        let mut data = [0b1111_1111, 0b1111_1111];
        B9::set(0, &mut data, 0);
        assert_eq!(&[0b0000_0000, 0b1111_1110][..], &data[..]);

        let mut data = [0b1111_1111, 0b1111_1111];
        B9::set(1, &mut data, 0);
        assert_eq!(&[0b0000_0001, 0b1111_1100][..], &data[..]);
    }

    #[test]
    fn test_load32() {
        let data = [0b1111_1111, 0b1111_1111, 0b1111_1111, 0b1111_1111];
        let r = u32::load(0, 32, &data);
        assert_eq!(0b1111_1111_1111_1111_1111_1111_1111_1111, r);

        let data = [0b1111_1110, 0b1111_1111, 0b1111_1111, 0b1111_1111];
        let r = u32::load(1, 31, &data);
        assert_eq!(0b0111_1111_1111_1111_1111_1111_1111_1111, r);

        let data = [0b1111_1110, 0b1111_1111, 0b1111_1111, 0b1111_1111, 0b0000_0001];
        let r = u32::load(1, 32, &data);
        assert_eq!(0b1111_1111_1111_1111_1111_1111_1111_1111, r);
    }

    #[test]
    fn test_store32() {
        let mut data = [0b0000_0000, 0b0000_0000, 0b0000_0000, 0b0000_0000];
        u32::store(0, 32, &mut data, 0b1111_1111_1111_1111_1111_1111_1111_1111);
        assert_eq!(&[0b1111_1111, 0b1111_1111, 0b1111_1111, 0b1111_1111], &data);

        let mut data = [0b0000_0001, 0b0000_0000, 0b0000_0000, 0b0000_0000];
        u32::store(1, 31, &mut data, 0b0111_1111_1111_1111_1111_1111_1111_1111);
        assert_eq!(&[0b1111_1111, 0b1111_1111, 0b1111_1111, 0b1111_1111], &data);

        let mut data = [0b0000_0000, 0b0000_0000, 0b0000_0000, 0b0000_0000, 0b0000_0000];
        u32::store(1, 32, &mut data, 0b1111_1111_1111_1111_1111_1111_1111_1111);
        assert_eq!(&[0b1111_1110, 0b1111_1111, 0b1111_1111, 0b1111_1111, 0b0000_0001], &data);
    }

    #[test]
    fn test_get17() {
        let data = [0b1111_1111, 0b1111_1111, 0b1111_1111, 0b1111_1111];
        let r = B17::get(0, &data);
        assert_eq!(0b0000_0001_1111_1111_1111_1111, r);

        let data = [0b0000_0000, 0b0000_0000, 0b1111_1110, 0b1111_1111];
        let r = B17::get(0, &data);
        assert_eq!(0, r);

        let data = [0b1111_1110, 0b1111_1111, 0b1111_1111, 0b0000_0011];
        let r = B17::get(1, &data);
        assert_eq!(0b0000_0001_1111_1111_1111_1111, r);
    }

    #[test]
    fn test_set17() {
        let mut data = [0b0000_0000, 0b0000_0000, 0b0000_0000, 0b0000_0000];
        B17::set(0, &mut data, 1);
        assert_eq!(&[0b0000_0001, 0b0000_0000, 0b0000_0000, 0b0000_0000][..], &data[..]);

        let mut data = [0b1111_1111, 0b1111_1111, 0b1111_1111, 0b1111_1111];
        B17::set(0, &mut data, 0);
        assert_eq!(&[0b0000_0000, 0b0000_0000, 0b1111_1110, 0b1111_1111][..], &data[..]);

        let mut data = [0b1111_1111, 0b1111_1111, 0b1111_1111, 0b1111_1111];
        B17::set(1, &mut data, 0);
        assert_eq!(&[0b0000_0001, 0b0000_0000, 0b1111_1100, 0b1111_1111][..], &data[..]);
    }

    #[test]
    fn test_load64() {
        let data = [0b1111_1111, 0b1111_1111, 0b1111_1111, 0b1111_1111, 0b1111_1111, 0b1111_1111, 0b1111_1111, 0b1111_1111];
        let r = u64::load(0, 64, &data);
        assert_eq!(0b1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111, r);

        let data = [0b1111_1110, 0b1111_1111, 0b1111_1111, 0b1111_1111, 0b1111_1111, 0b1111_1111, 0b1111_1111, 0b1111_1111];
        let r = u64::load(1, 63, &data);
        assert_eq!(0b0111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111, r);

        let data = [0b1111_1110, 0b1111_1111, 0b1111_1111, 0b1111_1111, 0b1111_1111, 0b1111_1111, 0b1111_1111, 0b1111_1111, 0b0000_0001];
        let r = u64::load(1, 64, &data);
        assert_eq!(0b1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111, r);
    }

    #[test]
    fn test_store64() {
        let mut data = [0b0000_0000, 0b0000_0000, 0b0000_0000, 0b0000_0000, 0b0000_0000, 0b0000_0000, 0b0000_0000, 0b0000_0000];
        u64::store(0, 64, &mut data, 0b1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111);
        assert_eq!(&[0b1111_1111, 0b1111_1111, 0b1111_1111, 0b1111_1111, 0b1111_1111, 0b1111_1111, 0b1111_1111, 0b1111_1111], &data);

        let mut data = [0b0000_0001, 0b0000_0000, 0b0000_0000, 0b0000_0000, 0b0000_0000, 0b0000_0000, 0b0000_0000, 0b0000_0000];
        u64::store(1, 63, &mut data, 0b0111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111);
        assert_eq!(&[0b1111_1111, 0b1111_1111, 0b1111_1111, 0b1111_1111, 0b1111_1111, 0b1111_1111, 0b1111_1111, 0b1111_1111], &data);

        let mut data = [0b0000_0000, 0b0000_0000, 0b0000_0000, 0b0000_0000, 0b0000_0000, 0b0000_0000, 0b0000_0000, 0b0000_0000, 0b0000_0000];
        u64::store(1, 64, &mut data, 0b1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111_1111);
        assert_eq!(&[0b1111_1110, 0b1111_1111, 0b1111_1111, 0b1111_1111, 0b1111_1111, 0b1111_1111, 0b1111_1111, 0b1111_1111, 0b0000_0001], &data);
    }

    #[test]
    fn test_get33() {
        let data = [0b1111_1111, 0b1111_1111, 0b1111_1111, 0b1111_1111, 0b1111_1111];
        let r = B33::get(0, &data);
        assert_eq!(0b0000_0001_1111_1111_1111_1111_1111_1111_1111_1111, r);

        let data = [0b0000_0000, 0b0000_0000, 0b0000_0000, 0b0000_0000, 0b1111_1110];
        let r = B33::get(0, &data);
        assert_eq!(0, r);

        let data = [0b1111_1110, 0b1111_1111, 0b1111_1111, 0b1111_1111, 0b0000_0011];
        let r = B33::get(1, &data);
        assert_eq!(0b0000_0001_1111_1111_1111_1111_1111_1111_1111_1111, r);
    }

    #[test]
    fn test_set33() {
        let mut data = [0b0000_0000, 0b0000_0000, 0b0000_0000, 0b0000_0000, 0b0000_0000];
        B33::set(0, &mut data, 1);
        assert_eq!(&[0b0000_0001, 0b0000_0000, 0b0000_0000, 0b0000_0000, 0b0000_0000][..], &data[..]);

        let mut data = [0b1111_1111, 0b1111_1111, 0b1111_1111, 0b1111_1111, 0b1111_1111];
        B33::set(0, &mut data, 0);
        assert_eq!(&[0b0000_0000, 0b0000_0000, 0b0000_0000, 0b0000_0000, 0b1111_1110][..], &data[..]);

        let mut data = [0b1111_1111, 0b1111_1111, 0b1111_1111, 0b1111_1111, 0b1111_1111];
        B33::set(1, &mut data, 0);
        assert_eq!(&[0b0000_0001, 0b0000_0000, 0b0000_0000, 0b0000_0000, 0b1111_1100][..], &data[..]);
    }

}
