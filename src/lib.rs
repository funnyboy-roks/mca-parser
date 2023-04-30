//! # mca-parser
//!
//! mca-parser does exactly what it says on the tin.  It parses Minecraft's mca files into a format that
//! can be used by rust programs.
//!
//! The Minecraft Wiki is incredibly helpful for detailed information about the region format and
//! the chunks within, I'd recommend using it as a reference when using this crate:
//!
//! - [Region File Format](https://minecraft.fandom.com/wiki/Region_file_format)
//! - [Chunk format](https://minecraft.fandom.com/wiki/Chunk_format)
//!
//! ## Usage Example
//!
//! ```
//! // Get a region from a given file
//! let my_region = mca_parser::from_file("r.0.0.mca")?;
//!
//! // Get the chunk at (0, 0)
//! let my_chunk = my_region.get_chunk((0, 0))?;
//!
//! // Get the nbt data for that chunk
//! let my_nbt = my_chunk.get_nbt()?;
//! ```

use anyhow::{bail, Context};
use miniz_oxide::inflate;
use std::{
    collections::{hash_map::Values, HashMap},
    convert::From,
    fs::{self, File},
    io::{Error, ErrorKind, Read},
    path::Path,
    vec::Vec,
};

pub mod nbt;
use nbt::ChunkNbt;

/// A simple macro that converts a 4 byte array/slice/vec/etc into a u32 using big_endian
/// _This is used rather than [`u32::from_be_bytes`] because it consumes the array_
macro_rules! big_endian {
    ($arr: expr) => {{
        let val = $arr;
        ((val[0] as u32) << 24 | (val[1] as u32) << 16 | (val[2] as u32) << 8 | (val[3] as u32))
    }};
}

/// Represents a chunk's location in the region file
///
/// See <https://minecraft.fandom.com/wiki/Region_file_format#Chunk_location>
#[derive(Debug, Copy, Clone)]
pub(crate) struct Location {
    /// Represents the distance in 4096 byte sectors from the beginning of the file
    offset: u32, // Technically only 3 bytes, but I don't want to use a [u8; 3]

    /// Represents the count of the sectors in which the chunk data is stored.
    /// _Note: The actual size of the chunk data is probably less than `sector_count * 4096`_
    sector_count: u8, // Count of sectors from the beginning see the wiki for more info
}

impl From<[u8; 4]> for Location {
    fn from(value: [u8; 4]) -> Self {
        Self {
            offset: big_endian!(&[0, value[0], value[1], value[2]]),
            sector_count: value[3],
        }
    }
}

/// Represents the compression type for a chunk's payload
///
/// See <https://minecraft.fandom.com/wiki/Region_file_format#Payload>
#[derive(Debug, Clone, Copy)]
pub enum CompressionType {
    GZip,         // RFC1952   Unused in Practice
    Zlib,         // RFC1950
    Uncompressed, //           Unused in Practice
}

impl From<u8> for CompressionType {
    /// Get the CompressionType from an integer
    /// Expects 1, 2, or 3, and will return `CompressionType::Zlib` if provided with anything else
    fn from(value: u8) -> Self {
        match value {
            1 => CompressionType::GZip,
            2 => CompressionType::Zlib,
            3 => CompressionType::Uncompressed,
            _ => CompressionType::Zlib, // Default to Zlib (as that's the only one that should be used in practice)
        }
    }
}

/// Represnts a chunk's payload
///
/// See <https://minecraft.fandom.com/wiki/Region_file_format#Payload>
#[derive(Debug)]
pub struct ChunkPayload {
    pub length: u32,
    pub compression_type: CompressionType,
    pub compressed_data: Vec<u8>,
    // TODO: Add `data` item for the data, which will need to be parsed from NBT
}

/// Represents all data for any given chunk that can be taken from the region file
#[derive(Debug)]
pub struct Chunk {
    pub timestamp: u32,
    pub payload: ChunkPayload,
}

