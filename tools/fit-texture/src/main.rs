//! Makes a texture pack picture from your own image, to take the place of one
//! of the game's textures. See docs/MODDING.md.
//!
//!     cargo fit-texture <extracted PNG> <your image> [--scale 2] [--fill] [--out <folder>]
//!
//! (an alias, in .cargo/config.toml, for `cargo run --release --manifest-path
//! tools/fit-texture/Cargo.toml --`).

use std::path::{Path, PathBuf};

use image::imageops::{self, FilterType};
use image::RgbaImage;

const TITLE_ID: &str = "00040000000E5C00";

const USAGE: &str = "\
Makes a texture pack picture from your own image, to take the place of one of
the game's textures. See docs/MODDING.md.

usage: cargo fit-texture <extracted PNG> <your image> [--scale 2] [--fill] [--out <folder>]

<extracted PNG> is the texture to replace, as tools/extract wrote it (under
<assets>/textures/). Your image (PNG, JPEG, WebP, BMP or GIF) is fitted into
the area the original's visible pixels take up (a character inside a mostly
transparent texture stays where and as big as the game expects it), keeping
its proportions, centered across and resting on the bottom of that area, with
the rest left transparent. --fill covers the whole texture instead, for ones
with no transparent space, like icons and backgrounds.

The picture is --scale times the original's size (2 by default, up to 8), for
more detail than the game's own, and is saved under the name Zakuro looks for,
from the pack-names.txt next to the extracted textures, in Zakuro's mods
folder for the game (or in --out). Restart the game to see it.";

fn fail(text: &str) -> ! {
    eprintln!("error: {text}");
    std::process::exit(1);
}

/// a box: x, y, width, height
type Area = (u32, u32, u32, u32);

/// the box the visible pixels take up: an alpha over 16, as FFmpeg's
/// `alphaextract,bbox=min_val=16` the Python version used
fn visible_area(image: &RgbaImage) -> Option<Area> {
    let mut found: Option<(u32, u32, u32, u32)> = None;
    for (x, y, pixel) in image.enumerate_pixels() {
        if pixel[3] > 16 {
            let (x1, y1, x2, y2) = found.unwrap_or((x, y, x, y));
            found = Some((x1.min(x), y1.min(y), x2.max(x), y2.max(y)));
        }
    }
    found.map(|(x1, y1, x2, y2)| (x1, y1, x2 - x1 + 1, y2 - y1 + 1))
}

/// round(a * b / c), as FFmpeg's av_rescale
fn rescale(a: u32, b: u32, c: u32) -> u32 {
    ((a as u64 * b as u64 + c as u64 / 2) / c as u64).max(1) as u32
}

/// your image's size scaled to fit inside (cover = false) or cover
/// (cover = true) w x h, keeping its proportions
fn fit_size(image: (u32, u32), w: u32, h: u32, cover: bool) -> (u32, u32) {
    let (iw, ih) = image;
    let (tw, th) = (rescale(h, iw, ih), rescale(w, ih, iw));
    if cover {
        (w.max(tw), h.max(th))
    } else {
        (w.min(tw), h.min(th))
    }
}

/// the picture: your image fitted into area, scaled, on a transparent canvas
fn compose(yours: &RgbaImage, size: (u32, u32), area: Area, cover: bool, scale: u32) -> RgbaImage {
    let (x, y, w, h) = (area.0 * scale, area.1 * scale, area.2 * scale, area.3 * scale);
    let (sw, sh) = fit_size(yours.dimensions(), w, h, cover);
    let mut scaled = imageops::resize(yours, sw, sh, FilterType::Lanczos3);
    // what's over is cropped, keeping the middle
    let (cw, ch) = (sw.min(w), sh.min(h));
    if (cw, ch) != (sw, sh) {
        scaled = imageops::crop_imm(&scaled, (sw - cw) / 2, (sh - ch) / 2, cw, ch).to_image();
    }
    let mut canvas = RgbaImage::new(size.0 * scale, size.1 * scale);
    // centered across, resting on the bottom
    imageops::overlay(&mut canvas, &scaled, (x + (w - cw) / 2) as i64, (y + h - ch) as i64);
    canvas
}

/// the picture's name, from the pack-names.txt above the extracted PNG
fn pack_name(extracted: &Path) -> String {
    let extracted = std::path::absolute(extracted).unwrap_or_else(|_| extracted.to_path_buf());
    for folder in extracted.ancestors().skip(1) {
        let names = folder.join("pack-names.txt");
        if !names.is_file() {
            continue;
        }
        let relative: Vec<String> = extracted
            .strip_prefix(folder)
            .unwrap()
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect();
        let relative = relative.join("/");
        let text = std::fs::read_to_string(&names).unwrap_or_else(|e| fail(&format!("could not read {}: {e}", names.display())));
        for line in text.lines() {
            if let Some((path, name)) = line.split_once('\t') {
                if path == relative {
                    return name.split('\t').next().unwrap_or(name).to_owned();
                }
            }
        }
        fail(&format!("{relative} is not in {}; extract the textures again with tools/extract", names.display()));
    }
    fail(&format!("no pack-names.txt above {}; extract the textures with tools/extract", extracted.display()))
}

