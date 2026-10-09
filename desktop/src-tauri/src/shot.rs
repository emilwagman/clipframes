//! Pictures of the screen: the element just picked, a dragged area, the frames of a clip.
//!
//! Clipframes' own windows are marked as protected content, so the system leaves them out and a
//! picture shows the app underneath, without the highlight or the bar on top.

use crate::element::Rect;
use std::path::Path;

/// Saves a picture of `rect` (in the units the picker reports) as a PNG and returns its size
/// in pixels. `max_width` shrinks wide pictures: clip frames do not need every pixel.
pub fn capture_to_file(rect: &Rect, path: &Path, max_width: Option<u32>) -> Result<(u32, u32), String> {
    if rect.width < 1.0 || rect.height < 1.0 {
        return Err("Nothing to capture.".into());
    }
    platform::capture_to_file(rect, path, max_width)
}

/// Width and height from a PNG file's header.
pub fn png_size(path: &Path) -> Option<(u32, u32)> {
    use std::io::Read;
    let mut head = [0u8; 24];
    std::fs::File::open(path).ok()?.read_exact(&mut head).ok()?;
    (&head[1..4] == b"PNG").then(|| (u32::from_be_bytes([head[16], head[17], head[18], head[19]]), u32::from_be_bytes([head[20], head[21], head[22], head[23]])))
}

/// RGBA pixels, top row first.
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl Image {
    /// The same picture at most `max_width` wide, each new pixel the average of those it covers.
    pub fn fit(self, max_width: u32) -> Image {
        if self.width <= max_width || max_width == 0 {
            return self;
        }
        let (sw, sh) = (self.width as usize, self.height as usize);
        let dw = max_width as usize;
        let dh = (sh * dw / sw).max(1);
        let mut out = vec![0u8; dw * dh * 4];
        for y in 0..dh {
            let (y0, y1) = (y * sh / dh, ((y + 1) * sh / dh).max(y * sh / dh + 1).min(sh));
            for x in 0..dw {
                let (x0, x1) = (x * sw / dw, ((x + 1) * sw / dw).max(x * sw / dw + 1).min(sw));
                let mut sum = [0u32; 4];
                for sy in y0..y1 {
                    let row = &self.rgba[(sy * sw + x0) * 4..(sy * sw + x1) * 4];
                    for px in row.chunks_exact(4) {
                        for c in 0..4 {
                            sum[c] += px[c] as u32;
                        }
                    }
                }
                let n = ((y1 - y0) * (x1 - x0)) as u32;
                let o = (y * dw + x) * 4;
                for c in 0..4 {
                    out[o + c] = (sum[c] / n) as u8;
                }
            }
        }
        Image { width: dw as u32, height: dh as u32, rgba: out }
    }

    pub fn save_png(&self, path: &Path) -> Result<(), String> {
        let file = std::fs::File::create(path).map_err(|e| e.to_string())?;
        let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), self.width, self.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        // Screens are flat colour and text: the fast setting already shrinks them well.
        encoder.set_compression(png::Compression::Fast);
        let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
        writer.write_image_data(&self.rgba).map_err(|e| e.to_string())
    }
}

#[cfg(windows)]
mod platform {
    use super::Image;
    use crate::element::Rect;
    use std::ffi::c_void;
    use std::path::Path;

    type Handle = *mut c_void;

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

    #[link(name = "user32")]
    extern "system" {
        fn GetDC(hwnd: Handle) -> Handle;
        fn ReleaseDC(hwnd: Handle, dc: Handle) -> i32;
    }

    #[link(name = "gdi32")]
    extern "system" {
        fn CreateCompatibleDC(dc: Handle) -> Handle;
        fn CreateCompatibleBitmap(dc: Handle, width: i32, height: i32) -> Handle;
        fn SelectObject(dc: Handle, object: Handle) -> Handle;
        fn BitBlt(dest: Handle, x: i32, y: i32, width: i32, height: i32, src: Handle, sx: i32, sy: i32, rop: u32) -> i32;
        fn GetDIBits(dc: Handle, bitmap: Handle, start: u32, lines: u32, bits: *mut c_void, info: *mut BitmapInfoHeader, usage: u32) -> i32;
        fn DeleteObject(object: Handle) -> i32;
        fn DeleteDC(dc: Handle) -> i32;
    }

    const SRCCOPY: u32 = 0x00CC_0020;
    /// Includes layered windows of other apps (menus, tooltips). Ours are protected, so left out.
    const CAPTUREBLT: u32 = 0x4000_0000;

