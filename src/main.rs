use std::process::exit;
use std::thread::{sleep, spawn};
use std::time::{Duration, Instant};

use enigo::{
    Button,
    Coordinate::Abs,
    Direction::{Click, Press, Release},
    Enigo, Mouse, Settings,
};
use fischy::geometry::{Dimensions, Point, Region};
use fischy::get_roblox;
use fischy::{ScreenRecorder, check_running, raise};
use image::{Rgb, RgbImage};
use log::{error, info, warn};
use rand::Rng;
use rdev::EventType::KeyPress;
use rdev::{Event, Key, listen};
use simple_logger::SimpleLogger;

// Color targets
struct ColorTarget {
    color: Rgb<u8>,
    variation: u8,
}

impl ColorTarget {
    fn matches(&self, pixel: Rgb<u8>) -> bool {
        let Rgb([tr, tg, tb]) = self.color;
        let v = i16::from(self.variation);

        (i16::from(pixel[0]) - i16::from(tr)).abs() <= v
            && (i16::from(pixel[1]) - i16::from(tg)).abs() <= v
            && (i16::from(pixel[2]) - i16::from(tb)).abs() <= v
    }
}

fn main() {
    SimpleLogger::new()
        .with_level(log::LevelFilter::Info)
        .init()
        .unwrap_or_else(|err| eprintln!("Failed initialize logger: {err}"));
    info!("Starting Roblox Fishing Macro");

    // Notice
    warn!(
        r"To make this program work:
                                        - hide the quest (top-right book button).
                                        - be sure to run Roblox maximised on your primary screen
                                        - if asked, share your whole screen"
    );

    // Check that Roblox is running
    let roblox = get_roblox();
    assert!(check_running(roblox), "Roblox not found.");
    info!("Roblox found.");

    match raise(roblox) {
        Ok(()) => info!("Raised Roblox window"),
        Err(err) => warn!("Failed raising roblox window: {err}"),
    }

    // Define color targets
    let color_fish = vec![
        ColorTarget {
            color: Rgb([0x43, 0x4b, 0x5b]),
            variation: 3,
        },
        ColorTarget {
            color: Rgb([0x4a, 0x4a, 0x5c]),
            variation: 4,
        },
        ColorTarget {
            color: Rgb([0x47, 0x51, 0x5d]),
            variation: 4,
        },
    ];

    let color_white = vec![ColorTarget {
        color: Rgb([0xff, 0xff, 0xff]),
        variation: 15,
    }];

    let color_bar = vec![
        ColorTarget {
            color: Rgb([0x84, 0x85, 0x87]),
            variation: 4,
        },
        ColorTarget {
            color: Rgb([0x78, 0x77, 0x73]),
            variation: 4,
        },
        ColorTarget {
            color: Rgb([0x7a, 0x78, 0x73]),
            variation: 4,
        },
    ];

    // Register keybinds to close the script
    register_keybinds();

    let mut enigo = Enigo::new(&Settings::default()).expect("Failed to initialize I/O engine");
    let mut recorder = ScreenRecorder::new().expect("Failed to initialize screen monitoring");

    // Get screen dimensions
    let screen_dim = Dimensions {
        width: recorder.width,
        height: recorder.height,
    };
    info!(
        "Detected screen dimensions: {}x{}",
        screen_dim.width, screen_dim.height
    );

    // Calculate regions based on screen dimensions
    let mini_game_region = calculate_mini_game_region(&screen_dim);
    let shake_region = calculate_shake_region(&screen_dim);

    // Initial reel
    let mut last_shake_time = Instant::now();
    reels(&mut enigo, &mut last_shake_time);

    let mut rod_control_window_width: Option<i32> = None;
    loop {
        // Check for shake
        if let Some(Point { x, y }) = check_shake(&mut enigo, &mut recorder, &shake_region) {
            // Click at the shake position
            info!("Shake @ {x},{y}");
            enigo
                .move_mouse(x.cast_signed(), y.cast_signed(), Abs)
                .expect("Failed moving mouse to shake bubble");
            enigo
                .button(Button::Left, Click)
                .expect("Failed clicking to shake bubble");

            last_shake_time = Instant::now();
            continue;
        }

        // If 7 seconds passed since last shake, reel again
        if last_shake_time.elapsed() > Duration::from_secs(7) {
            reels(&mut enigo, &mut last_shake_time);
            continue;
        }

        // Take screenshot for processing
        let screen = recorder
            .take_screenshot()
            .expect("Failed taking screenshot");

        // Find fish and white markers
        if let (Some(_), Some(_)) = (
            search_color(&screen, &color_fish, &mini_game_region),
            search_color(&screen, &color_white, &mini_game_region),
        ) {
            // Initialize control width if not set
            if rod_control_window_width.is_none() {
                rod_control_window_width = Some(
                    calculate_control_width(&screen, &mini_game_region)
                        .unwrap_or((mini_game_region.get_size().width as f32 * 0.12) as i32),
                );
            }

            let control_val = rod_control_window_width.unwrap_or_default();

            // Main fishing logic loop
            info!("Fishing...");
            fishing_loop(
                &mut enigo,
                &mut recorder,
                &color_fish,
                &color_bar,
                &color_white,
                &mini_game_region,
                control_val,
                &screen_dim,
            );
            info!("Fishing ended!");

            // After fishing interaction, reel again
            sleep(Duration::from_secs(2));
            reels(&mut enigo, &mut last_shake_time);
        }
    }
}

