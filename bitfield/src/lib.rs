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
pub use bitfield_impl::{bitfield, BitfieldSpecifier};

use std::marker::PhantomData;
use std::ops::RangeInclusive;
use std::convert::{TryInto, TryFrom};

pub mod checks {
    pub trait TotalSizeModEight<const N: usize> {}
    pub trait TotalSizeIsMultipleOfEightBits: TotalSizeModEight<0> {}
}

mod private {
    use super::*;

    pub trait Num {
        const BITS_RANGE: RangeInclusive<u32>;

        fn assert_len(len: usize) {
            if !Self::BITS_RANGE.contains(&(len as u32)) {
                panic!();
            }
        }

        fn view(off: usize, len: usize, data: &[u8]) -> &[u8] {
            if !Self::BITS_RANGE.contains(&(len as u32)) {
                panic!();
            }

            let begin = off >> 3;
            let end = (off + len - 1) >> 3; // inclusive
            if data.is_empty() || end + 1 > data.len() {
                panic!();
            }

            &data[begin..=end]
        }

        fn view_mut(off: usize, len: usize, data: &mut [u8]) -> &mut [u8] {
            if !Self::BITS_RANGE.contains(&(len as u32)) {
                panic!();
            }

            let begin = off >> 3;
            let end = (off + len - 1) >> 3; // inclusive
            if data.is_empty() || end + 1 > data.len() {
                panic!();
            }

            &mut data[begin..=end]
        }
    }

    pub trait Load: Num {
        fn load(off: usize, len: usize, data: &[u8]) -> Self;
    }

    pub trait Store: Num {
        fn store(off: usize, len: usize, data: &mut [u8], val: Self);
    }
}

fn split(data: &[u8]) -> (u8, &[u8], u8) {
    if data.len() < 2 {
        panic!();
    }

    let h = data[0];
    let t = data[data.len() - 1];
    (h, &data[1..data.len() - 1], t)
}

impl private::Num for u8 {
    const BITS_RANGE: RangeInclusive<u32> = 1 ..= Self::BITS;
}

impl private::Num for u16 {
    const BITS_RANGE: RangeInclusive<u32> = u8::BITS + 1 ..= Self::BITS;
}

impl private::Num for u32 {
    const BITS_RANGE: RangeInclusive<u32> = u16::BITS + 1 ..= Self::BITS;
}

impl private::Num for u64 {
    const BITS_RANGE: RangeInclusive<u32> = u32::BITS + 1 ..= Self::BITS;
}

impl private::Load for u8 {
    fn load(off: usize, len: usize, data: &[u8]) -> u8 {
        let data = <Self as private::Num>::view(off, len, data);
        let mask = Self::MAX >> (Self::BITS - len as u32);
        let off = off as u32 % u8::BITS;

        match data {
            &[b] => u8::from_le_bytes([b]) >> off & mask,
            &[b, o] => (u16::from_le_bytes([b, o]) >> off) as u8 & mask,
            _ => unreachable!(),
        }
    }
}

impl private::Load for u16 {
    fn load(off: usize, len:usize, data: &[u8]) -> u16 {
        let data = <Self as private::Num>::view(off, len, data);
        let mask = Self::MAX >> (Self::BITS - len as u32);
        let off = off as u32 % u8::BITS;

        match data {
            &[b1, b2] => Self::from_le_bytes([b1, b2]) >> off & mask,
            &[b1, b2, o] => (u32::from_le_bytes([b1, b2, o, 0]) >> off) as Self & mask,
            z => unreachable!("{:?}", z),
        }
    }
}

impl private::Load for u32 {
    fn load(off: usize, len: usize, data: &[u8]) -> u32 {
        let data = <Self as private::Num>::view(off, len, data);
        let mask = Self::MAX >> (Self::BITS - len as u32);
        let off = off as u32 % u8::BITS;

        match data {
            &[b1, b2, b3] => Self::from_le_bytes([b1, b2, b3, 0]) >> off & mask,
            &[b1, b2, b3, b4] => Self::from_le_bytes([b1, b2, b3, b4]) >> off & mask,
            &[b1, b2, b3, b4, o] => (u64::from_le_bytes([b1, b2, b3, b4, o, 0, 0, 0]) >> off) as Self & mask,
            z => unreachable!("{:?}", z),
        }
    }
}

