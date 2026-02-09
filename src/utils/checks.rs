use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

use enigo::{Button, Direction::Click, Enigo, Key, Keyboard, Mouse};
use image::{Rgb, RgbImage};
use log::info;

use crate::{
    Scroller, sleep,
    utils::{
        colors::ColorTarget,
        geometry::{Point, Region},
    },
};

fn text_detection(xs: &[u32], y_min: u32, y_max: u32, img: &RgbImage) -> usize {
    xs.iter()
        .map(|&x| {
            (y_min..y_max)
                .map(|y| ColorTarget::brightness(img.get_pixel(x, y)))
                .collect::<Vec<_>>()
                .windows(2)
                .filter(|w| w[0].abs_diff(w[1]) > 100)
                .count()
        })
        .sum::<usize>()
}

/// Close scoreboard if open
///
/// # Panics
/// If couldn't close scoreboard
pub fn scoreboard_check(enigo: &mut Enigo, img: &RgbImage) {
    let x_positions = [
        img.width() * 98 / 100, // C$
        img.width() * 94 / 100, // Level
        img.width() * 86 / 100, // People
    ];
    let [y_min, y_max] = [img.height() * 5 / 100, img.height() / 2];

    #[cfg(feature = "imageproc")]
    {
        use crate::utils::debug::Drawable;
        use std::sync::Arc;

        x_positions
            .map(|x| Region {
                point1: Point { x: x - 1, y: y_min },
                point2: Point { x: x + 1, y: y_max },
            })
            .to_vec()
            .draw_async(Arc::new(img.clone()), "roblox_scoreboard.png", true);
    }

    // TODO: Conservative treshold, could we go lower?
    if text_detection(&x_positions, y_min, y_max, img) >= 10 {
        enigo.key(Key::Tab, Click).expect("Couldn't press <TAB>");
    }
}

/// Close chat if open
///
/// # Panics
/// If couldn't close the chat
pub fn chat_check(enigo: &mut Enigo, img: &RgbImage, roblox_anchor: &Point, cond: &AtomicBool) {
    // Where to click
    let button = Point {
        x: roblox_anchor.x + img.width() * 5 / 100,
        y: roblox_anchor.y - 5,
    };

    // Where to check
    let area = Region {
        point1: Point {
            x: button.x - img.width() * 8 / 1000,
            y: button.y - img.height() * 5 / 1000,
        },
        point2: Point {
            x: button.x + img.width() / 100,
            y: button.y + img.height() * 2 / 100,
        },
    };

    #[cfg(feature = "imageproc")]
    {
        use crate::utils::debug::Drawable;
        use std::sync::Arc;

        button
            .clone()
            .draw_async(Arc::new(img.clone()), "roblox_chat_button.png", true);
        area.clone()
            .draw_async(Arc::new(img.clone()), "roblox_chat_button_area.png", true);
    }

    let white_color_open_chat_button = ColorTarget {
        color: Rgb([0xf7, 0xf7, 0xf8]),
        variation: 2,
    };
    let [x_min, y_min, x_max, y_max] = area.corners();
    let matching_pixels = (x_min..x_max)
        .flat_map(|x| (y_min..y_max).map(move |y| (x, y)))
        .filter(|(x, y)| white_color_open_chat_button.matches(img.get_pixel(*x, *y)))
        .count();

    let check = matching_pixels
        / usize::try_from(((x_max - x_min) * (y_max - y_min)) / 100).unwrap_or(usize::MAX)
        > 20; // treshold percentage of the same color in the area

    if check {
        enigo
            .move_mouse_ig_abs(button.x.cast_signed(), button.y.cast_signed())
            .expect("Couldn't move mouse to chat button");
        sleep(Duration::from_millis(100), cond); // we may move the mouse too fast
        enigo
            .button(Button::Left, Click)
            .expect("Couldn't close the chat");
    }
}

