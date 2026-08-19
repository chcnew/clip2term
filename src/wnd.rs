use std::sync::{Arc, Mutex};

use anyhow::{bail, Context, Result};
use windows::core::w;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::DataExchange::{
    AddClipboardFormatListener, RemoveClipboardFormatListener,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, UnregisterHotKey, MOD_ALT, MOD_NOREPEAT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW,
    GetWindowLongPtrW, RegisterClassExW, SetWindowLongPtrW, ShowWindow, TranslateMessage,
    CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, GWLP_USERDATA, MSG, SW_HIDE, WM_CLIPBOARDUPDATE,
    WM_DESTROY, WM_HOTKEY, WNDCLASSEXW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_OVERLAPPED,
};

use crate::app::AppState;
use crate::clipboard;
use crate::hotkey;

const HOTKEY_ID: i32 = 1;
const CLASS_NAME: windows::core::PCWSTR = w!("clip2term.ClipboardWnd");

pub fn run(state: Arc<Mutex<AppState>>, mut on_message: impl FnMut()) -> Result<()> {
    let hinstance = unsafe { GetModuleHandleW(None)? };
    let wc = WNDCLASSEXW {
        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(wndproc),
        hInstance: hinstance.into(),
        lpszClassName: CLASS_NAME,
        ..Default::default()
    };
    let atom = unsafe { RegisterClassExW(&wc) };
    if atom == 0 {
        bail!("RegisterClassExW 失败");
    }
    // 必须是隐藏的顶层窗口：HWND_MESSAGE 收不到 WM_CLIPBOARDUPDATE。
    let hwnd = unsafe {
        CreateWindowExW(
            WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW,
            CLASS_NAME,
            w!("clip2term"),
            WS_OVERLAPPED,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            0,
            0,
            None,
            None,
            hinstance,
            None,
        )?
    };
    unsafe {
        let _ = ShowWindow(hwnd, SW_HIDE);
    }
    let ptr = Arc::into_raw(Arc::clone(&state));
    unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, ptr as isize) };
    unsafe {
        AddClipboardFormatListener(hwnd).context("AddClipboardFormatListener 失败")?;
        RegisterHotKey(hwnd, HOTKEY_ID, MOD_ALT | MOD_NOREPEAT, u32::from(b'V'))
            .context("注册 Alt+V 失败，可能已被其它程序占用")?;
    }

    let mut msg = MSG::default();
    loop {
        let ret = unsafe { GetMessageW(&mut msg, None, 0, 0) };
        if ret.0 == 0 || ret.0 == -1 {
            break;
        }
        unsafe {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        on_message();
    }

    unsafe {
        let _ = UnregisterHotKey(hwnd, HOTKEY_ID);
        let _ = RemoveClipboardFormatListener(hwnd);
        let _ = DestroyWindow(hwnd);
    }
    Ok(())
}

fn state_from_hwnd(hwnd: HWND) -> Option<Arc<Mutex<AppState>>> {
    let ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) } as *const Mutex<AppState>;
    if ptr.is_null() {
        return None;
    }
    let arc = unsafe { Arc::from_raw(ptr) };
    let cloned = Arc::clone(&arc);
    std::mem::forget(arc);
    Some(cloned)
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_CLIPBOARDUPDATE => {
            if let Some(state) = state_from_hwnd(hwnd) {
                std::thread::spawn(move || clipboard::on_update(&state));
            }
            LRESULT(0)
        }
        WM_HOTKEY => {
            if wparam.0 == HOTKEY_ID as usize {
                if let Some(state) = state_from_hwnd(hwnd) {
                    std::thread::spawn(move || hotkey::paste_path(&state));
                }
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            let ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) } as *const Mutex<AppState>;
            if !ptr.is_null() {
                unsafe {
                    SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                    drop(Arc::from_raw(ptr));
                }
            }
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}