impl private::Load for u64 {
    fn load(off: usize, len: usize, data: &[u8]) -> u64 {
        let data = <Self as private::Num>::view(off, len, data);
        let mask = Self::MAX >> (Self::BITS - len as u32);
        let off = off as u32 % u8::BITS;

        match data {
            &[b1, b2, b3, b4, b5] => Self::from_le_bytes([b1, b2, b3, b4, b5, 0, 0, 0]) >> off & mask,
            &[b1, b2, b3, b4, b5, b6] => Self::from_le_bytes([b1, b2, b3, b4, b5, b6, 0, 0]) >> off & mask,
            &[b1, b2, b3, b4, b5, b6, b7] => Self::from_le_bytes([b1, b2, b3, b4, b5, b6, b7, 0]) >> off & mask,
            &[b1, b2, b3, b4, b5, b6, b7, b8] => Self::from_le_bytes([b1, b2, b3, b4, b5, b6, b7, b8]) >> off & mask,
            &[b1, b2, b3, b4, b5, b6, b7, b8, o] => (u128::from_le_bytes([b1, b2, b3, b4, b5, b6, b7, b8, o, 0, 0, 0, 0, 0, 0, 0]) >> off) as Self & mask,
            z => unreachable!("{:?}", z),
        }
    }
}

impl private::Store for u8 {
    fn store(off: usize, len:usize, data: &mut [u8], val: u8) {
        let data = <Self as private::Num>::view_mut(off, len, data);
        let mask = Self::MAX >> (Self::BITS - len as u32);
        let val = val & mask;

        match data {
            &mut [b] => {
                let off = off as u32 % u8::BITS;
                let m = mask << off;
                data[0] = b & !m | (val << off)
            }
            &mut [h, t] => {
                let mh = u8::MAX << (off as u32 % u8::BITS);
                let mt = u8::MAX << ((off + len) as u32 % u8::BITS);
                let buf = ((val as u16) << (off as u32 % u8::BITS)).to_le_bytes();
                let (head, _, tail) = split(&buf[..data.len()]);

                data[0] = h & !mh | head;
                data[data.len() - 1] = t & mt | tail;
            }
            _ => unreachable!(),
        }
    }
}

impl private::Store for u16 {
    fn store(off: usize, len:usize, data: &mut [u8], val: u16) {
        let data = <Self as private::Num>::view_mut(off, len, data);
        let mask = Self::MAX >> (Self::BITS - len as u32);
        let val = val & mask;

        match data {
            &mut [h, t] => {
                let mh = u8::MAX << (off as u32 % u8::BITS);
                let mt = u8::MAX << ((off + len) as u32 % u8::BITS);
                let buf = ((val as Self) << (off as u32 % u8::BITS)).to_le_bytes();
                let (head, _, tail) = split(&buf[..data.len()]);

                data[0] = h & !mh | head;
                data[data.len() - 1] = t & mt | tail;
            }
            &mut [h, _, t] => {
                let mh = u8::MAX << (off as u32 % u8::BITS);
                let mt = u8::MAX << ((off + len) as u32 % u8::BITS);
                let buf = ((val as u32) << (off as u32 % u8::BITS)).to_le_bytes();
                let (head, buf, tail) = split(&buf[..data.len()]);

                let last = data.len() - 1;
                data[0] = h & !mh | head;
                data[1 .. last].copy_from_slice(buf);
                data[last] = t & mt | tail;
            }
            _ => unreachable!(),
        }
    }
}

