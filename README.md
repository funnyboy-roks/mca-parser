# mca-parser

A library for parsing Minecraft's [Region files](https://minecraft.wiki/w/Region_file_format)

## Usage

```rs
// Create a Region from an open file
let mut file = File::open("r.0.0.mca")?;
let region = Region::from_reader(&mut file)?;

// `chunk` is raw chunk data, so we need to parse it
let chunk = region.get_chunk(0, 0)?;
if let Some(chunk) = chunk {
    // Parse the raw chunk data into structured NBT format
    let parsed = chunk.parse()?;
    println!("{:?}", parsed.status);
} else {
    // If the chunk is None, it has not been generated
    println!("Chunk has not been generated.");
}
```