/// Search a specific colors in the region
fn search_color(screen: &RgbImage, targets: &[ColorTarget], region: &Region) -> Option<Point> {
    let [x_min, y_min, x_max, y_max] = region.corners();

    for y in y_min..=y_max {
        for x in x_min..=x_max {
            if x >= screen.width() || y >= screen.height() {
                continue;
            }

            let pixel = screen.get_pixel(x, y);
            if targets.iter().any(|t| t.matches(*pixel)) {
                return Some(Point { x, y });
            }
        }
    }

    None
}

#[allow(clippy::too_many_arguments)]
fn fishing_loop(
    enigo: &mut Enigo,
    recorder: &mut ScreenRecorder,
    color_fish: &[ColorTarget],
    color_bar: &[ColorTarget],
    color_white: &[ColorTarget],
    mini_game_region: &Region,
    control_val: i32,
    screen_dim: &Dimensions,
) {
    warn!("init with c={control_val}");
    // let [width, height] = mini_game_region.get_size();
    // let screen_dim = ScreenDimensions { width, height };
    loop {
        let screen = recorder.take_screenshot().unwrap();

        // Get current fish position
        let fish_x =
            if let Some(Point { x, .. }) = search_color(&screen, color_fish, mini_game_region) {
                info!("found fish at {x}");
                x.cast_signed()
            } else {
                warn!("ending 0: nofish");
                enigo.button(Button::Left, Release).expect("Packup the rod");
                break;
            };

        // Check if fish is very far left or very far right
        if fish_x < mini_game_region.point1.x.cast_signed() + (control_val as f32 * 0.8) as i32 {
            info!("Giving some slack...");
            enigo
                .button(Button::Left, Release)
                .expect("Failed giving slack");
            continue;
        } else if fish_x
            > mini_game_region.point2.x.cast_signed() - (control_val as f32 * 0.8) as i32
        {
            info!("Tighting the line...");
            enigo
                .button(Button::Left, Press)
                .expect("Failed keeping the line tight");
            continue;
        }

        // Find the bar position
        let bar_pos = if let Some(white_pos) =
            search_color(&screen, color_white, mini_game_region).map(|p| p.x.cast_signed())
        {
            white_pos + (control_val as f32 * 0.5) as i32
        } else if let Some(bar_pos) =
            search_color(&screen, color_bar, mini_game_region).map(|p| p.x.cast_signed())
        {
            error!("How we call that?");
            bar_pos
        } else {
            info!("Didn't find any bar color");

            warn!("ending1");
            enigo.button(Button::Left, Release).expect("Packup the rod");
            break; // FIXME: should we break ?
        };

        // Calculate range between fish and bar
        let range = fish_x - bar_pos;
        info!("Distance fish/position: {range}");

        if range >= 0 {
            info!("==> vers la droite!");
            // Positive range handling
            enigo.button(Button::Left, Press).expect("Clicking failed");
            let hold_timer = Instant::now();
            let current_pos = bar_pos;

            // Vérifie 1 truc
            let mut success = false;
            loop {
                let screen = recorder
                    .take_screenshot()
                    .expect("Taking screenshot failed");

                if let Some(fish_x) =
                    search_color(&screen, color_fish, mini_game_region).map(|p| p.x.cast_signed())
                {
                    let range = fish_x - current_pos;
                    let hold_time = hold_formula(range, &mini_game_region.get_size());

                    info!("found fish at {fish_x} - range is {range} - will hold for {hold_time}");
                    if hold_timer.elapsed() >= Duration::from_millis(hold_time as u64) {
                        success = true;
                        break;
                    }

                    // Check if fish is still valid
                    info!("Hold until elapsed or fish left region invalid");
                    if fish_x < mini_game_region.point1.x.cast_signed()
                        || fish_x > mini_game_region.point2.x.cast_signed()
                        || wait_for_time(recorder, mini_game_region, 10, color_fish)
                    {
                        break;
                    }
                } else {
                    info!("did not foud any fish");
                    break;
                }
            }

            enigo
                .button(Button::Left, Release)
                .expect("Releasing after losing fish failed");
            if success {
                sleep(Duration::from_millis(
                    (hold_timer.elapsed().as_millis() as f64 * 0.6) as u64,
                ));
            }
        } else {
            info!("==> vers la gauche!");
            // Negative range handling
            let hold_timer = Instant::now();
            enigo.button(Button::Left, Release).unwrap();
            let range_abs = range.abs();
            let mut continue_now = false;

            // Wait proportionally to the range
            let wait_time =
                (hold_formula(range_abs, &mini_game_region.get_size()) as f32 * 0.7) as u64;
            if wait_for_time(recorder, mini_game_region, wait_time, color_fish) {
                continue;
            }

            loop {
                let screen = recorder
                    .take_screenshot()
                    .expect("Taking screenshot failed");

                // Get current fish position
                let fish_pos = search_color(&screen, color_fish, mini_game_region);
                if fish_pos.is_none()
                    || search_color(&screen, color_white, mini_game_region).is_some()
                {
                    break;
                }

                if wait_for_time(recorder, mini_game_region, 10, color_fish) {
                    continue_now = true;
                    break;
                }

                // Check if bar has caught up to fish
                if let Some(current_bar_pos) =
                    search_color(&screen, color_bar, mini_game_region).map(|p| p.x.cast_signed())
                {
                    let adjusted_pos = current_bar_pos - (screen_dim.width as f32 * 0.04) as i32;
                    if adjusted_pos
                        <= fish_pos
                            .map(|p| p.x.cast_signed())
                            .expect("Fish not found?")
                    {
                        break;
                    }
                }
            }

            if continue_now {
                continue;
            }

            // Start pressing after waiting
            enigo.button(Button::Left, Press).expect("Can't click");
            wait_for_time(
                recorder,
                mini_game_region,
                hold_timer.elapsed().as_millis() as u64,
                color_fish,
            );
            enigo
                .button(Button::Left, Release)
                .expect("Can't release click");
        }
    }
}

