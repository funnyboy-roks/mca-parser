use std::{
    convert::From,
    fs::File,
    io::{Error, ErrorKind, Read, Result},
    path::Path,
    vec::Vec,
};

macro_rules! big_endian {
    ($arr: expr) => {{
        let val = $arr;
        ((val[0] as u32) << 24 | (val[1] as u32) << 16 | (val[2] as u32) << 8 | (val[3] as u32))
    }};
}

/// Represents a chunk's location in the region file
/// See https://minecraft.fandom.com/wiki/Region_file_format#Chunk_location
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
/// See https://minecraft.fandom.com/wiki/Region_file_format#Payload
#[derive(Debug, Clone, Copy)]
pub enum CompressionType {
    GZip,         // RFC1952 - Unused in Practice
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
/// See https://minecraft.fandom.com/wiki/Region_file_format#Payload
#[derive(Debug, Clone, Copy)]
pub struct ChunkPayload {
    pub length: u32,
    pub compression_type: CompressionType,
    // TODO: Add `data` item for the data, which will need to be parsed from NBT
}

/// Represents all data for any given chunk that can be taken from the region file
#[derive(Debug, Clone, Copy)]
pub struct Chunk {
    pub timestamp: u32,
    pub payload: ChunkPayload,
}

/// Represents the contents of a region file
#[derive(Debug)]
pub struct Region {
    /// The list of chunks contained in this region
    pub chunks: [Option<Chunk>; 1024],
    /// Represents the coords in the world of this region in the order of (x, z)
    /// To find these from actual in-game coords, one must divide by 32 for the x and z (or >> 5)
    pub coords: (i32, i32),
}

/// The struct used for parsing the region data
#[derive(Debug)]
pub(crate) struct RegionParser<'a> {
    reader: &'a mut File,
    locations: [Location; 1024], // 1024 * 4 byte for the locations of the chunks in the chunk data
    timestamps: [u32; 1024],     // 1024 * 4 byte for the timestamps of the last modifications
}

impl<'a> RegionParser<'a> {
    /// Create a RegionParser to do the parsing of the file
    pub fn new(reader: &'a mut File) -> Self {
        Self {
            reader,
            locations: [Location::from([0; 4]); 1024],
            timestamps: [0; 1024],
        }
    }

    /// Do the actual parsing for the region file
    /// The `coords` arg is used for the world location of the region (like r.0.0.mca -> (0, 0))
    pub fn parse(&'a mut self, coords: (i32, i32)) -> Result<Region> {
        let mut bytes = [0_u8; 4];
        for i in 0..1024 {
            // Read the first 1024 * 4 bytes (Location Data 4 bytes each)
            let read = self.reader.read(&mut bytes)?;
            if read < 4 {
                return Err(Error::from(ErrorKind::UnexpectedEof));
            }
            self.locations[i] = Location::from(bytes)
        }

        for i in 0..1024 {
            // Read the next 1024 * 4 bytes (Timestamp Data 4 bytes each)
            let read = self.reader.read(&mut bytes)?;
            if read < 4 {
                return Err(Error::from(ErrorKind::UnexpectedEof));
            }
            self.timestamps[i] = big_endian!(&bytes);
        }

        // The rest is chunk data...
        let chunks = self.parse_chunks()?;
        let rg = Region { chunks, coords };
        Ok(rg)
    }

    fn parse_chunks(&'a mut self) -> Result<[Option<Chunk>; 1024]> {
        // Grab the rest of the bytes as the locations are not in order and we'll have to jump
        // around the rest of the file quite a bit
        let mut rest = Vec::new();
        self.reader.read_to_end(&mut rest)?;

        // Each sector must be 4096 (and they're padded), so if the remaining bytes is not that
        // long, then there is something wrong.
        if rest.len() < 4096 {
            return Err(Error::from(ErrorKind::UnexpectedEof));
        }

        let mut chunks = [None; 1024];
        // Iterate over each location (could be timestamps or 0..1024) and get the chunk for that
        // location
        for (i, location) in self.locations.iter().enumerate() {
            let chunk = self.parse_chunk(location, &rest)?;
            chunks[i] = chunk.map(|payload| Chunk {
                timestamp: self.timestamps[i],
                payload,
            });
        }
        Ok(chunks)
    }

