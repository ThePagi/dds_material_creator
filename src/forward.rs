use image::{io::Reader as ImageReader, DynamicImage};
use image::{GenericImage, GenericImageView, Rgba};
use image_dds::ddsfile::Dds;
use image_dds::{dds_from_image, ImageFormat};
use std::collections::HashMap;
use std::fs::File;
use std::path::{Path, PathBuf};
use crate::Args;
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
    pub inner_depth: Option<DynamicImage>,
    pub subsurface: Option<DynamicImage>,
    pub backlight: Option<DynamicImage>,
    pub metallic: Option<DynamicImage>,
    pub glossiness: Option<DynamicImage>,
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

fn create_textures(images: InputImages, args: &Args) -> Vec<(&'static str, Dds)> {
    let mut textures = Vec::new();

    if let Some(tex) = create_diffuse(&images, args) {
        textures.push(("", tex));
    }
    if let Some(tex) = create_normal(&images, args) {
        textures.push(("_n", tex));
    }
    if let Some(tex) = create_generic(&images.glow, ImageProps::RGB, args) {
        textures.push(("_g", tex));
    }
    if let Some(tex) = create_generic(&images.skin_tint, ImageProps::RGB, args) {
        textures.push(("_sk", tex));
    }
    if let Some(tex) = create_generic(&images.height, ImageProps::Grayscale, args) {
        textures.push(("_p", tex));
    }
    if let Some(tex) = create_generic(&images.cubemap, ImageProps::Grayscale, args) {
        textures.push(("_e", tex));
    }
    if args.complex_parallax {
        if let Some(tex) = create_complex_parallax(&images, args) {
            textures.push(("_m", tex));
        }
    } else if let Some(tex) = create_generic(&images.env_mask, ImageProps::Grayscale, args) {
        textures.push(("_m", tex));
    }
    if let Some(tex) = create_inner(&images, args) {
        textures.push(("_i", tex));
    }
    if let Some(tex) = create_generic(&images.subsurface, ImageProps::RGB, args) {
        textures.push(("_subsurface", tex));
    }
    if let Some(tex) = create_generic(&images.specular, ImageProps::Grayscale, args) {
        textures.push(("_s", tex));
    }
    if let Some(tex) = create_generic(&images.backlight, ImageProps::RGB, args) {
        textures.push(("_b", tex));
    }
    textures
}

fn create_complex_parallax(images: &InputImages, args: &Args) -> Option<Dds> {
    let (w, h) = {
        if let Some(img) = &images.env_mask{
            (img.width(), img.height())
        }
        else if let Some(img) = &images.glossiness{
            (img.width(), img.height())
        }
        else if let Some(img) = &images.metallic{
            (img.width(), img.height())
        }
        else if let Some(img) = &images.height{
            (img.width(), img.height())
        }
        else{
            println!("Error: Complex parallax material selected, but none of the images (R: env_mask, G: glossiness, B: metallic, A: height) available!");
            return None
        }
    };
    let mut res = image::RgbaImage::new(w, h);
    for y in 0..res.height() {
        for x in 0..res.width() {
            res.put_pixel(x, y, Rgba([0, 5, 0, 255]));
        }
    }
    if let Some(img) = &images.env_mask{
        for y in 0..img.height() {
            for x in 0..img.width() {
                let p = img.get_pixel(x, y);
                res.get_pixel_mut(x, y).0[0] = p.0[0];
            }
        }
    }
    if let Some(img) = &images.glossiness{
        for y in 0..img.height() {
            for x in 0..img.width() {
                let p = img.get_pixel(x, y);
                res.get_pixel_mut(x, y).0[1] = p.0[0];
            }
        }
    }
    if let Some(img) = &images.metallic{
        for y in 0..img.height() {
            for x in 0..img.width() {
                let p = img.get_pixel(x, y);
                res.get_pixel_mut(x, y).0[2] = p.0[0];
            }
        }
    }
    if let Some(img) = &images.height{
        for y in 0..img.height() {
            for x in 0..img.width() {
                let p = img.get_pixel(x, y);
                res.get_pixel_mut(x, y).0[3] = p.0[0];
            }
        }
    }
    Some(
        dds_from_image(
            &res,
            pick_format(ImageProps::RGBFullAlpha, args.archaic_format, args.high_quality),
            image_dds::Quality::Slow,
            image_dds::Mipmaps::GeneratedAutomatic,
        )
        .unwrap(),
    )
}

fn create_generic(image: &Option<DynamicImage>, props: ImageProps, args: &Args) -> Option<Dds> {
    if let Some(img) = image {
        let mut res = image::RgbaImage::new(img.width(), img.height());
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

fn create_inner(images: &InputImages, args: &Args) -> Option<Dds> {
    if let Some(img) = &images.inner_diffuse {
        let mut res = image::RgbaImage::new(img.width(), img.height());
        let props = if images.inner_depth.is_some() {
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
        if let Some(depth) = &images.inner_depth {
            for y in 0..depth.height() {
                for x in 0..depth.width() {
                    let p = depth.get_pixel(x, y);
                    res.get_pixel_mut(x, y).0[3] = p.0[0]; // set inner_depth.r to result.a
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
        if args.terrain_parallax {
            if let Some(height) = &images.height {
                props = ImageProps::RGBFullAlpha;
                for y in 0..height.height() {
                    for x in 0..height.width() {
                        let p = height.get_pixel(x, y);
                        res.get_pixel_mut(x, y).0[3] = p.0[0]; // set height.r to result.a
                    }
                }
            } else {
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

pub fn run_forward(args: &Args, in_dir: &PathBuf, out_dir: &PathBuf){
    let fnames = match get_file_paths(in_dir.as_path()){
        Ok(fnames) => fnames,
        Err(e) => {println!("Critical error, cannot get file paths: {}", e); return;},
    };
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
        inner_depth: load_input_image(fnames.get("inner_depth")),
        subsurface: load_input_image(fnames.get("subsurface")),
        backlight: load_input_image(fnames.get("backlight")),
        metallic: load_input_image(fnames.get("metallic")),
        glossiness: load_input_image(fnames.get("glossiness")),
    };

    let textures = create_textures(images, &args);
    for (suffix, tex) in textures {
        let out_path = out_dir.join(args.name.clone() + suffix + ".dds");
        println!("Writing: {}", out_path.display());
        let mut file = match File::create(out_path){
            Ok(f) => f,
            Err(e) => {println!("Error, cannot create texture file at {}! {}", out_dir.display(), e); continue;},
        };
        if let Err(e) = tex.write(&mut file){
            println!("Error, cannot write into texture file! {}", e);
        }
    }
}