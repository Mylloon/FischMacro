use enigo::{
    Button,
    Coordinate::Abs,
    Direction::{Click, Press, Release},
    Enigo, Mouse, Settings,
};
use image::RgbImage;
use rand::Rng;
use std::thread::sleep;
use std::time::{Duration, Instant};
use sysinfo::{ProcessRefreshKind, RefreshKind, System};
use xcap::Monitor;

// Constants for screen dimensions
struct ScreenDimensions {
    width: u32,
    height: u32,
}

// Color targets
struct ColorTarget {
    color: (u8, u8, u8),
    variation: u8,
}

fn main() {
    println!("Starting Roblox Fishing Macro");

    // Notice
    println!("To make this program work, hide the quest (top-right book button).");

    // Check that Roblox is running
    assert!(check_running("sober"), "Roblox not found.");
    println!("Roblox found.");

    // Get screen dimensions
    let screen_dim = get_screen_dimensions().unwrap();
    println!(
        "Detected screen dimensions: {}x{}",
        screen_dim.width, screen_dim.height
    );

    // Define color targets
    let color_fish = vec![
        ColorTarget {
            color: (0x43, 0x4b, 0x5b),
            variation: 3,
        },
        ColorTarget {
            color: (0x4a, 0x4a, 0x5c),
            variation: 4,
        },
        ColorTarget {
            color: (0x47, 0x51, 0x5d),
            variation: 4,
        },
    ];

    let color_white = vec![ColorTarget {
        color: (0xff, 0xff, 0xff),
        variation: 15,
    }];

    let color_bar = vec![
        ColorTarget {
            color: (0x84, 0x85, 0x87),
            variation: 4,
        },
        ColorTarget {
            color: (0x78, 0x77, 0x73),
            variation: 4,
        },
        ColorTarget {
            color: (0x7a, 0x78, 0x73),
            variation: 4,
        },
    ];

    // Calculate regions based on screen dimensions
    let mini_game_region = calculate_mini_game_region(&screen_dim);
    let shake_region = calculate_shake_region(&screen_dim);

    // Start the main loop
    let mut last_shake_time = Instant::now();
    let mut control: Option<i32> = None;
    let mut enigo = Enigo::new(&Settings::default()).unwrap();

    // Initial reel
    reels(&mut enigo);

    loop {
        // Check for shake
        if let Some((click_x, click_y)) = check_shake(&mut enigo, &shake_region) {
            // Click at the shake position
            println!("Shake @ {click_x},{click_y}");
            enigo.move_mouse(click_x, click_y, Abs).unwrap();
            enigo.button(Button::Left, Click).unwrap();
            last_shake_time = Instant::now();
        }

        // If 7 seconds passed since last shake, reel again
        if last_shake_time.elapsed() > Duration::from_secs(7) {
            sleep(Duration::from_millis(250));
            reels(&mut enigo);
            last_shake_time = Instant::now();
        }

        // Take screenshot for processing
        let screen = take_screenshot().unwrap();

        // Find fish and white markers
        if let (Some(_), Some(_)) = (
            search_color(&screen, &color_fish, &mini_game_region),
            search_color(&screen, &color_white, &mini_game_region),
        ) {
            // Initialize control width if not set
            if control.is_none() {
                control = calculate_control_width(&screen, &mini_game_region);
                if control.is_none() {
                    // Default calculation based on screen width
                    control = Some((screen_dim.width as f32 / 800.0 * 97.0) as i32);
                }
            }

            let control_val = control.unwrap();

            // Main fishing logic loop
            loop {
                let screen = take_screenshot().unwrap();

                // Get current fish position
                let fish_pos = search_color(&screen, &color_fish, &mini_game_region);
                if fish_pos.is_none() {
                    break;
                }

                let fish_x = fish_pos.unwrap();

                // Check if fish is too far left or right
                if fish_x < mini_game_region.0 + (control_val as f32 * 0.8) as i32 {
                    enigo.button(Button::Left, Release).unwrap();
                    continue;
                } else if fish_x > mini_game_region.2 - (control_val as f32 * 0.8) as i32 {
                    enigo.button(Button::Left, Press).unwrap();
                    continue;
                }

                // Find the bar position
                let bar_pos = if let Some(white_pos) =
                    search_color(&screen, &color_white, &mini_game_region)
                {
                    white_pos + (control_val as f32 * 0.5) as i32
                } else if let Some(bar_pos) = search_color(&screen, &color_bar, &mini_game_region) {
                    bar_pos
                } else {
                    println!("Didn't find any bar color");
                    continue;
                };

                // Calculate range between fish and bar
                let range = fish_x - bar_pos;

                if range >= 0 {
                    // Positive range handling
                    enigo.button(Button::Left, Press).unwrap();
                    let hold_timer = Instant::now();
                    let original_pos = bar_pos;

                    let mut success = false;
                    loop {
                        let screen = take_screenshot().unwrap();
                        let fish_pos = search_color(&screen, &color_fish, &mini_game_region);

                        if let Some(fish_x) = fish_pos {
                            let range = fish_x - original_pos;
                            let hold_time = hold_formula(range, screen_dim.width);

                            if hold_timer.elapsed() >= Duration::from_millis(hold_time as u64) {
                                success = true;
                                break;
                            }

                            // Check if fish is still valid
                            if fish_x < mini_game_region.0
                                || fish_x > mini_game_region.2
                                || wait_for_time(&mini_game_region, 10)
                            {
                                break;
                            }
                        } else {
                            break;
                        }
                    }

                    if success {
                        enigo.button(Button::Left, Release).unwrap();
                        sleep(Duration::from_millis(
                            (hold_timer.elapsed().as_millis() as f64 * 0.6) as u64,
                        ));
                    }
                } else {
                    // Negative range handling
                    let hold_timer = Instant::now();
                    enigo.button(Button::Left, Release).unwrap();
                    let range_abs = range.abs();
                    let mut continue_now = false;

                    // Wait proportionally to the range
                    let wait_time = (hold_formula(range_abs, screen_dim.width) as f32 * 0.7) as u64;
                    if wait_for_time(&mini_game_region, wait_time) {
                        continue;
                    }

                    loop {
                        let screen = take_screenshot().unwrap();

                        // Get current fish position
                        let fish_pos = search_color(&screen, &color_fish, &mini_game_region);
                        if fish_pos.is_none()
                            || search_color(&screen, &color_white, &mini_game_region).is_some()
                        {
                            break;
                        }

                        if wait_for_time(&mini_game_region, 10) {
                            continue_now = true;
                            break;
                        }

                        // Check if bar has caught up to fish
                        if let Some(current_bar_pos) =
                            search_color(&screen, &color_bar, &mini_game_region)
                        {
                            let adjusted_pos =
                                current_bar_pos - ((screen_dim.width as f32 / 800.0) * 30.0) as i32;
                            if adjusted_pos <= fish_pos.unwrap() {
                                break;
                            }
                        }
                    }

                    if continue_now {
                        continue;
                    }

                    // Start pressing after waiting
                    enigo.button(Button::Left, Press).unwrap();
                    if wait_for_time(&mini_game_region, hold_timer.elapsed().as_millis() as u64) {
                        enigo.button(Button::Left, Release).unwrap();
                        continue;
                    }

                    enigo.button(Button::Left, Release).unwrap();
                }
            }

            // After fishing interaction, reel again
            sleep(Duration::from_secs(1));
            reels(&mut enigo);
            last_shake_time = Instant::now();
        }
    }
}

