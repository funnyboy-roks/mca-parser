#![warn(missing_docs)]
//! # mca-parser
//!
//! A library for parsing Minecraft's [Region files](https://minecraft.wiki/w/Region_file_format)
//!
//! ## Usage
//!
//! This library should be pretty simple to use,
//!
//! ```no_run
//! # use mca_parser::*;
//! # use std::fs::File;
//! // Create a Region from an open file
//! let mut file = File::open("r.0.0.mca")?;
//! let region = Region::from_reader(&mut file)?;
//!
//! // `chunk` is raw chunk data, so we need to parse it
//! let chunk = region.get_chunk(0, 0)?;
//! if let Some(chunk) = chunk {
//!     // Parse the raw chunk data into structured NBT format
//!     let parsed = chunk.parse()?;
//!     println!("{:?}", parsed.status);
//! } else {
//!     // If the chunk is None, it has not been generated
//!     println!("Chunk has not been generated.");
//! }
//! # Ok::<(), mca_parser::error::Error>(())
//! ```

// TODO: Figure out a nice way to encode the types of coordinates into the type system
//     - i.e. relative vs absolute and region vs chunk vs block
//     - probably use a trait of some kind so that we can convert between them easily (maybe
//     just `Into`)

use std::{
    collections::HashMap,
    io::{self, Read},
    ops::Deref,
    path::{Path, PathBuf},
};

use bigendian::BigEndian;
use error::Error;

pub use data::*;
pub use error::Result;

mod bigendian;
pub mod data;
pub mod error;
pub mod nbt;
#[macro_use]
mod util;

#[cfg(test)]
mod test;

/// Represents a region file, with methods to access data within it.
///
/// <https://minecraft.wiki/w/Region_file_format>
#[derive(Debug, Eq, PartialEq)]
#[repr(C)]
pub struct Region {
    locations: [Location; 1024],
    timestamps: [BigEndian<4>; 1024],
    data: [u8],
}

impl Region {
    /// Parse this slice into a Region.  This does no input validation except confirm that the 8KiB
    /// header is there, further validation is done in [`Region::get_chunk`] and [`Chunk::parse`]
    /// to help prevent unnecessary memory allocation.
    ///
    /// Note: Changing the data in the slice after calling this method will change the [`Region`]
    /// returned by this method, so it is advised against
    pub fn from_slice(slice: &[u8]) -> Result<&Region> {
        if slice.len() < 8192 {
            Err(Error::MissingHeader)
        } else {
            // SAFETY: `Region` is (1024 * 4 * 2 = 8192) bytes + some extra data.  We have confirmed that we have
            // the 8192 byte header, so this pointer deref is okay.
            let ptr = &slice[..slice.len() - 8192] as *const [u8] as *const Region;
            Ok(unsafe { &*ptr })
        }
    }

