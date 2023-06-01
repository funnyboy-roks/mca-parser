//! This module contains all structs related to the nbt data in the chunks
//!
//! Every field for all structs in this class have been renamed to snake_case from whichever case
//! Mojang used.
//!
//! &lt;rant&gt;  
//! Mojang has the worst naming conventions ever! Sometimes they use snake_case, sometimes they use
//! PascalCase, other times they use camelCase, sometimes it's SCREAMING_SNAKE_CASE!  This is so
//! annoying when dealing with Mojang things!  Feel free to look at the name changes on _almost
//! everything_ just to make it happy and you'll be just as annoyed as I am!  
//! &lt;/rant&gt;

use fastnbt::{self, IntArray, LongArray, Value};
use serde::Deserialize;

/// The represents that chunk's nbt data stored in the region file
///
/// See <https://minecraft.fandom.com/wiki/Chunk_format#NBT_structure>
#[derive(Deserialize, Debug)]
pub struct ChunkNbt {
    #[serde(rename = "DataVersion")]
    pub data_version: i32,
    #[serde(rename = "Level")]
    pub level: Level,
}

impl ChunkNbt {
    //pub fn get_block(&self, pos: super::BlockPosition) -> anyhow::Result<BlockState> {
    //    dbg!(&pos);

    //    let section = pos.y / 16; // Sections are 16 block tall
    //    let section = self
    //        .level
    //        .sections
    //        .iter()
    //        .find(|sc| sc.y == section as i8)
    //        .context("sub-chunk does not exist.")?;

    //    // format is yxz
    //    let index = (pos.y % 16) * 16 * 16 + (pos.z % 16) * 16 + (pos.x % 16);

    //    let data = &section.block_states;

    //    // The count of bits for each palette entry
    //    let bit_count = ((section.palette.len() as f32).log2().ceil() as usize).max(4);

    //    let f = change_array_elt_size(data, bit_count);

    //    dbg!(&f);
    //    todo!()
    //}
}

//const fn nibbles(byte: u8) -> (u8, u8) {
//    (0xf0 & byte, 0x0f & byte)
//}

///// This _theoretically_ should not need more than 12 bits, but I'm not acutally 100% sure on that.
//fn change_array_elt_size(data: &LongArray, bit_count: usize) -> Vec<u16> {
//    let values_per_long = i64::BITS as usize / bit_count;
//
//    let mask = (1 << bit_count) - 1;
//
//    let mut out = Vec::with_capacity(data.len() * values_per_long);
//    for long in data.iter() {
//        for i in 0..values_per_long {
//            let item = mask & (long >> (i * bit_count));
//            out.push(item as u16);
//        }
//    }
//    out
//}

/// This does not contain _all_ of the values associated with the level, due to the fact that the
/// mcwiki has outdated information on this.  These are just some of the values that I got while
/// digging through the data myself.
// TODO: Add the rest of the fields
#[derive(Deserialize, Debug)]
pub struct Level {
    // TODO: This is probably an enum
    #[serde(rename = "Status")]
    pub status: String,
    #[serde(rename = "zPos")]
    pub z_pos: i32,
    #[serde(rename = "LastUpdate")]
    pub last_update: i64,
    #[serde(rename = "starlight.light_version")]
    pub starlight_light_version: i32,
    #[serde(rename = "Biomes")]
    pub biomes: IntArray,
    #[serde(rename = "InhabitedTime")]
    pub inhabited_time: i64,
    #[serde(rename = "xPos")]
    pub x_pos: i32,
    #[serde(rename = "HeightMaps")]
    pub height_maps: Option<Heightmaps>,
    #[serde(rename = "TileEntities")]
    pub tile_entities: Vec<Value>, // TODO: Can probably be replaced with an enum
    #[serde(rename = "isLightOn")]
    pub is_light_on: bool,
    #[serde(rename = "TileTicks")]
    pub tile_ticks: Vec<Value>, // TODO: I believe this actually has a specific format.
    #[serde(rename = "Sections")]
    pub sections: Vec<Value>,
}

// byte Nibble4(byte[] arr, int index) {
// 	return index%2 == 0 ? arr[index/2]&0x0F : (arr[index/2]>>4)&0x0F;
// }
//
// int BlockPos = y*16*16 + z*16 + x;
//
// compound Block = Palette[change_array_element_size(BlockStates,Log2(length(Palette)))[BlockPos]];
//
// string BlockName = Block.Name;
//
// compound BlockState = Block.Properties;
//
// byte Blocklight = Nibble4(BlockLight, BlockPos);
//
// byte Skylight = Nibble4(SkyLight, BlockPos);

/// The represents part of a chunk's nbt data stored in the region file
///
/// See <https://minecraft.fandom.com/wiki/Chunk_format#NBT_structure>
#[derive(Deserialize, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub struct Heightmaps {
    pub motion_blocking: Option<LongArray>,
    pub motion_blocking_no_leaves: Option<LongArray>,
    pub ocean_floor: Option<LongArray>,
    pub ocean_floor_wg: Option<LongArray>,
    pub world_surface: Option<LongArray>,
    pub world_surface_wg: Option<LongArray>,
}

/// The represents a section(subchunk) from a chunk's nbt data stored in the region file
///
/// This does _not_ contain all fields due to the incorrect information on the wiki.
///
/// See <https://minecraft.fandom.com/wiki/Chunk_format#NBT_structure>
#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct ChunkSection {
    pub block_states: Option<LongArray>,
    pub palette: Option<Value>, // TODO: Can probably become an enum
    pub y: i8,
}

/// The represents part of a chunk's nbt data stored in the region file
///
/// See <https://minecraft.fandom.com/wiki/Chunk_format#NBT_structure>
#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct BlockStates {
    pub palette: Vec<BlockState>,
    pub data: Option<LongArray>,
}

/// The represents part of a chunk's nbt data stored in the region file
///
/// See <https://minecraft.fandom.com/wiki/Chunk_format#NBT_structure>
#[derive(Deserialize, Debug)]
pub struct BlockState {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Properties")]
    pub properties: Option<Value>,
}

/// The represents part of a chunk's nbt data stored in the region file
///
/// See <https://minecraft.fandom.com/wiki/Chunk_format#NBT_structure>
#[derive(Deserialize, Debug)]
pub struct Biomes {
    pub palette: Vec<String>,
    pub data: Option<LongArray>,
}

/// The represents part of a chunk's nbt data stored in the region file
///
/// See <https://minecraft.fandom.com/wiki/Chunk_format#NBT_structure>
#[derive(Deserialize, Debug)]
pub struct Biome {
    #[serde(rename = "Name")]
    pub name: String,
}

/// This represents part of a chunk's nbt data stored in the region file
///
/// See <https://minecraft.fandom.com/wiki/Chunk_format#Tile_tick_format>
#[derive(Deserialize, Debug)]
pub struct TileTick {
    /// The ID of the block; used to activate the correct block update procedure.
    pub i: String,
    /// If multiple tile ticks are scheduled for the same tick, tile ticks with lower p are processed first. If they also have the same p, the order is unknown.
    pub p: i32,
    /// The number of ticks until processing should occur. May be negative when processing is overdue.
    pub t: i32,
    pub x: i32,
    pub y: i32,
    pub z: i32,
}
