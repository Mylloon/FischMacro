use std::{
    sync::{Arc, Mutex},
    thread::spawn,
};

use image::RgbImage;
use log::warn;
use scap::{
    capturer::{Capturer, Options, Resolution},
    frame::Frame,
};
use sysinfo::{ProcessRefreshKind, RefreshKind, System};

#[cfg(target_os = "linux")]
use crate::utils::kwin::{search_windows_kde, window_activate_kde};
use crate::utils::{
    colors::ColorTarget,
    geometry::{Point, Region},
};

mod utils;

pub use crate::utils::{args, colors, fishing, geometry, helpers};

#[must_use]
#[cfg(target_os = "linux")]
pub fn get_roblox_executable_name<'a>() -> &'a str {
    "sober"
}

#[cfg(target_os = "windows")]
pub fn get_roblox_executable_name<'a>() -> &'a str {
    "RobloxPlayerBeta.exe"
}

/// Check if a process is running
#[must_use]
pub fn check_running(name: &str) -> bool {
    let sys = System::new_with_specifics(
        RefreshKind::nothing().with_processes(ProcessRefreshKind::everything()),
    );

    sys.processes()
        .values()
        .any(|process| process.name() == name)
}

/// Raise a window
///
/// # Errors
/// Erroring if can't raise the asked program
#[cfg(target_os = "linux")]
pub fn raise(program: &str) -> Result<(), String> {
    // KDE specific
    if is_kde() {
        search_windows_kde(program)
            .and_then(|s| window_activate_kde(&s))
            // TODO...
            .map_err(|err| format!("Something went wrong: {err:#?}."))
    } else {
        Err("Only KDE Plasma is supported on Linux.".into())
    }
}

#[cfg(target_os = "windows")]
pub fn raise(_program: &str) -> Result<(), String> {
    Err("Windows is not currently supported".into())
}

#[cfg(target_os = "linux")]
fn is_kde() -> bool {
    std::env::var("DESKTOP_SESSION")
        .map(|v| v.eq("plasma"))
        .unwrap_or(false)
}

pub struct ScreenRecorder {
    old_frame: Arc<Mutex<Frame>>,

    pub width: u32,
    pub height: u32,
}

impl ScreenRecorder {
    /// Initialize screen recording
    ///
    /// # Errors
    /// Can't capture screen
    pub fn new() -> Result<Self, String> {
        // Check if the platform is supported
        if !scap::is_supported() {
            return Err("Platform not supported".into());
        }

        // Check if we have permission to capture screen
        // If we don't, request it.
        if !scap::has_permission() {
            warn!("Permission not granted. Requesting permission...");
            if !scap::request_permission() {
                return Err("Permission denied".into());
            }
        }

        // Create capturer on primary display
        let mut capturer = Capturer::build(Options {
            fps: 10,
            target: None, // None means primary display
            show_cursor: false,
            show_highlight: false, // border around what is being captured
            output_resolution: Resolution::Captured,
            ..Default::default()
        })
        .map_err(|e| format!("Can't capture the screen: {e}"))?;

        // Start capturing
        capturer.start_capture();

        #[cfg(not(target_os = "linux"))]
        let [width, height] = capturer.get_output_frame_size();

        // Compute from a frame
        #[cfg(target_os = "linux")]
        let [width, height] = capturer
            .get_next_frame()
            .map(|frame| match frame {
                Frame::YUVFrame(f) => [f.width, f.height],
                Frame::RGB(f) => [f.width, f.height],
                Frame::RGBx(f) => [f.width, f.height],
                Frame::XBGR(f) => [f.width, f.height],
                Frame::BGRx(f) => [f.width, f.height],
                Frame::BGR0(f) => [f.width, f.height],
                Frame::BGRA(f) => [f.width, f.height],
            })
            .map(|l| l.map(i32::cast_unsigned))
            .map_err(|e| format!("{e}"))?;

        // We will always have a frame
        let first_frame = capturer
            .get_next_frame()
            .map_err(|e| format!("Can't receive frames: {e}"))?;
        let old_frame = Arc::new(Mutex::new(first_frame));

        // We have to create a thread that consume all our frames to prevent a memory explosion
        let frame_clone = Arc::clone(&old_frame);
        spawn(move || {
            while let Ok(frame) = capturer.get_next_frame() {
                // Try to store the latest frame
                if let Ok(mut guard) = frame_clone.try_lock() {
                    *guard = frame;
                }
            }
        });

        Ok(Self {
            old_frame,
            width,
            height,
        })
    }

    fn take_frame(&mut self) -> Result<Frame, String> {
        match self.old_frame.lock() {
            Ok(f) => Ok(f.clone()),
            Err(e) => Err(format!("Can't read stored frame: {e}")),
        }
    }

    /// Take a screenshot
    ///
    /// # Errors
    /// Received unprocessable frame
    pub fn take_screenshot(&mut self) -> Result<RgbImage, String> {
        self.take_frame()
            .and_then(|f| match f {
                Frame::RGB(rgb) => Ok(RgbImage::from_raw(
                    rgb.width.cast_unsigned(),
                    rgb.height.cast_unsigned(),
                    rgb.data,
                )),
                Frame::RGBx(rgb) => Ok(RgbImage::from_raw(
                    rgb.width.cast_unsigned(),
                    rgb.height.cast_unsigned(),
                    rgb.data
                        .chunks(4)
                        .flat_map(|pixel| pixel.iter().take(3))
                        .copied()
                        .collect(),
                )),
                Frame::XBGR(bgr) => Ok(RgbImage::from_raw(
                    bgr.width.cast_unsigned(),
                    bgr.height.cast_unsigned(),
                    bgr.data
                        .chunks(4)
                        .flat_map(|pixel| [pixel[3], pixel[2], pixel[1]])
                        .collect(),
                )),
                Frame::BGRx(bgr) => Ok(RgbImage::from_raw(
                    bgr.width.cast_unsigned(),
                    bgr.height.cast_unsigned(),
                    bgr.data
                        .chunks(4)
                        .flat_map(|pixel| [pixel[2], pixel[1], pixel[0]])
                        .collect(),
                )),
                Frame::BGR0(bgr) => Ok(RgbImage::from_raw(
                    bgr.width.cast_unsigned(),
                    bgr.height.cast_unsigned(),
                    bgr.data
                        .chunks(4)
                        .flat_map(|pixel| [pixel[2], pixel[1], pixel[0]])
                        .collect(),
                )),
                Frame::BGRA(bgra) => Ok(RgbImage::from_raw(
                    bgra.width.cast_unsigned(),
                    bgra.height.cast_unsigned(),
                    bgra.data
                        .chunks(4)
                        .flat_map(|pixel| [pixel[2], pixel[1], pixel[0]])
                        .collect(),
                )),
                Frame::YUVFrame(_) => unimplemented!(),
            })
            .and_then(|f| f.ok_or("Can't convert image from raw data".into()))
    }
}

/// Search a specific colors in the region from left to right
///
/// # Panics
/// tmps
#[must_use]
pub fn search_color_ltr(
    screen: &RgbImage,
    targets: &[ColorTarget],
    region: &Region,
) -> Option<Point> {
    let [x_min, y_min, x_max, y_max] = region.corners();

    let y = y_min.midpoint(y_max);

    (x_min..=x_max)
        .find(|&x| targets.iter().any(|t| t.matches(*screen.get_pixel(x, y))))
        .map(|x| Point { x, y })
}