    /// Create a Region from an array if the size is known at compile time.
    ///
    /// # Safety
    /// - `N` >= 8192
    /// - Array _should_ contain valid bytes for a region file, though if it doesn't, that issue
    /// will be caught in [`Region::get_chunk`] and [`Chunk::parse`]
    ///
    /// # Usage
    ///
    /// The intended usage of this method is as a const value:
    ///
    /// ```
    /// # use mca_parser::Region;
    /// const REGION: &Region = unsafe { Region::from_array(include_bytes!("../test/r.0.0.mca")) };
    /// ```
    ///
    /// This method will panic if `N` < 8192, thus failing to compile when used as a const value:
    ///
    /// ```compile_fail
    /// const REGION: &Region = unsafe { Region::from_array(&[0; 16]) };
    /// ```
    pub const unsafe fn from_array<const N: usize>(arr: &[u8; N]) -> &'static Region {
        assert!(N >= 8192);
        &*(std::ptr::slice_from_raw_parts(arr as *const u8, N - 8192) as *const Region)
    }

    /// A method for ease of use, effectively does the same thing as calling [`Read::read_to_end`]
    /// and then passing that to [`Region::from_slice`], with the only difference being that it
    /// returns an owned box rather than a reference.
    ///
    /// # Usage
    ///
    /// ```
    /// # use mca_parser::*;
    /// # use std::fs::File;
    /// let mut file = File::open("./test/r.0.0.mca")?;
    /// let region = Region::from_reader(&mut file)?;
    /// # Ok::<_, error::Error>(())
    /// ```
    pub fn from_reader<R>(r: &mut R) -> Result<Box<Region>>
    where
        R: Read,
    {
        use std::mem::ManuallyDrop;

        let mut vec = ManuallyDrop::new(Vec::new());
        r.read_to_end(&mut vec)?;

        if vec.len() < 8192 {
            Err(Error::MissingHeader)
        } else {
            // SAFETY: `Region` is (1024 * 4 * 2 = 8192) bytes + some extra data.
            let slice =
                unsafe { std::slice::from_raw_parts_mut(vec.as_mut_ptr(), vec.len() - 8192) };
            // SAFETY: We know that the vec is allocated on the heap, so we can form a box from it
            Ok(unsafe { Box::from_raw(slice as *mut [u8] as *mut Region) })
        }
    }

    /// Convert x and z into the correct index into the `locations` and `timestamps` arrays
    ///
    /// # Panics
    ///
    /// - If `x` and `z` are not within `0..=31`
    // This is a simple calculation, and I'm sure the compiler would inline it, but just to make sure
    #[inline(always)]
    const fn chunk_index(x: u32, z: u32) -> usize {
        assert!(x < 32);
        assert!(z < 32);

        z as usize * 32 + x as usize
    }

    /// Validate that this Region contains all valid chunks by trying to parse every chunk.
    ///
    /// # Important Note
    ///
    /// - This method is obviously slow and uses a decent amount of memory.  It is
    /// recommended to assume the data is correct and validate it as you use the
    /// [`Region::get_chunk`] and [`Chunk::parse`] methods.
    /// - This method should only be used when you absolutely _need_ to validate the data is
    /// correct and can't use the [`Region::get_chunk`] and [`Chunk::parse`] methods
    pub fn validate(&self) -> Result<()> {
        for x in 0..32 {
            for z in 0..32 {
                if let Some(chunk) = self.get_chunk(x, z)? {
                    chunk.parse()?;
                }
            }
        }
        Ok(())
    }

    /// Get a timestamp for a chunk in this [`Region`]
    ///
    /// # Panics
    ///
    /// - If `x` and `z` are not within `0..=31`
    pub const fn get_timestamp(&self, x: u32, z: u32) -> u32 {
        self.timestamps[Self::chunk_index(x, z)].as_u32()
    }

    /// Check if the chunk at `x` and `z` have been generated
    ///
    /// # Panics
    ///
    /// - If `x` and `z` are not within `0..=31`
    pub const fn has_chunk(&self, x: u32, z: u32) -> bool {
        !self.locations[Self::chunk_index(x, z)].is_empty()
    }

    /// Get a chunk from this [`Region`] using relative coordinates within the region
    ///
    /// # Return Values
    ///
    /// - `Err` if data is invalid
    /// - `Ok(None)` if the data is valid, but there is no chunk generated
    /// - `Ok(Some(&Chunk))` if the data is valid and the chunk exists
    ///
    /// This will return a `&Chunk` which references this `Region`, if you want an owned
    /// version, call [`Chunk::boxed`] on the returned chunk.
    ///
    /// # Panics
    ///
    /// - If `x` and `z` are not within `0..=31`
    pub fn get_chunk(&self, chunk_x: u32, chunk_z: u32) -> Result<Option<&Chunk>> {
        let loc = &self.locations[Self::chunk_index(chunk_x, chunk_z)];
        let offset: u32 = loc.offset.into();

        if loc.is_empty() {
            return Ok(None);
        }

        // Subtract 2 from the offset to account for the 2 * 4096 bytes that we took from the
        // beginning for the location and timestamps
        let start = (offset - 2) as usize * 4096;

        if self.data.len() < start + 4 {
            return Err(Error::UnexpectedEof);
        }

        // SAFETY: We know that we have these bytes because it's checked above and according to the
        // minecraft wiki, these bytes are the length and since we specifically grab `4`
        // bytes, we know that `BigEndian<4>` is valid.
        let len = u32::from(unsafe { *(self.data[start..][..4].as_ptr() as *const BigEndian<4>) })
            as usize;

        if self.data.len() < start + 4 + len {
            return Err(Error::UnexpectedEof);
        }

        // SAFETY: We have checked that we have `len` bytes after the starting point of `start +
        // 4`, so we can trivially convert that to a Chunk
        let chunk = unsafe {
            &*(core::ptr::slice_from_raw_parts(self.data[start + 4..].as_ptr(), len - 1)
                as *const Chunk)
        };

        Ok(Some(chunk))
    }

    /// Get a chunk from this [`Region`] using relative block coordinates within the region
    ///
    /// # Return Values
    ///
    /// - `Err` if data is invalid
    /// - `Ok(None)` if the data is valid, but there is no chunk generated
    /// - `Ok(Some(&Chunk))` if the data is valid and the chunk exists
    ///
    /// This will return a `&Chunk` which references this `Region`, if you want an owned
    /// version, call [`Chunk::boxed`] on the returned chunk.
    ///
    /// # Panics
    ///
    /// - If `x` and `z` are not within `0..=511`
    pub fn get_chunk_from_block(&self, block_x: u32, block_z: u32) -> Result<Option<&Chunk>> {
        self.get_chunk(block_x / 16, block_z / 16)
    }
}

