use crate::Args;
use image::{DynamicImage, ImageOutputFormat, GrayImage, GenericImage, Luma};
use image_dds::{image_from_dds};
use std::{
    collections::HashMap,
    path::{Path, PathBuf}, fs::File,
};

fn get_dds_file_paths<P>(path: P) -> std::io::Result<HashMap<String, PathBuf>>
where
    P: AsRef<Path> + std::fmt::Debug,
{
    // Get a list of all entries in the folder
    let entries = std::fs::read_dir(path)?;
    // Extract the filenames from the directory entries and store them in a vector
    let file_names: HashMap<String, PathBuf> = entries
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            if path.is_file() && path.extension().is_some() && path.extension()? == "dds" {
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

fn create_images(name: String, path: PathBuf) -> Vec<(String, DynamicImage)> {
    let tex = match image_dds::ddsfile::Dds::read(File::open(path.clone()).unwrap()){
        Ok(t) => t,
        Err(e) => {println!("Error, can't read dds at {}: {}", path.display(), e); return vec![];},
    };
    let img = match image_from_dds(&tex, 0){
        Ok(img) => img,
        Err(e) => {println!("Error, can't tranform dds to image: {}", e); return vec![];},
    };
    let mut res: Vec<(String, DynamicImage)> = vec![];
    let rgb = DynamicImage::ImageRgb8(DynamicImage::ImageRgba8(img.clone()).into_rgb8());
    res.push((name.clone(), rgb));
    if !img.pixels().all(|p| p.0[3] == 255){
        let mut a = GrayImage::new(img.width(), img.height());
        for y in 0..img.height() {
            for x in 0..img.width() {
                let p = img.get_pixel(x, y);
                a.put_pixel(x, y, Luma([p.0[3]])); // set height.r to result.a
            }
        }
        res.push((name + "_alpha", DynamicImage::ImageLuma8(a)));
    }
    return res;
}


pub fn run_backward(args: &Args, in_dir: &PathBuf, out_dir: &PathBuf) {
    let paths = get_dds_file_paths(in_dir).unwrap();
    let mut images = vec![];
    for (name, path) in paths {
        images.extend_from_slice(&create_images(name, path));
    }
    for (name, img) in images {
        let out_path = out_dir.join(args.name.clone() + name.as_str() + ".png");
        println!("Writing: {}", out_path.display());
        let mut file = match File::create(out_path){
            Ok(f) => f,
            Err(e) => {println!("Error, cannot create texture file at {}! {}", out_dir.display(), e); continue;},
        };
        if let Err(e) = img.write_to(&mut file, ImageOutputFormat::Png){
            println!("Error, cannot write into texture file! {}", e);
        }
    }
}

