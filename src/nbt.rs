//! This module contains all structs related to the nbt data in the chunks
//!
//! Note: Many of the descriptions for the fields/structs in this module come directly from the
//! Minecraft Wiki, with some occasional modifications to make them make more sense (at least in
//! this context).  The latest update to these descriptions was on 5 March 2024, and I'll try to
//! keep them updated.
//!
//! &lt;rant&gt;  
//! Mojang has the worst naming conventions ever! Sometimes they use snake_case, sometimes they use
//! PascalCase, other times they use camelCase, sometimes it's SCREAMING_SNAKE_CASE!  This is so
//! annoying when dealing with Mojang things!  Feel free to look at the name changes on _almost
//! every field in this module_ just to make it happy and you'll be just as annoyed as I am!  
//! &lt;/rant&gt;

use fastnbt::{self, LongArray, Value};
use serde::Deserialize;

/// Represents a namespace that can show up in the game
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Namespace {
    /// Default namespace for every vanilla item/block/etc
    Minecraft,
    /// Custom namespace, used in mods/datapacks/etc
    Custom(String),
}

impl From<&str> for Namespace {
    fn from(value: &str) -> Self {
        if value == "minecraft" {
            Self::Minecraft
        } else {
            Self::Custom(value.into())
        }
    }
}

/// A struct which represents a key with a namespace
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct NamespacedKey {
    /// The namespace of this key
    pub namespace: Namespace,
    /// The key itself
    pub key: String,
}

impl NamespacedKey {
    /// Create a new NamespacedKey from a namsepace and key
    pub fn new(namespace: impl AsRef<str>, key: String) -> Self {
        Self {
            namespace: Namespace::from(namespace.as_ref()),
            key,
        }
    }

    /// Create a new NamespacedKey using the `minecraft` namespace
    pub fn minecraft(key: String) -> Self {
        Self {
            namespace: Namespace::Minecraft,
            key,
        }
    }
}

impl From<&str> for NamespacedKey {
    fn from(value: &str) -> Self {
        if let Some((ns, k)) = value.split_once(':') {
            Self::new(ns, k.into())
        } else {
            Self::minecraft(value.into())
        }
    }
}

impl<'de> serde::Deserialize<'de> for NamespacedKey {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(NamespacedKey::from(<&str>::deserialize(deserializer)?))
    }
}

/// The represents that chunk's nbt data stored in the region file
///
/// - See <https://minecraft.wiki/w/Chunk_format#NBT_structure>
#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct ChunkNbt {
    /// Version of the chunk NBT structure.
    #[serde(rename = "DataVersion")]
    pub data_version: i32,
    /// `x` position of the chunk (in absolute chunks from world `x`, `z` origin, __not__ relative to the region).
    #[serde(rename = "xPos")]
    pub x_pos: i32,
    /// `z` position of the chunk (in absolute chunks from world `x`, `z` origin, __not__ relative to the region).
    #[serde(rename = "zPos")]
    pub z_pos: i32,
    /// Lowest Y section in chunk
    #[serde(rename = "yPos")]
    pub y_pos: i32,
    /// Defines the world generation status of this chunk
    ///
    /// All status except [`Status::Full`] are used for chunks called proto-chunks, in other words,
    /// for chunks with incomplete generation.
    #[serde(rename = "Status")]
    pub status: Status,
    /// Tick when the chunk was last saved.
    #[serde(rename = "LastUpdate")]
    pub last_update: i64,
    /// List of block entities in this chunk
    pub block_entities: Vec<Value>, // TODO: Can probably be replaced with an enum
    /// Several different heightmaps corresponding to 256 values compacted at 9 bits per value
    /// (lowest being 0, highest being 384, both values inclusive).
    #[serde(rename = "Heightmaps")]
    pub height_maps: HeightMaps,
    /// List of "active" liquids in this chunk waiting to be updated
    pub fluid_ticks: Vec<Value>, // - See Tile Tick Format
    /// List of "active" blocks in this chunk waiting to be updated. These are used to save the
    /// state of redstone machines or falling sand, and other activity
    pub block_ticks: Vec<Value>,
    ///  The cumulative number of ticks players have been in this chunk. Note that this value
    ///  increases faster when more players are in the chunk. Used for Regional Difficulty.
    #[serde(rename = "InhabitedTime")]
    pub inhabited_time: i64,
    /// This appears to be biome blending data, although more testing is needed to confirm.
    pub blending_data: BlendingData,
    /// A List of 24  Lists that store the positions of blocks that need to receive an update when
    /// a proto-chunk turns into a full chunk, packed in  Shorts. Each list corresponds to specific
    /// section in the height of the chunk
    #[serde(rename = "PostProcessing")]
    pub post_processing: [Vec<Value>; 24],
    /// Structure data in this chunk
    pub structures: Value,
    /// A list of the sections in this chunk
    ///
    /// All sections in the world's height are present in this list, even those who are empty (filled with air).
    pub sections: Vec<ChunkSection>,
}

