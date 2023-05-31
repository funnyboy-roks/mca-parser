# mca-parser

This is a simple library for parsing Minecraft's `.mca` region file
format.

For more information on the format, see the Minecraft wiki page for the
[Region File
Format](https://minecraft.fandom.com/wiki/Region_file_format)

## NBT Inaccuracies

The structure of the nbt data in the files changes in each version.  If
the data has changed and is incorrect in this lib, please create an
issue or even a pr to fix it.

As of this moment of writing, the MC Wiki's data on the chunk nbt format
seems to be very incorrect and I'm having to dig through the data
myself.  If you would like to help me with this, please feel free to
reach out and create a pr.

## Testing

Right now, for testing this library, we have to read from files, so one
will need to create a directory in the root of the project called `test`
with the following contents:

```
test/
├──r.0.0.mca
└──regions/
   └── r.5.5.mca
```

## Todo

- [ ] Region file parsing
	- [x] Parse the chunk size
	- [x] Parse the chunk data
	- [ ] Add methods to the `Chunk` struct:
		- [ ] Get block at position
		- [ ] Get various other chunk information
- [ ] World parsing
	- Go through the directory provided and look for the first
	  instance of a world that starts with "DIM", then in that
	  directory look for a directory called "region".  This
	  directory should contain all of the region files for the given
	  world. *(Note: This is only true for server worlds as they're
	  already broken up into multiple files, for singleplayer
	  worlds, there are multiple directories in the world folder
	  that would match these descriptions)*
- [x] Directory parsing
