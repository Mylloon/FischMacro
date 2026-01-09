use image::{Rgb, RgbImage};
use log::{trace, warn};

use crate::{colors::ColorTarget, geometry::Region, helpers::BadCast, search_color_ltr};

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
    is_real_length: bool,
    // /// https://fischipedia.org/wiki/Fishing_Rods#Control
    pub control_percentage: i32,
}

impl Rod {
    #[must_use]
    pub fn new(screen: &RgbImage, mini_game_region: &Region, control_minimal: f32) -> Self {
        let length = Self::get_length(screen, mini_game_region);
        Rod {
            internals: Hook {
                position: None,
                length: length.unwrap_or_else(|e| e),
                fish_on: false,
            },
            is_real_length: length.is_ok(),
            control_percentage: (100. * control_minimal + 30.).bad_cast(),
        }
    }

    /// Returns min & max theorical control values
    #[must_use]
    pub fn get_min_max_control_values() -> (f32, f32) {
        // Default value (0) is 30% width bar
        (-0.3, 0.7)
    }

    /// Find the width of the hook
    ///
    /// NOTE: Works only when fish is on the hook (e.g. at the very start of the fishing process)
    fn calculate_initial_hook_width(screen: &RgbImage, region: &Region) -> Option<i32> {
        let [x_min, y_min, x_max, y_max] = region.corners().map(u32::cast_signed);
        let middle_y = y_min.midpoint(y_max);

        let white = ColorTarget {
            color: Rgb([255, 255, 255]),
            variation: 21,
        };

        // Since we are at initial position, we know that the hook is on the fish,
        // so the hook is white
        let is_white = |x: i32| {
            (0..screen.width().cast_signed()).contains(&x)
                && white.matches(*screen.get_pixel(x.cast_unsigned(), middle_y.cast_unsigned()))
        };

        let left = (x_min..=x_max).find(|&x| is_white(x));
        let right = (x_min..=x_max).rev().find(|&x| is_white(x));

        left.zip(right).map(|(l, r)| r - l)
    }

    /// Find length
    /// Ok(x) means exact length
    /// Err(x) means computed length
    fn get_length(screen: &RgbImage, mini_game_region: &Region) -> Result<u32, u32> {
        Self::calculate_initial_hook_width(screen, mini_game_region).map_or(
            {
                // Default percentage value: https://fischipedia.org/wiki/Fishing_Rods#Control

                let percentage = 30;
                warn!("Hook not found, falling back to {percentage}% of minigame window");
                Err((mini_game_region.get_size().width.cast_signed().bad_cast()
                    * (percentage.bad_cast() / 100.))
                    .bad_cast()
                    .cast_unsigned())
            },
            |e| {
                warn!("Found hook: using {e} as length");
                Ok(e.cast_unsigned())
            },
        )
    }

    /// Refresh hook data
    fn update_hook(
        &mut self,
        image: &RgbImage,
        color_hook: &[ColorTarget],
        color_white: &[ColorTarget],
        mini_game_region: &Region,
    ) {
        // Update hook pos if we did not found final value - yet
        if !self.is_real_length
            && let Ok(l) = Self::get_length(image, mini_game_region)
        {
            self.is_real_length = true;
            self.internals.length = l;
        }

        let hook_pos = if let Some(pos) =
            search_color_ltr(image, color_white, mini_game_region).map(|p| p.x)
        {
            trace!("Hook found with fish in it");
            // Fish in the hook
            Some(pos)
        } else if let Some(pos) = search_color_ltr(image, color_hook, mini_game_region).map(|p| p.x)
        {
            trace!("Hook found but fish is not in it");
            // Fish outside of the hook
            Some(pos)
        } else {
            trace!("Hook not found");
            None
        };

        self.internals.position = hook_pos.map(|p| HookPosition {
            absolute_beg_x: p,
            absolute_mid_x: p + (self.internals.length / 2),
            absolute_end_x: p + self.internals.length,
        });
        self.internals.fish_on = hook_pos.is_some();
    }

    /// Returns fresh info about the hook
    ///
    /// # Panics
    /// Not be able to take a screenshot
    pub fn find_hook(
        &mut self,
        image: &RgbImage,
        color_hook: &[ColorTarget],
        color_white: &[ColorTarget],
        mini_game_region: &Region,
    ) -> Hook {
        self.update_hook(image, color_hook, color_white, mini_game_region);
        self.internals.clone()
    }
}