/// Represents a file which holds a Region
#[derive(Debug, Clone)]
pub struct RegionFile {
    /// The path to this region file on disk
    pub path: PathBuf,
}

impl RegionFile {
    /// Create a [`RegionFile`] from a path to a file
    pub fn new<P>(path: P) -> Self
    where
        P: AsRef<Path>,
    {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }
}

/// Create an iterator over the contents of a directory, allowing each region within to be parsed
pub fn parse_directory<P>(path: P) -> io::Result<impl Iterator<Item = RegionFile>>
where
    P: AsRef<Path>,
{
    let path = path.as_ref();
    assert!(path.is_dir());

    let rd = std::fs::read_dir(path)?;

    let iter = rd.filter_map(|de| {
        let de = de.ok()?;

        let path = de.path();
        if !path.is_file() {
            return None;
        }

        Some(RegionFile::new(path))
    });

    Ok(iter)
}

/// An enum which represents Minecraft's IDs for a dimension
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum DimensionID {
    /// ID: `0`
    Overworld,
    /// ID: `-1`
    Nether,
    /// ID: `1`
    End,
    /// A custom DimensionID
    Custom(i32),
}

impl DimensionID {
    /// Get the id of this dimension as a number
    pub fn id(&self) -> i32 {
        match self {
            Self::Overworld => 0,
            Self::Nether => -1,
            Self::End => 1,
            Self::Custom(n) => *n,
        }
    }
}

impl From<i32> for DimensionID {
    fn from(value: i32) -> Self {
        match value {
            0 => Self::Overworld,
            -1 => Self::Nether,
            1 => Self::End,
            n => Self::Custom(n),
        }
    }
}

/// A wrapper around [`Region`] that allows either a reference to be used or a Box
/// over return types.
///
/// [`Deref`] is implemented for this enum, so in theory, there should never be a need to match on
/// this.
///
/// This is primarily used in the [`RegionParser`] trait, so that the implementers can return
/// either a reference to or a box of a [`Region`].
#[derive(Debug)]
pub enum RegionRef<'a> {
    /// Borrowed Region (via reference)
    Borrowed(&'a Region),
    /// Owned Region (via box)
    Owned(Box<Region>),
}

impl<'a> From<&'a Region> for RegionRef<'a> {
    fn from(value: &'a Region) -> Self {
        Self::Borrowed(value)
    }
}

impl From<Box<Region>> for RegionRef<'_> {
    fn from(value: Box<Region>) -> Self {
        Self::Owned(value)
    }
}

impl Deref for RegionRef<'_> {
    type Target = Region;

    fn deref(&self) -> &Self::Target {
        match self {
            RegionRef::Borrowed(r) => r,
            RegionRef::Owned(r) => r,
        }
    }
}

/// A trait which represents something that can be parsed into a region and optionally contains
/// information about which region in the world it is.
pub trait RegionParser {
    /// Parse this into a [`Region`] and return it through [`RegionRef`] so that we can have either
    /// owned or or as a reference
    fn parse(&self) -> Result<RegionRef<'_>>;

    /// Get the position in the world (using
    /// [region coordinates](https://minecraft.wiki/w/Region_file_format#Location)) of the region that will
    /// be parsed by this [`RegionParser`] if there is no information as to which region this is,
    /// then [`None`] should be returned.
    fn position(&self) -> Option<(i32, i32)>;
}

impl RegionParser for RegionFile {
    fn position(&self) -> Option<(i32, i32)> {
        let filename = self.path.file_name()?.to_string_lossy();
        let mut parts = filename.split('.');
        if parts.next() != Some("r") {
            return None;
        }

        let Some(Ok(x)) = parts.next().map(|s| s.parse()) else {
            return None;
        };

        let Some(Ok(z)) = parts.next().map(|s| s.parse()) else {
            return None;
        };

        if parts.next() != Some("mca") {
            return None;
        }

        Some((x, z))
    }

    fn parse(&self) -> Result<RegionRef<'_>> {
        let mut file = std::fs::File::open(&self.path)?;
        Ok(Region::from_reader(&mut file)?.into())
    }
}

