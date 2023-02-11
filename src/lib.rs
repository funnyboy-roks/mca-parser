use std::{
    convert::From,
    fs::{self, File},
    io::{Read, Result},
    path::Path,
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
            offset: parse_big_endian(&[0, value[0], value[1], value[2]]),
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

#[derive(Debug, Clone, Copy)]
pub struct Chunk {
    timestamp: u32,
    size: u32,
}

#[derive(Debug)]
pub struct Region {
    chunks: [Chunk; 1024],
}

impl Region {
    fn new(locations: [Location; 1024], timestamps: [u32; 1024], reader: &mut File) -> Self {
        let chunks = timestamps.map(|timestamp| Chunk {
            timestamp,
            size: 0,
        });
        
        Region {
            chunks,
        }
    }
}

impl<'a> RegionParser<'a> {
    pub fn new(reader: &'a mut File) -> Self {
        Self {
            reader,
            locations: [Location::from([0; 4]); 1024],
            timestamps: [0; 1024],
        }
    }

    pub fn parse(&mut self) -> Result<Region> {
        let mut bytes = [0_u8; 4];
        for i in 0..1024 {
            // Read the first 1024 bytes (Location Data)
            self.reader.read(&mut bytes)?;
            self.locations[i] = Location::from(bytes)
        }
        for i in 0..1024 {
            // Read the next 1024 bytes (Timestamp Data)
            self.reader.read(&mut bytes)?;
            self.timestamps[i] = parse_big_endian(&bytes);
        }
        Ok(Region::new(self.locations, self.timestamps, &mut self.reader))
        // The rest is chunk data...
    }
}

fn parse_big_endian(bytes: &[u8; 4]) -> u32 {
    (bytes[0] as u32) << 24 | (bytes[1] as u32) << 16 | (bytes[2] as u32) << 8 | (bytes[3] as u32)
}

/// Parse a single ".mca" file into a Region.  This will return an error if the file is not a valid
/// Region file.
pub fn from_file(file_path: &str) -> Result<Region> {
    let mut f = File::open(file_path)?;
    let mut parser = RegionParser::new(&mut f);
    parser.parse()
}

/// Get a Vec of Regions by parsing all region files in the current folder.  If the file does not
/// end with ".mca", then it will be ignored.
pub fn from_directory(dir_path: &str) -> Result<Vec<Region>> {
    let f = File::open(dir_path);
    let md = f?.metadata()?;
    assert!(md.is_dir(), "Provided file must be a directory!");
    let contents = fs::read_dir(Path::new(dir_path))?;
    let mut out = Vec::new();
    for entry in contents {
        match entry {
            Ok(file) => {
                if file.file_name().to_str().unwrap().ends_with(".mca") {
                    let rg = from_file(file.file_name().to_str().expect("Invalid filename!"));
                    if rg.is_ok() {
                        out.push(rg.unwrap());
                    }
                }
            }
            Err(a) => return Err(a),
        };
    }
    Ok(out)
}

/// Get a list of regions from a world directory
/// This will go into the folder specified and look for the first folder that starts with "DIM",
/// then look inside that folder for a folder called "region".  This folder should contain all of
/// the regions.  If any of these values does not hold, then it will return an Error.
pub fn from_world(world_path: &str) -> Result<Vec<Region>> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn big_endian() {
        assert_eq!(parse_big_endian(&[0_u8; 4]), 0);
        assert_eq!(
            parse_big_endian(&[1_u8; 4]),
            0b00000001_00000001_00000001_00000001
        );
        assert_eq!(
            parse_big_endian(&[0b11111111_u8; 4]),
            0b11111111_11111111_11111111_11111111
        );
        assert_eq!(
            parse_big_endian(&[1_u8, 0_u8, 1_u8, 0_u8]),
            0b00000001_00000000_00000001_00000000
        );
    }

    #[test]
    fn reading() {
        let rg = from_file("/home/funnyboy_roks/dev/minecraft/mca-parser/test/r.0.0.mca");
        assert!(rg.is_ok(), "Unable to read test file: {:?}", rg)
    }
}