/// Start the fishing process
fn reels(enigo: &mut Enigo, last_shake_time: &mut Instant) {
    // Move mouse
    enigo.move_mouse(80, 400, Abs).expect("Can't move mouse");

    println!("Reeling...");
    // Casting motion
    enigo
        .button(Button::Left, Press)
        .expect("Can't backswing: failed to press mouse button");
    sleep(Duration::from_millis(rand::rng().random_range(600..=1200)));
    enigo
        .button(Button::Left, Release)
        .expect("Can't release the line: failed to release mouse button");

    sleep(Duration::from_millis(rand::rng().random_range(1000..=1200)));
    *last_shake_time = Instant::now();
}

/// Find where is the mini-game bar
fn calculate_mini_game_region(screen_dim: &Dimensions) -> Region {
    Region {
        point1: Point {
            x: (screen_dim.width as f32 * 0.28) as u32,
            y: (screen_dim.height as f32 * 0.8) as u32,
        },
        point2: Point {
            x: (screen_dim.width as f32 * 0.72) as u32,
            y: (screen_dim.height as f32 * 0.85) as u32,
        },
    }
}

/// Find where shake bubble appears
fn calculate_shake_region(screen_dim: &Dimensions) -> Region {
    Region {
        point1: Point {
            x: (screen_dim.width as f32 * 0.005) as u32,
            y: (screen_dim.height as f32 * 0.185) as u32,
        },
        point2: Point {
            x: (screen_dim.width as f32 * 0.84) as u32,
            y: (screen_dim.height as f32 * 0.65) as u32,
        },
    }
}

