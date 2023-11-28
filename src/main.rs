use argh::FromArgs;

use std::error::Error;
use std::path::{PathBuf};

mod forward;
mod backward;
use forward::run_forward;
use backward::run_backward;

// TODO: implement complex skin material (glossiness in G channel of specular map)

#[derive(FromArgs)]
/// Converts image files to Skyrim textures. The resulting textures are composed and named according to the Skyrim conventions, mipmaps are generated.
/// Input files are recognized by file names (without suffix) and all of them are optional:
/// diffuse, normal, specular, glow, skin_tint, height, cubemap, env_mask, inner_diffuse, inner_depth, subsurface, backlight, metallic, glossiness.
/// All textures for which the required images are provided will be generated. Images that combine into one texture must have the same resolution!
/// The common supported formats are png, tif, jpg and bmp.
/// The textures used ingame depend on the meshes' property flags, just use the ones you need!
/// For details on supported image formats look at the default features of the image crate (https://docs.rs/image).
/// For details on texture composition and names see Texture Slots section at https://wiki.beyondskyrim.org/wiki/Arcane_University:NIF_Data_Format.
/// For details on complex parallax textures see https://modding.wiki/en/skyrim/developers/complex-parallax-materials
struct Args {
    #[argh(option, short = 'n', default = "String::from(\"\")")]
    /// the name of the resulting textures. For example, the normal map file will be named name_n.dds
    pub name: String,
    #[argh(switch, short = 'h')]
    /// force diffuse textures to use BC7 instead of BC1 (normals always use BC7). BC7 should better represent subtle changes or gradients, but uses significantly more space
    pub high_quality: bool,
    #[argh(switch, short = 'a')]
    /// only use older formats (BC1 and BC3) compatible with Skyrim LE. Only use if you target games that do not support BC4 and BC7
    pub archaic_format: bool,
    #[argh(switch, short = 't')]
    /// will write height information instead of transparency to the alpha channel of the diffuse texture. Used for parallax on landscape/terrain textures.
    pub terrain_parallax: bool,
    #[argh(switch, short = 'c')]
    /// will write complex parallax information (R: env_mask, G: glossiness, B: metallic, A: height) into the environment map. Used for parallax on object textures.
    pub complex_parallax: bool,
    #[argh(option, short = 'i')]
    /// specifies the input directory. By default the current working directory is used
    pub input_dir: Option<PathBuf>,
    #[argh(option, short = 'o')]
    /// specifies the output directory. By default 'output' directory is created in the input directory
    pub output_dir: Option<PathBuf>,
    #[argh(switch, short = 'b')]
    /// run the conversion backward (dds -> png). It only splits off alpha channel. Keep in mind that dds is lossy, the lost detail can't be retrieved.
    pub backward: bool,
}


fn main() -> Result<(), Box<dyn Error>> {
    let args: Args = argh::from_env();
    let dir = if let Some(p) = &args.input_dir {
        p.clone()
    } else {
        match std::env::current_dir() {
            Ok(p) => p,
            Err(e) => {
                println!("Critical error, Cannot access the working directory: {}", e);
                println!("You can set input directory with the -i flag.");
                return Ok(());
            }
        }
    };
    println!("Using input directory: {}", dir.display());
    let mut out_dir = if let Some(p) = &args.output_dir {
        p.clone()
    } else {
        dir.join("output")
    };
    if let Err(e) = std::fs::create_dir_all(out_dir.clone()) {
        println!("Error creating output dir: {}", e);
        println!("Will try to save in the input directory.");
        out_dir = dir.clone();
    }
    if args.backward{
        run_backward(&args, &dir, &out_dir);
    }
    else{
        run_forward(&args, &dir, &out_dir);

    }

    Ok(())
}
