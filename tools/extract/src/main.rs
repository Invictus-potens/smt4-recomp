//! Extracts Shin Megami Tensei IV's assets from your own decrypted ROM and
//! converts the ones it can to common formats. See docs/ASSETS.md.
//!
//!     smt4-extract <rom> <out dir> [--only raw,textures,models,text,audio,video]
//!
//! What it writes under <out dir>:
//!
//! - romfs/      every file of the game's RomFS, as it is
//! - textures/   .stex and .cmp textures as PNG
//! - models/     .bam and .bcmdl models as plain CGFX (.bcres), for
//!   SPICA, Ohana3DS or Noesis
//! - text/       .mbm messages as UTF-8 text
//! - audio/      .bcstm music and .bcwav sounds as WAV, with vgmstream-cli
//! - video/      .moflex videos as MP4, with ffmpeg
//!
//! Nothing it writes is meant to be shared: it's all the game's.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

use rayon::prelude::*;
use zakuro_fs::romfs::{FileEntry, RomFs};
use zakuro_fs::Title;
use zakuro_gpu::texture::{sample_texel, TextureFormat};

const TITLE_ID: u64 = 0x0004_0000_000E_5C00;
const KINDS: [&str; 6] = ["raw", "textures", "models", "text", "audio", "video"];

struct Asset {
    /// the path in the RomFS, without the leading slash
    path: String,
    entry: FileEntry,
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut positional = Vec::new();
    let mut only: BTreeSet<String> = KINDS.iter().map(|k| k.to_string()).collect();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--only" if i + 1 < args.len() => {
                only = args[i + 1].split(',').map(|k| k.trim().to_owned()).collect();
                if let Some(bad) = only.iter().find(|k| !KINDS.contains(&k.as_str())) {
                    eprintln!("unknown kind {bad}, the kinds are {}", KINDS.join(","));
                    return ExitCode::FAILURE;
                }
                i += 2;
            }
            "-h" | "--help" => {
                println!("usage: smt4-extract <rom> <out dir> [--only {}]", KINDS.join(","));
                return ExitCode::SUCCESS;
            }
            other => {
                positional.push(other.to_owned());
                i += 1;
            }
        }
    }
    let [rom, out] = positional.as_slice() else {
        eprintln!("usage: smt4-extract <rom> <out dir> [--only {}]", KINDS.join(","));
        return ExitCode::FAILURE;
    };

    let title = match Title::load(rom) {
        Ok(title) => title,
        Err(error) => {
            eprintln!("could not read {rom}: {error} (it has to be a decrypted .3ds/.cci/.cxi)");
            return ExitCode::FAILURE;
        }
    };
    if title.program_id() != TITLE_ID {
        eprintln!("{rom} is title {:016X}, this is for Shin Megami Tensei IV (USA), {TITLE_ID:016X}", title.program_id());
        return ExitCode::FAILURE;
    }
    let Some(romfs) = &title.romfs else {
        eprintln!("{rom} has no RomFS this can read");
        return ExitCode::FAILURE;
    };
    let assets = list(romfs);
    let total: u64 = assets.iter().map(|a| a.entry.data_size).sum();
    println!("{} files, {} MiB", assets.len(), total >> 20);

    let out = PathBuf::from(out);
    let read = |asset: &Asset| -> Vec<u8> {
        title.read_romfs(&asset.entry, 0, asset.entry.data_size as usize).map(<[u8]>::to_vec).unwrap_or_default()
    };
    let with_ext = |ext: &[&str]| -> Vec<&Asset> {
        assets.iter().filter(|a| ext.iter().any(|e| a.path.to_ascii_lowercase().ends_with(e))).collect()
    };
    let mut failed = 0;

    if only.contains("raw") {
        failed += run("raw files", &assets.iter().collect::<Vec<_>>(), |asset| {
            write(&out.join("romfs").join(&asset.path), &read(asset))
        });
    }
    if only.contains("textures") {
        failed += run("textures", &with_ext(&[".stex", ".cmp"]), |asset| {
            let data = read(asset);
            let stex = if data.first() == Some(&0x11) { lz11(&data)? } else { data };
            let png = stex_to_png(&stex)?;
            // a few .cmp files sit next to an .stex of the same name
            let ext = if asset.path.to_ascii_lowercase().ends_with(".cmp") { "cmp.png" } else { "png" };
            write(&out.join("textures").join(&asset.path).with_extension(ext), &png)
        });
    }
    if only.contains("models") {
        failed += run("models", &with_ext(&[".bam", ".bcmdl"]), |asset| {
            let data = read(asset);
            // a few .bam files are an empty ATBC header, with no model
            if data.len() <= 0x20 {
                return Ok(());
            }
            let cgfx = cgfx(&data)?;
            write(&out.join("models").join(&asset.path).with_extension("bcres"), cgfx)
        });
    }
    if only.contains("text") {
        failed += run("text", &with_ext(&[".mbm"]), |asset| {
            let text = mbm_to_text(&read(asset))?;
            write(&out.join("text").join(&asset.path).with_extension("txt"), text.as_bytes())
        });
    }
    if only.contains("audio") {
        failed += external("audio", "vgmstream-cli", &with_ext(&[".bcstm", ".bcwav"]), &out, "wav", &read, |input, output| {
            vec!["-o".into(), output.into(), input.into()]
        });
    }
    if only.contains("video") {
        failed += external("video", "ffmpeg", &with_ext(&[".moflex"]), &out, "mp4", &read, |input, output| {
            ["-hide_banner", "-loglevel", "error", "-y", "-i"]
                .iter()
                .map(|s| s.to_string())
                .chain([input.to_owned(), output.to_owned()])
                .collect()
        });
    }

    println!("\ndone, in {}", out.display());
    if failed > 0 {
        println!("{failed} files could not be converted, see above");
    }
    ExitCode::SUCCESS
}

