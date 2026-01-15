use std::ops::AddAssign;
use std::process::exit;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use clap::Parser;
use enigo::{
    Axis::Vertical,
    Button,
    Coordinate::{Abs, Rel},
    Direction::{Click, Press, Release},
    Enigo, Mouse, Settings,
};
use fischy::utils::clickers::{
    appraise_items, fetch_crab_cages, place_crab_cages, sell_items, summon_totem,
};
use fischy::utils::fishing::MiniGame;
use fischy::utils::{
    colors::ColorTarget,
    fishing::Rod,
    geometry::{Dimensions, Point, Region},
    helpers::BadCast,
};
use fischy::{ScreenRecorder, Scroller, Stats, check_running, get_roblox_executable_name, sleep};
use image::Rgb;
use log::{debug, info, warn};
use rand::Rng;
use rdev::{Event, EventType::KeyPress, Key, listen};
use window_raiser::raise;

static SHUTDOWN: AtomicBool = AtomicBool::new(false);

#[derive(Parser)]
#[command(
    version,
    about,
    long_about = r#"
To make this program work:
    - hide the quest (top-right book button) and scoreboard (tab)
    - be sure to run Roblox maximised on your primary screen"#
)]
#[allow(clippy::struct_excessive_bools)]
struct Args {
    /// Sleep between shakes to prevent capturing the cursor
    #[arg(long, default_value_t = 400)]
    lag: u32,

    /// Maximum shake count
    #[arg(short, long, default_value_t = 20)]
    max_shake_count: u8,

    /// Do only the shake part
    #[arg(long, default_value_t = false)]
    shake_only: bool,

    /// Compute and print stats
    #[arg(short, long, default_value_t = false)]
    stats: bool,

    /// Disable camera setup looking down at the start
    #[arg(long)]
    no_camera_setup: bool,

    /// Debugging purposes
    #[arg(short, long, default_value_t = false)]
    verbose: bool,

    /// Click n times to put crab cages (you have to be already holding it being able to put them)
    #[arg(long, num_args(0..=1), default_missing_value = "65535")]
    place_crab_cages: Option<u16>,

    /// Retrieves crab cages (you have to be able to see the button to collect them)
    #[arg(long, num_args(0..=1), default_missing_value = "65535")]
    fetch_crab_cages: Option<u16>,

    /// Retrieves crab cages (you have to be able to see the button to collect them)
    #[arg(long, num_args(0..=1), default_missing_value = "65535")]
    summon_totem: Option<u16>,

    /// Sell items (you have to see the "I'd like to sell this" with 2 dialogs options)
    #[arg(long, num_args(0..=1), default_missing_value = "65535")]
    sell_items: Option<u16>,

    /// Appraise items (you have to see the "Can you appraise this fish?" with 3 dialogs options).
    /// You will have to press `Return` to do another appraisal
    #[arg(long)]
    appraise_items: bool,
}

/// Init logger based on debug level
fn init_logger(verbose: bool) {
    let mut builder = env_logger::Builder::new();
    if verbose {
        builder.filter_level(log::LevelFilter::Info);
    } else {
        builder.filter_level(log::LevelFilter::Warn);
    }
    builder.init();
}

fn pre_init() -> Args {
    let args = Args::parse();
    init_logger(args.verbose);

    info!("Starting Roblox Fishing Macro");
    if args.verbose {
        info!("== Debug mode enabled ==");
    }

    // Check that Roblox is running
    let roblox = get_roblox_executable_name();
    assert!(check_running(roblox), "Roblox not found.");
    info!("Roblox found.");

    match raise(roblox) {
        Ok(()) => info!("Raised Roblox window"),
        Err(err) => {
            warn!("Failed raising roblox window: {err}");
            let wait = 3;
            warn!("You have to focus it yourself (waiting {wait} seconds).");
            sleep(Duration::from_secs(wait), &SHUTDOWN);
        }
    }

    // Register keybinds to close the script
    register_keybinds();

    args
}