fn mods_folder() -> PathBuf {
    let env = |key: &str| std::env::var_os(key).filter(|v| !v.is_empty()).map(PathBuf::from);
    let home = || env("HOME").unwrap_or_else(|| PathBuf::from("."));
    let base = if cfg!(windows) {
        env("APPDATA").unwrap_or_else(|| fail("APPDATA is not set")).join("zakuro")
    } else if cfg!(target_os = "macos") {
        home().join("Library").join("Application Support").join("zakuro")
    } else {
        env("XDG_DATA_HOME").unwrap_or_else(|| home().join(".local").join("share")).join("zakuro")
    };
    base.join("mods").join(TITLE_ID).join("textures")
}

fn open(path: &Path) -> RgbaImage {
    image::open(path).unwrap_or_else(|e| fail(&format!("could not read {}: {e}", path.display()))).to_rgba8()
}

fn main() {
    let mut paths: Vec<PathBuf> = Vec::new();
    let (mut scale, mut fill, mut out) = (2u32, false, None::<PathBuf>);
    let mut args = std::env::args_os().skip(1);
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("-h" | "--help") => {
                println!("{USAGE}");
                return;
            }
            Some("--fill") => fill = true,
            Some("--scale") => {
                let value = args.next().and_then(|v| v.to_str()?.parse().ok());
                scale = value.unwrap_or_else(|| fail("--scale needs a number, 1 to 8"));
            }
            Some("--out") => out = Some(args.next().map(PathBuf::from).unwrap_or_else(|| fail("--out needs a folder"))),
            Some(flag) if flag.starts_with("--") => fail(&format!("unknown option {flag}\n\n{USAGE}")),
            _ => paths.push(PathBuf::from(arg)),
        }
    }
    let [extracted, yours] = &paths[..] else { fail(&format!("needs the extracted PNG and your image\n\n{USAGE}")) };
    for path in [extracted, yours] {
        if !path.is_file() {
            fail(&format!("{} not found", path.display()));
        }
    }
    let scale = scale.clamp(1, 8);
    let name = pack_name(extracted);
    let original = open(extracted);
    let size = original.dimensions();
    let visible = if fill { None } else { visible_area(&original) };
    let (area, cover) = match visible {
        Some(area) if (area.2, area.3) != size => (area, false), // fits inside, leaving the rest transparent
        _ => ((0, 0, size.0, size.1), true),                     // covers it, cropping what's over
    };
    let picture = compose(&open(yours), size, area, cover, scale);

    let out_dir = out.unwrap_or_else(mods_folder);
    std::fs::create_dir_all(&out_dir).unwrap_or_else(|e| fail(&format!("could not make {}: {e}", out_dir.display())));
    let target = out_dir.join(&name);
    picture
        .save_with_format(&target, image::ImageFormat::Png)
        .unwrap_or_else(|e| fail(&format!("could not save {}: {e}", target.display())));
    let kind = if cover {
        "the whole texture".to_owned()
    } else {
        format!("its visible area, {}x{} at {},{}", area.2, area.3, area.0, area.1)
    };
    let yours_name = yours.file_name().unwrap_or(yours.as_os_str()).to_string_lossy();
    println!("{yours_name} fitted into {kind}, at {}x{}", size.0 * scale, size.1 * scale);
    println!("saved as {}", target.display());
    println!("restart the game to see it");
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    #[test]
    fn finds_the_visible_area() {
        let mut texture = RgbaImage::new(8, 8);
        assert_eq!(visible_area(&texture), None);
        texture.put_pixel(2, 3, Rgba([0, 0, 0, 255]));
        texture.put_pixel(5, 7, Rgba([0, 0, 0, 17]));
        texture.put_pixel(7, 0, Rgba([0, 0, 0, 16])); // too faint to count
        assert_eq!(visible_area(&texture), Some((2, 3, 4, 5)));
    }

    #[test]
    fn fits_and_covers() {
        // a wide picture in a tall box: as wide as the box, or as tall
        assert_eq!(fit_size((200, 100), 50, 80, false), (50, 25));
        assert_eq!(fit_size((200, 100), 50, 80, true), (160, 80));
    }

    #[test]
    fn rests_on_the_bottom_of_the_area() {
        let yours = RgbaImage::from_pixel(10, 10, Rgba([255, 0, 0, 255]));
        // a 4x8 area at 2,0 of an 8x8 texture, twice the size
        let picture = compose(&yours, (8, 8), (2, 0, 4, 8), false, 2);
        assert_eq!(picture.dimensions(), (16, 16));
        assert_eq!(visible_area(&picture), Some((4, 8, 8, 8)));
        // covering: the whole canvas
        let picture = compose(&yours, (8, 4), (0, 0, 8, 4), true, 1);
        assert_eq!(visible_area(&picture), Some((0, 0, 8, 4)));
    }
}
