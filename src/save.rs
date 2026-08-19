use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use image::RgbaImage;

use crate::clipboard::ClipboardImage;

pub fn save_png(dir: &Path, image: &ClipboardImage, keep_last: usize) -> Result<PathBuf> {
    fs::create_dir_all(dir).with_context(|| format!("创建目录失败: {}", dir.display()))?;
    let buf = RgbaImage::from_raw(image.width, image.height, image.bytes.clone())
        .context("图像尺寸与像素数据不匹配")?;
    let path = unique_path(dir);
    buf.save(&path)
        .with_context(|| format!("写入 PNG 失败: {}", path.display()))?;
    cleanup(dir, keep_last)?;
    if !path.exists() {
        bail!("刚保存的截图已被清理，请增大 keep_last");
    }
    Ok(path)
}

fn unique_path(dir: &Path) -> PathBuf {
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S-%3f");
    let mut path = dir.join(format!("{stamp}.png"));
    let mut n = 1u32;
    while path.exists() {
        path = dir.join(format!("{stamp}-{n}.png"));
        n += 1;
    }
    path
}

fn cleanup(dir: &Path, keep_last: usize) -> Result<()> {
    let mut files: Vec<(std::time::SystemTime, PathBuf)> = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("png") {
            continue;
        }
        let modified = entry
            .metadata()
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        files.push((modified, path));
    }
    files.sort_by(|a, b| b.0.cmp(&a.0));
    for (_, path) in files.into_iter().skip(keep_last) {
        let _ = fs::remove_file(path);
    }
    Ok(())
}

pub fn path_for_paste(path: &Path) -> String {
    let s = path.to_string_lossy();
    if s.chars().any(|c| c.is_whitespace()) {
        format!("\"{s}\"")
    } else {
        s.into_owned()
    }
}