/// every file in the RomFS, with its path
fn list(romfs: &RomFs) -> Vec<Asset> {
    let mut out = Vec::new();
    let Ok(root) = romfs.root() else { return out };
    let mut stack = vec![(root, String::new())];
    while let Some((dir, prefix)) = stack.pop() {
        for (_, file) in romfs.files(&dir) {
            out.push(Asset { path: format!("{prefix}{}", file.name), entry: file });
        }
        for (_, sub) in romfs.subdirs(&dir) {
            let path = if sub.name.is_empty() { prefix.clone() } else { format!("{prefix}{}/", sub.name) };
            stack.push((sub, path));
        }
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
    out
}

/// runs convert over assets in parallel, printing progress and failures,
/// and returns how many failed
fn run(what: &str, assets: &[&Asset], convert: impl Fn(&Asset) -> Result<(), String> + Sync) -> usize {
    println!("\n== {what}: {} files", assets.len());
    let done = AtomicUsize::new(0);
    let failed = AtomicUsize::new(0);
    let step = (assets.len() / 10).max(1);
    assets.par_iter().for_each(|asset| {
        if let Err(error) = convert(asset) {
            failed.fetch_add(1, Ordering::Relaxed);
            eprintln!("  {}: {error}", asset.path);
        }
        let n = done.fetch_add(1, Ordering::Relaxed) + 1;
        if n.is_multiple_of(step) || n == assets.len() {
            println!("  {n} of {}", assets.len());
        }
    });
    failed.into_inner()
}

/// converts assets with an external program, when it's on PATH
fn external(
    what: &str,
    program: &str,
    assets: &[&Asset],
    out: &Path,
    ext: &str,
    read: &(dyn Fn(&Asset) -> Vec<u8> + Sync),
    arguments: impl Fn(&str, &str) -> Vec<String> + Sync,
) -> usize {
    let found = Command::new(program).arg("-h").stdout(Stdio::null()).stderr(Stdio::null()).status().is_ok();
    if !found {
        println!("\n== {what}: skipped, {program} isn't on PATH (the files are in romfs/ as they are)");
        return 0;
    }
    let scratch = out.join(".scratch");
    let failed = run(what, assets, |asset| {
        // the programs read a file, so the asset goes to a scratch one first
        let name = asset.path.replace(['/', '\\'], "_");
        let input = scratch.join(&name);
        write(&input, &read(asset))?;
        let output = out.join(what).join(&asset.path).with_extension(ext);
        if let Some(dir) = output.parent() {
            fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let status = Command::new(program)
            .args(arguments(&input.to_string_lossy(), &output.to_string_lossy()))
            .stdout(Stdio::null())
            .status()
            .map_err(|e| e.to_string())?;
        let _ = fs::remove_file(&input);
        if status.success() { Ok(()) } else { Err(format!("{program} failed")) }
    });
    let _ = fs::remove_dir_all(&scratch);
    failed
}

fn write(path: &Path, data: &[u8]) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    fs::write(path, data).map_err(|e| format!("{}: {e}", path.display()))
}

fn u32_at(data: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes(data.get(offset..offset + 4)?.try_into().ok()?))
}

