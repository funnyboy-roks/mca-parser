# mca-parser

This is a simple library for parsing Minecraft's `.mca` region file
format.

For more information on the format, see the Minecraft wiki page for the
[Region File
Format](https://minecraft.fandom.com/wiki/Region_file_format)

## WIP

Note: this lib is in a very early WIP stage.  The only implemented
features at the moment is parsing regions from a folder and a file, and
these "parsed" regions only have the chunk timestamps.  It does not
parse the rest of the chunk data yet.

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
- [ ] Directory parsing
