#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod clipboard;
mod config;
mod hotkey;
mod picker;
mod save;
mod tray;
mod wnd;

use std::sync::{Arc, Mutex};

use anyhow::Result;
use windows::core::w;
use windows::Win32::Foundation::GetLastError;
use windows::Win32::Foundation::ERROR_ALREADY_EXISTS;
use windows::Win32::System::Threading::CreateMutexW;

use crate::app::AppState;
use crate::config::Config;

fn main() {
    if let Err(err) = run() {
        eprintln!("clip2term 启动失败: {err:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    acquire_single_instance()?;
    let config = Config::load()?;
    std::fs::create_dir_all(&config.save_dir)?;
    let state = Arc::new(Mutex::new(AppState::new(config)));
    let tray = tray::TrayApp::new(&state)?;
    wnd::run(state.clone(), move || tray.poll(&state))?;
    Ok(())
}

fn acquire_single_instance() -> Result<()> {
    unsafe {
        let mutex = CreateMutexW(None, true, w!("Local\\clip2term.single"))?;
        if GetLastError() == ERROR_ALREADY_EXISTS {
            anyhow::bail!("clip2term 已在运行");
        }
        // HANDLE 无 Drop，进程退出时内核会释放互斥量。
        let _ = mutex;
    }
    Ok(())
}