/// LZ11, the compression .cmp files use: a 0x11 byte, the decompressed size,
/// then flag bytes each saying which of the next 8 items are copies.
fn lz11(data: &[u8]) -> Result<Vec<u8>, String> {
    let header = u32_at(data, 0).ok_or("too short")?;
    let (mut size, mut pos) = ((header >> 8) as usize, 4);
    if size == 0 {
        size = u32_at(data, 4).ok_or("too short")? as usize;
        pos = 8;
    }
    let mut out = Vec::with_capacity(size);
    let byte = |pos: &mut usize| -> Result<usize, String> {
        let b = *data.get(*pos).ok_or("ends early")?;
        *pos += 1;
        Ok(b as usize)
    };
    while out.len() < size {
        let flags = byte(&mut pos)?;
        for bit in (0..8).rev() {
            if out.len() >= size {
                break;
            }
            if flags & (1 << bit) == 0 {
                out.push(byte(&mut pos)? as u8);
                continue;
            }
            let b0 = byte(&mut pos)?;
            let (length, distance) = match b0 >> 4 {
                0 => {
                    let (b1, b2) = (byte(&mut pos)?, byte(&mut pos)?);
                    (((b0 & 0xF) << 4 | b1 >> 4) + 0x11, ((b1 & 0xF) << 8 | b2) + 1)
                }
                1 => {
                    let (b1, b2, b3) = (byte(&mut pos)?, byte(&mut pos)?, byte(&mut pos)?);
                    (((b0 & 0xF) << 12 | b1 << 4 | b2 >> 4) + 0x111, ((b2 & 0xF) << 8 | b3) + 1)
                }
                n => {
                    let b1 = byte(&mut pos)?;
                    (n + 1, ((b0 & 0xF) << 8 | b1) + 1)
                }
            };
            if distance > out.len() {
                return Err("copies from before the start".into());
            }
            for _ in 0..length {
                out.push(out[out.len() - distance]);
            }
        }
    }
    out.truncate(size);
    Ok(out)
}

/// Atlus's STEX texture: "STEX", 0, the GL target, width, height, GL type,
/// GL format, data size, data offset, then the texture in the PICA200's own
/// tiled layout, which Zakuro decodes.
fn stex_to_png(stex: &[u8]) -> Result<Vec<u8>, String> {
    if stex.get(0..4) != Some(b"STEX") {
        return Err("not an STEX texture".into());
    }
    let field = |i: usize| u32_at(stex, i * 4).ok_or("header too short");
    let (width, height, gl_type, gl_format, offset) = (field(3)?, field(4)?, field(5)?, field(6)?, field(8)?);
    let format = texture_format(gl_type, gl_format).ok_or(format!("unknown format {gl_format:#X}/{gl_type:#X}"))?;
    // the size in the header is sometimes bigger than the data (the space
    // set aside for it, it seems), so it's worked out from the format
    let needed = (width as u64 * height as u64 * format.bits_per_pixel() as u64 / 8) as u32;
    let data = stex.get(offset as usize..(offset + needed) as usize).ok_or("data past the end")?;
    if width == 0 || height == 0 || !width.is_multiple_of(8) || !height.is_multiple_of(8) {
        return Err(format!("odd size {width}x{height}"));
    }
    let mut pixels = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            pixels.extend_from_slice(&sample_texel(data, format, width, x, y));
        }
    }
    let mut png_bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut png_bytes, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
        writer.write_image_data(&pixels).map_err(|e| e.to_string())?;
    }
    Ok(png_bytes)
}

/// the PICA200 format for a GL type and format pair, as the CTR SDK's GL
/// names them (GL_RGBA_NATIVE_DMP and so on)
fn texture_format(gl_type: u32, gl_format: u32) -> Option<TextureFormat> {
    const UNSIGNED_BYTE: u32 = 0x1401;
    const SHORT_4444: u32 = 0x8033;
    const SHORT_5551: u32 = 0x8034;
    const SHORT_565: u32 = 0x8363;
    const BYTE_44: u32 = 0x6760;
    const BITS_4: u32 = 0x6761;
    Some(match (gl_format, gl_type) {
        (0x6752, UNSIGNED_BYTE) => TextureFormat::Rgba8,
        (0x6752, SHORT_5551) => TextureFormat::Rgba5551,
        (0x6752, SHORT_4444) => TextureFormat::Rgba4,
        (0x6754, UNSIGNED_BYTE) => TextureFormat::Rgb8,
        (0x6754, SHORT_565) => TextureFormat::Rgb565,
        (0x6756, UNSIGNED_BYTE) => TextureFormat::A8,
        (0x6756, BITS_4) => TextureFormat::A4,
        (0x6757, UNSIGNED_BYTE) => TextureFormat::L8,
        (0x6757, BITS_4) => TextureFormat::L4,
        (0x6758, UNSIGNED_BYTE) => TextureFormat::La8,
        (0x6758, BYTE_44) => TextureFormat::La4,
        (0x675A, _) => TextureFormat::Etc1,
        (0x675B, _) => TextureFormat::Etc1A4,
        _ => return None,
    })
}

