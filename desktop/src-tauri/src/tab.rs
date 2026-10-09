//! The tab: a small mark that appears where Clipframes was used before. Clicking it opens the bar.
//!
//! It can be on screen for hours, so on Windows it is a plain window drawn by the system from
//! one picture, not a web view: a web view there costs hundreds of megabytes even when idle.

/// The tab's side, in CSS pixels.
pub const SIZE: f64 = 44.0;

#[cfg(windows)]
pub use native::Tab;

#[cfg(windows)]
mod native {
    use crate::shot::Image;
    use std::cell::RefCell;
    use std::ffi::c_void;
    use std::sync::mpsc;
    use std::thread;

    type Handle = *mut c_void;

    #[repr(C)]
    struct WndClass {
        size: u32,
        style: u32,
        proc_: unsafe extern "system" fn(Handle, u32, usize, isize) -> isize,
        class_extra: i32,
        window_extra: i32,
        instance: Handle,
        icon: Handle,
        cursor: Handle,
        background: Handle,
        menu_name: *const u16,
        class_name: *const u16,
        icon_small: Handle,
    }

    #[repr(C)]
    struct Msg {
        hwnd: Handle,
        message: u32,
        wparam: usize,
        lparam: isize,
        time: u32,
        x: i32,
        y: i32,
    }

    #[repr(C)]
    struct BitmapInfoHeader {
        size: u32,
        width: i32,
        height: i32,
        planes: u16,
        bit_count: u16,
        compression: u32,
        size_image: u32,
        x_ppm: i32,
        y_ppm: i32,
        clr_used: u32,
        clr_important: u32,
    }

    #[repr(C)]
    struct Pair(i32, i32);

    #[repr(C)]
    struct Blend {
        op: u8,
        flags: u8,
        alpha: u8,
        format: u8,
    }

    #[link(name = "user32")]
    extern "system" {
        fn RegisterClassExW(class: *const WndClass) -> u16;
        fn CreateWindowExW(ex: u32, class: *const u16, name: *const u16, style: u32, x: i32, y: i32, w: i32, h: i32, parent: Handle, menu: Handle, instance: Handle, param: Handle) -> Handle;
        fn DefWindowProcW(hwnd: Handle, msg: u32, wparam: usize, lparam: isize) -> isize;
        fn GetMessageW(msg: *mut c_void, hwnd: Handle, min: u32, max: u32) -> i32;
        fn DispatchMessageW(msg: *const Msg) -> isize;
        fn PostMessageW(hwnd: Handle, msg: u32, wparam: usize, lparam: isize) -> i32;
        fn SetWindowPos(hwnd: Handle, after: Handle, x: i32, y: i32, w: i32, h: i32, flags: u32) -> i32;
        fn ShowWindow(hwnd: Handle, how: i32) -> i32;
        fn LoadCursorW(instance: Handle, name: *const u16) -> Handle;
        fn UpdateLayeredWindow(hwnd: Handle, dest: Handle, at: *const Pair, size: *const Pair, src: Handle, from: *const Pair, key: u32, blend: *const Blend, flags: u32) -> i32;
        fn SetWindowDisplayAffinity(hwnd: Handle, affinity: u32) -> i32;
        fn GetDpiForSystem() -> u32;
        fn GetDC(hwnd: Handle) -> Handle;
        fn ReleaseDC(hwnd: Handle, dc: Handle) -> i32;
    }

    #[link(name = "gdi32")]
    extern "system" {
        fn CreateCompatibleDC(dc: Handle) -> Handle;
        fn CreateDIBSection(dc: Handle, info: *const BitmapInfoHeader, usage: u32, bits: *mut *mut c_void, section: Handle, offset: u32) -> Handle;
        fn SelectObject(dc: Handle, object: Handle) -> Handle;
        fn DeleteObject(object: Handle) -> i32;
        fn DeleteDC(dc: Handle) -> i32;
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn GetModuleHandleW(name: *const u16) -> Handle;
    }

    const WS_POPUP: u32 = 0x8000_0000;
    const WS_EX_LAYERED: u32 = 0x0008_0000;
    const WS_EX_TOPMOST: u32 = 0x0000_0008;
    const WS_EX_TOOLWINDOW: u32 = 0x0000_0080;
    const WS_EX_NOACTIVATE: u32 = 0x0800_0000;
    const WM_CLOSE: u32 = 0x0010;
    const WM_LBUTTONUP: u32 = 0x0202;
    const WM_APP_SHOW: u32 = 0x8001;
    const WM_APP_HIDE: u32 = 0x8002;
    const HWND_TOPMOST: Handle = -1isize as Handle;
    const SWP_NOSIZE: u32 = 0x0001;
    const SWP_NOACTIVATE: u32 = 0x0010;
    const SWP_SHOWWINDOW: u32 = 0x0040;
    const WDA_EXCLUDEFROMCAPTURE: u32 = 0x11;
    const IDC_HAND: usize = 32649;

    thread_local! {
        static ON_CLICK: RefCell<Option<Box<dyn Fn()>>> = const { RefCell::new(None) };
    }

