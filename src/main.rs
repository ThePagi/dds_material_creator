use argh::FromArgs;
use image::{io::Reader as ImageReader, DynamicImage};
use image::{GenericImage, GenericImageView, ImageBuffer};
use image_dds::ddsfile::Dds;
use image_dds::{dds_from_image, dds_from_imagef32, ImageFormat};
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::path::{Path, PathBuf};

#[derive(FromArgs)]
/// Converts image files to Skyrim textures. The resulting textures are composed and named according to the Skyrim conventions.
/// Input files are recognized by file names (without suffix) and all of them are optional:
/// diffuse, normal, specular, glow, skin_tint, height, cubemap, env_mask, inner_diffuse, subsurface, backlight.
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
    /// will write complex parallax information (R: env mask, G: glossy, B: metallic, A: height) into the environment map. Used for parallax on object textures.
    pub complex_parallax: bool,
    #[argh(option, short = 'i')]
    /// specifies the input directory. By default the current working directory is used
    pub input_dir: Option<PathBuf>,
    #[argh(option, short = 'o')]
    /// specifies the output directory. By default 'output' directory is created in the input directory
    pub output_dir: Option<PathBuf>,
}

enum ImageProps {
    Grayscale,
    RGB,
    RGBFullAlpha,
    RGBCutoutAlpha,
    Uncompressed,
}

struct InputImages {
    pub diffuse_alpha: Option<DynamicImage>,
    pub normal: Option<DynamicImage>,
    pub specular: Option<DynamicImage>,
    pub glow: Option<DynamicImage>,
    pub skin_tint: Option<DynamicImage>,
    pub height: Option<DynamicImage>,
    pub cubemap: Option<DynamicImage>,
    pub env_mask: Option<DynamicImage>,
    pub inner_diffuse: Option<DynamicImage>,
    pub subsurface: Option<DynamicImage>,
    pub backlight: Option<DynamicImage>,
}

fn get_file_paths<P>(path: P) -> std::io::Result<HashMap<String, PathBuf>>
where
    P: AsRef<Path> + std::fmt::Debug,
{
    // Get a list of all entries in the folder
    let entries = std::fs::read_dir(path)?;
    // Extract the filenames from the directory entries and store them in a vector
    let file_names: HashMap<String, PathBuf> = entries
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            if path.is_file() {
                Some((
                    Path::new(path.file_name()?)
                        .file_stem()?
                        .to_str()?
                        .to_owned(),
                    path,
                ))
            } else {
                None
            }
        })
        .collect();

    Ok(file_names)
}

fn pick_format(properties: ImageProps, use_old_format: bool, high_quality: bool) -> ImageFormat {
    match use_old_format {
        true => match properties {
            ImageProps::Grayscale => ImageFormat::BC1Unorm,
            ImageProps::RGB => ImageFormat::BC1Unorm,
            ImageProps::RGBFullAlpha => ImageFormat::BC3Unorm,
            ImageProps::RGBCutoutAlpha => ImageFormat::BC1Unorm,
            ImageProps::Uncompressed => ImageFormat::R8G8B8A8Unorm,
        },
        false => match properties {
            ImageProps::Grayscale => ImageFormat::BC4Unorm,
            ImageProps::RGB => {
                if high_quality {
                    ImageFormat::BC7Unorm
                } else {
                    ImageFormat::BC1Unorm
                }
            }
            ImageProps::RGBFullAlpha => ImageFormat::BC7Unorm,
            ImageProps::RGBCutoutAlpha => {
                if high_quality {
                    ImageFormat::BC7Unorm
                } else {
                    ImageFormat::BC1Unorm
                }
            }
            ImageProps::Uncompressed => ImageFormat::R8G8B8A8Unorm,
        },
    }
}

fn load_input_image<P>(path: Option<P>) -> Option<DynamicImage>
where
    P: AsRef<Path> + std::fmt::Debug,
{
    if path.is_none() {
        return None;
    }
    let path = path.unwrap();
    let path_readable = path
        .as_ref()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();
    match ImageReader::open(path) {
        Ok(reader) => match reader.decode() {
            Ok(img) => {
                println!("Found {}, pixel type {:?}.", path_readable, img.color());
                Some(img)
            }
            Err(e) => {
                println!(
                    "Error decoding {}, file will be ignored. Details: {}",
                    path_readable, e
                );
                None
            }
        },
        Err(_) => {
            //println!("Error opening {}: {}", path_readable, e);
            None
        }
    }
}

fn create_textures(
    images: InputImages, args: &Args
) -> Vec<(&'static str, Dds)> {
    let mut textures = Vec::new();

    if let Some(tex) = create_diffuse(&images, args) {
        textures.push(("", tex));
    }
    if let Some(tex) = create_normal(&images, args) {
        textures.push(("_n", tex));
    }
    if let Some(tex) = create_specular(&images, args) {
        textures.push(("_s", tex));
    }

    textures
}

fn create_specular(images: &InputImages, args: &Args) -> Option<Dds> {
    if let Some(img) = &images.specular {
        let mut res = image::RgbaImage::new(img.width(), img.height());
        let props = ImageProps::Grayscale;
        if let Err(e) = res.copy_from(img, 0, 0) {
            println!(
                "Error: Cannot copy from diffuse image to rgba8 texture! {}",
                e
            );
            println!("The format: {:?}", img.color());
            return None;
        }
        let format = pick_format(props, args.archaic_format, args.high_quality);
        Some(
            dds_from_image(
                &res,
                format,
                image_dds::Quality::Slow,
                image_dds::Mipmaps::GeneratedAutomatic,
            )
            .unwrap(),
        )
    } else {
        None
    }
}