/// the CGFX inside a model: .bcmdl files are one, .bam ones have an Atlus
/// "ATBC" header in front of it
fn cgfx(data: &[u8]) -> Result<&[u8], String> {
    let start = data.windows(4).position(|w| w == b"CGFX").ok_or("no CGFX inside")?;
    let size = u32_at(data, start + 0x0C).ok_or("CGFX header too short")? as usize;
    data.get(start..start + size).ok_or_else(|| "CGFX runs past the end".to_owned())
}

/// MSG2 messages: a table of (id, size, offset) after the header, each
/// message Shift-JIS text with F8 control codes, ending in FF FF
fn mbm_to_text(data: &[u8]) -> Result<String, String> {
    if data.get(4..8) != Some(b"MSG2") {
        return Err("not an MSG2 file".into());
    }
    let count = u32_at(data, 0x10).ok_or("header too short")? as usize;
    let table = u32_at(data, 0x14).ok_or("header too short")? as usize;
    let mut out = String::new();
    for i in 0..count {
        let entry = table + i * 0x10;
        let (Some(id), Some(size), Some(offset)) = (u32_at(data, entry), u32_at(data, entry + 4), u32_at(data, entry + 8)) else {
            break;
        };
        if size == 0 {
            continue;
        }
        let message = data.get(offset as usize..(offset + size) as usize).ok_or(format!("message {id} past the end"))?;
        out.push_str(&format!("[{id}]\n{}\n\n", decode_message(message)));
    }
    Ok(out)
}

/// one message, as text: Shift-JIS, with F8 control codes and FF FF at the
/// end. The codes written out are the ones the game uses most:
///
/// - `F8 01` a new line, `F8 02` waits for a button before the next box
/// - `F8 12 <name> 00 00` who speaks, shown as {speaker Name}
/// - `F8 13 <u16> <u16> <u32>` the voice clip, shown as {voice id}
/// - anything else as {XX args}, with one 16-bit argument unless the table
///   in code_arguments says otherwise
fn decode_message(bytes: &[u8]) -> String {
    let mut out = String::new();
    let mut text = Vec::new();
    let flush = |text: &mut Vec<u8>, out: &mut String| {
        if !text.is_empty() {
            let (decoded, _, _) = encoding_rs::SHIFT_JIS.decode(text);
            // the game writes Latin text in full-width letters
            out.extend(decoded.chars().map(half_width));
            text.clear();
        }
    };
    let u16_at = |i: usize| bytes.get(i..i + 2).map(|b| u16::from_le_bytes([b[0], b[1]])).unwrap_or(0);
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            0xFF if bytes.get(i + 1) == Some(&0xFF) => {
                flush(&mut text, &mut out);
                i += 2;
            }
            0xF8 => {
                flush(&mut text, &mut out);
                let code = bytes.get(i + 1).copied().unwrap_or(0);
                i += 2;
                match code {
                    0x01 => out.push('\n'),
                    0x02 => out.push_str("{next}"),
                    0x12 => {
                        let end = (i..bytes.len().saturating_sub(1)).find(|&j| bytes[j] == 0 && bytes[j + 1] == 0).unwrap_or(bytes.len());
                        let (name, _, _) = encoding_rs::SHIFT_JIS.decode(&bytes[i..end]);
                        out.push_str(&format!("{{speaker {}}}\n", name.chars().map(half_width).collect::<String>()));
                        i = end + 2;
                    }
                    0x13 => {
                        let voice = u32_at(bytes, i + 4).unwrap_or(0);
                        out.push_str(&format!("{{voice {voice}}}"));
                        i += 8;
                    }
                    _ => {
                        out.push_str(&format!("{{{code:02X}"));
                        for _ in 0..code_arguments(code) {
                            out.push_str(&format!(" {}", u16_at(i)));
                            i += 2;
                        }
                        out.push('}');
                    }
                }
            }
            0x00 => i += 1,
            byte => {
                // a Shift-JIS lead byte takes the next one with it
                let two = matches!(byte, 0x81..=0x9F | 0xE0..=0xEF);
                text.push(byte);
                if two && i + 1 < bytes.len() {
                    text.push(bytes[i + 1]);
                    i += 1;
                }
                i += 1;
            }
        }
    }
    flush(&mut text, &mut out);
    out.trim_end_matches('\n').to_owned()
}

/// how many 16-bit arguments a control code takes, worked out from the
/// game's files
fn code_arguments(code: u8) -> usize {
    match code {
        0x7B => 4,
        0x14 | 0x7C => 2,
        _ => 1,
    }
}

fn half_width(c: char) -> char {
    match c {
        '\u{3000}' => ' ',
        '\u{FF01}'..='\u{FF5E}' => char::from_u32(c as u32 - 0xFEE0).unwrap_or(c),
        _ => c,
    }
}