fn main() {
    let args = pre_init();

    let mut enigo = Enigo::new(&Settings::default()).expect("Failed to initialize I/O engine");
    let mut recorder = ScreenRecorder::new().expect("Failed to initialize screen monitoring");

    info!(
        "Detected screen dimensions: {}x{}",
        recorder.dimensions.width, recorder.dimensions.height
    );

    // scoreboard_check(&mut enigo, &mut recorder);
    // chat_check(&mut enigo, &mut recorder);

    // Calculate regions based on screen dimensions
    let mut mini_game_region = recorder.dimensions.calculate_mini_game_region();
    let shake_region = recorder.dimensions.calculate_shake_region();
    let safe_point = recorder
        .dimensions
        .calculate_safe_point(&vec![&mini_game_region, &shake_region])
        .expect("Couldn't find any safe point, no region found.");

    #[cfg(feature = "imageproc")]
    {
        use fischy::utils::debug::Drawable;
        use std::sync::Arc;

        let screen = Arc::new(
            recorder
                .take_screenshot()
                .expect("Couldn't take screenshot"),
        );

        mini_game_region
            .clone()
            .draw_async(screen.clone(), "mini_game.png", true);
        shake_region
            .clone()
            .draw_async(screen.clone(), "shake_region.png", true);
        safe_point
            .clone()
            .draw_async(screen, "safe_point.png", true);
    }

    if let Some(clicks) = args.place_crab_cages {
        place_crab_cages(&mut enigo, &safe_point, clicks, &SHUTDOWN);
        exit(0);
    }

    if let Some(cages) = args.fetch_crab_cages {
        fetch_crab_cages(&mut enigo, &safe_point, cages, &SHUTDOWN);
        exit(0);
    }

    if let Some(totems) = args.summon_totem {
        summon_totem(&mut enigo, &safe_point, totems, &SHUTDOWN);
        exit(0);
    }

    if let Some(items) = args.sell_items {
        sell_items(&mut enigo, &safe_point, items, &mut recorder, &SHUTDOWN);
        exit(0);
    }

    if args.appraise_items {
        appraise_items(&mut enigo, &safe_point, &mut recorder, &SHUTDOWN);
        exit(0);
    }

    let mut stats = Stats::new(args.stats);

    if !args.no_camera_setup {
        initialize_viewpoint(&mut enigo, &recorder.dimensions, &SHUTDOWN);
    }

    macro_loop(
        &mut enigo,
        &mut recorder,
        &safe_point,
        &mut mini_game_region,
        &shake_region,
        &args,
        &mut stats,
    );

    stats.print();
}

/// Shake the rod and catch fishes
fn macro_loop(
    enigo: &mut Enigo,
    recorder: &mut ScreenRecorder,
    safe_point: &Point,
    mini_game: &mut MiniGame,
    shake_region: &Region,
    args: &Args,
    stats: &mut Stats,
) {
    let mut last_shake_time = Instant::now();
    let mut shake_count = 0;

    // Initial reel
    reels(
        enigo,
        &mut last_shake_time,
        &mut shake_count,
        safe_point,
        stats,
    );

    while !SHUTDOWN.load(Ordering::Relaxed) {
        // Check for shake
        if let Some(Point { x, y }) = check_shake(enigo, recorder, shake_region, safe_point, args) {
            // server_alive_check(&mut enigo, &mut recorder);

            // Click at the shake position
            info!("Shake @ ({x}, {y})");
            enigo
                .move_mouse(x.cast_signed(), y.cast_signed(), Abs)
                .expect("Failed moving mouse to shake bubble");
            sleep(Duration::from_millis(100), &SHUTDOWN); // we may move the mouse too fast
            enigo
                .button(Button::Left, Click)
                .expect("Failed clicking to shake bubble");

            stats.shakes.add_assign(1);

            // Too much tries
            if shake_count > args.max_shake_count {
                reels(
                    enigo,
                    &mut last_shake_time,
                    &mut shake_count,
                    safe_point,
                    stats,
                );
                continue;
            }

            shake_count += 1;
            last_shake_time = Instant::now();
            continue;
        }

        // Timeout
        if last_shake_time.elapsed() > Duration::from_secs(5) {
            reels(
                enigo,
                &mut last_shake_time,
                &mut shake_count,
                safe_point,
                stats,
            );
            continue;
        }

        // Take screenshot for processing
        let screen = recorder
            .take_screenshot()
            .expect("Failed taking screenshot");

        let is_hooked = mini_game.any_fish_hooked(&screen);

        // Called during the first fish
        if mini_game.rod.is_none() && is_hooked {
            if let Ok(()) = mini_game.refine_area(&screen) {
                info!("Updating minigame structure");
            } else {
                info!("Failed updating the minigame structure");
                continue;
            }

            #[cfg(feature = "imageproc")]
            {
                use fischy::utils::debug::Drawable;
                use std::sync::Arc;

                mini_game.clone().draw_async(
                    Arc::new(screen.clone()),
                    "mini_game_refined.png",
                    true,
                );
            }

            mini_game.initialize_rod(Rod::new(&screen, mini_game));
        }

        // When one fish is on the hook
        if is_hooked {
            // Main fishing logic loop
            info!("Fishing...");
            fishing_loop(enigo, recorder, mini_game, args, stats);
            info!("Fishing ended!");

            // After fishing interaction, reel again
            sleep(Duration::from_secs(2), &SHUTDOWN);
            reels(
                enigo,
                &mut last_shake_time,
                &mut shake_count,
                safe_point,
                stats,
            );
        }
    }
}

