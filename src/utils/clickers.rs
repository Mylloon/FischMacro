use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

use enigo::{Button, Coordinate::Abs, Direction::Click, Enigo, Mouse};
use rdev::{EventType, Key, simulate};

use crate::{sleep, utils::geometry::Point};

/// Place a crab cage, assume that the player already set-up everything and we are left clicking
///
/// # Panics
/// Couldn't use the mouse
pub fn place_crab_cages(enigo: &mut Enigo, safe_point: &Point, clicks: u16, cond: &AtomicBool) {
    // Move mouse
    enigo
        .move_mouse(safe_point.x.cast_signed(), safe_point.y.cast_signed(), Abs)
        .expect("Can't move mouse");

    let mut clicked = 0;
    while clicked < clicks && !cond.load(Ordering::Relaxed) {
        enigo
            .button(Button::Left, Click)
            .expect("Couldn't place crab cage");
        sleep(Duration::from_millis(100), cond);
        clicked += 1;
    }
}

/// Fetch crab cages, assume that the player already set-up everything and we are left collecting
///
/// # Panics
/// Couldn't use the keyboard
pub fn fetch_crab_cages(enigo: &mut Enigo, safe_point: &Point, cages: u16, cond: &AtomicBool) {
    // Move mouse
    enigo
        .move_mouse(safe_point.x.cast_signed(), safe_point.y.cast_signed(), Abs)
        .expect("Can't move mouse");

    let key_e = Key::KeyE;
    let infinite = cages == u16::MAX;
    let mut n = 0;
    while (infinite || n < cages) && !cond.load(Ordering::Relaxed) {
        // TMP: We use rdev as it's currently more reliable than enigo for using keyboard
        simulate(&EventType::KeyPress(key_e)).expect("Couldn't press {key_e}");
        sleep(Duration::from_secs(1), cond);
        simulate(&EventType::KeyRelease(key_e)).expect("Couldn't release {key_e}");
        if !infinite {
            n += 1;
        }
    }
}
