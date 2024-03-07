use std::fmt::Debug;

/// Represents a number of `N` bytes that is stored in BigEndian format
#[repr(transparent)]
#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) struct BigEndian<const N: usize> {
    inner: [u8; N],
}

impl<const N: usize> From<[u8; N]> for BigEndian<N> {
    fn from(value: [u8; N]) -> Self {
        Self { inner: value }
    }
}

macro_rules! be_impl {
    ($N: literal => $num_type: ty: $var: ident => $e: expr) => {
        impl From<BigEndian<$N>> for $num_type {
            fn from(be: BigEndian<$N>) -> Self {
                let $var = be.inner;
                Self::from_be_bytes($e)
            }
        }

        impl Debug for BigEndian<$N> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", <$num_type>::from(*self))
            }
        }
    };
    ($N: literal => $num_type: ty) => {
        be_impl!($N => $num_type: a => a);
    };
}

impl From<u32> for BigEndian<4> {
    fn from(value: u32) -> Self {
        Self::from(value.to_be_bytes())
    }
}

impl<const N: usize> BigEndian<N> {
    // intended for use in testing, if we ever need this fn, we can remove the `#[cfg(test)]`
    // attribute
    #[cfg(test)]
    pub const fn into_bytes(self) -> [u8; N] {
        self.inner
    }

    pub const fn as_u32(&self) -> u32 {
        if N > 4 {
            panic!();
        }

        match N {
            4 => u32::from_be_bytes([self.inner[0], self.inner[1], self.inner[2], self.inner[3]]),
            3 => u32::from_be_bytes([0, self.inner[0], self.inner[1], self.inner[2]]),
            2 => u32::from_be_bytes([0, 0, self.inner[0], self.inner[1]]),
            1 => u32::from_be_bytes([0, 0, 0, self.inner[0]]),
            _ => unreachable!(),
        }
    }
}

be_impl!(3 => u32: a => [0, a[0], a[1], a[2]]); // if we have only three bytes, make the top one zero
be_impl!(4 => u32);

#[test]
fn test() {
    let be = BigEndian::from([0, 0, 0, 1]);
    assert_eq!(be.as_u32(), 1);
    assert_eq!(u32::from(be), 1);

    let be = BigEndian::from([0, 0, 1]);
    assert_eq!(be.as_u32(), 1);
    assert_eq!(u32::from(be), 1);
}

#[test]
#[should_panic]
fn test_invalid_n() {
    let be = BigEndian::from([0, 0, 0, 0, 1]);
    let n = be.as_u32(); // Should Panic

    // if it doesn't, please tell me the value
    dbg!(n);
}