    fn parse_chunk(&'a self, loc: &Location, bytes: &'a Vec<u8>) -> Result<Option<ChunkPayload>> {
        let start = (loc.offset - 2) as usize * 4096_usize; // Subtract two from the offset to
                                                            // account for the 8192 bytes that we
                                                            // took from the beginning for the
                                                            // location and timestamps
        if start + 4 > bytes.len() {
            return Err(Error::from(ErrorKind::UnexpectedEof));
        }

        let length = big_endian!(&bytes[start..(start + 4)]);
        if (loc.offset == 0 && loc.sector_count == 0) || length == 0 {
            return Ok(None);
        }
        let compression_type = CompressionType::from(bytes[start + 4]);

        let chunk_end = start + 5 + length as usize;
        if chunk_end > bytes.len() {
            return Err(Error::from(ErrorKind::UnexpectedEof));
        }
        // TODO: Uncompress and parse this as NBT
        let _compressed_data = &bytes[(start + 5)..chunk_end];

        Ok(Some(ChunkPayload {
            length,
            compression_type,
        }))
    }
}

/// Parse a single ".mca" file into a Region.  This will return an error if the file is not a valid
/// Region file.  The coordinates of the region is taken from the name (r.0.0.mca -> (0, 0)), if
/// the filename does not fit this format, (0, 0) will be used
pub fn from_file(file_path: &str) -> Result<Region> {
    let mut f = File::open(file_path)?;
    let mut parser = RegionParser::new(&mut f);
    let name = Path::new(file_path).file_name();
    if let Some(name) = name {
        let parts: Vec<_> = name.to_str().unwrap().split(".").collect();
        let mut coords: (i32, i32) = (0, 0);
        if parts.len() >= 3 {
            coords.0 = parts[1].parse().unwrap();
            coords.1 = parts[2].parse().unwrap();
        }
        let rg = parser.parse(coords)?;
        Ok(rg)
    } else {
        Err(Error::from(ErrorKind::InvalidInput))
    }
}

/// Get a Vec of Regions by parsing all region files in the current folder.  If the file does not
/// end with ".mca", then it will be ignored.
pub fn from_directory(_dir_path: &str) -> Result<Vec<Region>> {
    todo!()
}

/// Get a list of regions from a world directory
/// This will go into the folder specified and look for the first folder that starts with "DIM",
/// then look inside that folder for a folder called "region".  This folder should contain all of
/// the regions.  If any of these values does not hold, then it will return an Error.
pub fn from_world(_world_path: &str) -> Result<Vec<Region>> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! big_endian_test {
        ($arr: expr => $value: literal) => {
            assert_eq!(big_endian!(&$arr), $value);
        };
        ($arr: expr ;=> $value: literal) => {
            assert_ne!(big_endian!(&$arr), $value);
        };
    }

    #[test]
    fn big_endian() {
        big_endian_test!([0_u8; 4] => 0);
        big_endian_test!([1_u8; 4] => 0x01_01_01_01);
        big_endian_test!([0xff_u8; 4] => 0xff_ff_ff_ff);
        big_endian_test!([1_u8, 0_u8, 1_u8, 0_u8] => 0x01_00_01_00);

        big_endian_test!([0_u8; 4] ;=> 1);
        big_endian_test!([1_u8; 4] ;=> 0);
    }

    #[test]
    fn reading() {
        let file_path = "/home/funnyboy_roks/dev/minecraft/mca-parser/test/r.0.0.mca";
        let rg = from_file(file_path);
        assert!(rg.is_ok(), "Unable to read test file: {:?}", rg);
        let rg = rg.unwrap();
        assert_eq!(
            rg.coords,
            (0, 0),
            "Invalid coords read from filename: {:?}",
            rg.coords
        );
    }
}