/// Close quest panel if open
///
/// # Panics
/// If couldn't close the quest panel
pub fn quest_check(enigo: &mut Enigo, img: &RgbImage, roblox_anchor: &Point, cond: &AtomicBool) {
    let final_y = roblox_anchor.y + img.height() * 45 / 1000; // 4.5%
    let possible_y = roblox_anchor.y + img.height() * 338 / 1000; // 33.8%
    let start_x = roblox_anchor.x + img.width() * 5 / 100;
    let end_x = img.width() * 35 / 100;

    #[cfg(feature = "imageproc")]
    {
        use crate::utils::debug::Drawable;
        use std::sync::Arc;

        vec![
            Region {
                point1: Point {
                    x: roblox_anchor.x,
                    y: final_y - 1,
                },
                point2: Point {
                    x: end_x,
                    y: final_y + 1,
                },
            },
            Region {
                point1: Point {
                    x: roblox_anchor.x,
                    y: possible_y - 1,
                },
                point2: Point {
                    x: end_x,
                    y: possible_y + 1,
                },
            },
        ]
        .draw_async(Arc::new(img.clone()), "quest_arrow_search.png", true);
    }

    let white = ColorTarget {
        color: Rgb([0xff, 0xff, 0xff]),
        variation: 0,
    };

    let segment_width = img.width() / 1000; // 0.1%
    if let Some(found_x) = [final_y, possible_y].iter().find_map(|&y| {
        (start_x..end_x).rev().find(|&x| {
            (x..x + segment_width)
                .filter(|&px| white.matches(img.get_pixel(px, y)))
                .count()
                >= segment_width as usize
        })
    }) {
        #[cfg(feature = "imageproc")]
        {
            use crate::utils::debug::Drawable;
            use std::sync::Arc;

            Point {
                x: found_x,
                y: final_y,
            }
            .draw_async(Arc::new(img.clone()), "quest_arrow.png", true);
        }

        enigo
            .move_mouse_ig_abs(found_x.cast_signed(), final_y.cast_signed())
            .expect("Couldn't move mouse to quest arrow");
        sleep(Duration::from_millis(100), cond);
        enigo
            .button(Button::Left, Click)
            .expect("Couldn't close the quest panel");
    }
}

/// Close if server have shutdown
pub fn server_alive_check(img: &RgbImage, cond: &AtomicBool) -> bool {
    let (win_w, win_h) = img.dimensions();
    let (popup_w, popup_h) = (win_w * 10 / 100, win_h * 10 / 100);

    let disconnected_popup = Region {
        point1: Point {
            x: win_w / 2 - popup_w,
            y: win_h / 2 - popup_h,
        },
        point2: Point {
            x: win_w / 2 + popup_w,
            y: win_h / 2 + popup_h,
        },
    };

    #[cfg(feature = "imageproc")]
    {
        use crate::utils::debug::Drawable;
        use std::sync::Arc;

        disconnected_popup.clone().draw_async(
            Arc::new(img.clone()),
            "disconnected_popup.png",
            true,
        );
    }

    let gray = ColorTarget {
        color: Rgb([0x39, 0x3b, 0x3d]),
        variation: 0,
    };

    let [x_min, y_min, x_max, y_max] = disconnected_popup.corners();
    let matching_pixels = (x_min..x_max)
        .flat_map(|x| (y_min..y_max).map(move |y| (x, y)))
        .filter(|(x, y)| gray.matches(img.get_pixel(*x, *y)))
        .count();

    let treshold = 70; // percentage of the same color in the area
    let check = matching_pixels
        / usize::try_from(((x_max - x_min) * (y_max - y_min)) / 100).unwrap_or(usize::MAX)
        > treshold;
    if check {
        info!("Closing due to server offline...");
        cond.store(true, Ordering::Relaxed);
    }

    !check
}

/// Dismiss max treasure maps warning
///
/// # Panics
/// If couldn't close the treasure maps warning
pub fn treasure_maps_check(enigo: &mut Enigo, img: &RgbImage, cond: &AtomicBool) {
    let x_positions = [
        img.width() * 94 / 100,
        img.width() * 91 / 100,
        img.width() * 89 / 100,
    ];

    let half = img.height() / 2 + img.height() / 100;
    let shift = img.height() * 5 / 100;
    let [y_min, y_max] = [half - shift, half + shift];

    #[cfg(feature = "imageproc")]
    {
        use crate::utils::debug::Drawable;
        use std::sync::Arc;

        x_positions
            .map(|x| Region {
                point1: Point { x: x - 1, y: y_min },
                point2: Point { x: x + 1, y: y_max },
            })
            .to_vec()
            .draw_async(Arc::new(img.clone()), "treasure_maps.png", true);
    }

    if text_detection(&x_positions, y_min, y_max, img) >= 10 {
        let p = Point {
            x: img.width() * 92 / 100,
            y: img.height() * 58 / 100,
        };

        #[cfg(feature = "imageproc")]
        {
            use crate::utils::debug::Drawable;
            use std::sync::Arc;

            p.clone()
                .draw_async(Arc::new(img.clone()), "treasure_maps_button.png", true);
        }

        enigo
            .move_mouse_ig_abs(p.x.cast_signed(), p.y.cast_signed())
            .expect("Couldn't move mouse to dismiss button");
        sleep(Duration::from_millis(100), cond);
        enigo
            .button(Button::Left, Click)
            .expect("Couldn't dismiss the treasure maps warning");
    }
}
