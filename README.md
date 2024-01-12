atlas
=====
atlas is a simple texture atlas generator. it uses several versions of the [MAXRECTS algorithm](https://docplayer.net/21175043-A-thousand-ways-to-pack-the-bin-a-practical-approach-to-two-dimensional-rectangle-bin-packing.html) to pack multiple .png textures into a single .png file + a text file describing where each of the original textures is. it also includes tools to unpack a texture atlas and to test which version of the algorithm performs best for a given set of inputs.

installation
------------
just run ```cargo build --release```

the program was developed on the nightly version of rust, but stable should also work. 

the executable will be created in the ```target/release``` directory.

usage
-----
atlas includes 3 sub-commands: pack, unpack and query.

## atlas pack
recursively scans all folders/files provided as sources looking for .png files, and builds a single large .png texture at the given output. an output argument of ```my-folder/foo``` will create two files: ```my-folder/foo.png``` and ```my-folder/foo.json```.

usage: ```atlas pack [options ...] <sources ...> <output>```

the available command-line options are:

```-o```<br>
by default, atlas will not overwrite anything. this option enables overwriting.

```-s <SPACING>```<br>
adds extra ```<SPACING>``` transparent pixels between the textures.

```-p <PAGE_SIZE>```<br>
instead of generating the smallest possible texture to include all the sources, generate multiple pages of a fixed size. the size format is ```[w]x[h]```, e.g. ```atlas pack -p 256x512``` will generate pages of 256x512 pixels. page size must be at least large enough to contain the largest images (by width and height) in the source set.

```-q```<br>
quiet mode, don't print anything to stdout.

```--po2```<br>
generate a texture with power-of-two sizes. this option is always less space-efficient than not using it, but there are some use cases.

```--no-dedup```<br>
by default, if atlas finds two textures that are byte-for-byte equal in the source set, it will generate only one copy in the output file and point both file names to the same position. if this behavior is somehow undesirable, this option disables it.

```--short```<br>
```--area```<br>
```--rotate```<br>
these options make the program use slightly different versions of the MAXRECTS algorithm to pack the textures. see the documentation for the **[query](#atlas-query)** command for details.

## atlas unpack
as the name suggests, unpack does the opposite of pack. it takes a texture atlas **description** (the .json file, not the .png file!) and attempts to reproduce the source textures. the folder structure used to create the atlas will not be reproduced, instead all the textures will be dropped in the directory provided as output. if ```<output_directory>``` does not exist, the program will attempt to create it. the unpacked textures won't necessarily be byte-for-bye equal to the original files, but they will contain the same pixel data.

usage: ```atlas unpack [options ...] <source> <output_directory>```

the available command-line options are:

```-o```<br>
enable overwriting.

```-q```<br>
quiet mode, don't print anything to stdout.

## atlas query
the rectangle packing problem is [NP-complete](https://en.wikipedia.org/wiki/Rectangle_packing#Packing_different_rectangles_in_a_minimum-area_rectangle) and there is no optimal solution for the general case. the MAXRECTS paper suggests a few variations of the basic algorithm, and atlas implements some of them. the query command tests all the variations against a given set of sources and prints a report on their efficiency for this particular case. efficiency in this case is defined by ```(total area of the sources) / (total area of output)```. the smallest the output, the better. an efficiency of >100% is achievable in some special cases due to deduplication.

usage: ```atlas query [options ...] <sources ...>```

there are three parameters that can be tweaked:
* whether or not 90-degree rotation is allowed
* how to sort the source textures, by the largest long side or largest short side
* how to choose a position for a texture in the atlas, by best area fit or by shortest distance from the origin

the efficiency of these options has a lot of variation for each particular set of sources. by default, atlas will use **no rotation**, **long side sort** and **shortest distance fit**. if you run a query on your sources, the program will suggest the command that produces the best results.

the available command-line options are:
```-s <SPACING>```<br>
```-p <PAGE_SIZE>```<br>
```--no-dedup```<br>
these function the same way as in the [pack](#atlas-pack) command, and will be taken into account by the query.

## usage example
to demonstrate atlas, [this set of graphics](https://opengameart.org/content/seven-kingdoms) from [opengameart.org](https://opengameart.org) will be used. it's a good test data set because it's big, comes in a relatively deep folder structure, includes many duplicates and sprites of wildly different sizes.

first unpack the folder, then use ```atlas query SevenKingdoms_graphics```, which produces
```
SETTINGS                                            SIZE      EFFICIENCY
------------------------------------------------------------------------
short side, distance, rotation             1p, 4412x4404          96.66%
short side, area, rotation                 1p, 4419x4410          96.38%
short side, distance, no rotation          1p, 4432x4430          95.66%
short side, area, no rotation              1p, 4467x4483          93.79%
long side, area, rotation                  1p, 4488x4468          93.66%
long side, area, no rotation               1p, 4481x4510          92.93%
long side, distance, rotation              1p, 4499x4505          92.67%
long side, distance, no rotation           1p, 4542x4512          91.65%
------------------------------------------------------------------------
for the most efficient packing of these sources, use this command:
	atlas pack --short --rotate [sources] [output]

if texture rotation is not allowed, use this command:
	atlas pack --short [sources] [output]
```

if the game/program where the textures will be used supports 90-degree UV rotation, the best way to pack these textures is ```atlas pack --short --rotate```, if not, the best option is ```atlas pack --short```.
