use std::mem::size_of;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use anyhow::Result;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE,
    VIRTUAL_KEY, VK_MENU, VK_V,
};

use crate::app::AppState;
use crate::clipboard;
use crate::save;

pub fn paste_path(state: &Arc<Mutex<AppState>>) {
    if let Err(err) = paste_path_inner(state) {
        eprintln!("clip2term: Alt+V 注入失败: {err:#}");
    }
}

fn paste_path_inner(state: &Arc<Mutex<AppState>>) -> Result<()> {
    {
        let s = state.lock().expect("app state poisoned");
        if !s.enabled {
            return Ok(());
        }
    }
    // 等 Alt/V 松开，并补发 KEYUP，避免注入时仍按着 Alt。
    thread::sleep(Duration::from_millis(30));
    release_alt_v()?;
    let path = {
        let last = state.lock().expect("app state poisoned").last_path.clone();
        match last {
            Some(p) if p.exists() => p,
            _ => match clipboard::capture_if_image(state)? {
                Some(p) => p,
                None => return Ok(()),
            },
        }
    };
    let text = save::path_for_paste(&path);
    send_unicode(&text)
}

fn release_alt_v() -> Result<()> {
    let inputs = [key_vk(VK_V, true), key_vk(VK_MENU, true)];
    send(&inputs)
}

fn send_unicode(text: &str) -> Result<()> {
    let mut inputs: Vec<INPUT> = Vec::with_capacity(text.len() * 2);
    for ch in text.encode_utf16() {
        inputs.push(key_unicode(ch, false));
        inputs.push(key_unicode(ch, true));
    }
    send(&inputs)
}

fn key_vk(vk: VIRTUAL_KEY, up: bool) -> INPUT {
    let mut flags = Default::default();
    if up {
        flags |= KEYEVENTF_KEYUP;
    }
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

fn key_unicode(ch: u16, up: bool) -> INPUT {
    let mut flags = KEYEVENTF_UNICODE;
    if up {
        flags |= KEYEVENTF_KEYUP;
    }
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(0),
                wScan: ch,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

fn send(inputs: &[INPUT]) -> Result<()> {
    if inputs.is_empty() {
        return Ok(());
    }
    let sent = unsafe { SendInput(inputs, size_of::<INPUT>() as i32) };
    if sent as usize != inputs.len() {
        anyhow::bail!("SendInput 只发送了 {sent}/{} 个事件", inputs.len());
    }
    Ok(())
}