/// Catch a fish!
fn fishing_loop(
    enigo: &mut Enigo,
    recorder: &mut ScreenRecorder,
    mini_game: &mut MiniGame,
    args: &Args,
    stats: &mut Stats,
) {
    let fish_color = &[
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

    let loop_time = Instant::now();
    let mut previous_hook_x = 0;
    while !SHUTDOWN.load(Ordering::Relaxed) {
        let screen = recorder
            .take_screenshot()
            .expect("Couldn't take screenshot");

        // Get current fish position
        // FIXME: Support rods with "slashing" abilities
        let fish_x =
            if let Some(Point { x, .. }) = mini_game.search_color_mid_ltr(&screen, fish_color) {
                if args.shake_only {
                    continue;
                }
                x.cast_signed()
            } else {
                info!("The bite is over");
                enigo.button(Button::Left, Release).expect("Packup the rod");
                stats.add_fishing_time(loop_time.elapsed().as_secs());
                break;
            };

        let hook = mini_game.find_hook(&screen);

        // Check if fish is very far left or very far right
        let mini_game_fish_percentage = (((fish_x - mini_game.point1.x.cast_signed()).bad_cast()
            / mini_game.get_size().width.cast_signed().bad_cast())
            * 100.)
            .bad_cast();
        // % treshold defining extreme edge (half of the hookbar)
        let mini_game_percentage_treshold = (((hook.length / 2) * 100).cast_signed().bad_cast()
            / mini_game.get_size().width.cast_signed().bad_cast())
        .bad_cast();
        if mini_game_fish_percentage < mini_game_percentage_treshold {
            info!("Giving some slack...");
            enigo
                .button(Button::Left, Release)
                .expect("Failed giving slack");
            continue;
        } else if mini_game_fish_percentage > 100 - mini_game_percentage_treshold {
            info!("Tighting the line...");
            enigo
                .button(Button::Left, Press)
                .expect("Failed keeping the line tight");
            continue;
        }

        if !hook.fish_on {
            info!("Didn't find any color corresponding to the hook");
            warn!("It's hard to keep up with this fish");
            enigo.button(Button::Left, Release).expect("Packup the rod");
            continue;
        }

        // Calculate range between fish and hook
        let hook_x = hook
            .position
            .expect("Can't find the hook")
            .absolute_mid_x
            .cast_signed();
        let range = fish_x - hook_x;
        let speed = hook_x - previous_hook_x;
        info!("Distance fish ({fish_x}) and hook ({hook_x}) is {range}");

        // % of the hook bar
        if range.abs() <= ((hook.length.cast_signed().bad_cast() / 2.) * 0.60).bad_cast()
            && speed.abs() < 20
            && range < hook.length.cast_signed()
        {
            // Spamming to stay more easily in range, only if speed is not too high
            enigo.button(Button::Left, Click).expect("Clicking failed");
        }

        // let _ = mini_game.any_fish_hooked(&screen);
        if range >= 0 {
            info!("==> To the right! ==>");
            enigo.button(Button::Left, Press).expect("Pressing failed");
        } else {
            info!("<== To the left! <==");
            enigo
                .button(Button::Left, Release)
                .expect("Releasing failed");
        }

        let hold_time = hold_formula(range, speed, &mini_game.get_size());

        info!("Found fish at x={fish_x} - distance fish<->hook is {range} - holding {hold_time}ms");
        wait_until(recorder, mini_game, hold_time.into(), fish_color);

        previous_hook_x = hook_x; // update previous hook position
    }
}

/// Start the fishing process
fn reels(
    enigo: &mut Enigo,
    last_shake_time: &mut Instant,
    shake_count: &mut u8,
    safe_point: &Point,
    stats: &mut Stats,
) {
    // Move mouse
    enigo
        .move_mouse(safe_point.x.cast_signed(), safe_point.y.cast_signed(), Abs)
        .expect("Can't move mouse");

    // Click to be sure we are not shaking
    enigo
        .button(Button::Left, Click)
        .expect("Can't click before reel");
    sleep(
        Duration::from_millis(rand::rng().random_range(60..=80)),
        &SHUTDOWN,
    );

    info!("Reeling...");
    // Casting motion
    enigo
        .button(Button::Left, Press)
        .expect("Can't backswing: failed to press mouse button");
    sleep(
        Duration::from_millis(rand::rng().random_range(600..=1200)),
        &SHUTDOWN,
    );
    enigo
        .button(Button::Left, Release)
        .expect("Can't release the line: failed to release mouse button");

    if stats.enabled {
        stats.reels.add_assign(1);
    }

    *shake_count = 0;
    *last_shake_time = Instant::now();
}

/// Determine how long we should hold the line in milliseconds
/// based on distances between the fish and the middle of the hook
fn hold_formula(gap: i32, estimated_speed: i32, full_area: &Dimensions) -> u32 {
    let gap_abs = gap.abs().bad_cast();
    let max_gap = full_area.width.cast_signed().bad_cast() / 2.;

    let normalized_gap = (gap_abs / max_gap).clamp(0., 1.);
    // let eased = t * t; // quadratic curve
    let eased = normalized_gap.powi(3); // cubic curve

    let min_ms = 10.;
    let max_ms = 1500.;

    let mut ms = min_ms + eased * (max_ms - min_ms);

    // Stopping power by reducing holding time
    if gap.signum() == estimated_speed.signum() {
        let max_speed = 200.;
        // INFO: We could increase brake force by multiplying this value
        let v = (estimated_speed.abs().bad_cast() / max_speed).clamp(0., 1.);
        let brake_boost = 1. + v.powi(2) * 0.5;
        ms /= brake_boost;
    }

    ms.round().bad_cast().cast_unsigned()
}

/// Returns the coordinates of the shake bubble
fn check_shake(
    #[allow(unused_variables)] enigo: &mut Enigo,
    recorder: &mut ScreenRecorder,
    region: &Region,
    #[allow(unused_variables)] safe_point: &Point,
    args: &Args,
) -> Option<Point> {
    let [x_min, y_min, x_max, y_max] = region.corners().map(u32::cast_signed);

    // Move cursor out of the region (Sober creates a custom cursor)
    #[cfg(target_os = "linux")]
    {
        // Sleep 1 / 3 to be sure we correctly move the cursor
        sleep(
            Duration::from_millis(
                (args.lag.cast_signed().bad_cast() * 1. / 3.)
                    .bad_cast()
                    .cast_unsigned()
                    .into(),
            ),
            &SHUTDOWN,
        );
        enigo
            .move_mouse(safe_point.x.cast_signed(), safe_point.y.cast_signed(), Abs)
            .expect("Can't move mouse");
        // Sleep 2 / 3 to be sure the screenshot won't capture our cursor
        sleep(
            Duration::from_millis(
                (args.lag.cast_signed().bad_cast() * 2. / 3.)
                    .bad_cast()
                    .cast_unsigned()
                    .into(),
            ),
            &SHUTDOWN,
        );
    }

    #[cfg(not(target_os = "linux"))]
    {
        sleep(Duration::from_millis(args.lag.into()), &SHUTDOWN);
    }

    let pure_white = ColorTarget {
        color: Rgb([0xff, 0xff, 0xff]),
        variation: 1,
    };

    // Check image from bottom to top helps up leveraging broadcast messages that are
    // overlaping with the shaking area
    let screen = recorder.take_screenshot().expect("Can't take screenshot");

    let shake_point = (y_min..=y_max)
        .rev()
        .flat_map(|y| (x_min..=x_max).map(move |x| (x, y)))
        .find_map(|(x, y)| {
            let pixel = screen.get_pixel(x.cast_unsigned(), y.cast_unsigned());

            pure_white.matches(pixel).then(|| Point {
                x: x.cast_unsigned() + 20,
                y: y.cast_unsigned() - 10,
            })
        });

    #[cfg(feature = "imageproc")]
    {
        use fischy::utils::debug::Drawable;
        use std::sync::Arc;

        if let Some(p) = shake_point.clone() {
            p.draw_async(Arc::new(screen.clone()), "shakes/point.png", false);
        }
    }

    shake_point
}

/// Stop waiting until:
/// 1. Fish in the window
/// 2. Fish escaped and at least half of time is out
/// 3. Time runs out
fn wait_until(
    recorder: &mut ScreenRecorder,
    mini_game: &mut MiniGame,
    time_ms: u64,
    fish_color: &[ColorTarget],
) {
    let deadline = Instant::now() + Duration::from_millis(time_ms);

    // % of time
    let acceptable = Instant::now()
        + Duration::from_millis(
            (i32::try_from(time_ms.cast_signed())
                .expect("Couldn't cast")
                .bad_cast()
                * 0.5)
                .bad_cast()
                .cast_unsigned()
                .into(),
        );

    while !SHUTDOWN.load(Ordering::Relaxed) {
        let now = Instant::now();
        if now >= deadline {
            info!("Waited long enough");
            break;
        }

        let screen = recorder.take_screenshot().expect("Can't take screenshot");

        let hook = mini_game.find_hook(&screen);
        if let Some(hook_position) = hook.position {
            debug!(
                "Hook percentage: ~{}% ",
                (hook.length.cast_signed().bad_cast()
                    / mini_game.get_size().width.cast_signed().bad_cast()
                    * 100.)
                    .bad_cast(),
            );

            let fish_x = mini_game
                .search_color_mid_ltr(&screen, fish_color)
                .map(|p| p.x);

            // If no fish found or fish is outside valid range, return whether fish was found
            match fish_x {
                None => {
                    info!("Fish escaped");
                    return;
                }
                Some(x)
                    if x >= hook_position.absolute_beg_x
                        && x <= hook_position.absolute_end_x
                        && now >= acceptable =>
                {
                    info!("Fish is in the range and we waited long enough");
                    return;
                }
                _ => { /* Keep waiting */ }
            }
        }
    }

    info!("Did not succeed to put the fish in the hook, deciding another action...");
}

/// Initialize where the player is looking
fn initialize_viewpoint(enigo: &mut Enigo, screen_dims: &Dimensions, cond: &AtomicBool) {
    let padding = (screen_dims.width.cast_signed().bad_cast() * 0.2).bad_cast();

    // "Safepoint"
    enigo
        .move_mouse(
            screen_dims.width.cast_signed() / 2,
            screen_dims.height.cast_signed() - padding,
            Abs,
        )
        .expect("Going to safepoint failed");

    // Looking at the floor
    #[cfg(target_os = "linux")]
    let movement = (0, screen_dims.height.cast_signed() / 2);

    #[cfg(not(target_os = "linux"))]
    let movement = (0, screen_dims.height.cast_signed() / -2);

    let steps = 2;
    (0..=steps).for_each(|_| {
        enigo.button(Button::Right, Press).expect("Pressing failed");
        sleep(Duration::from_millis(100), cond);
        enigo
            .move_mouse(movement.0, movement.1, Rel)
            .expect("Going down failed");
        sleep(Duration::from_millis(100), cond);

        // Release
        enigo
            .button(Button::Right, Release)
            .expect("Releasing failed");
        sleep(Duration::from_millis(100), cond);

        // Back to initial point
        enigo
            .move_mouse(-movement.0, -movement.1, Rel)
            .expect("Resetting position failed");
        sleep(Duration::from_millis(100), cond);
    });

    // Zoom
    enigo
        .scroll_ig(-Enigo::max_scroll(), Vertical)
        .expect("Can't zoom in");
    enigo.scroll_ig(1, Vertical).expect("Can't zoom out");
}

/// Register specific keypress that will stop the program
fn register_keybinds() {
    thread::spawn(|| {
        listen(|e| {
            if let Event {
                event_type: KeyPress(Key::Escape | Key::Space),
                ..
            } = e
            {
                info!("Closing due to key press...");
                SHUTDOWN.store(true, Ordering::Relaxed);
            }
        })
        .expect("Can't listen to keyboard");
    });
}

// /// Close scoreboard if open
// fn scoreboard_check(enigo: &mut Enigo, recorder: &mut ScreenRecorder) {
//     todo!("Check if scoreboard is open => Close it by pressing <tab>")
// }

// /// Close chat if open
// fn chat_check(enigo: &mut Enigo, recorder: &mut ScreenRecorder) {
//     todo!("Check if chat is open => Close it by pressing the button")
// }

// /// Close if server have shutdown
// fn server_alive_check(enigo: &mut Enigo, recorder: &mut ScreenRecorder) {
//     todo!("Check if popup in the center => Exit the program in this case")
// }
