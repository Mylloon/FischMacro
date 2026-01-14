use std::{
    ops::{Deref, DerefMut},
    slice,
};

use image::{Rgb, RgbImage};

use crate::utils::{
    colors::ColorTarget,
    geometry::{Point, Region},
    helpers::BadCast,
};

#[derive(Clone)]
pub struct HookPosition {
    /// Left side position
    pub absolute_beg_x: u32,
    /// Middle of the hook
    pub absolute_mid_x: u32,
    /// Right side of the hook
    pub absolute_end_x: u32,
}

#[derive(Clone)]
pub struct Hook {
    /// Absolution position
    pub position: Option<HookPosition>,

    /// Relative length
    pub length: u32,

    /// If the fish is currently on the hook
    pub fish_on: bool,
}

pub struct Rod {
    internals: Hook,
}

impl Rod {
    #[must_use]
    pub fn new(screen: &RgbImage, mini_game_region: &Region) -> Self {
        match Self::search_hook(screen, mini_game_region) {
            Some((length, position)) => Rod {
                internals: Hook {
                    position: Some(position),
                    length,
                    fish_on: false,
                },
            },
            None => Rod {
                internals: {
                    // Default percentage value: https://fischipedia.org/wiki/Fishing_Rods#Control
                    let percentage = 30;
                    Hook {
                        position: None,
                        length: (mini_game_region.get_size().width.cast_signed().bad_cast()
                            * (percentage.bad_cast() / 100.))
                            .bad_cast()
                            .cast_unsigned(),
                        fish_on: false,
                    }
                },
            },
        }
    }

    /// Find the hook
    ///
    /// # Return
    /// Couple (hook's length, hook's position)
    fn search_hook(screen: &RgbImage, region: &Region) -> Option<(u32, HookPosition)> {
        let [x_min, y_min, x_max, y_max] = region.corners();
        let y = y_min.midpoint(y_max);
        let gap_tolerance = 35; // take into account arrows and fish that overlap the hook bar

        let brightnesses = (x_min..=x_max)
            .map(|x| (x, ColorTarget::brightness(screen.get_pixel(x, y))))
            .collect::<Vec<_>>();

        let threshold = {
            let mut sorted = brightnesses.iter().map(|(_, b)| *b).collect::<Vec<_>>();
            sorted.sort_unstable();
            // We use 3% because it's probably the smallest the hook bar can get
            sorted[sorted.len() * 97 / 100]
        };

        // Find contiguous bright segments with gap tolerance
        let segments: Vec<_> = brightnesses
            .iter()
            .scan(None, |state, &(x, brightness)| {
                let mut res = None;
                if brightness >= threshold {
                    *state = Some(match state.take() {
                        Some((start, _)) => (start, x),
                        None => (x, x),
                    });
                    res = *state;
                } else if let Some((start, last)) = *state {
                    if x - last > gap_tolerance {
                        res = Some((start, last));
                        *state = None;
                    } else {
                        *state = Some((start, last));
                    }
                }
                Some(res)
            })
            .flatten()
            .collect();

        segments
            .into_iter()
            .map(|(l, r)| (r - l, l, r))
            // Keep longest segment
            .max_by_key(|(width, _, _)| *width)
            .map(|(width, l, r)| {
                // INFO: we could map only once and create directly our structure,
                //       but since we can get MANY segments (like ~400),
                //       we only do that for the chosen one
                (
                    width,
                    HookPosition {
                        absolute_beg_x: l,
                        absolute_mid_x: l + width / 2,
                        absolute_end_x: r,
                    },
                )
            })
    }

    /// Refresh hook data
    fn update_hook(&mut self, image: &RgbImage, mini_game_region: &Region) {
        let hook_data = Self::search_hook(image, mini_game_region);
        self.internals.fish_on = hook_data.is_some();
        if let Some((l, hook_pos)) = hook_data {
            self.internals.length = l;
            self.internals.position = Some(hook_pos);
        }
    }