/* Helper functions */

/// Find primary monitor
fn get_monitor() -> Option<Monitor> {
    Monitor::all()
        .ok()?
        .iter()
        .find(|x| x.is_primary().unwrap_or_default())
        .cloned()
}

/// Find screen dimensions
fn get_screen_dimensions() -> Result<ScreenDimensions, &'static str> {
    let screen = get_monitor().ok_or("No primary monitor found")?;

    if let (Ok(width), Ok(height)) = (screen.width(), screen.height()) {
        Ok(ScreenDimensions { width, height })
    } else {
        Err("Can't find screen dimensions")
    }
}

/// Check if a process is running
fn check_running(name: &'static str) -> bool {
    let sys: System = System::new_with_specifics(
        RefreshKind::nothing().with_processes(ProcessRefreshKind::everything()),
    );

    sys.processes()
        .values()
        .any(|process| process.name() == name)
}

fn take_screenshot() -> Result<RgbImage, &'static str> {
    let screen = get_monitor().ok_or("No primary monitor found")?;

    let image = screen
        .capture_image()
        .map_err(|_| "Failed to capture image")?;

    RgbImage::from_raw(
        image.width(),
        image.height(),
        image
            .chunks(4)
            .flat_map(|pixel| pixel.iter().take(3)) // Take only R, G, B
            .copied()
            .collect(),
    )
    .ok_or("Failed to create RgbImage from raw data")
}

fn search_color(
    screen: &RgbImage,
    targets: &Vec<ColorTarget>,
    region: &(i32, i32, i32, i32),
) -> Option<i32> {
    let (x_min, y_min, x_max, y_max) = *region;

    for y in y_min..=y_max {
        for x in x_min..=x_max {
            if x < 0 || y < 0 || x >= screen.width() as i32 || y >= screen.height() as i32 {
                continue;
            }

            let pixel = screen.get_pixel(x as u32, y as u32);

            for target in targets {
                let (tr, tg, tb) = target.color;
                let var = target.variation;

                if (i16::from(pixel[0]) - i16::from(tr)).abs() <= i16::from(var)
                    && (i16::from(pixel[1]) - i16::from(tg)).abs() <= i16::from(var)
                    && (i16::from(pixel[2]) - i16::from(tb)).abs() <= i16::from(var)
                {
                    return Some(x);
                }
            }
        }
    }

    None
}