    fn capture(rect: &Rect) -> Result<Image, String> {
        let (x, y, w, h) = (rect.x.round() as i32, rect.y.round() as i32, rect.width.round() as i32, rect.height.round() as i32);
        unsafe {
            let screen = GetDC(std::ptr::null_mut());
            if screen.is_null() {
                return Err("No screen to capture.".into());
            }
            let memory = CreateCompatibleDC(screen);
            let bitmap = CreateCompatibleBitmap(screen, w, h);
            let previous = SelectObject(memory, bitmap);
            let copied = BitBlt(memory, 0, 0, w, h, screen, x, y, SRCCOPY | CAPTUREBLT);
            let mut info = BitmapInfoHeader { size: 40, width: w, height: -h, planes: 1, bit_count: 32, compression: 0, size_image: 0, x_ppm: 0, y_ppm: 0, clr_used: 0, clr_important: 0 };
            let mut pixels = vec![0u8; (w as usize) * (h as usize) * 4];
            let lines = if copied != 0 { GetDIBits(memory, bitmap, 0, h as u32, pixels.as_mut_ptr().cast(), &mut info, 0) } else { 0 };
            SelectObject(memory, previous);
            DeleteObject(bitmap);
            DeleteDC(memory);
            ReleaseDC(std::ptr::null_mut(), screen);
            if lines == 0 {
                return Err("The screen could not be read.".into());
            }
            // The system hands back blue, green, red and an unused byte.
            for px in pixels.chunks_exact_mut(4) {
                px.swap(0, 2);
                px[3] = 255;
            }
            Ok(Image { width: w as u32, height: h as u32, rgba: pixels })
        }
    }

    pub fn capture_to_file(rect: &Rect, path: &Path, max_width: Option<u32>) -> Result<(u32, u32), String> {
        let image = capture(rect)?;
        let image = match max_width {
            Some(max) => image.fit(max),
            None => image,
        };
        image.save_png(path)?;
        Ok((image.width, image.height))
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use crate::element::Rect;
    use std::path::Path;
    use std::process::Command;

    /// The system's own tool writes the file. It asks for Screen Recording permission in
    /// Clipframes' name the first time, and leaves protected windows out.
    pub fn capture_to_file(rect: &Rect, path: &Path, _max_width: Option<u32>) -> Result<(u32, u32), String> {
        let region = format!("{},{},{},{}", rect.x.round(), rect.y.round(), rect.width.round(), rect.height.round());
        let status = Command::new("/usr/sbin/screencapture").args(["-x", "-t", "png", "-R", &region]).arg(path).status().map_err(|e| e.to_string())?;
        if !status.success() {
            return Err("The screen could not be captured. Allow Clipframes under Screen Recording in System Settings.".into());
        }
        super::png_size(path).ok_or_else(|| "The screen could not be captured. Allow Clipframes under Screen Recording in System Settings.".to_string())
    }
}

#[cfg(not(any(windows, target_os = "macos")))]
mod platform {
    use crate::element::Rect;
    use std::path::Path;

    pub fn capture_to_file(_rect: &Rect, _path: &Path, _max_width: Option<u32>) -> Result<(u32, u32), String> {
        Err("Screenshots are not built for Linux yet.".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shrinking_averages_the_pixels_it_covers() {
        // Two by two, black and white in a checker: every half is mid grey.
        let image = Image { width: 2, height: 2, rgba: vec![0, 0, 0, 255, 255, 255, 255, 255, 255, 255, 255, 255, 0, 0, 0, 255] };
        let small = image.fit(1);
        assert_eq!((small.width, small.height), (1, 1));
        assert_eq!(small.rgba, vec![127, 127, 127, 255]);
    }

    #[test]
    fn a_picture_already_narrow_enough_is_left_alone() {
        let image = Image { width: 3, height: 1, rgba: vec![9; 12] };
        let same = image.fit(1600);
        assert_eq!((same.width, same.height, same.rgba.len()), (3, 1, 12));
    }

    #[test]
    fn a_saved_png_reports_its_size() {
        let path = std::env::temp_dir().join(format!("clipframes-shot-{}.png", std::process::id()));
        Image { width: 5, height: 3, rgba: vec![200; 60] }.save_png(&path).unwrap();
        assert_eq!(png_size(&path), Some((5, 3)));
        std::fs::remove_file(&path).unwrap();
    }
}
