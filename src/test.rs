use super::*;
use std::{
    fs::{self, File},
    io::{Seek, SeekFrom},
};

/// Load the region as const so that we don't have to load it in every test
const REGION: &Region = unsafe { Region::from_array(include_bytes!("../test/r.0.0.mca")) };

const EXPECTED_CHUNK_LEN: usize = 10717;
const EXPECTED_DATA_VERSION: i32 = 3700;

macro_rules! assert_matches {
    ($a: expr, $b: pat) => {
        assert!(
            matches!($a, $b),
            concat!("provided: {:?}, expected: ", stringify!($b)),
            $a,
        )
    };
}

#[test]
fn test_slice() {
    let vec = fs::read("./test/r.0.0.mca").unwrap();
    let reg = Region::from_slice(&vec).unwrap();

    // Confirm that we're using all of the bytes
    assert_eq!(std::mem::size_of_val(reg), vec.len());

    let chunk = reg.get_chunk(0, 0).unwrap().unwrap();
    assert_eq!(chunk.compression_type, CompressionType::Zlib);
    assert_eq!(chunk.len(), EXPECTED_CHUNK_LEN);
    let parsed = chunk.parse().unwrap();
    assert_eq!(parsed.data_version, EXPECTED_DATA_VERSION);

    assert_matches!(Region::from_slice(&[0u8; 16]), Err(Error::MissingHeader));
}

#[test]
fn test_reader() {
    let mut file = File::open("./test/r.0.0.mca").unwrap();
    let reg = Region::from_reader(&mut file).unwrap();

    // Confirm that we're using all of the bytes
    assert_eq!(
        std::mem::size_of_val(reg.as_ref()) as u64,
        file.seek(SeekFrom::End(0)).unwrap()
    );

    assert_eq!(*REGION, *reg);

    // We should be able to drop the file now, since we've read all of its data
    drop(file);

    // Confirm the data a bit
    let chunk = reg.get_chunk(0, 0).unwrap().unwrap();
    assert_eq!(chunk.compression_type, CompressionType::Zlib);
    assert_eq!(chunk.len(), EXPECTED_CHUNK_LEN);

    let parsed = chunk.parse().unwrap();
    assert_eq!(parsed.data_version, EXPECTED_DATA_VERSION);

    assert_matches!(
        Region::from_reader(&mut &[0; 16][..]),
        Err(Error::MissingHeader)
    );
}

#[test]
fn test_array() {
    let bytes = include_bytes!("../test/r.0.0.mca");
    let reg = unsafe { Region::from_array(bytes) };

    // Confirm that we're using all of the bytes
    assert_eq!(std::mem::size_of_val(reg), bytes.len());

    // Confirm the data a bit
    let chunk = reg.get_chunk(0, 0).unwrap().unwrap();
    assert_eq!(chunk.compression_type, CompressionType::Zlib);
    assert_eq!(chunk.len(), EXPECTED_CHUNK_LEN);

    let parsed = chunk.parse().unwrap();
    assert_eq!(parsed.data_version, EXPECTED_DATA_VERSION);
}

#[test]
fn test_boxed() {
    let chunk = REGION.get_chunk(0, 0).unwrap().unwrap();
    let box_chunk = chunk.boxed();
    // The data from `chunk` and `box_chunk` should be identical
    assert_eq!(*box_chunk, *chunk);

    let parsed = chunk.parse().unwrap();
    let box_parsed = box_chunk.parse().unwrap();
    // Just to double check that the data is identical
    assert_eq!(parsed, box_parsed);
}

#[test]
fn test_get_timestamp() {
    let points = [(0, 0), (1, 2), (2, 5), (8, 10), (25, 29), (31, 31)];

    let expected = [
        1709838433, 1709837896, 1709837897, 1709837898, 1697927255, 1689320056,
    ];

    for ((x, z), expected) in points.iter().zip(expected.iter()) {
        let ts = REGION.get_timestamp(*x, *z);
        // dbg!(ts);
        assert_eq!(ts, *expected, "Checking timestamp at {:?}", (x, z));
    }
}

#[test]
fn test_bounds_checking() {
    let points = [(0, 32), (32, 0), (32, 32)];
    for (x, z) in points {
        // NOTE: Using `catch_unwind` here rather than `#[should_panic]` because we want to check
        // many things, not have it end on the first panic.
        assert!(
            std::panic::catch_unwind(|| REGION.get_chunk(x, z)).is_err(),
            "Checking {:?} out of bounds",
            (x, z)
        );
    }
}

#[test]
fn test_no_chunks() {
    let bytes = &include_bytes!("../test/r.0.0.mca")[..8192];
    let reg = Region::from_slice(bytes).unwrap();

    assert_matches!(reg.get_chunk(0, 0), Err(Error::UnexpectedEof));
}

#[test]
fn test_not_enough_chunk_data() {
    let bytes = &include_bytes!("../test/r.0.0.mca")[..8192];
    let reg = Region::from_slice(bytes).unwrap();

    assert_matches!(reg.get_chunk(0, 0), Err(Error::UnexpectedEof));
}

