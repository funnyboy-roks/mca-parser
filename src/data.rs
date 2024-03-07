//! Module which holds much of the data related structs that are not nbt

use std::ops::Deref;

use miniz_oxide::inflate;

use crate::{bigendian::BigEndian, nbt, positive_mod, Result};

/// A type of compression used by a chunk
///
/// <https://minecraft.wiki/w/Region_file_format#Payload>
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum CompressionType {
    /// RFC1952   Unused in Practice
    GZip = 1,
    /// RFC1950
    Zlib = 2,
    ///
    Uncompressed = 3,
    /// Since 24w04a -- enabled in server.properties
    LZ4 = 4,
    /// Since 24w05a -- for third-party servers
    Custom = 127,
}

/// The location of a chunk in the file, stored in the header
///
/// <https://minecraft.wiki/w/Region_file_format#Chunk_location>
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[repr(C)]
pub(crate) struct Location {
    pub offset: BigEndian<3>,
    pub sector_count: u8,
}

impl Location {
    pub const fn is_empty(&self) -> bool {
        self.offset.as_u32() == 0 && self.sector_count == 0
    }
}

/// A parsed chunk, which owns its NBT data
///
/// The full NBT structure can be accessed through the [`Deref`] implementation to [`nbt::ChunkNbt`]
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedChunk {
    nbt: nbt::ChunkNbt,
}

impl Deref for ParsedChunk {
    type Target = nbt::ChunkNbt;

    fn deref(&self) -> &Self::Target {
        &self.nbt
    }
}

/// Represents one chunk in a region
#[derive(Debug, Eq, PartialEq)]
#[repr(C)]
pub struct Chunk {
    /// The compression type used for the data in this chunk
    pub compression_type: CompressionType,
    compressed_data: [u8],
}

impl Chunk {
    /// Allocate this [`Chunk`] into a new [`Box`] which is owned by the caller
    pub fn boxed(&self) -> Box<Self> {
        let mut b = vec![0u8; std::mem::size_of_val(self)];
        // SAFETY: We need to decrease the length of the vector by one so that the fat pointer has
        // the correct length value.  This is okay since we're shortening the len and we know that
        // the data has been initialised.  We could use the `.truncate()` method, but we do not
        // want to drop the last item, since we still need it.
        unsafe { b.set_len(b.len() - 1) };
        let b = b.into_boxed_slice();
        // SAFETY: We have allocated enough data in the box to call it `Box<Self>`
        let mut b: Box<Self> = unsafe { std::mem::transmute(b) };

        b.as_mut().compression_type = self.compression_type;
        b.compressed_data.copy_from_slice(&self.compressed_data);

        b
    }

    /// Parse this chunk into a [`ParsedChunk`]
    ///
    /// Allocates a new [`Vec`] into which the compressed data will be uncompressed and then parses
    /// the nbt from that [`Vec`]
    pub fn parse(&self) -> Result<ParsedChunk> {
        match self.compression_type {
            CompressionType::GZip => todo!(),
            CompressionType::Zlib => {
                let data = &self.compressed_data;
                let uncompressed = inflate::decompress_to_vec_zlib(data)?;
                Ok(ParsedChunk {
                    nbt: fastnbt::from_bytes(&uncompressed)?,
                })
            }
            CompressionType::Uncompressed => todo!(),
            CompressionType::LZ4 => todo!(),
            CompressionType::Custom => todo!(),
        }
    }

    /// Get the length of the compressed data within this chunk
    pub fn len(&self) -> usize {
        self.compressed_data.len()
    }
}

impl ParsedChunk {
    /// Get a chunk section (or subchunk) from the given `block_y` value which is the y value of a _block_ within
    /// the chunk
    pub fn get_chunk_section_at(&self, block_y: i32) -> Option<&nbt::ChunkSection> {
        let subchunk_y = (block_y / 16) as i8;

        self.sections.iter().find(|s| s.y == subchunk_y)
    }

    /// Get a block from a chunk using block_{x,y,z}.  The x and z coordinates are relative to the chunk,
    /// and the y coordinate is absolute, so (0, 0, 0) is block 0, 0 in the chunk and y=0 in the
    /// world.
    pub fn get_block(&self, block_x: u32, block_y: i32, block_z: u32) -> Option<nbt::BlockState> {
        let subchunk = self.get_chunk_section_at(block_y)?;

        assert!(block_x < 16);
        assert!(block_z < 16);

        let block_y: u32 = positive_mod!(block_y, 16) as u32;

        let bs = subchunk.clone().block_states?;

        let block_states: Vec<_> = if let Some(data) = bs.data {
            data.iter().map(|n| *n as u64).collect()
        } else {
            return Some(nbt::BlockState {
                name: "minecraft:air".into(),
                properties: None,
            });
        };

        let bits = std::cmp::max((bs.palette.len() as f32).log2().ceil() as u32, 4);

        let block_index = block_y * 16 * 16 + block_z * 16 + block_x;
        let block = get_item_in_packed_slice(&block_states, block_index as usize, bits);

        Some(bs.palette[block as usize].clone())
    }

    /// Get a block from a chunk using block_{x,y,z}.  The coordinates are absolute in the
    /// world, so (0, 0, 0) is the block at x=0, y=0, z=0.
    ///
    /// Note: This is only truly valid if this chunk is the chunk which contains that block,
    /// otherwise it's not correct.
    pub fn get_block_from_absolute_coords(
        &self,
        block_x: u32,
        block_y: i32,
        block_z: u32,
    ) -> Option<nbt::BlockState> {
        self.get_block(block_x % 16, block_y, block_z % 16)
    }
}

fn get_item_in_packed_slice(slice: &[u64], index: usize, bits: u32) -> u64 {
    let nums_per_u64 = u64::BITS / bits;
    assert_eq!(
        (slice.len() as u32),
        ((4096. / nums_per_u64 as f32).ceil() as u32)
    );
    let index_in_num = index as u32 % nums_per_u64;
    let shifted_num = slice[index / nums_per_u64 as usize] >> bits * index_in_num;
    shifted_num & (2u64.pow(bits) - 1)
}

#[test]
fn test_get_item_in_packed_slice() {
    let slice = &[0; 128];
    assert_eq!(get_item_in_packed_slice(slice, 15, 2), 0);
    let slice = &[0; 456];
    assert_eq!(get_item_in_packed_slice(slice, 15, 7), 0);
}