impl private::Store for u32 {
    fn store(off: usize, len:usize, data: &mut [u8], val: u32) {
        let data = <Self as private::Num>::view_mut(off, len, data);
        let mask = Self::MAX >> (Self::BITS - len as u32);
        let val = val & mask;

        match data {
            &mut [h, _, t] | &mut [h, _, _, t] => {
                let mh = u8::MAX << (off as u32 % u8::BITS);
                let mt = u8::MAX << ((off + len) as u32 % u8::BITS);
                let buf = ((val as Self) << (off as u32 % u8::BITS)).to_le_bytes();
                let (head, buf, tail) = split(&buf[..data.len()]);

                let last = data.len() - 1;
                data[0] = h & !mh | head;
                data[1 .. last].copy_from_slice(buf);
                data[last] = t & mt | tail;
            }
            &mut [h, _, _, _, t] => {
                let mh = u8::MAX << (off as u32 % u8::BITS);
                let mt = u8::MAX << ((off + len) as u32 % u8::BITS);
                let buf = ((val as u64) << (off as u32 % u8::BITS)).to_le_bytes();
                let (head, buf, tail) = split(&buf[..data.len()]);

                let last = data.len() - 1;
                data[0] = h & !mh | head;
                data[1 .. last].copy_from_slice(buf);
                data[last] = t & mt | tail;
            }
            _ => unreachable!(),
        }
    }
}

impl private::Store for u64 {
    fn store(off: usize, len:usize, data: &mut [u8], val: u64) {
        let data = <Self as private::Num>::view_mut(off, len, data);
        let mask = Self::MAX >> (Self::BITS - len as u32);
        let val = val & mask;

        match data {
            &mut [h, _, _, _, t] | &mut [h, _, _, _, _, t] | &mut [h, _, _, _, _, _, t] | &mut [h, _, _, _, _, _, _, t] => {
                let mh = u8::MAX << (off as u32 % u8::BITS);
                let mt = u8::MAX << ((off + len) as u32 % u8::BITS);
                let buf = ((val as Self) << (off as u32 % u8::BITS)).to_le_bytes();
                let (head, buf, tail) = split(&buf[..data.len()]);

                let last = data.len() - 1;
                data[0] = h & !mh | head;
                data[1 .. last].copy_from_slice(buf);
                data[last] = t & mt | tail;
            }
            &mut [h, _, _, _, _, _, _, _, t] => {
                let mh = u8::MAX << (off as u32 % u8::BITS);
                let mt = u8::MAX << ((off + len) as u32 % u8::BITS);
                let buf = ((val as u128) << (off as u32 % u8::BITS)).to_le_bytes();
                let (head, buf, tail) = split(&buf[..data.len()]);

                let last = data.len() - 1;
                data[0] = h & !mh | head;
                data[1 .. last].copy_from_slice(buf);
                data[last] = t & mt | tail;
            }
            _ => unreachable!(),
        }
    }
}

pub trait Specifier {
    const BITS: usize;
    type Type;

    fn get(off: usize, data: &[u8]) -> Self::Type {
        match Self::BITS {
            0..=8 => Self::from_u8(<u8 as private::Load>::load(off, Self::BITS, data)),
            9..=16 => Self::from_u16(<u16 as private::Load>::load(off, Self::BITS, data)),
            17..=32 => Self::from_u32(<u32 as private::Load>::load(off, Self::BITS, data)),
            33..=64 => Self::from_u64(<u64 as private::Load>::load(off, Self::BITS, data)),
            _ => unreachable!(),
        }
    }

    fn set(off: usize, data: &mut [u8], val: Self::Type) {
        match Self::BITS {
            0..=8 => <u8 as private::Store>::store(off, Self::BITS, data, Self::to_u8(val)),
            9..=16 => <u16 as private::Store>::store(off, Self::BITS, data, Self::to_u16(val)),
            17..=32 => <u32 as private::Store>::store(off, Self::BITS, data, Self::to_u32(val)),
            33..=64 => <u64 as private::Store>::store(off, Self::BITS, data, Self::to_u64(val)),
            _ => unreachable!(),
        }
    }