impl Chunk {
    /// Get the nbt data for the chunk
    /// _Note: This uses quite a bit of memory as it needs to decompress all of the compressed data_
    pub fn get_nbt(&self) -> anyhow::Result<ChunkNbt> {
        let uncompressed = inflate::decompress_to_vec_zlib(&self.payload.compressed_data);
        let uncompressed = uncompressed.map_err(|_| Error::from(ErrorKind::UnexpectedEof))?;
        Ok(fastnbt::from_bytes(&uncompressed).context("Error parsing nbt bytes")?)
    }
}

#[derive(Debug, PartialEq, Eq, Default, Hash, Clone, Copy)]
pub struct RegionPosition {
    x: i32,
    z: i32,
}

impl From<ChunkPosition> for RegionPosition {
    fn from(value: ChunkPosition) -> Self {
        Self {
            x: value.x / 32,
            z: value.z / 32,
        }
    }
}

impl From<BlockPosition> for RegionPosition {
    fn from(value: BlockPosition) -> Self {
        Self {
            x: value.x / 16 / 32,
            z: value.z / 16 / 32,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Default, Hash, Clone, Copy)]
pub struct ChunkPosition {
    x: i32,
    z: i32,
}

impl From<BlockPosition> for ChunkPosition {
    fn from(value: BlockPosition) -> Self {
        Self {
            x: value.x / 16,
            z: value.z / 16,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Default, Hash, Clone, Copy)]
pub struct SubChunkPosition {
    x: i32,
    y: i32,
    z: i32,
}

impl From<BlockPosition> for SubChunkPosition {
    fn from(value: BlockPosition) -> Self {
        Self {
            x: value.x / 16,
            y: value.y / 16,
            z: value.z / 16,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Default, Hash, Clone, Copy)]
pub struct BlockPosition {
    x: i32,
    y: i32,
    z: i32,
}

/// Represents the contents of a region file
#[derive(Debug)]
pub struct Region {
    /// The list of chunks contained in this region
    pub chunks: [Option<Chunk>; 1024],
    /// Represents the coords in the world of this region in the order of (x, z)
    /// To find these from actual in-game coords, one must divide by 32 for the x and z (or >> 5)
    pub coords: RegionPosition,
}

impl Region {
    /// Return the chunk at a given x and y coordinate relative to the region or `None` if it has
    /// not been generated.
    /// To get the coords actual in-game coords, one must use `(n % 32) >> 4` where `n` is the
    /// current x or z coord.
    pub fn get_chunk(&self, x: usize, z: usize) -> Option<&Chunk> {
        // This expression comes from the mcwiki,
        // see <https://minecraft.fandom.com/wiki/Region_file_format#Header>
        (&self.chunks[((x & 31) + (z & 31) * 32)]).as_ref()
    }
}

/// The struct used for parsing the region data
#[derive(Debug)]
pub struct RegionParser {
    reader: File,
    locations: [Location; 1024], // 1024 * 4 byte for the locations of the chunks in the chunk data
    timestamps: [u32; 1024],     // 1024 * 4 byte for the timestamps of the last modifications
    coords: Option<RegionPosition>,
}

impl RegionParser {
    /// Create a RegionParser to do the parsing of the file
    pub fn new(reader: File) -> Self {
        Self {
            reader,
            locations: [Location::from([0; 4]); 1024],
            timestamps: [0; 1024],
            coords: None,
        }
    }

    /// Create a RegionParser to do the parsing of the file
    pub fn with_coords(reader: File, coords: Option<RegionPosition>) -> Self {
        Self {
            reader,
            locations: [Location::from([0; 4]); 1024],
            timestamps: [0; 1024],
            coords,
        }
    }

