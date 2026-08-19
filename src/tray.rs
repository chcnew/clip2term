use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use tray_icon::menu::{Menu, MenuEvent, MenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};
use windows::Win32::UI::WindowsAndMessaging::PostQuitMessage;

use crate::app::AppState;

pub struct TrayApp {
    tray: TrayIcon,
    toggle: MenuItem,
    set_dir: MenuItem,
    open: MenuItem,
    quit: MenuItem,
}

impl TrayApp {
    pub fn new(state: &Arc<Mutex<AppState>>) -> Result<Self> {
        let enabled = state.lock().expect("app state poisoned").enabled;
        let toggle = MenuItem::with_id("toggle", toggle_label(enabled), true, None);
        let set_dir = MenuItem::with_id("set_dir", "设置截图目录...", true, None);
        let open = MenuItem::with_id("open", "打开截图目录", true, None);
        let quit = MenuItem::with_id("quit", "退出", true, None);
        let menu = Menu::new();
        menu.append_items(&[&toggle, &set_dir, &open, &quit])
            .context("创建托盘菜单失败")?;
        let tray = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip(tooltip(enabled))
            .with_icon(make_icon()?)
            .with_title("clip2term")
            .build()
            .context("创建托盘图标失败")?;
        Ok(Self {
            tray,
            toggle,
            set_dir,
            open,
            quit,
        })
    }

    pub fn poll(&self, state: &Arc<Mutex<AppState>>) {
        while let Ok(event) = MenuEvent::receiver().try_recv() {
            if event.id == self.toggle.id() {
                let mut s = state.lock().expect("app state poisoned");
                s.enabled = !s.enabled;
                let enabled = s.enabled;
                drop(s);
                self.toggle.set_text(toggle_label(enabled));
                let _ = self.tray.set_tooltip(Some(tooltip(enabled)));
            } else if event.id == self.set_dir.id() {
                match crate::picker::pick_folder() {
                    Ok(Some(dir)) => {
                        let mut s = state.lock().expect("app state poisoned");
                        s.config.save_dir = dir.clone();
                        let save_result = s.config.save();
                        drop(s);
                        if let Err(err) = save_result {
                            eprintln!("clip2term: 保存配置失败: {err:#}");
                        }
                        let _ = std::fs::create_dir_all(&dir);
                    }
                    Ok(None) => {}
                    Err(err) => eprintln!("clip2term: 选择目录失败: {err:#}"),
                }
            } else if event.id == self.open.id() {
                let dir = state
                    .lock()
                    .expect("app state poisoned")
                    .config
                    .save_dir
                    .clone();
                let _ = std::fs::create_dir_all(&dir);
                let _ = std::process::Command::new("explorer").arg(&dir).spawn();
            } else if event.id == self.quit.id() {
                unsafe { PostQuitMessage(0) };
            }
        }
    }
}

fn toggle_label(enabled: bool) -> &'static str {
    if enabled {
        "暂停"
    } else {
        "继续"
    }
}

fn tooltip(enabled: bool) -> String {
    if enabled {
        "clip2term：监听中 · Alt+V 粘贴路径".to_string()
    } else {
        "clip2term：已暂停".to_string()
    }
}

fn make_icon() -> Result<Icon> {
    const SIZE: u32 = 32;
    let img = image::load_from_memory(include_bytes!("../assets/icon.png"))
        .context("读取图标 PNG 失败")?
        .into_rgba8();
    let resized = image::imageops::resize(&img, SIZE, SIZE, image::imageops::FilterType::Lanczos3);
    Icon::from_rgba(resized.into_raw(), SIZE, SIZE).context("生成托盘图标失败")
}