#[test]
fn test_missing_chunk() {
    let mut bytes: Vec<u8> = Vec::new();

    let chunk_data = [0, 0, 0, 0];

    bytes.extend([0, 0, 2, 2]); // locations[0] = Location { offset: 2, sector_count: 2 };
    bytes.extend([0; 1023 * 4]); // locations[1..1024] = {0}
    bytes.extend([0; 1024 * 4]); // timestamps[..1024] = {0}
    bytes.extend(BigEndian::from(chunk_data.len() as u32 + 10 + 10).into_bytes());
    bytes.push(2);
    bytes.extend(chunk_data);

    let reg = Region::from_slice(&bytes).unwrap();

    // Chunk is missing because locations is 0
    assert_matches!(reg.get_chunk(0, 0), Err(Error::UnexpectedEof));
}

#[test]
fn test_invalid_chunks_compress() {
    let mut bytes: Vec<u8> = Vec::new();

    let chunk_data = [0, 0, 0, 0];

    bytes.extend([0, 0, 2, 2]); // locations[0] = Location { offset: 2, sector_count: 2 };
    bytes.extend([0; 1023 * 4]); // locations[1..1024] = {0}
    bytes.extend([0; 1024 * 4]); // timestamps[..1024] = {0}
    bytes.extend(BigEndian::from(chunk_data.len() as u32 + 1).into_bytes());
    bytes.push(2);
    bytes.extend(chunk_data);

    let reg = Region::from_slice(&bytes).unwrap();
    let chunk = dbg!(reg.get_chunk(0, 0).unwrap().unwrap());

    assert_matches!(chunk.parse().unwrap_err(), Error::DecompressError(_));
}

#[test]
fn test_invalid_chunks_invalid_nbt() {
    let mut bytes: Vec<u8> = Vec::new();

    // Compressed version of [0, 0, 0, 0]
    let chunk_data = [120, 218, 99, 96, 96, 96, 0, 0, 0, 4, 0, 1];

    bytes.extend([0, 0, 2, 2]); // locations[0] = Location { offset: 2, sector_count: 2 };
    bytes.extend([0; 1023 * 4]); // locations[1..1024] = {0}
    bytes.extend([0; 1024 * 4]); // timestamps[..1024] = {0}
    bytes.extend(BigEndian::from(chunk_data.len() as u32 + 1).into_bytes());
    bytes.push(2);
    bytes.extend(chunk_data);

    let reg = Region::from_slice(&bytes).unwrap();
    let chunk = dbg!(reg.get_chunk(0, 0).unwrap().unwrap());

    assert_matches!(chunk.parse().unwrap_err(), Error::NbtError(_));
}

#[test]
fn test_missing_chunks() {
    let mut bytes: Vec<u8> = Vec::new();

    bytes.extend([0; 1024 * 4]); // locations[1..1024] = {0}
    bytes.extend([0; 1024 * 4]); // timestamps[..1024] = {0}

    let reg = Region::from_slice(&bytes).unwrap();
    assert_matches!(reg.get_chunk(0, 0), Ok(None));
}

#[test]
fn test_region_file() {
    let rf = RegionFile::new("./test/r.0.0.mca");

    {
        let rf = RegionFile::new("./test/r.-1.100.mca");
        assert_eq!(rf.position(), Some((-1, 100)));
        let rf = RegionFile::new("./test/r.10.-100.mca");
        assert_eq!(rf.position(), Some((10, -100)));
        let rf = RegionFile::new("./test/r.20.10.mca");
        assert_eq!(rf.position(), Some((20, 10)));
    }

    let region = rf.parse().unwrap();
    assert_eq!(*REGION, *region);
}

#[test]
fn test_parse_directory() {
    let dir: Vec<_> = parse_directory("./test/regions")
        .unwrap()
        .filter_map(|rf| rf.position())
        .collect();

    dbg!(dir);
}

#[test]
fn test_validate() {
    // REGION.validate().unwrap();
}

#[test]
fn test_has_chunk() {
    assert!(REGION.has_chunk(0, 0));
}

#[test]
fn test_dim_id() {
    for i in -50..50 {
        let did = DimensionID::from(i);
        assert_eq!(did.id(), i);
    }
}

#[test]
fn test_dimension() {
    let dim = Dimension::from_path("./test/regions").unwrap();
    dim.regions().for_each(|r| {
        dbg!(r.parse().unwrap().has_chunk(0, 0));
    });
}

#[test]
fn test_heightmaps() {
    let chunk = REGION.get_chunk(0, 0).unwrap().unwrap().parse().unwrap();

    let mb = &chunk.height_maps.motion_blocking.as_ref().unwrap();
    // for x in 0..16 {
    //     for z in 0..16 {
    //         eprintln!("{:?} = {}", (x, z), mb.get_height(x, z));
    //     }
    // }
    dbg!(mb.get_height(0, 0));
}

#[test]
fn test_block_in_chunk() {
    let chunk = REGION.get_chunk(0, 0).unwrap().unwrap().parse().unwrap();

    // mostly checking to confirm it doesn't crash
    assert_eq!(
        *chunk.get_block(4, 84, 10).unwrap(),
        nbt::BlockState {
            name: nbt::NamespacedKey::minecraft("grass_block".into()),
            properties: Some(fastnbt::nbt!({
                "snowy": "false",
            })),
        }
    );

    assert_eq!(chunk.get_block(13, 200, 15), None)
}