/// Represents a Dimension in a Minecraft world
pub struct Dimension<R> {
    /// The ID for the dimension, see [`DimensionID`]
    pub id: Option<DimensionID>,
    regions: HashMap<(i32, i32), R>,
}

impl Dimension<RegionFile> {
    /// Create a dimension from a path to a directory, the directory's name is used to get the id
    /// if it is in the form of `DIM{id}`.
    pub fn from_path<P>(path: P) -> io::Result<Self>
    where
        P: AsRef<Path>,
    {
        let path = path.as_ref();
        let file = path.file_name();
        let id = file
            .and_then(|n| {
                n.to_string_lossy()
                    .strip_prefix("DIM")
                    .and_then(|n| n.parse().ok())
            })
            .map(|n: i32| n.into());

        Ok(Self::from_iter(id, parse_directory(path)?))
    }
}

impl<R> Dimension<R>
where
    R: RegionParser,
{
    /// Construct a [`Dimension`] from an iterator which yields items which implement the
    /// [`RegionParser`] trait.
    ///
    /// Every parser in the iterator must be able to determine a position, otherwise this call will
    /// panic.
    ///
    /// Note: this call consumes the iterator, but does _not_ call [`RegionParser::parse`] on the
    /// items.
    pub fn from_iter<I>(id: Option<DimensionID>, iter: I) -> Self
    where
        I: Iterator<Item = R>,
    {
        Self {
            id,
            regions: iter.map(|rf| (rf.position().unwrap(), rf)).collect(),
        }
    }

    /// Check if this dimension has a region at this location
    pub fn has_region(&self, region_x: i32, region_z: i32) -> bool {
        self.regions.contains_key(&(region_x, region_z))
    }

    /// Parse a region file at the given location (using [region coordinates](https://minecraft.wiki/w/Region_file_format#Location))
    ///
    /// # Panics
    ///
    /// If the region does not exist in this Dimension, use [`Dimension::has_region`] to check
    /// before making a call to this method.
    pub fn parse_region(&self, region_x: i32, region_z: i32) -> Result<RegionRef> {
        self.regions[&(region_x, region_z)].parse()
    }

    /// Get an iterator over the [`RegionParser`]s contained in this [`Dimension`]
    pub fn regions(&self) -> impl Iterator<Item = &R> {
        self.regions.values()
    }

    /// Get an iterator over the locations of regions in this [`Dimension`] in the format of (x, z).
    pub fn locations(&self) -> impl Iterator<Item = &(i32, i32)> {
        self.regions.keys()
    }

    /// Get a region from an absolute chunk location (i.e. the "Chunk:" line in the F3
    /// screen)
    ///
    /// # Return Values
    ///
    /// - `Ok(None)` if the region does not exist
    /// - `Ok(Some(Region))` if the region exists and parsed successfully
    /// - `Err(_)` if the region failed to parse
    pub fn get_region_from_chunk(&self, chunk_x: i32, chunk_z: i32) -> Result<Option<RegionRef>> {
        // self.has_region(chunk_x / 32, chunk_z / 32)
        //     .then(|| self.parse_region(chunk_x / 32, chunk_z / 32))
        if self.has_region(chunk_x / 32, chunk_z / 32) {
            Ok(Some(self.parse_region(chunk_x / 32, chunk_z / 32)?))
        } else {
            Ok(None)
        }
    }

    /// Get a chunk from an absolute chunk location (i.e. the "Chunk:" line in the F3
    /// screen)
    ///
    /// Note: This is only recommended if you only need one chunk from this region, otherwise, you
    /// should use [`Dimension::parse_region`], [`Region::get_chunk`], and [`Chunk::parse`].  Using
    /// those methods over this one also allows for more fine-grained control over error handling.
    ///
    /// # Return Values
    ///
    /// - `Ok(None)` if the region does not exist
    /// - `Ok(Some(ParsedChunk))` if everything parsed successfully
    /// - `Err(_)` if the region/chunk failed to parse
    pub fn get_chunk_in_world(&self, chunk_x: i32, chunk_z: i32) -> Result<Option<ParsedChunk>> {
        let region = self.get_region_from_chunk(chunk_x, chunk_z);

        match region {
            Ok(None) => Ok(None),
            Ok(Some(region)) => {
                match region.get_chunk(
                    positive_mod!(chunk_x, 32) as u32,
                    positive_mod!(chunk_z, 32) as u32,
                ) {
                    Ok(Some(chunk)) => match chunk.parse() {
                        Ok(p) => Ok(Some(p)),
                        Err(e) => Err(e),
                    },
                    Ok(None) => Ok(None),
                    Err(e) => Err(e),
                }
            }
            Err(e) => Err(e),
        }
    }
}
