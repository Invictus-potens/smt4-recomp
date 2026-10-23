//! Patches 3dsrecomp's generated C at sites that aren't function entries, so a
//! RECOMP_OVERRIDE can't reach them. Step 5 of the build runs this after
//! every `3dsrecomp port`; on its own, from the project root:
//!
//!     cargo build-game --patch-only
//!
//! Each patch is anchored on the instruction comment 3dsrecomp writes before
//! the code for each ARM instruction ("/* ADDRESS OPCODE */"), so it fails
//! loudly instead of patching the wrong place if a new port moves things
//! around. Re-running is a no-op.
//!
//! Then the patched files are recompiled with the flags 3dsrecomp uses and put
//! into code/librecomp.a, so `cargo build` picks them up. CC and AR pick the
//! compiler and archiver (default gcc and ar, MinGW-w64's on Windows, Apple's
//! clang and ar on macOS).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::{fail, run, sorted_files};

/// the flags 3dsrecomp compiles generated code with (src/compile.rs)
const CFLAGS: [&str; 4] = ["-O2", "-ffp-contract=off", "-fno-math-errno", "-w"];

const DECLS: [&str; 4] = [
    "extern uint32_t smt4_boost_exp(Context *ctx, uint32_t gain);\n",
    "extern uint32_t smt4_boost_demon_exp(Context *ctx, uint32_t address);\n",
    "extern uint32_t smt4_widen_aspect(Context *ctx, uint32_t bits);\n",
    "extern uint32_t smt4_talk_debug(Context *ctx, uint32_t index, uint32_t value);\n",
];

pub struct Patch {
    pub desc: &'static str,
    /// the instruction comment it's anchored on
    pub anchor: &'static str,
    /// the text right after it
    pub old: &'static str,
    pub new: &'static str,
}

pub const PATCHES: [Patch; 5] = [
    Patch {
        desc: "hero EXP from battle results: hero.exp(r1) + gain(r0) at 0x0018CF84",
        anchor: "/* 0018CF84 E0817000 */",
        old: "    uint32_t m = reg0_;",
        new: "    uint32_t m = smt4_boost_exp(ctx, reg0_); /* smt4-recomp: EXP multiplier */",
    },
    // the gain is boosted where it's loaded, and written back, because
    // 0x0018D230 later re-reads it through a pointer to undo the add
    Patch {
        desc: "demon EXP from battle results: load of results[0x35B8 + id*4] at 0x0018D190",
        anchor: "/* 0018D190 E5900000 */",
        old: "    uint32_t v = mem_read32(ctx, oa);",
        new: "    uint32_t v = smt4_boost_demon_exp(ctx, oa); /* smt4-recomp: EXP multiplier */",
    },
    Patch {
        desc: "widescreen: aspect (s1) on entry to the perspective builder at 0x002B761C",
        anchor: "/* 002B761C E92D4010 */",
        old: "    { uint32_t base = reg13_, p = base - 8, fin = base - 8;",
        new: "    vfp1_ = smt4_widen_aspect(ctx, vfp1_); /* smt4-recomp: widescreen */\n    { uint32_t base = reg13_, p = base - 8, fin = base - 8;",
    },
    Patch {
        desc: "widescreen: aspect (s3) on entry to the camera's SetPerspective at 0x002C1FFC",
        anchor: "/* 002C1FFC E92D4070 */",
        old: "    { uint32_t base = reg13_, p = base - 16, fin = base - 16;",
        new: "    vfp3_ = smt4_widen_aspect(ctx, vfp3_); /* smt4-recomp: widescreen */\n    { uint32_t base = reg13_, p = base - 16, fin = base - 16;",
    },
    // r4 is the talk context, oa = r4 + 0x22A8 + index * 4
    Patch {
        desc: "easy recruit: talk debug setting read by script native 0x0259 at 0x002F1298",
        anchor: "/* 002F1298 E59002A8 */",
        old: "    uint32_t v = mem_read32(ctx, oa);",
        new: "    uint32_t v = smt4_talk_debug(ctx, (oa - reg4_ - 0x22A8u) >> 2, mem_read32(ctx, oa)); /* smt4-recomp: easy recruit */",
    },
];

fn find(haystack: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    haystack.get(from..)?.windows(needle.len()).position(|w| w == needle).map(|i| i + from)
}

fn count(haystack: &[u8], needle: &[u8]) -> usize {
    let mut n = 0;
    let mut at = 0;
    while let Some(i) = find(haystack, needle, at) {
        n += 1;
        at = i + needle.len();
    }
    n
}

/// what patching a file's text did
#[derive(Debug, PartialEq)]
pub enum Outcome {
    Patched,
    Already,
}

/// applies one patch to a file's text, anchor and all already checked to be
/// in it
pub fn apply(text: &mut Vec<u8>, patch: &Patch) -> Result<Outcome, String> {
    let anchor = patch.anchor.as_bytes();
    let at = find(text, anchor, 0).ok_or_else(|| format!("{}: {} is gone", patch.desc, patch.anchor))?;
    // up to the next instruction comment
    let window_end = find(text, b"/*", at + anchor.len())
        .ok_or_else(|| format!("{}: nothing after {}", patch.desc, patch.anchor))?;
    let marker = patch.new.split("/*").next().unwrap_or("").trim().as_bytes();
    let near = &text[at..(at + anchor.len() + 400).min(text.len())];
    if find(near, marker, 0).is_some() {
        return Ok(Outcome::Already);
    }
    let window = &text[at..window_end];
    if count(window, patch.old.as_bytes()) != 1 {
        return Err(format!("{}: expected exactly one {:?} after {}", patch.desc, patch.old, patch.anchor));
    }
    let old_at = at + find(window, patch.old.as_bytes(), 0).unwrap();
    text.splice(old_at..old_at + patch.old.len(), patch.new.bytes());
    let missing: String = DECLS.iter().filter(|d| find(text, d.as_bytes(), 0).is_none()).copied().collect();
    if !missing.is_empty() {
        // before the first generated function
        let first = find(text, b"\nvoid f_", 0).ok_or("no generated function to put the declarations before")? + 1;
        text.splice(first..first, missing.bytes());
    }
    Ok(Outcome::Patched)
}