    /// Do the actual parsing for the region file
    /// The `coords` arg is used for the world location of the region (like r.0.0.mca -> (0, 0))
    pub fn parse(&mut self) -> anyhow::Result<Region> {
        let mut bytes = [0_u8; 4];
        for i in 0..1024 {
            // Read the first 1024 * 4 bytes (Location Data 4 bytes each)
            let read = self.reader.read(&mut bytes)?;
            if read < 4 {
                bail!(Error::from(ErrorKind::UnexpectedEof));
            }
            self.locations[i] = Location::from(bytes);
        }

        for i in 0..1024 {
            // Read the next 1024 * 4 bytes (Timestamp Data 4 bytes each)
            let read = self.reader.read(&mut bytes)?;
            if read < 4 {
                bail!(Error::from(ErrorKind::UnexpectedEof));
            }
            self.timestamps[i] = big_endian!(&bytes);
        }

        // The rest is chunk data...
        let chunks = self.parse_chunks()?;
        let rg = Region {
            chunks,
            coords: self.coords.unwrap_or_default(),
        };
        Ok(rg)
    }

    fn parse_chunks(&mut self) -> anyhow::Result<[Option<Chunk>; 1024]> {
        // Grab the rest of the bytes as the locations are not in order and we'll have to jump
        // around the rest of the file quite a bit
        let mut rest = Vec::new();
        self.reader.read_to_end(&mut rest)?;

        // Each sector must be 4096 (and they're padded), so if the remaining bytes is not that
        // long, then there is something wrong.
        if rest.len() < 4096 {
            bail!(Error::from(ErrorKind::UnexpectedEof));
        }

        //let mut chunks = [&None; 1024];
        let mut chunks = Vec::with_capacity(1024);
        // Iterate over each location (could be timestamps or 0..1024) and get the chunk for that
        // location
        for (i, location) in self.locations.iter().enumerate() {
            let chunk = self.parse_chunk(location, &rest)?;
            chunks.push(chunk.map(|payload| Chunk {
                timestamp: self.timestamps[i],
                payload,
            }));
        }
        Ok(chunks.try_into().expect("Can't convert vec into array."))
    }

    fn parse_chunk(&self, loc: &Location, bytes: &Vec<u8>) -> anyhow::Result<Option<ChunkPayload>> {
        if loc.offset == 0 && loc.sector_count == 0 {
            return Ok(None);
        }
        let start = (loc.offset - 2) as usize * 4096_usize; // Subtract two from the offset to
                                                            // account for the 8192 bytes that we
                                                            // took from the beginning for the
                                                            // location and timestamps
        if start + 4 > bytes.len() {
            bail!(Error::from(ErrorKind::UnexpectedEof));
        }

        let length = big_endian!(&bytes[start..(start + 4)]);
        let compression_type = CompressionType::from(bytes[start + 4]);

        let chunk_end = start + 5 + length as usize;
        if chunk_end > bytes.len() {
            bail!(Error::from(ErrorKind::UnexpectedEof));
        }

        let compressed_data = (&bytes[(start + 5)..chunk_end]).into();
        // TODO: Parse the uncompressed_data as NBT using fastnbt
        Ok(Some(ChunkPayload {
            length,
            compression_type,
            compressed_data,
        }))
    }
}

fn pos_from_name(name: &str) -> Option<RegionPosition> {
    let parts: Vec<_> = name.split(".").collect();

    if parts.len() >= 3
        && parts[0] == "r"
        && parts[1].parse::<i32>().is_ok() // confirm that the second and third parts are nums
        && parts[2].parse::<i32>().is_ok()
    {
        Some(RegionPosition {
            x: parts[1].parse().expect("Checked in the conditional"),
            z: parts[2].parse().expect("Checked in the conditional"),
        })
    } else {
        None
    }
}

/// Parse a single ".mca" file into a Region.  This will return an error if the file is not a valid
/// Region file.  The coordinates of the region is taken from the name (r.0.0.mca -> (0, 0)), if
/// the filename does not fit this format, (0, 0) will be used
pub fn from_file(file_path: &str) -> anyhow::Result<Region> {
    let f = File::open(file_path)?;
    let name = Path::new(file_path).file_name();
    if let Some(name) = name {
        let coords = pos_from_name(name.to_str().unwrap());
        let mut parser = RegionParser::with_coords(f, coords);
        let rg = parser.parse()?;

        Ok(rg)
    } else {
        bail!(Error::from(ErrorKind::InvalidInput))
    }
}

/// Represents the id for any given dimension, using the default values that Minecraft uses:
/// -1: Nether
/// 0: Overworld
/// 1: End
///
/// And `Other` for any other non-standard ids
#[derive(Debug)]
pub enum DimensionID {
    /// ID 0
    Overworld,
    /// ID -1
    Nether,
    /// ID 1
    End,
    Other(i32),
}

impl DimensionID {
    pub fn id(&self) -> i32 {
        match self {
            Self::Overworld => 0,
            Self::Nether => -1,
            Self::End => 1,
            Self::Other(n) => *n,
        }
    }
}

impl From<i32> for DimensionID {
    fn from(value: i32) -> Self {
        match value {
            0 => Self::Overworld,
            -1 => Self::Nether,
            1 => Self::End,
            n => Self::Other(n),
        }
    }
}

/// Represents a Dimension with its id and its regions
#[derive(Debug)]
pub struct Dimension {
    pub id: DimensionID,
    pub regions: HashMap<RegionPosition, RegionParser>,
}

impl Dimension {
    /// Get a new dimension using id and path to region files
    ///
    /// Returns Result since [`Self::parsers_from_dir`] can fail
    fn new(id: Option<i32>, dir_path: &str) -> anyhow::Result<Self> {
        Ok(Self {
            id: id.unwrap_or(0).into(),
            regions: Self::parsers_from_dir(dir_path)?,
        })
    }

