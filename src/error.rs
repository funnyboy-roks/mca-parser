//! Module which contains information about potential errors that may occur while using this crate

use std::{fmt::Debug, io};

/// The general error type which wraps other errors
#[derive(Debug)]
pub enum Error {
    /// An error that may occur when parsing NBT data
    NbtError(fastnbt::error::Error),
    /// An error that may occur when decompressing data
    DecompressError(miniz_oxide::inflate::DecompressError),
    /// An error that may occur when interaction with an [`io`] item
    IoError(io::Error),
    /// An error that may occur when the header of a region file is missing
    MissingHeader,
    /// An error that may occur when more data is expected by a parser than is provided
    UnexpectedEof,
    /// A custom error type that is not used within this crate, but may be needed for implementors
    /// of the traits within this crate.
    Custom(String),
}

macro_rules! error_wrap {
    ($type: ty => $val: ident) => {
        impl From<$type> for Error {
            fn from(value: $type) -> Self {
                Error::$val(value)
            }
        }
    };
}

error_wrap!(fastnbt::error::Error => NbtError);
error_wrap!(miniz_oxide::inflate::DecompressError => DecompressError);
error_wrap!(std::io::Error => IoError);

/// A type alias used throughout the create to reduce repetition of the [`Error`] enum
pub type Result<T> = std::result::Result<T, Error>;
