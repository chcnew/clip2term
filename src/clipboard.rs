use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use anyhow::Result;
use arboard::Clipboard;

use crate::app::AppState;
use crate::save;

pub struct ClipboardImage {
    pub width: u32,
    pub height: u32,
    pub bytes: Vec<u8>,
}

pub fn on_update(state: &Arc<Mutex<AppState>>) {
    if let Err(err) = capture_if_image(state) {
        eprintln!("clip2term: 处理剪切板失败: {err:#}");
    }
}

pub fn capture_if_image(state: &Arc<Mutex<AppState>>) -> Result<Option<std::path::PathBuf>> {
    let (dir, keep_last) = {
        let s = state.lock().expect("app state poisoned");
        if !s.enabled {
            return Ok(None);
        }
        (s.config.save_dir.clone(), s.config.keep_last)
    };
    let Some(image) = read_image_with_retry() else {
        return Ok(None);
    };
    let path = save::save_png(&dir, &image, keep_last)?;
    state.lock().expect("app state poisoned").last_path = Some(path.clone());
    Ok(Some(path))
}

fn read_image_with_retry() -> Option<ClipboardImage> {
    for _ in 0..8 {
        thread::sleep(Duration::from_millis(40));
        match read_image() {
            Ok(Some(image)) => return Some(image),
            Ok(None) => return None,
            Err(_) => continue,
        }
    }
    None
}

fn read_image() -> Result<Option<ClipboardImage>> {
    let mut clipboard = Clipboard::new()?;
    match clipboard.get_image() {
        Ok(img) => {
            if img.width == 0 || img.height == 0 {
                return Ok(None);
            }
            let expected = img.width.saturating_mul(img.height).saturating_mul(4);
            if img.bytes.len() != expected {
                return Ok(None);
            }
            Ok(Some(ClipboardImage {
                width: img.width as u32,
                height: img.height as u32,
                bytes: img.bytes.into_owned(),
            }))
        }
        Err(arboard::Error::ContentNotAvailable) => Ok(None),
        Err(err) => Err(err.into()),
    }
}