    /// Get dimension parsers from a directory
    fn parsers_from_dir(dir_path: &str) -> anyhow::Result<HashMap<RegionPosition, RegionParser>> {
        let dir = fs::read_dir(dir_path)?;
        let mut out = HashMap::new();
        for path in dir {
            let path = path?.path();
            let path = path.to_str();
            if let Some(file_path) = path {
                let f = File::open(file_path)?;
                let name = Path::new(file_path).file_name();
                if let Some(name) = name {
                    let coords = pos_from_name(name.to_str().unwrap());
                    if let Some(coords) = coords {
                        let parser = RegionParser::with_coords(f, Some(coords));
                        out.insert(coords, parser);
                        continue;
                    }
                }
            }
            anyhow::bail!("File path did not contain coords: {:?}", path);
        }
        Ok(out)
    }

    /// Get the regions in this Dimension
    pub fn get_regions(&self) -> Values<RegionPosition, RegionParser> {
        self.regions.values()
    }

    /// Get a specific region in this dimension using the region coordinates
    pub fn get_region(&self, coords: RegionPosition) -> Option<&RegionParser> {
        self.regions.get(&coords)
    }

    /// Get a specific region in this dimension using the chunk coordinates
    pub fn get_region_at_chunk(&self, coords: ChunkPosition) -> Option<&RegionParser> {
        self.get_region(coords.into())
    }

    /// Get a specific region in this dimension using the block coordinates
    ///
    /// _Note: The `y` value is unused as it has no impact on the region chosen._
    pub fn get_region_at_block(&self, coords: BlockPosition) -> Option<&RegionParser> {
        self.get_region(coords.into())
    }
}

/// Get a Vec of Regions by parsing all region files in the current folder.  If the file does not
/// end with ".mca", then it will be ignored.
pub fn from_directory(dir_path: &str) -> anyhow::Result<Dimension> {
    Dimension::new(None, dir_path)
}

/// Get a list of regions from a singleplayer world directory
///
/// A singleplayer world is formatted like this:
/// ```text
/// world/
/// ├─ region/
/// │  ├─ <regions ...>
/// ├─ DIM##/
/// │  ├─ region/
/// │  │  ├─ <regions ...>
/// ```
/// (where `<regions ...>` is the list of regions, and `DIM##` is either `DIM1` or `DIM-1`
/// This function should get the `region/` folder if present otherwise go to one of the `DIM##`
/// folders, which should make it work for server world files, since the `world/region/` folder is
/// not present for nether/end
///
/// TODO: Another function that will return all regions for all worlds in the singleplayer folder
pub fn from_singleplayer_world(_world_path: &str) -> anyhow::Result<Vec<Region>> {
    todo!()
}

#[cfg(test)]
mod test;