fn create_normal(images: &InputImages, args: &Args) -> Option<Dds> {
    if let Some(img) = &images.normal {
        let mut res = image::RgbaImage::new(img.width(), img.height());
        let props = if images.specular.is_some() {
            ImageProps::RGBFullAlpha
        } else if img.color().has_alpha() {
            ImageProps::RGBFullAlpha
        } else {
            ImageProps::RGB
        };
        if let Err(e) = res.copy_from(img, 0, 0) {
            println!(
                "Error: Cannot copy from diffuse image to rgba8 texture! {}",
                e
            );
            println!("The format: {:?}", img.color());
            return None;
        }
        if let Some(spec) = &images.specular {
            for y in 0..spec.height() {
                for x in 0..spec.width() {
                    let p = spec.get_pixel(x, y);
                    res.get_pixel_mut(x, y).0[3] = p.0[0]; // set specular.r to result.a
                }
            }
        }
        let format = pick_format(
            props,
            args.archaic_format,
            true, /* BC1 does badly with normal maps */
        );
        Some(
            dds_from_image(
                &res,
                format,
                image_dds::Quality::Slow,
                image_dds::Mipmaps::GeneratedAutomatic,
            )
            .unwrap(),
        )
    } else {
        None
    }
}

fn determine_image_props(img: &DynamicImage) -> Option<ImageProps> {
    match img.color() {
        image::ColorType::L8 => Some(ImageProps::Grayscale),
        image::ColorType::La8 => Some(ImageProps::Grayscale),
        image::ColorType::Rgb8 => Some(ImageProps::RGB),
        image::ColorType::Rgba8 => Some(
            if img
                .as_rgba8()
                .unwrap()
                .iter()
                .all(|p| *p == u8::MIN || *p == u8::MAX)
            {
                ImageProps::RGBCutoutAlpha
            } else {
                ImageProps::RGBFullAlpha
            },
        ),
        image::ColorType::L16 => Some(ImageProps::Grayscale),
        image::ColorType::La16 => Some(ImageProps::Grayscale),
        image::ColorType::Rgb16 => Some(ImageProps::RGB),
        image::ColorType::Rgba16 => Some(
            if img
                .as_rgba16()
                .unwrap()
                .iter()
                .all(|p| *p == u16::MIN || *p == u16::MAX)
            {
                ImageProps::RGBCutoutAlpha
            } else {
                ImageProps::RGBFullAlpha
            },
        ),
        image::ColorType::Rgb32F => Some(ImageProps::RGB),
        image::ColorType::Rgba32F => Some(ImageProps::RGBFullAlpha),
        _ => {
            println!("Unsupported pixel format {:?}! Skipping...", img.color());
            None
        }
    }
}

fn create_diffuse(images: &InputImages, args: &Args) -> Option<Dds> {
    if let Some(img) = &images.diffuse_alpha {
        let mut res = image::RgbaImage::new(img.width(), img.height());
        let mut props = determine_image_props(img)?;
        if let Err(e) = res.copy_from(img, 0, 0) {
            println!(
                "Error: Cannot copy from diffuse image to rgba8 texture! {}",
                e
            );
            println!("The format: {:?}", img.color());
            return None;
        }
        if args.terrain_parallax{
            if let Some(height) = &images.height{
                props = ImageProps::RGBFullAlpha;
                for y in 0..height.height() {
                    for x in 0..height.width() {
                        let p = height.get_pixel(x, y);
                        res.get_pixel_mut(x, y).0[3] = p.0[0]; // set height.r to result.a
                    }
                }
            }
            else{
                println!("Error: Terrain parallax selected, but no height image supplied!");
            }

        }
        let format = pick_format(props, args.archaic_format, args.high_quality);
        Some(
            dds_from_image(
                &res,
                format,
                image_dds::Quality::Slow,
                image_dds::Mipmaps::GeneratedAutomatic,
            )
            .unwrap(),
        )
    } else {
        None
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Args = argh::from_env();
    let dir = if let Some(p) = &args.input_dir {
        p.clone()
    } else {
        std::env::current_dir()?
    };
    let hq = false;
    let old_format = false;
    let output_name = "mytex".to_owned();
    println!("Using input directory: {}", dir.display());
    let fnames = get_file_paths(dir.as_path())?;
    let images = InputImages {
        diffuse_alpha: load_input_image(fnames.get("diffuse")), 
        normal: load_input_image(fnames.get("normal")),
        specular: load_input_image(fnames.get("specular")),
        glow: load_input_image(fnames.get("glow")),
        skin_tint: load_input_image(fnames.get("skin_tint")),
        height: load_input_image(fnames.get("height")),
        cubemap: load_input_image(fnames.get("cubemap")),
        env_mask: load_input_image(fnames.get("env_mask")),
        inner_diffuse: load_input_image(fnames.get("inner_diffuse")),
        subsurface: load_input_image(fnames.get("subsurface")),
        backlight: load_input_image(fnames.get("backlight")),
    };
    let mut out_dir = if let Some(p) = &args.output_dir {
        p.clone()
    } else {
        dir.join("output")
    };
    if let Err(e) = std::fs::create_dir_all(out_dir.clone()) {
        println!("Error creating output dir: {}", e);
        println!("Will try to save in the input directory.");
        out_dir = dir;
    }

    let textures = create_textures(images, &args);
    for (suffix, tex) in textures {
        let out_path = out_dir.join(output_name.clone() + suffix + ".dds");
        println!("Writing: {}", out_path.display());
        let mut file = File::create(out_path)?;
        tex.write(&mut file)?;
    }

    Ok(())
}
