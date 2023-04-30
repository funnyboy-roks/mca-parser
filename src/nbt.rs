//! This module contains all structs related to the nbt data in the chunks

use fastnbt::{self, ByteArray, LongArray, Value};
use serde::Deserialize;

/// The represents that chunk's nbt data stored in the region file
///
/// See <https://minecraft.fandom.com/wiki/Chunk_format#NBT_structure>
#[derive(Deserialize, Debug)]
pub struct ChunkNbt {
    #[serde(rename = "DataVersion")]
    pub data_version: i32,
    #[serde(rename = "xPos")]
    pub x_pos: i32,
    #[serde(rename = "yPos")]
    pub y_pos: i32,
    #[serde(rename = "Status")]
    pub status: String,
    #[serde(rename = "LastUpdate")]
    pub last_update: i64,
    pub block_entities: Vec<Value>,
    #[serde(rename = "CarvingMasks")]
    pub carving_masks: Option<Value>,
    #[serde(rename = "Heightmaps")]
    pub heightmaps: Heightmaps,
    #[serde(rename = "Lights")]
    pub lights: Option<Vec<Value>>,
    #[serde(rename = "Entities")]
    pub entities: Option<Vec<Value>>,
    pub fluid_ticks: Vec<Value>,
    pub block_ticks: Vec<Value>,
    #[serde(rename = "InhabitedTime")]
    pub inhabited_time: i64,
    #[serde(rename = "PostProcessing")]
    pub post_processing: Vec<Value>,
    pub structures: Value, // TODO: This
}

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
/// See <https://minecraft.fandom.com/wiki/Chunk_format#NBT_structure>
#[derive(Deserialize, Debug)]
pub struct ChunkSection {
    #[serde(rename = "Y")]
    pub y: i8,
    pub block_states: BlockStates,
    pub biomes: Biomes,
    #[serde(rename = "BlockLight")]
    pub block_light: ByteArray,
    #[serde(rename = "SkyLight")]
    pub sky_light: ByteArray,
}

/// The represents part of a chunk's nbt data stored in the region file
///
/// See <https://minecraft.fandom.com/wiki/Chunk_format#NBT_structure>
#[derive(Deserialize, Debug)]
pub struct BlockStates {
    pub palette: Vec<BlockState>,
    pub data: LongArray,
}

/// The represents part of a chunk's nbt data stored in the region file
///
/// See <https://minecraft.fandom.com/wiki/Chunk_format#NBT_structure>
#[derive(Deserialize, Debug)]
pub struct BlockState {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Properties")]
    pub properties: Value,
}

/// The represents part of a chunk's nbt data stored in the region file
///
/// See <https://minecraft.fandom.com/wiki/Chunk_format#NBT_structure>
#[derive(Deserialize, Debug)]
pub struct Biomes {
    pub palette: Vec<Biome>,
    pub data: LongArray,
}

/// The represents part of a chunk's nbt data stored in the region file
///
/// See <https://minecraft.fandom.com/wiki/Chunk_format#NBT_structure>
#[derive(Deserialize, Debug)]
pub struct Biome {
    #[serde(rename = "Name")]
    pub name: String,
}