/// Possible statuses for the `status` field in [`ChunkNbt`]
#[derive(Deserialize, Debug, Clone, PartialEq)]
pub enum Status {
    /// `minecraft:empty`
    #[serde(rename = "minecraft:empty")]
    Empty,
    /// `minecraft:structure_starts`
    #[serde(rename = "minecraft:structure_starts")]
    StructureStarts,
    /// `minecraft:structure_references`
    #[serde(rename = "minecraft:structure_references")]
    StructureReferences,
    /// `minecraft:biomes`
    #[serde(rename = "minecraft:biomes")]
    Biomes,
    /// `minecraft:noise`
    #[serde(rename = "minecraft:noise")]
    Noise,
    /// `minecraft:surface`
    #[serde(rename = "minecraft:surface")]
    Surface,
    /// `minecraft:carvers`
    #[serde(rename = "minecraft:carvers")]
    Carvers,
    /// `minecraft:features`
    #[serde(rename = "minecraft:features")]
    Features,
    /// `minecraft:light`
    #[serde(rename = "minecraft:light")]
    Light,
    /// `minecraft:spawn`
    #[serde(rename = "minecraft:spawn")]
    Spawn,
    /// `minecraft:full`
    #[serde(rename = "minecraft:full")]
    Full,
}

/// From the wiki: This appears to be biome blending data, although more testing is needed to confirm.
///
/// - See <https://minecraft.wiki/w/Chunk_format#NBT_structure>
#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct BlendingData {
    /// [More information needed]
    pub min_section: i32,
    /// [More information needed]
    pub max_section: i32,
}

/// Several different heightmaps corresponding to 256 values compacted at 9 bits per value (lowest
/// being 0, highest being 384, both values inclusive).
///
/// - See <https://minecraft.wiki/w/Chunk_format#NBT_structure>  
/// - See <https://minecraft.wiki/w/Heightmap>
#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub struct HeightMaps {
    /// Stores the Y-level of the highest block whose material blocks motion (i.e. has a collision
    /// box) or blocks that contains a fluid (water, lava, or waterlogging blocks).
    pub motion_blocking: HeightMap,
    /// Stores the Y-level of the highest block whose material blocks motion (i.e. has a collision
    /// box), or blocks that contains a fluid (water, lava, or waterlogging blocks), except various
    /// leaves. Used only on the server side.
    pub motion_blocking_no_leaves: HeightMap,
    /// Stores the Y-level of the highest block whose material blocks motion (i.e. has a collision
    /// box). One exception is carpets, which are considered to not have a collision box to
    /// heightmaps. Used only on the server side.
    pub ocean_floor: HeightMap,
    /// Stores the Y-level of the highest block whose material blocks motion (i.e. has a collision
    /// box). Used only during world generation, and automatically deleted after carvers are
    /// generated.
    pub ocean_floor_wg: Option<HeightMap>,
    /// Stores the Y-level of the highest non-air (all types of air) block.
    pub world_surface: HeightMap,
    /// Stores the Y-level of the highest non-air (all types of air) block. Used only during world
    /// generation, and automatically deleted after carvers are generated.
    pub world_surface_wg: Option<HeightMap>,
}