    fn from_u8(v: u8) -> Self::Type {
        Self::from(v as u64)
    }
    fn from_u16(v: u16) -> Self::Type {
        Self::from(v as u64)
    }
    fn from_u32(v: u32) -> Self::Type {
        Self::from(v as u64)
    }
    fn from_u64(v: u64) -> Self::Type {
        Self::from(v as u64)
    }
    fn from(v: u64) -> Self::Type;

    fn to_u8(v: Self::Type) -> u8 {
        Self::to(v) as u8
    }
    fn to_u16(v: Self::Type) -> u16 {
        Self::to(v) as u16
    }
    fn to_u32(v: Self::Type) -> u32 {
        Self::to(v) as u32
    }
    fn to_u64(v: Self::Type) -> u64 {
        Self::to(v)
    }
    fn to(v: Self::Type) -> u64;
}

pub struct Bn<I, const N: usize>(PhantomData<I>);

impl<I, const N: usize> Specifier for Bn<I, N> where I: private::Load + private::Store + TryFrom<u8> + TryFrom<u16> + TryFrom<u32> + TryFrom<u64> + TryInto<u8> + TryInto<u16> + TryInto<u32> + TryInto<u64> {
    const BITS: usize = N;
    type Type = I;

    fn from_u8(v: u8) -> Self::Type {
        if let Ok(v) = Self::Type::try_from(v) { v } else { panic!() }
    }
    fn from_u16(v: u16) -> Self::Type {
        if let Ok(v) = Self::Type::try_from(v) { v } else { panic!() }
    }
    fn from_u32(v: u32) -> Self::Type {
        if let Ok(v) = Self::Type::try_from(v) { v } else { panic!() }
    }
    fn from_u64(v: u64) -> Self::Type {
        if let Ok(v) = Self::Type::try_from(v) { v } else { panic!() }
    }
    fn from(v: u64) -> Self::Type {
        if let Ok(v) = Self::Type::try_from(v) { v } else { panic!() }
    }

    fn to_u8(v: Self::Type) -> u8 {
        if let Ok(v) = Self::Type::try_into(v) { v } else { panic!() }
    }
    fn to_u16(v: Self::Type) -> u16 {
        if let Ok(v) = Self::Type::try_into(v) { v } else { panic!() }
    }
    fn to_u32(v: Self::Type) -> u32 {
        if let Ok(v) = Self::Type::try_into(v) { v } else { panic!() }
    }
    fn to_u64(v: Self::Type) -> u64 {
        if let Ok(v) = Self::Type::try_into(v) { v } else { panic!() }
    }
    fn to(v: Self::Type) -> u64 {
        if let Ok(v) = Self::Type::try_into(v) { v } else { panic!() }
    }
}

impl Specifier for bool {
    const BITS: usize = 1;
    type Type = Self;

    fn from_u8(v: u8) -> Self::Type {
        v == 1
    }
    fn from(v: u64) -> Self::Type {
        Self::from_u8(v as u8)
    }

    fn to_u8(v: Self::Type) -> u8 {
        if v { 1 } else { 0 }
    }
    fn to(v: Self::Type) -> u64 {
        Self::to_u8(v) as u64
    }
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
    use super::private::*;

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

    #[test]
    fn test_edge() {
        let mut data = [0u8; 4];
        //B9::set(0, &mut data, 0b1100_0011_1);
        //B6::set(9, &mut data, 0b101_010);
        B13::set(9 + 6, &mut data, 0x1675);
        //B4::set(9 + 6 + 13, &mut data, 0b1110);

        println!("{:?}", data);
        //assert_eq!(B9::get(0, &data), 0b1100_0011_1);
        //assert_eq!(B6::get(9, &data), 0b101_010);
        assert_eq!(B13::get(9 + 6, &data), 0x1675);
        //assert_eq!(B4::get(9 + 6 + 13, &data), 0b1110);
    }
}
