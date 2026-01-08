use std::process::exit;
use std::thread::{sleep, spawn};
use std::time::{Duration, Instant};

use clap::Parser;
use enigo::{
    Button,
    Coordinate::Abs,
    Direction::{Click, Press, Release},
    Enigo, Mouse, Settings,
};
use fischy::colors::{COLOR_BAR, COLOR_FISH, COLOR_WHITE, ColorTarget};
use fischy::geometry::{Dimensions, Point, Region};
use fischy::get_roblox_executable_name;
use fischy::helpers::BadCast;
use fischy::{ScreenRecorder, check_running, raise};
use image::{Rgb, RgbImage};
use log::{error, info, warn};
use rand::Rng;
use rdev::EventType::KeyPress;
use rdev::{Event, Key, listen};

#[derive(Parser)]
#[command(
    version,
    about,
    long_about = r#"
To make this program work:
    - hide the quest (top-right book button) and scoreboard (tab)
    - be sure to run Roblox maximised on your primary screen"#
)]
struct Args {
    /// Write images for debugging purposes
    #[arg(long, default_value_t = false)]
    debug: bool,
}

fn init_logger(debug_mode: bool) {
    let mut builder = env_logger::Builder::new();
    if debug_mode {
        builder.filter_level(log::LevelFilter::Info);
    } else {
        builder.filter_level(log::LevelFilter::Warn);
    }
    builder.init();
}

fn main() {
    let args = Args::parse();
    init_logger(args.debug);

    info!("Starting Roblox Fishing Macro");
    if args.debug {
        info!("== Debug mode enabled ==");
    }

    // Check that Roblox is running
    let roblox = get_roblox_executable_name();
    assert!(check_running(roblox), "Roblox not found.");
    info!("Roblox found.");

    match raise(roblox) {
        Ok(()) => info!("Raised Roblox window"),
        Err(err) => warn!("Failed raising roblox window: {err}"),
    }

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
            search_color(&screen, COLOR_FISH, &mini_game_region),
            search_color(&screen, COLOR_WHITE, &mini_game_region),
        ) {
            // Initialize control width if not set
            if rod_control_window_width.is_none() {
                rod_control_window_width = Some(
                    calculate_control_width(&screen, &mini_game_region).unwrap_or(
                        (mini_game_region.get_size().width.cast_signed().bad_cast() * 0.12)
                            .bad_cast(),
                    ),
                );
            }

            // Main fishing logic loop
            info!("Fishing...");
            fishing_loop(
                &mut enigo,
                &mut recorder,
                COLOR_FISH,
                COLOR_BAR,
                COLOR_WHITE,
                &mini_game_region,
                rod_control_window_width.unwrap_or_default(),
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

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
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
    loop {
        let screen = recorder
            .take_screenshot()
            .expect("Couldn't take screenshot");

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
        if fish_x
            < mini_game_region.point1.x.cast_signed() + (control_val.bad_cast() * 0.8).bad_cast()
        {
            info!("Giving some slack...");
            enigo
                .button(Button::Left, Release)
                .expect("Failed giving slack");
            continue;
        } else if fish_x
            > mini_game_region.point2.x.cast_signed() - (control_val.bad_cast() * 0.8).bad_cast()
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
            white_pos + (control_val.bad_cast() * 0.5).bad_cast()
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
                    if hold_timer.elapsed()
                        >= Duration::from_millis(hold_time.cast_unsigned().into())
                    {
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
                    info!("did not found any fish");
                    break;
                }
            }

            enigo
                .button(Button::Left, Release)
                .expect("Releasing after losing fish failed");
            if success {
                sleep(Duration::from_millis(u64::from(
                    (i32::try_from(hold_timer.elapsed().as_millis())
                        .unwrap_or_default()
                        .bad_cast()
                        * 0.6)
                        .bad_cast()
                        .cast_unsigned(),
                )));
            }
        } else {
            info!("==> vers la gauche!");
            // Negative range handling
            let hold_timer = Instant::now();
            enigo
                .button(Button::Left, Release)
                .expect("Can't release click");
            let range_abs = range.abs();
            let mut continue_now = false;

            // Wait proportionally to the range
            let wait_time: u64 = (hold_formula(range_abs, &mini_game_region.get_size()).bad_cast()
                * 0.7)
                .bad_cast()
                .try_into()
                .unwrap_or_default();
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
                    let adjusted_pos = current_bar_pos
                        - (screen_dim.width.cast_signed().bad_cast() * 0.04).bad_cast();
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
                u64::try_from(hold_timer.elapsed().as_millis()).unwrap_or_default(),
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
            x: (screen_dim.width.cast_signed().bad_cast() * 0.28)
                .bad_cast()
                .cast_unsigned(),
            y: (screen_dim.height.cast_signed().bad_cast() * 0.8)
                .bad_cast()
                .cast_unsigned(),
        },
        point2: Point {
            x: (screen_dim.width.cast_signed().bad_cast() * 0.72)
                .bad_cast()
                .cast_unsigned(),
            y: (screen_dim.height.cast_signed().bad_cast() * 0.85)
                .bad_cast()
                .cast_unsigned(),
        },
    }
}

/// Find where shake bubble appears
fn calculate_shake_region(screen_dim: &Dimensions) -> Region {
    Region {
        point1: Point {
            x: (screen_dim.width.cast_signed().bad_cast() * 0.005)
                .bad_cast()
                .cast_unsigned(),
            y: (screen_dim.height.cast_signed().bad_cast() * 0.185)
                .bad_cast()
                .cast_unsigned(),
        },
        point2: Point {
            x: (screen_dim.width.cast_signed().bad_cast() * 0.84)
                .bad_cast()
                .cast_unsigned(),
            y: (screen_dim.height.cast_signed().bad_cast() * 0.65)
                .bad_cast()
                .cast_unsigned(),
        },
    }
}

/// Find the width of the control bar
fn calculate_control_width(screen: &RgbImage, region: &Region) -> Option<i32> {
    let [x_min, y_min, x_max, y_max] = region.corners().map(u32::cast_signed);
    let middle_y = y_min + (y_max - y_min) / 2;

    let is_white = |x: i32| {
        (0..screen.width().cast_signed()).contains(&x) && {
            let p = screen.get_pixel(x.cast_unsigned(), middle_y.cast_unsigned());
            p[0] > 234 && p[1] > 234 && p[2] > 234
        }
    };

    let left = (x_min..=x_max).find(|&x| is_white(x));
    let right = (x_min..=x_max).rev().find(|&x| is_white(x));

    left.zip(right).map(|(l, r)| r - l)
}

fn hold_formula(gap: i32, full_area: &Dimensions) -> i32 {
    info!("info: gap={gap} , w={}", full_area.width);
    ((500. * gap.bad_cast() / ((full_area.width.cast_signed().bad_cast()) * 0.5)).bad_cast())
        .clamp(200, 2000)
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
            let pixel = image.get_pixel(x.cast_unsigned(), y.cast_unsigned());

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
