use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

use enigo::{Axis::Vertical, Button, Coordinate::Abs, Direction::Click, Enigo, Mouse};
use log::info;
use rdev::{EventType, Key, simulate};

use crate::{
    ScreenRecorder, sleep,
    utils::{geometry::Point, helpers::BadCast},
};

/// Place a crab cage, assume that the player already set-up everything and we are left clicking
///
/// # Panics
/// Couldn't use the mouse
pub fn place_crab_cages(enigo: &mut Enigo, safe_point: &Point, clicks: u16, cond: &AtomicBool) {
    // Move mouse
    enigo
        .move_mouse(safe_point.x.cast_signed(), safe_point.y.cast_signed(), Abs)
        .expect("Can't move mouse");

    let infinite = clicks == u16::MAX;
    let mut remaining = clicks;
    while (infinite || remaining > 0) && !cond.load(Ordering::Relaxed) {
        enigo
            .button(Button::Left, Click)
            .expect("Couldn't place crab cage");
        sleep(Duration::from_millis(100), cond);
        if !infinite {
            remaining -= 1;
            info!("{remaining} remaining");
        }
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
    let mut remaining = cages;
    while (infinite || remaining > 0) && !cond.load(Ordering::Relaxed) {
        // TMP: We use rdev as it's currently more reliable than enigo for using keyboard
        simulate(&EventType::KeyPress(key_e)).expect("Couldn't press {key_e}");
        sleep(Duration::from_secs(1), cond);
        simulate(&EventType::KeyRelease(key_e)).expect("Couldn't release {key_e}");
        if !infinite {
            remaining -= 1;
            info!("{remaining} remaining");
        }
    }
}

/// Summon totem, assume that the player already set-up everything and we are left collecting
///
/// # Panics
/// Couldn't use the mouse
pub fn summon_totem(enigo: &mut Enigo, safe_point: &Point, totems: u16, cond: &AtomicBool) {
    // Move mouse
    enigo
        .move_mouse(safe_point.x.cast_signed(), safe_point.y.cast_signed(), Abs)
        .expect("Can't move mouse");

    let infinite = totems == u16::MAX;
    let mut remaining = totems;
    while (infinite || remaining > 0) && !cond.load(Ordering::Relaxed) {
        enigo
            .button(Button::Left, Click)
            .expect("Couldn't summon totem");
        sleep(Duration::from_secs(19), cond);
        if !infinite {
            remaining -= 1;
            info!("{remaining} remaining");
        }
    }
}

/// Sell items, assume that the player entered the dialog "I'd like to sell this" with 2 options
///
/// # Panics
/// Couldn't use the mouse
pub fn sell_items(
    enigo: &mut Enigo,
    safe_point: &Point,
    items: u16,
    recorder: &mut ScreenRecorder,
    cond: &AtomicBool,
) {
    let item = Point {
        x: (recorder.dimensions.width.cast_signed().bad_cast() * 0.36)
            .bad_cast()
            .cast_unsigned(),
        y: (recorder.dimensions.height.cast_signed().bad_cast() * 0.66)
            .bad_cast()
            .cast_unsigned(),
    };
    let dialog = Point {
        x: (recorder.dimensions.width.cast_signed().bad_cast() * 0.63)
            .bad_cast()
            .cast_unsigned(),
        y: (recorder.dimensions.height.cast_signed().bad_cast() * 0.53)
            .bad_cast()
            .cast_unsigned(),
    };

    // Move mouse
    enigo
        .move_mouse(safe_point.x.cast_signed(), safe_point.y.cast_signed(), Abs)
        .expect("Can't move mouse");
    enigo.scroll(-8, Vertical).expect("Can't zoom in");
    enigo.scroll(3, Vertical).expect("Can't zoom out");

    #[cfg(feature = "imageproc")]
    {
        use crate::utils::debug::Drawable;
        use std::sync::Arc;

        sleep(Duration::from_millis(100), cond);
        let screen = Arc::new(
            recorder
                .take_screenshot()
                .expect("Couldn't take screenshot"),
        );

        item.clone()
            .draw_async(screen.clone(), "sell_item.png", true);
        dialog.clone().draw_async(screen, "sell_dialog.png", true);
    }

    // TODO: when in infinite mode, detect by color changes if there is items left to sell
    let infinite = items == u16::MAX;
    let mut remaining = items;
    while (infinite || remaining > 0) && !cond.load(Ordering::Relaxed) {
        // Item
        enigo
            .move_mouse(item.x.cast_signed(), item.y.cast_signed(), Abs)
            .expect("Couldn't move mouse to item");
        enigo
            .button(Button::Left, Click)
            .expect("Couldn't select item");

        // Sell
        enigo
            .move_mouse(dialog.x.cast_signed(), dialog.y.cast_signed(), Abs)
            .expect("Couldn't move mouse to dialog");
        enigo
            .button(Button::Left, Click)
            .expect("Couldn't sell item");

        sleep(Duration::from_secs(2), cond);
        if !infinite {
            remaining -= 1;
            info!("{remaining} remaining");
        }
    }
}
