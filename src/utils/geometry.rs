use std::ops::Add;

use image::{Rgb, RgbImage};

use crate::utils::{colors::ColorTarget, fishing::MiniGame};

#[derive(Clone)]
pub struct Point {
    pub x: u32,
    pub y: u32,
}

impl Add<(i32, i32)> for Point {
    type Output = Point;

    fn add(self, rhs: (i32, i32)) -> Self::Output {
        Point {
            x: ((self.x.cast_signed()) + rhs.0).cast_unsigned(),
            y: ((self.y.cast_signed()) + rhs.1).cast_unsigned(),
        }
    }
}

/// Area from point 1 to point 2
#[derive(Clone)]
pub struct Region {
    pub point1: Point,
    pub point2: Point,
}

/// Constants for screen dimensions
pub struct Dimensions {
    pub width: u32,
    pub height: u32,
}

impl Dimensions {
    #[must_use]
    pub fn find_roblox_button(&self, img: &RgbImage) -> Option<Point> {
        // Find white color of the top left roblox button
        let roblox_button_color = ColorTarget {
            color: Rgb([0xf7, 0xf7, 0xf8]),
            variation: 2,
        };

        let x = self.width * 2 / 100;

        let pos = (0..=(self.height / 2)).rev().find_map(|y| {
            roblox_button_color
                .matches(img.get_pixel(x, y))
                .then_some(Point { x, y })
        });

        #[cfg(feature = "imageproc")]
        {
            if let Some(p) = pos.clone() {
                use crate::utils::debug::Drawable;
                use std::sync::Arc;

                p.clone()
                    .draw_async(Arc::new(img.clone()), "roblox_button.png", true);
            }
        }

        pos
    }

    /// Returns coordinates out of the region
    /// Useful when having to move the mouse out of any zones
    #[must_use]
    pub fn calculate_safe_point(&self, regions: &Vec<&Region>) -> Option<Point> {
        // Use a screen margin of 10%
        let margin_x = self.width * 10 / 100;
        let margin_y = self.height * 10 / 100;

        let allowed_min_x = margin_x;
        let allowed_max_y = self.height - margin_y;

        regions
            .iter()
            .map(|r| {
                let [x_min, _, _, y_max] = r.corners();
                let padding = 20; // extra safety

                // go before and below the region, but not past left and bottom margin
                Point {
                    x: x_min.saturating_sub(padding).max(allowed_min_x),
                    y: (y_max + padding).min(allowed_max_y),
                }
            })
            .min_by_key(|p| p.x + p.y)
    }

    /// Find where is the mini-game region
    #[must_use]
    pub fn calculate_mini_game_region(&self) -> MiniGame {
        MiniGame::new(Region {
            point1: Point {
                x: self.width * 28 / 100,
                y: self.height * 79 / 100,
            },
            point2: Point {
                x: self.width * 72 / 100,
                y: self.height * 90 / 100,
            },
        })
    }

    /// Find where shake bubble appears
    #[must_use]
    pub fn calculate_shake_region(&self, roblox_button_pos: Option<Point>) -> Region {
        let default_region = Region {
            point1: Point {
                x: self.width * 5 / 1000, // 0.5%
                y: self.height * 23 / 100,
            },
            point2: Point {
                x: self.width * 84 / 100,
                y: self.height * 65 / 100,
            },
        };

        if let Some(p) = roblox_button_pos {
            // Derive our region from the roblox button position
            return Region {
                point1: Point {
                    x: default_region.point1.x,
                    y: p.y + self.height * 125 / 1000, // + 12.5%
                },
                point2: Point {
                    x: default_region.point2.x,
                    y: p.y + self.height * 60 / 100,
                },
            };
        }

        default_region
    }
}

impl Region {
    #[must_use]
    pub fn get_size(&self) -> Dimensions {
        Dimensions {
            width: self.point2.x - self.point1.x,
            height: self.point2.y - self.point1.y,
        }
    }

    /// Returns corners: `[x_min, y_min, x_max, y_max]`
    #[must_use]
    pub fn corners(&self) -> [u32; 4] {
        [self.point1.x, self.point1.y, self.point2.x, self.point2.y]
    }
}