    /// Returns fresh info about the hook
    ///
    /// # Panics
    /// Not be able to take a screenshot
    pub fn find_hook(&mut self, image: &RgbImage, mini_game_region: &Region) -> Hook {
        self.update_hook(image, mini_game_region);
        self.internals.clone()
    }
}

/// Enhanced region
pub struct MiniGame {
    /// Mini-game bar
    outer: Region,
    /// Rod bar
    pub rod: Option<Rod>,
}

impl Deref for MiniGame {
    type Target = Region;

    fn deref(&self) -> &Self::Target {
        &self.outer
    }
}

impl DerefMut for MiniGame {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.outer
    }
}

impl MiniGame {
    #[must_use]
    pub fn new(region: Region) -> MiniGame {
        MiniGame {
            outer: region,
            rod: None,
        }
    }

    /// Search if the fish is hooked based on the mouse above the minigame
    #[must_use]
    pub fn any_fish_hooked(&self, screen: &RgbImage) -> bool {
        // Use mouse above minigame bar
        let approx_y = (screen.height().cast_signed().bad_cast() * 0.72)
            .bad_cast()
            .cast_unsigned();
        let mouse = Region {
            point1: Point {
                x: self.point1.x - 5 + self.get_size().width / 2,
                y: approx_y + 1,
            },
            point2: Point {
                x: self.point1.x + 9 + self.get_size().width / 2,
                y: approx_y + 8,
            },
        };

        // #[cfg(feature = "imageproc")]
        // {
        //     use crate::utils::debug::Drawable;
        //     use std::sync::Arc;

        //     mouse
        //         .clone()
        //         .draw_async(Arc::new(screen.clone()), "mouses/0.png", false);
        // }

        let [x_min, y_min, x_max, y_max] = mouse.corners();
        let y = y_min.midpoint(y_max);
        let consecutive = 10;
        (x_min..=x_max.saturating_sub(consecutive - 1)).any(|x_start| {
            let Rgb([r1, g1, b1]) = *screen.get_pixel(x_start, y);
            (0..consecutive).all(|i| {
                let Rgb([r2, g2, b2]) = *screen.get_pixel(x_start + i, y);
                let tolerance = 3;
                let brightness = ColorTarget::brightness(screen.get_pixel(x_start + i, y));
                let correct_brightness =
                    (210..240).contains(&brightness) || (90..120).contains(&brightness);
                correct_brightness
                    && (i16::from(r1) - i16::from(r2)).abs() <= tolerance
                    && (i16::from(g1) - i16::from(g2)).abs() <= tolerance
                    && (i16::from(b1) - i16::from(b2)).abs() <= tolerance
            })
        })
    }

    /// This HAS to be called at the very beginning of the fishing process
    /// It refine the global mini-game area to precisely it coordinates
    /// This shouldn't change accross hooks
    ///
    /// # Errors
    /// If no control-arrows found
    pub fn refine_area(&mut self, img: &RgbImage) -> Result<(), String> {
        // Attempt to find arrows in both halves
        let color = slice::from_ref(&ColorTarget {
            color: Rgb([0x5f, 0x3b, 0x34]),
            variation: 4,
        });
        let (left, right) = self
            .search_color_left_half(img, color)
            .zip(self.search_color_right_half(img, color))
            .ok_or("Couldn't find arrows")?;

        // Update points with offsets
        self.point1 = left + (20, -10);
        self.point2 = right + (-20, 20);

        Ok(())
    }

    pub fn initialize_rod(&mut self, rod: Rod) {
        self.rod = Some(rod);
    }

    /// Returns fresh info about the hook
    ///
    /// # Panics
    /// If there is no rod stored
    pub fn find_hook(&mut self, image: &RgbImage) -> Hook {
        self.rod
            .as_mut()
            .expect("Couldn't find rod")
            .find_hook(image, &self.outer)
    }
}
