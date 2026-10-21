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
    // 终端只能粘贴文字。截图本身是图片，Ctrl+V 会没有内容，所以把路径放回剪贴板。
    publish_path(&path)?;
    Ok(Some(path))
}

fn publish_path(path: &std::path::Path) -> Result<()> {
    let text = save::path_for_paste(path);
    let mut clipboard = Clipboard::new()?;
    let mut last_err = None;
    for _ in 0..8 {
        match clipboard.set_text(text.as_str()) {
            Ok(()) => return Ok(()),
            Err(err) => {
                last_err = Some(err);
                thread::sleep(Duration::from_millis(40));
            }
        }
    }
    Err(last_err
        .unwrap_or_else(|| arboard::Error::ContentNotAvailable)
        .into())
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

#[cfg(test)]
mod tests {
    use std::borrow::Cow;
    use std::sync::{Arc, Mutex};

    use arboard::{Clipboard, ImageData};

    use crate::app::AppState;
    use crate::config::Config;
    use crate::save;

    use super::capture_if_image;

    #[test]
    fn captured_screenshot_can_be_pasted_as_text() {
        let dir = std::env::temp_dir().join(format!("clip2term-paste-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let state = Arc::new(Mutex::new(AppState {
            enabled: true,
            last_path: None,
            config: Config {
                save_dir: dir.clone(),
                keep_last: 10,
            },
        }));
        let pixels = [
            255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 0, 255,
        ];
        {
            let mut clipboard = Clipboard::new().expect("clipboard");
            clipboard
                .set_image(ImageData {
                    width: 2,
                    height: 2,
                    bytes: Cow::Borrowed(&pixels),
                })
                .expect("put image on clipboard");
        }

        let path = capture_if_image(&state)
            .expect("capture")
            .expect("screenshot should be saved");
        assert!(path.exists());

        let mut clipboard = Clipboard::new().expect("clipboard");
        let text = clipboard
            .get_text()
            .expect("terminal paste needs text on the clipboard");
        assert_eq!(text, save::path_for_paste(&path));

        assert!(capture_if_image(&state).expect("second capture").is_none());
        assert_eq!(clipboard.get_text().expect("text remains"), text);
        let _ = std::fs::remove_dir_all(&dir);
    }
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