    unsafe extern "system" fn window_proc(hwnd: Handle, msg: u32, wparam: usize, lparam: isize) -> isize {
        match msg {
            WM_LBUTTONUP => {
                ON_CLICK.with(|f| {
                    if let Some(f) = f.borrow().as_ref() {
                        f()
                    }
                });
                0
            }
            WM_APP_SHOW => {
                // 4 is "show, in its normal state, without taking the keyboard": it also brings
                // the tab back if something minimized it (Win+D, a window manager, a script).
                ShowWindow(hwnd, 4);
                SetWindowPos(hwnd, HWND_TOPMOST, wparam as i32, lparam as i32, 0, 0, SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW);
                0
            }
            WM_APP_HIDE => {
                ShowWindow(hwnd, 0);
                0
            }
            // Asked to close (Alt+F4 while it has the keyboard, or another program closing
            // windows): nothing happens. Destroyed, the tab could never come back, and hidden
            // behind the app's back it would not be shown again until the place changed. The
            // pin in the bar is how the tab is turned off.
            WM_CLOSE => 0,
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(Some(0)).collect()
    }

    /// The tab's picture at the size this screen needs.
    fn picture(side: u32) -> Option<Image> {
        let decoder = png::Decoder::new(std::io::Cursor::new(&include_bytes!("../icons/tab.png")[..]));
        let mut reader = decoder.read_info().ok()?;
        let mut rgba = vec![0u8; reader.output_buffer_size()];
        let info = reader.next_frame(&mut rgba).ok()?;
        (info.color_type == png::ColorType::Rgba).then(|| Image { width: info.width, height: info.height, rgba }.fit(side))
    }

    /// Hands the picture to the window, pixels and transparency together.
    unsafe fn paint(hwnd: Handle, image: &Image) {
        let (w, h) = (image.width as i32, image.height as i32);
        let screen = GetDC(std::ptr::null_mut());
        let memory = CreateCompatibleDC(screen);
        let info = BitmapInfoHeader { size: 40, width: w, height: -h, planes: 1, bit_count: 32, compression: 0, size_image: 0, x_ppm: 0, y_ppm: 0, clr_used: 0, clr_important: 0 };
        let mut bits: *mut c_void = std::ptr::null_mut();
        let bitmap = CreateDIBSection(screen, &info, 0, &mut bits, std::ptr::null_mut(), 0);
        if !bitmap.is_null() && !bits.is_null() {
            let out = std::slice::from_raw_parts_mut(bits as *mut u8, (w * h * 4) as usize);
            // The system wants blue, green, red, each already multiplied by the transparency.
            for (dst, src) in out.chunks_exact_mut(4).zip(image.rgba.chunks_exact(4)) {
                let a = src[3] as u32;
                dst[0] = (src[2] as u32 * a / 255) as u8;
                dst[1] = (src[1] as u32 * a / 255) as u8;
                dst[2] = (src[0] as u32 * a / 255) as u8;
                dst[3] = src[3];
            }
            let previous = SelectObject(memory, bitmap);
            let blend = Blend { op: 0, flags: 0, alpha: 255, format: 1 };
            UpdateLayeredWindow(hwnd, screen, std::ptr::null(), &Pair(w, h), memory, &Pair(0, 0), 0, &blend, 2);
            SelectObject(memory, previous);
            DeleteObject(bitmap);
        }
        DeleteDC(memory);
        ReleaseDC(std::ptr::null_mut(), screen);
    }

    pub struct Tab {
        hwnd: isize,
        side: i32,
    }

    impl Tab {
        /// Makes the tab, hidden. `on_click` runs on the tab's own thread.
        pub fn start(protected: bool, on_click: impl Fn() + Send + 'static) -> Option<Tab> {
            let (ready, made) = mpsc::channel::<Option<(isize, i32)>>();
            thread::Builder::new()
                .name("clipframes-tab".into())
                .spawn(move || unsafe {
                    let side = (super::SIZE * GetDpiForSystem() as f64 / 96.0).round() as i32;
                    let Some(image) = picture(side as u32) else {
                        let _ = ready.send(None);
                        return;
                    };
                    ON_CLICK.with(|f| *f.borrow_mut() = Some(Box::new(on_click)));
                    let instance = GetModuleHandleW(std::ptr::null());
                    let name = wide("ClipframesTab");
                    let class = WndClass {
                        size: std::mem::size_of::<WndClass>() as u32,
                        style: 0,
                        proc_: window_proc,
                        class_extra: 0,
                        window_extra: 0,
                        instance,
                        icon: std::ptr::null_mut(),
                        cursor: LoadCursorW(std::ptr::null_mut(), IDC_HAND as *const u16),
                        background: std::ptr::null_mut(),
                        menu_name: std::ptr::null(),
                        class_name: name.as_ptr(),
                        icon_small: std::ptr::null_mut(),
                    };
                    RegisterClassExW(&class);
                    let hwnd = CreateWindowExW(WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE, name.as_ptr(), wide("Clipframes").as_ptr(), WS_POPUP, 0, 0, side, side, std::ptr::null_mut(), std::ptr::null_mut(), instance, std::ptr::null_mut());
                    if hwnd.is_null() {
                        let _ = ready.send(None);
                        return;
                    }
                    paint(hwnd, &image);
                    if protected {
                        SetWindowDisplayAffinity(hwnd, WDA_EXCLUDEFROMCAPTURE);
                    }
                    let _ = ready.send(Some((hwnd as isize, side)));
                    let mut msg: Msg = std::mem::zeroed();
                    while GetMessageW((&mut msg as *mut Msg).cast(), std::ptr::null_mut(), 0, 0) > 0 {
                        DispatchMessageW(&msg);
                    }
                })
                .ok()?;
            let (hwnd, side) = made.recv().ok()??;
            Some(Tab { hwnd, side })
        }

        /// The tab's side in physical pixels.
        pub fn side(&self) -> i32 {
            self.side
        }

        /// Shows the tab with its top-left corner at this point, in physical pixels.
        pub fn show(&self, x: i32, y: i32) {
            unsafe { PostMessageW(self.hwnd as Handle, WM_APP_SHOW, x as usize, y as isize) };
        }

        pub fn hide(&self) {
            unsafe { PostMessageW(self.hwnd as Handle, WM_APP_HIDE, 0, 0) };
        }
    }
}