fn reels(enigo: &mut Enigo) {
    // Move mouse
    enigo.move_mouse(80, 400, Abs).unwrap();

    println!("Reeling...");
    // Click and hold
    enigo.button(Button::Left, Press).unwrap();
    sleep(Duration::from_millis(random_range(600, 1200)));
    enigo.button(Button::Left, Release).unwrap();
    sleep(Duration::from_millis(random_range(1000, 1200)));
}

fn calculate_mini_game_region(screen_dim: &ScreenDimensions) -> (i32, i32, i32, i32) {
    let x_left = (screen_dim.width as f32 / 2560.0 * 763.0) as i32;
    let y_left = (screen_dim.height as f32 / 1080.0 * 899.0) as i32;
    let x_right = (screen_dim.width as f32 / 2560.0 * 1796.0) as i32;
    let y_right = (screen_dim.height as f32 / 1080.0 * 939.0) as i32;

    (x_left, y_left, x_right, y_right)
}

fn calculate_shake_region(screen_dim: &ScreenDimensions) -> (i32, i32, i32, i32) {
    let x_left = (screen_dim.width as f32 / 800.0 * 100.0) as i32;
    let y_left = (screen_dim.height as f32 / 800.0 * 175.0) as i32;
    let x_right = (screen_dim.width as f32 / 800.0 * 700.0) as i32;
    let y_right = (screen_dim.height as f32 / 800.0 * 675.0) as i32;

    (x_left, y_left, x_right, y_right)
}

fn calculate_control_width(screen: &RgbImage, region: &(i32, i32, i32, i32)) -> Option<i32> {
    let (x_min, y_min, x_max, _) = *region;

    // Find white bar
    for _ in 0..50 {
        for x in x_min..=x_max {
            let y = y_min;
            if x < 0 || y < 0 || x >= screen.width() as i32 || y >= screen.height() as i32 {
                continue;
            }

            let pixel = screen.get_pixel(x as u32, y as u32);
            if pixel[0] > 240 && pixel[1] > 240 && pixel[2] > 240 {
                let control = (x_max - (x - x_min)) - x;
                if control > 0 {
                    return Some(control);
                }
            }
        }
    }

    None
}

fn hold_formula(pixel: i32, screen_width: u32) -> i32 {
    // Define data pairs for calculating hold times
    let data = [
        [0, 0],
        [16, 0],
        [132, 1],
        [217, 5],
        [365, 29],
        [450, 54],
        [534, 91],
        [632, 151],
        [736, 234],
        [817, 310],
        [900, 382],
        [997, 469],
        [1081, 541],
        [1164, 613],
        [1250, 686],
        [1347, 711],
        [1448, 721],
        [1531, 724],
        [1531, 9999],
    ];

    // Find appropriate data pair
    for i in 1..data.len() {
        let scaled_value = (data[i][1] as f32 * (screen_width as f32 / 800.0)) as i32;
        if pixel < scaled_value {
            let lower = [
                data[i - 1][0],
                (data[i - 1][1] as f32 * (screen_width as f32 / 800.0)) as i32,
            ];
            let upper = [data[i][0], scaled_value];

            // Linear interpolation
            let hold = lower[0] as f32
                + (pixel as f32 - lower[1] as f32) * (upper[0] as f32 - lower[0] as f32)
                    / (upper[1] as f32 - lower[1] as f32);

            return hold as i32;
        }
    }

    0
}

fn random_range(min: u64, max: u64) -> u64 {
    rand::rng().random_range(min..=max)
}

fn check_shake(enigo: &mut Enigo, region: &(i32, i32, i32, i32)) -> Option<(i32, i32)> {
    let (x_min, y_min, x_max, y_max) = *region;

    // Move cursor out of the region
    enigo.move_mouse(x_min, y_min - 50, Abs).unwrap();

    let image = take_screenshot().unwrap();

    for y in y_min..=y_max {
        for x in x_min..=x_max {
            let pixel = image.get_pixel(x as u32, y as u32);

            // Check for white pixel (shake indicator)
            if pixel[0] > 250 && pixel[1] > 250 && pixel[2] > 250 {
                return Some((x + 25, y));
            }
        }
    }

    None
}

fn wait_for_time(mini_game_region: &(i32, i32, i32, i32), time_ms: u64) -> bool {
    let start_time = Instant::now();

    loop {
        let screen = take_screenshot().unwrap();
        let color_fish = vec![
            ColorTarget {
                color: (0x43, 0x4b, 0x5b),
                variation: 3,
            },
            ColorTarget {
                color: (0x4a, 0x4a, 0x5c),
                variation: 4,
            },
            ColorTarget {
                color: (0x47, 0x51, 0x5d),
                variation: 4,
            },
        ];

        let fish_pos = search_color(&screen, &color_fish, mini_game_region);

        // If no fish found or fish is outside valid range, return whether fish was found
        if let Some(pos) = fish_pos {
            if pos < mini_game_region.0 || pos > mini_game_region.2 {
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
