Published at: [https://www.nexusmods.com/skyrimspecialedition/mods/105184](https://www.nexusmods.com/skyrimspecialedition/mods/105184)

This is a small utility for texture makers written in Rust. It takes all images with matching names from a folder and converts them to Skyrim (or Fallout) textures. There is a flag to force using old formats for Skyrim LE too.

The supported image names are: *diffuse, normal, specular, glow, skin_tint, height, cubemap, env_mask, inner_diffuse, inner_depth, subsurface, backlight, metallic, 
glossiness*. Some textures are combined from multiple images - for example the specular map is usually in the alpha channel of the normal map, so if you have *normal.png* and *specular.png* images, you will get a *name_n.dds*with normal information in RGB channels and specular in A channel. You will also get a *name_s.dds* specular texture, which is only used on some meshes.

This tool just generates textures, but that doesn't mean you will see them in game. For example to be able to see parallax mapping, you need parallax enabled meshes (for example 202X textures have some) and ENB or Community Shaders with the Complex Parallax Materials addon. Some textures are only used in specific meshes such as for NPCs; the tool doesn't know this, it generates everything it can.

Why do the images need specific names? 
This tool doesn't just convert various image formats to dds. It sets the dds compression depending on the type texture - is it grayscale, rgb, with cutout alpha or with full alpha. But its main goal is to let you avoid having to work with different data in channels of one image file, such as the previous example of normal+specular map. The tool makes all textures that it can from the available images, based on the table [here (Texture Slots section)](https://wiki.beyondskyrim.org/wiki/Arcane_University:NIF_Data_Format﻿) + terrain parallax and complex materials. Thanks to this, you can easily keep your data in separate images and just combine them at the end. 

Disclaimer: 
This is the first version of a tool made in two days, and I didn't test all possible cases. There might also be some peculiarities with some of the textures that I'm unaware of, as I don't really have modding experience. So please let me know if something doesn't work or if the output is wrong. Especially with the complex materials - the texture generation works, but I'm not sure how to even put it into the game and set up the mesh properly.  

Basic usage:
The most basic way to use this is to drop the .exe to the folder with your images and double-click it. It will create all textures that it can with your images and place them in a folder called *output*. If you want to customize the process, you can open a command line window (type cmd, press enter in the address bar of the File Explorer) and run the program from there. If you open a cmd in the folder with your images, you can enter the path to the program to run it on those images. If you are in a different folder, you can use the -i flag to set the input path manually.

Please read the flags below. You can set the name of the resulting textures, force high quality format, force LE supported formats, enable parallax/complex materials, and customize the input and output directory.

Usage info (help):

```
*> dds_material_creator.exe --help*
Usage: dds_material_creator.exe [-n ] [-h] [-a] [-t] [-c] [-i ] [-o ]

Converts image files to Skyrim textures. The resulting textures are composed and named according to the Skyrim conventions, mipmaps are generated. Input
 files are recognized by file names (without suffix) and all of them are optional: diffuse, normal, specular, glow, skin_tint, height, cubemap, env_mask, inner_diffuse, inner_depth, subsurface, backlight, metallic, glossiness. All textures for which the required images are provided will be generated. Images that combine into one texture must have the same resolution! The common supported formats are png, tif, jpg and bmp. The textures used ingame depend on the meshes' property flags, just use the ones you need
```
For details on supported image formats look at the default features of the image crate ([https://docs.rs/image](https://docs.rs/image)﻿). 
For details on texture composition and names see Texture Slots section at [https://wiki.beyondskyrim.org/wiki/Arcane_University:NIF_Data_Format](https://wiki.beyondskyrim.org/wiki/Arcane_University:NIF_Data_Format)﻿. 
For details on complex parallax textures see [https://modding.wiki/en/skyrim/developers/complex-parallax-materials](https://modding.wiki/en/skyrim/developers/complex-parallax-materials)﻿

```
Options:
  -n, --name        the name of the resulting textures. For example, the normal
                    map file will be named name_n.dds
  -h, --high-quality
                    force diffuse textures to use BC7 instead of BC1 (normals
                    always use BC7). BC7 should better represent subtle changes
                    or gradients, but uses significantly more space
  -a, --archaic-format
                    only use older formats (BC1 and BC3) compatible with Skyrim
                    LE. Only use if you target games that do not support BC4 and
                    BC7
  -t, --terrain-parallax
                    will write height information instead of transparency to the
                    alpha channel of the diffuse texture. Used for parallax on
                    landscape/terrain textures.
  -c, --complex-parallax
                    will write complex parallax information (R: env_mask, G:
                    glossiness, B: metallic, A: height) into the environment
                    map. Used for parallax on object textures.
  -i, --input-dir   specifies the input directory. By default the current
                    working directory is used
  -o, --output-dir  specifies the output directory. By default 'output'
                    directory is created in the input directory
  --help            display usage information
```
