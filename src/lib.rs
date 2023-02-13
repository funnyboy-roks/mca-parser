use std::{
    convert::From,
    fs::File,
    io::{self, Error, ErrorKind, Read, Result},
    vec::Vec,
};

#[derive(Debug, Copy, Clone)]
pub struct Location {
    offset: u32,      // Technically only 3 bytes, but I don't want to use a [u8; 3]
    sector_count: u8, // Count of sectors from the beginning see the wiki for more info
}

impl From<[u8; 4]> for Location {
    fn from(value: [u8; 4]) -> Self {
        Self {
            offset: u32::from_bytes_be(&[0, value[0], value[1], value[2]]),
            sector_count: value[3],
        }
    }
}

#[derive(Debug)]
pub struct RegionParser<'a> {
    reader: &'a mut File,
    locations: [Location; 1024], // 1024 * 4 byte for the locations of the chunks in the chunk data
    timestamps: [u32; 1024],     // 1024 * 4 byte for the timestamps of the last modifications
}

#[derive(Debug, Clone, Copy, Default)]
pub enum CompressionType {
    GZip, // RFC1952 - Unused in Practice
    #[default]
    Zlib, // RFC1950
    Uncompressed, //           Unused in Practice
}

impl From<u8> for CompressionType {
    fn from(value: u8) -> Self {
        match value {
            1 => CompressionType::GZip,
            2 => CompressionType::Zlib,
            3 => CompressionType::Uncompressed,
            _ => CompressionType::Zlib, // Default to Zlib (as that's the only one that should be used in practice), though ideally never hit the default
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ChunkPayload {
    pub size: u32,
    pub compression_type: CompressionType,
    // TODO: Add `data` item for the data, which will need to be parsed from NBT
}

#[derive(Debug, Clone, Copy)]
pub struct Chunk {
    pub timestamp: u32,
    pub payload: ChunkPayload,
}

#[derive(Debug)]
pub struct Region {
    pub chunks: [Option<Chunk>; 1024],
}

impl<'a> RegionParser<'a> {
    pub fn new(reader: &'a mut File) -> Self {
        Self {
            reader,
            locations: [Location::from([0; 4]); 1024],
            timestamps: [0; 1024],
        }
    }

    pub fn parse(&'a mut self) -> Result<Region> {
        let mut bytes = [0_u8; 4];
        for i in 0..1024 {
            // Read the first 1024 bytes (Location Data)
            self.reader.read(&mut bytes)?;
            self.locations[i] = Location::from(bytes)
        }
        for i in 0..1024 {
            // Read the next 1024 bytes (Timestamp Data)
            self.reader.read(&mut bytes)?;
            self.timestamps[i] = u32::from_bytes_be(&bytes);
        }
        // The rest is chunk data...
        let chunks = self.parse_chunks()?;
        let rg = Region { chunks };
        Ok(rg)
    }

    fn parse_chunks(&'a mut self) -> Result<[Option<Chunk>; 1024]> {
        let size = self.reader.metadata()?.len();
        let mut rest = Vec::with_capacity(size as usize);
        self.reader.read_to_end(&mut rest)?;
        let mut chunks = [None; 1024];
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
        let start = (loc.offset - 2) as usize * 0x1000_usize;
        if start + 4 > bytes.len() {
            return Err(Error::from(ErrorKind::UnexpectedEof));
        }

        let size = u32::from_bytes_be_slice(&bytes[start..(start + 4)]);
        if (loc.offset == 0 && loc.sector_count == 0) || size == 0 {
            return Ok(None);
        }
        let compression_type = CompressionType::from(bytes[start + 4]);

        let chunk_end = start + 5 + size as usize;
        if chunk_end > bytes.len() {
            return Err(Error::from(ErrorKind::UnexpectedEof));
        }
        // TODO: Uncompress and parse this as NBT
        let _compressed_data = &bytes[(start + 5)..chunk_end];

        Ok(Some(ChunkPayload {
            size,
            compression_type,
        }))
    }
}

trait BigEndian {
    fn from_bytes_be_slice(bytes: &[u8]) -> Self;
    fn from_bytes_be(bytes: &[u8; 4]) -> Self;
}

impl BigEndian for u32 {
    /// Parse into u32 from the array using BigEndian
    /// Preferred over from_be_bytes native fn as is consumes the array for some reason
    fn from_bytes_be(bytes: &[u8; 4]) -> Self {
        (bytes[0] as u32) << 24
            | (bytes[1] as u32) << 16
            | (bytes[2] as u32) << 8
            | (bytes[3] as u32)
    }
    fn from_bytes_be_slice(bytes: &[u8]) -> Self {
        (bytes[0] as u32) << 24
            | (bytes[1] as u32) << 16
            | (bytes[2] as u32) << 8
            | (bytes[3] as u32)
    }
}

/// Parse a single ".mca" file into a Region.  This will return an error if the file is not a valid
/// Region file.
pub fn from_file(file_path: &str) -> Result<Region> {
    let mut f = File::open(file_path)?;
    let mut parser = RegionParser::new(&mut f);
    let rg = parser.parse()?;
    Ok(rg)
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
    #[test]
    fn big_endian() {
        assert_eq!(u32::from_bytes_be(&[0_u8; 4]), 0);
        assert_eq!(
            u32::from_bytes_be(&[1_u8; 4]),
            0b00000001_00000001_00000001_00000001
        );
        assert_eq!(
            u32::from_bytes_be(&[0b11111111_u8; 4]),
            0b11111111_11111111_11111111_11111111
        );
        assert_eq!(
            u32::from_bytes_be(&[1_u8, 0_u8, 1_u8, 0_u8]),
            0b00000001_00000000_00000001_00000000
        );

        assert_eq!(u32::from_bytes_be_slice(&[0_u8; 4]), 0);
        assert_eq!(
            u32::from_bytes_be_slice(&[1_u8; 4]),
            0b00000001_00000001_00000001_00000001
        );
        assert_eq!(
            u32::from_bytes_be_slice(&[0b11111111_u8; 4]),
            0b11111111_11111111_11111111_11111111
        );
        assert_eq!(
            u32::from_bytes_be_slice(&[1_u8, 0_u8, 1_u8, 0_u8]),
            0b00000001_00000000_00000001_00000000
        );
    }

    #[test]
    fn reading() {
        let file_path = "/home/funnyboy_roks/dev/minecraft/mca-parser/test/r.0.0.mca";
        let rg = from_file(file_path);
        assert!(rg.is_ok(), "Unable to read test file: {:?}", rg)
    }
}