/// Wrapper type around a [`LongArray`] to abstract away the details of how the HeightMaps store
/// their data
///
/// Several different heightmaps corresponding to 256 values compacted at 9 bits per value (lowest
/// being 0, highest being 384, both values inclusive).
///
/// - See <https://minecraft.wiki/w/Chunk_format#NBT_structure>  
/// - See <https://minecraft.wiki/w/Heightmap>
#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(transparent)]
pub struct HeightMap {
    /// The 9-bit values are stored in an array of 37 Longs ([`u64`]), each containing 7 values (7×9 =
    /// 63; the last bit is unused). The 9-bit values are unsigned, and indicate the amount of blocks
    /// above the bottom of the world (y = -64).
    raw: LongArray,
}

impl HeightMap {
    /// Get the height of a chunk using this heightmap at a positon (relative to the chunk)
    pub fn get_height(&self, block_x: u32, block_z: u32) -> i32 {
        assert!(block_x < 16);
        assert!(block_z < 16);

        let index = (block_z * 16 + block_x) as usize;

        let num = dbg!(self.raw[index / 7]) as u64 >> dbg!((index % 7) * 9) & (2u64.pow(9) - 1);
        dbg!(num);

        num as i32 - 65
    }
}

/// - See <https://minecraft.wiki/w/Chunk_format#NBT_structure>
#[derive(Deserialize, Debug, PartialEq, Clone)]
pub struct BlockStates {
    /// Set of different block states used in this particular section.
    pub palette: Vec<BlockState>,
    ///  A packed array of 4096 indices pointing to the palette
    ///
    ///  If only one block state is present in the palette, this field is not required and the
    ///  block fills the whole section.
    ///
    ///  All indices are the same length. This length is set to the minimum amount of bits required
    ///  to represent the largest index in the palette, and then set to a minimum size of 4 bits.
    ///
    ///  The indices are not packed across multiple elements of the array, meaning that
    ///  if there is no more space in a given 64-bit integer for the whole next index, it starts
    ///  instead at the first (lowest) bit of the next 64-bit integer. Different sections of a
    ///  chunk can have different lengths for the indices.
    pub data: Option<LongArray>,
}

/// Data which represents a block in a chunk
///
/// - See <https://minecraft.wiki/w/Chunk_format#NBT_structure>
#[derive(Deserialize, Debug, PartialEq, Clone)]
pub struct BlockState {
    /// Block [resource location](https://minecraft.wiki/w/Resource_location)
    #[serde(rename = "Name")]
    pub name: NamespacedKey,
    /// Properties of the block state
    #[serde(rename = "Properties")]
    pub properties: Option<Value>,
}

/// - See <https://minecraft.wiki/w/Chunk_format#NBT_structure>
#[derive(Deserialize, Debug, PartialEq, Clone)]
pub struct Biomes {
    /// Set of different biomes used in this particular section.
    pub palette: Vec<String>,
    /// A packed array of 64 indices pointing to the palette
    ///
    /// If only one biome is present in the palette, this field is not required and the biome fills
    /// the whole section.
    ///
    /// All indices are the same length: the minimum amount of bits required to represent the
    /// largest index in the palette. These indices do not have a minimum size. Different chunks
    /// can have different lengths for the indices.
    pub data: Option<LongArray>,
}

/// - See <https://minecraft.wiki/w/Chunk_format#Tile_tick_format>
#[derive(Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct TileTick {
    /// The ID of the block; used to activate the correct block update procedure.
    #[serde(rename = "i")]
    pub id: String,
    /// If multiple tile ticks are scheduled for the same tick, tile ticks with lower priority are
    /// processed first. If they also have the same priority, the order is unknown.
    #[serde(rename = "p")]
    pub priority: i32,
    /// The number of ticks until processing should occur. May be negative when processing is
    /// overdue.
    #[serde(rename = "t")]
    pub ticks: i32,
    /// x position
    pub x: i32,
    /// y position
    pub y: i32,
    /// z position
    pub z: i32,
}

/// The represents a section (or subchunk) from a chunk's NBT data stored in the region file
///
/// - See <https://minecraft.wiki/w/Chunk_format#NBT_structure>
#[derive(Deserialize, Debug, PartialEq, Clone)]
pub struct ChunkSection {
    /// Block states of all blocks in this section
    pub block_states: Option<BlockStates>,
    /// y-value of the section
    #[serde(rename = "Y")]
    pub y: i8,
    /// Biomes used in this chunk
    pub biomes: Option<Biomes>,
}
