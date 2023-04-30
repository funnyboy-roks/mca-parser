use super::*;

macro_rules! big_endian_test {
    ($arr: expr => $value: literal) => {
        assert_eq!(big_endian!(&$arr), $value);
    };
}
macro_rules! big_endian_test_ne {
    ($arr: expr => $value: literal) => {
        assert_ne!(big_endian!(&$arr), $value);
    };
}

#[test]
fn big_endian() {
    big_endian_test!([0_u8; 4] => 0);
    big_endian_test!([1_u8; 4] => 0x01_01_01_01);
    big_endian_test!([0xff_u8; 4] => 0xff_ff_ff_ff);
    big_endian_test!([1_u8, 0_u8, 1_u8, 0_u8] => 0x01_00_01_00);

    big_endian_test_ne!([0_u8; 4] => 1);
    big_endian_test_ne!([1_u8; 4] => 0);
}

#[test]
fn read_file() {
    let file_path = "./test/r.0.0.mca";
    let rg = from_file(file_path);
    assert!(rg.is_ok(), "Unable to read test file: {:?}", rg);
    let rg = rg.unwrap();
    assert_eq!(
        rg.coords,
        RegionPosition { x: 0, z: 0 },
        "Invalid coords read from filename: {:?}",
        rg.coords
    );

    let chunk = rg.get_chunk(0, 0);

    assert!(
        chunk.is_some(),
        "Chunk at (0, 0) not found in region: {:?}",
        rg
    );

    let chunk = chunk.unwrap();
    let nbt = chunk.get_nbt();

    assert!(nbt.is_ok(), "Error when reading chunk nbt: {:?}", nbt);

    let _nbt = nbt.unwrap();
}

#[test]
fn read_dir() {
    let file_path = "./test/regions/";
    let rgs = from_directory(file_path);
    assert!(rgs.is_ok(), "Unable to read test dir: {:?}", rgs);
    let _rgs = rgs.unwrap();
}
