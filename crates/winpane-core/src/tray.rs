use std::ffi::c_void;

use windows::{
    core::*, Win32::Foundation::*, Win32::Graphics::Gdi::*, Win32::UI::Shell::*,
    Win32::UI::WindowsAndMessaging::*,
};

use crate::types::{Error, MenuItem, SurfaceId};
use crate::window::WM_TRAY_CALLBACK;

// --- TrayState ---

pub(crate) struct TrayState {
    pub hwnd: HWND,
    pub icon_id: u32,
    pub hicon: HICON,
    pub popup_surface: Option<SurfaceId>,
    pub popup_visible: bool,
    pub menu_items: Vec<MenuItem>,
}

// --- HICON creation from RGBA ---

/// Creates an HICON from RGBA8 pixel data.
///
/// The RGBA data is converted to BGRA for the color bitmap.
/// A monochrome mask bitmap (all zeros) is used; the alpha channel in the
/// color bitmap provides transparency.
pub(crate) unsafe fn create_hicon_from_rgba(
    data: &[u8],
    width: u32,
    height: u32,
) -> Result<HICON, Error> {
    let pixel_count = (width * height) as usize;
    if data.len() != pixel_count * 4 {
        return Err(Error::RenderError(format!(
            "Icon data size mismatch: expected {} bytes, got {}",
            pixel_count * 4,
            data.len()
        )));
    }

    // Create a 32-bit top-down DIB section for the color bitmap
    let bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width as i32,
            biHeight: -(height as i32), // negative = top-down
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0 as u32,
            ..Default::default()
        },
        ..Default::default()
    };

    let hdc = GetDC(HWND::default());
    let mut bits: *mut c_void = std::ptr::null_mut();

    let color_bmp = CreateDIBSection(hdc, &bmi, DIB_RGB_COLORS, &mut bits, None, 0)
        .map_err(|e| Error::RenderError(format!("CreateDIBSection: {e}")))?;
    let _ = ReleaseDC(HWND::default(), hdc);

    if bits.is_null() {
        let _ = DeleteObject(color_bmp);
        return Err(Error::RenderError(
            "DIB section bits pointer is null".into(),
        ));
    }

    // Copy RGBA -> BGRA into the DIB section buffer
    let dest = std::slice::from_raw_parts_mut(bits.cast::<u8>(), pixel_count * 4);
    for i in 0..pixel_count {
        let s = i * 4;
        dest[s] = data[s + 2]; // B
        dest[s + 1] = data[s + 1]; // G
        dest[s + 2] = data[s]; // R
        dest[s + 3] = data[s + 3]; // A
    }

    // Create monochrome mask bitmap (all zeros = use alpha from color bitmap)
    let mask_bmp = CreateBitmap(width as i32, height as i32, 1, 1, None);

    // Create the icon
    let icon_info = ICONINFO {
        fIcon: TRUE,
        xHotspot: 0,
        yHotspot: 0,
        hbmMask: mask_bmp,
        hbmColor: color_bmp,
    };

    let hicon = CreateIconIndirect(&icon_info)
        .map_err(|e| Error::RenderError(format!("CreateIconIndirect: {e}")))?;

    // Clean up bitmaps (the icon has its own copy)
    let _ = DeleteObject(color_bmp);
    let _ = DeleteObject(mask_bmp);

    Ok(hicon)
}

// --- Tray icon lifecycle ---

/// Creates a system tray icon.
pub(crate) unsafe fn create_tray_icon(
    hwnd: HWND,
    icon_id: u32,
    hicon: HICON,
    tooltip: &str,
) -> Result<(), Error> {
    let mut nid = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: icon_id,
        uFlags: NIF_ICON | NIF_TIP | NIF_MESSAGE,
        uCallbackMessage: WM_TRAY_CALLBACK,
        hIcon: hicon,
        ..Default::default()
    };

    // Copy tooltip into fixed-size UTF-16 array (max 127 chars + null)
    let tip_utf16: Vec<u16> = tooltip.encode_utf16().take(127).collect();
    for (i, &ch) in tip_utf16.iter().enumerate() {
        nid.szTip[i] = ch;
    }
    // Null terminator is already zero from Default

    let result = Shell_NotifyIconW(NIM_ADD, &nid);
    if !result.as_bool() {
        return Err(Error::WindowCreation(
            "Shell_NotifyIconW(NIM_ADD) failed".into(),
        ));
    }

    Ok(())
}

/// Updates the tray icon tooltip.
pub(crate) unsafe fn update_tray_tooltip(
    hwnd: HWND,
    icon_id: u32,
    tooltip: &str,
) -> Result<(), Error> {
    let mut nid = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: icon_id,
        uFlags: NIF_TIP,
        ..Default::default()
    };

    let tip_utf16: Vec<u16> = tooltip.encode_utf16().take(127).collect();
    for (i, &ch) in tip_utf16.iter().enumerate() {
        nid.szTip[i] = ch;
    }

    let _ = Shell_NotifyIconW(NIM_MODIFY, &nid);
    Ok(())
}

/// Updates the tray icon image.
pub(crate) unsafe fn update_tray_icon(hwnd: HWND, icon_id: u32, hicon: HICON) -> Result<(), Error> {
    let nid = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: icon_id,
        uFlags: NIF_ICON,
        hIcon: hicon,
        ..Default::default()
    };

    let _ = Shell_NotifyIconW(NIM_MODIFY, &nid);
    Ok(())
}

/// Removes the tray icon and destroys the HICON.
/// NIM_DELETE must come before DestroyIcon to avoid icon-in-use issues.
pub(crate) unsafe fn destroy_tray_icon(hwnd: HWND, icon_id: u32, hicon: HICON) {
    let nid = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: icon_id,
        ..Default::default()
    };
    let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
    let _ = DestroyIcon(hicon);
}

// --- Context menu ---

/// Shows a native popup context menu at the cursor position.
/// Returns the selected item ID, or 0 if dismissed.
pub(crate) unsafe fn show_tray_context_menu(hwnd: HWND, items: &[MenuItem]) -> u32 {
    if items.is_empty() {
        return 0;
    }

    let hmenu = match CreatePopupMenu() {
        Ok(m) => m,
        Err(_) => return 0,
    };

    for item in items {
        let label = HSTRING::from(&*item.label);
        let flags = if item.enabled {
            MF_STRING
        } else {
            MF_STRING | MF_GRAYED
        };
        let _ = AppendMenuW(hmenu, flags, item.id as usize, &label);
    }

    // Required for proper menu dismissal (documented Win32 behavior)
    let _ = SetForegroundWindow(hwnd);

    let mut cursor = POINT::default();
    let _ = GetCursorPos(&mut cursor);

    // TPM_RETURNCMD makes TrackPopupMenu return the selected item ID
    let selected = TrackPopupMenu(
        hmenu,
        TPM_RETURNCMD | TPM_BOTTOMALIGN | TPM_LEFTALIGN,
        cursor.x,
        cursor.y,
        0,
        hwnd,
        None,
    );

    // Standard workaround for Win32 menu dismissal bug
    let _ = PostMessageW(hwnd, WM_NULL, WPARAM(0), LPARAM(0));

    let _ = DestroyMenu(hmenu);

    if selected.as_bool() {
        selected.0 as u32
    } else {
        0
    }
}