/// Find the width of the control bar
fn calculate_control_width(screen: &RgbImage, region: &Region) -> Option<i32> {
    let [x_min, y_min, x_max, y_max] = region.corners().map(u32::cast_signed);
    let middle_y = y_min + (y_max - y_min) / 2;

    let is_white = |x: i32| {
        (0..screen.width() as i32).contains(&x) && {
            let p = screen.get_pixel(x as u32, middle_y as u32);
            p[0] > 234 && p[1] > 234 && p[2] > 234
        }
    };

    let left = (x_min..=x_max).find(|&x| is_white(x));
    let right = (x_min..=x_max).rev().find(|&x| is_white(x));

    left.zip(right).map(|(l, r)| r - l)
}

fn hold_formula(gap: i32, full_area: &Dimensions) -> i32 {
    info!("info: gap={gap} , w={}", full_area.width);
    ((300. * gap as f32 / ((full_area.width as f32) * 0.5)) as i32).clamp(0, 2000)
}

/// Returns the coordinates of the shake bubble
fn check_shake(enigo: &mut Enigo, screen: &mut ScreenRecorder, region: &Region) -> Option<Point> {
    let [x_min, y_min, x_max, y_max] = region.corners().map(u32::cast_signed);

    // Move cursor out of the region
    enigo
        .move_mouse(x_max, y_max + 30, Abs)
        .expect("Can't move mouse");

    // Sleep to be sure the screenshot won't capture our cursor
    // (Sober creating a custom cursor)
    #[cfg(target_os = "linux")]
    sleep(Duration::from_millis(300));

    let image = screen.take_screenshot().expect("Can't take screenshot");

    let white = ColorTarget {
        color: Rgb([0xff, 0xff, 0xff]),
        variation: 1,
    };

    for y in y_min..=y_max {
        for x in x_min..=x_max {
            let pixel = image.get_pixel(x as u32, y as u32);

            // Check for white pixel (shake indicator)
            if white.matches(*pixel) {
                // Wait because why not (FIXME WHYYYYYYYY)
                sleep(Duration::from_millis(100));

                return Some(Point {
                    x: x.cast_unsigned() + 20,
                    y: y.cast_unsigned() + 10,
                });
            }
        }
    }

    None
}

fn wait_for_time(
    recorder: &mut ScreenRecorder,
    mini_game_region: &Region,
    time_ms: u64,
    color_fish: &[ColorTarget],
) -> bool {
    let start_time = Instant::now();

    loop {
        let screen = recorder.take_screenshot().expect("Can't take screenshot");

        let fish_pos =
            search_color(&screen, color_fish, mini_game_region).map(|p| p.x.cast_signed());

        // If no fish found or fish is outside valid range, return whether fish was found
        if let Some(pos) = fish_pos {
            if pos < mini_game_region.point1.x.cast_signed()
                || pos > mini_game_region.point2.x.cast_signed()
            {
                return true;
            }
        } else {
            return false;
        }

        // If we've waited long enough, break the loop
        if start_time.elapsed() > Duration::from_millis(time_ms) {
            break;
        }
    }

    false
}

fn register_keybinds() {
    spawn(|| {
        listen(|e| {
            if let Event {
                event_type: KeyPress(Key::Escape | Key::Return | Key::Space),
                ..
            } = e
            {
                info!("Closing due to key press...");
                exit(0)
            }
        })
        .expect("Can't listen to keyboard");
    });
}