/// applies every patch; returns the files patched at all
fn patch_all(code: &Path) -> Vec<PathBuf> {
    let sources: Vec<PathBuf> = sorted_files(code, |name| name.starts_with("code") && name.ends_with(".c"));
    // the file each patch goes in, each read once
    let mut hits: Vec<Vec<usize>> = vec![Vec::new(); PATCHES.len()];
    let mut texts: BTreeMap<usize, Vec<u8>> = BTreeMap::new();
    for (index, path) in sources.iter().enumerate() {
        let text = std::fs::read(path).unwrap_or_else(|e| fail(&format!("could not read {}: {e}", path.display())));
        let mut any = false;
        for (patch, files) in PATCHES.iter().zip(&mut hits) {
            if find(&text, patch.anchor.as_bytes(), 0).is_some() {
                files.push(index);
                any = true;
            }
        }
        if any {
            texts.insert(index, text);
        }
    }
    let mut changed = std::collections::BTreeSet::new();
    for (patch, files) in PATCHES.iter().zip(&hits) {
        let [index] = files[..] else {
            fail(&format!("anchor {:?} found in {} files, expected 1", patch.anchor, files.len()))
        };
        let name = sources[index].file_name().unwrap().to_string_lossy().into_owned();
        match apply(texts.get_mut(&index).unwrap(), patch) {
            Ok(Outcome::Already) => println!("already patched: {} ({name})", patch.desc),
            Ok(Outcome::Patched) => {
                println!("patched: {} ({name})", patch.desc);
                changed.insert(index);
            }
            Err(error) => fail(&error),
        }
    }
    for index in &changed {
        let path = &sources[*index];
        std::fs::write(path, &texts[index]).unwrap_or_else(|e| fail(&format!("could not write {}: {e}", path.display())));
    }
    texts.keys().map(|&index| sources[index].clone()).collect()
}

/// recompiles the patched files and the overrides into code/librecomp.a
fn rebuild(root: &Path, code: &Path, files: Vec<PathBuf>, cc: &str, ar: &str) {
    // overrides/*.c are recompiled too, so editing an override's body doesn't
    // need a new port. An override at a *new* address still does: port is
    // what points the callers and the entry table at it.
    let mut sources = files;
    sources.extend(sorted_files(&root.join("overrides"), |name| name.ends_with(".c")));
    let mut objects = Vec::new();
    for path in &sources {
        let obj = code.join(format!("{}.o", path.file_stem().unwrap().to_string_lossy()));
        println!("compiling {}", path.file_name().unwrap().to_string_lossy());
        let mut args: Vec<std::ffi::OsString> = CFLAGS.iter().map(Into::into).collect();
        args.extend(["-I".into(), code.into(), "-c".into(), path.into(), "-o".into(), obj.clone().into()]);
        run(cc, &args, root);
        objects.push(obj);
    }
    // archive members are named after the file, so this replaces the objects
    // 3dsrecomp put there, and s writes the index again, without which
    // Apple's linker turns the archive down
    let mut args: Vec<std::ffi::OsString> = vec!["rs".into(), code.join("librecomp.a").into()];
    args.extend(objects.iter().map(Into::into));
    run(ar, &args, root);
    for obj in &objects {
        let _ = std::fs::remove_file(obj);
    }
    println!("updated librecomp.a with {} object(s)", objects.len());
}

pub fn patch(root: &Path, cc: &str, ar: &str) {
    let code = root.join("code");
    let files = patch_all(&code);
    rebuild(root, &code, files, cc, ar);
}

#[cfg(test)]
mod tests {
    use super::*;

    const GENERATED: &str = "#include \"recomp.h\"\n\nvoid f_0018CF00(Context *ctx) {\n/* 0018CF84 E0817000 */\n    uint32_t m = reg0_;\n    reg7_ = reg1_ + m;\n/* 0018CF88 E5847000 */\n    uint32_t m = reg0_;\n}\n";

    #[test]
    fn patches_once_and_declares() {
        let mut text = GENERATED.as_bytes().to_vec();
        assert_eq!(apply(&mut text, &PATCHES[0]), Ok(Outcome::Patched));
        let s = String::from_utf8(text.clone()).unwrap();
        assert!(s.contains("/* 0018CF84 E0817000 */\n    uint32_t m = smt4_boost_exp(ctx, reg0_); /* smt4-recomp"));
        // only the one after the anchor, not the next instruction's
        assert!(s.contains("/* 0018CF88 E5847000 */\n    uint32_t m = reg0_;"));
        // the declarations go before the first function, once
        let decl = s.find("extern uint32_t smt4_boost_exp").unwrap();
        assert!(decl < s.find("void f_").unwrap());
        assert_eq!(s.matches("extern uint32_t smt4_talk_debug").count(), 1);
        // a second run changes nothing
        assert_eq!(apply(&mut text, &PATCHES[0]), Ok(Outcome::Already));
        assert_eq!(String::from_utf8(text).unwrap(), s);
    }

    #[test]
    fn fails_when_the_code_moved() {
        let mut text = GENERATED.replace("    uint32_t m = reg0_;\n    reg7_", "    uint32_t m = reg2_;\n    reg7_").into_bytes();
        assert!(apply(&mut text, &PATCHES[0]).unwrap_err().contains("expected exactly one"));
    }
}
