use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

use image::RgbImage;
use log::warn;
use scap::{
    capturer::{Capturer, Options, Resolution},
    frame::Frame,
};
use sysinfo::{ProcessRefreshKind, RefreshKind, System};

use crate::utils::{
    colors::ColorTarget,
    geometry::{Dimensions, Point, Region},
};

pub mod utils;

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

pub struct ScreenRecorder {
    old_frame: Arc<Mutex<Frame>>,

    pub dimensions: Dimensions,
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
        thread::spawn(move || {
            while let Ok(frame) = capturer.get_next_frame() {
                // Try to store the latest frame
                if let Ok(mut guard) = frame_clone.try_lock() {
                    *guard = frame;
                }
            }
        });

        Ok(Self {
            old_frame,
            dimensions: Dimensions { width, height },
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

impl Region {
    /// Search a color in the region
    fn search_color_impl<Xs, Ys>(
        screen: &RgbImage,
        targets: &[ColorTarget],
        xs: Xs,
        ys: &Ys,
    ) -> Option<Point>
    where
        Xs: IntoIterator<Item = u32>,
        Ys: IntoIterator<Item = u32> + Clone,
    {
        xs.into_iter()
            .flat_map(|x| ys.clone().into_iter().map(move |y| (x, y)))
            .find(|&(x, y)| targets.iter().any(|t| t.matches(*screen.get_pixel(x, y))))
            .map(|(x, y)| Point { x, y })
    }

    /// Search a color in the middle row, left to right
    #[must_use]
    pub fn search_color_mid_ltr(
        &self,
        screen: &RgbImage,
        targets: &[ColorTarget],
    ) -> Option<Point> {
        let [x_min, y_min, x_max, y_max] = self.corners();
        let y = y_min.midpoint(y_max);
        Self::search_color_impl(screen, targets, x_min..=x_max, &[y])
    }

    /// Search a color in the left half
    #[must_use]
    pub fn search_color_left_half(
        &self,
        screen: &RgbImage,
        targets: &[ColorTarget],
    ) -> Option<Point> {
        let [x_min, y_min, _, y_max] = self.corners();
        let half_width = self.get_size().width / 2;
        Self::search_color_impl(
            screen,
            targets,
            x_min..=(x_min + half_width),
            &(y_min..=y_max),
        )
    }

    /// Search a color in the right half
    #[must_use]
    pub fn search_color_right_half(
        &self,
        screen: &RgbImage,
        targets: &[ColorTarget],
    ) -> Option<Point> {
        let [_, y_min, x_max, y_max] = self.corners();
        let half_width = self.get_size().width / 2;
        Self::search_color_impl(
            screen,
            targets,
            ((x_max - half_width)..=x_max).rev(),
            &(y_min..=y_max),
        )
    }
}

pub struct Stats {
    pub enabled: bool,
    /// Shake count
    pub shakes: Box<u64>,
    /// Reel count
    pub reels: Box<u64>,
    /// Fish count
    fishes: Box<u64>,
    /// Total fishing time in seconds
    total_fishing_time: Box<u64>,
    /// Maximum fishing time in seconds
    max_fishing_time: Box<u64>,
    /// Minimum fishing time in seconds
    min_fishing_time: Box<u64>,
}

impl Stats {
    #[must_use]
    pub fn new(enabled: bool) -> Self {
        Stats {
            enabled,
            reels: Box::new(0),
            shakes: Box::new(0),
            fishes: Box::new(0),
            total_fishing_time: Box::new(0),
            max_fishing_time: Box::new(u64::MIN),
            min_fishing_time: Box::new(u64::MAX),
        }
    }

    fn print_stats(self) {
        println!("Shake count: {}", self.shakes);
        println!("Reels tries count: {}", self.reels);
        println!("Missed reels count: {}", *self.reels - *self.fishes);
        println!("Fishes count: {}", self.fishes);
        if *self.max_fishing_time != u64::MIN {
            println!(
                "Average fishing time: {}s (maximum was {}s, minimum was {}s)",
                (*self.total_fishing_time / *self.reels),
                self.max_fishing_time,
                self.min_fishing_time
            );
        }
    }

    pub fn print(self) {
        if self.enabled {
            self.print_stats();
        }
    }

    pub fn add_fishing_time(&mut self, time: u64) {
        *self.fishes += 1;
        *self.total_fishing_time += time;
        *self.max_fishing_time = (*self.max_fishing_time).max(time);
        if time > 0 {
            *self.min_fishing_time = (*self.min_fishing_time).min(time);
        }
    }
}

/// Sleep for `duration`
pub fn sleep(duration: Duration, cond: &AtomicBool) {
    let chunk = Duration::from_millis(1);
    let start = Instant::now();

    while start.elapsed() < duration && !cond.load(Ordering::Relaxed) {
        let remaining = duration.checked_sub(start.elapsed()).unwrap_or_default();
        thread::sleep(if remaining < chunk { remaining } else { chunk });
    }
}
