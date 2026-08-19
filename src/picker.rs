use std::path::PathBuf;

use anyhow::{bail, Result};
use windows::core::{w, PWSTR};
use windows::Win32::System::Com::{
    CoInitializeEx, CoTaskMemFree, CoUninitialize, COINIT_APARTMENTTHREADED,
};
use windows::Win32::UI::Shell::{
    SHBrowseForFolderW, SHGetPathFromIDListW, BIF_NEWDIALOGSTYLE, BIF_RETURNONLYFSDIRS,
    BROWSEINFOW,
};

pub fn pick_folder() -> Result<Option<PathBuf>> {
    let hr = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    let initialized = if hr.is_err() {
        bail!("初始化 COM 失败: {hr}");
    } else {
        hr.0 == 0
    };

    let result = pick_folder_inner();

    if initialized {
        unsafe { CoUninitialize() };
    }
    result
}

fn pick_folder_inner() -> Result<Option<PathBuf>> {
    let mut display_name = [0u16; 260];
    let bi = BROWSEINFOW {
        lpszTitle: w!("选择截图保存目录"),
        ulFlags: BIF_RETURNONLYFSDIRS | BIF_NEWDIALOGSTYLE,
        pszDisplayName: PWSTR(display_name.as_mut_ptr()),
        ..Default::default()
    };
    let pidl = unsafe { SHBrowseForFolderW(&bi) };
    if pidl.is_null() {
        return Ok(None);
    }
    let mut path_buf = [0u16; 260];
    let ok = unsafe { SHGetPathFromIDListW(pidl, &mut path_buf) };
    unsafe { CoTaskMemFree(Some(pidl.cast())) };
    if !ok.as_bool() {
        bail!("无法解析所选目录");
    }
    let len = path_buf.iter().position(|&c| c == 0).unwrap_or(path_buf.len());
    let path = PathBuf::from(String::from_utf16_lossy(&path_buf[..len]));
    if path.as_os_str().is_empty() {
        return Ok(None);
    }
    Ok(Some(path))
}
