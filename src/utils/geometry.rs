use std::ops::Add;

use crate::utils::{fishing::MiniGame, helpers::BadCast};

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

// Constants for screen dimensions
pub struct Dimensions {
    pub width: u32,
    pub height: u32,
}

impl Dimensions {
    /// Returns coordinates out of the region
    /// Useful when having to move the mouse out of any zones
    #[must_use]
    pub fn calculate_safe_point(&self, regions: &Vec<&Region>) -> Option<Point> {
        // Use a screen margin of 10%
        let margin_x = (self.width.cast_signed().bad_cast() * 0.1)
            .bad_cast()
            .cast_unsigned();
        let margin_y = (self.height.cast_signed().bad_cast() * 0.1)
            .bad_cast()
            .cast_unsigned();

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
                x: (self.width.cast_signed().bad_cast() * 0.28)
                    .bad_cast()
                    .cast_unsigned(),
                y: (self.height.cast_signed().bad_cast() * 0.79)
                    .bad_cast()
                    .cast_unsigned(),
            },
            point2: Point {
                x: (self.width.cast_signed().bad_cast() * 0.72)
                    .bad_cast()
                    .cast_unsigned(),
                y: (self.height.cast_signed().bad_cast() * 0.9)
                    .bad_cast()
                    .cast_unsigned(),
            },
        })
    }

    /// Find where shake bubble appears
    // TODO: Make this more accurate by double-checking with "anchors"
    #[must_use]
    pub fn calculate_shake_region(&self) -> Region {
        Region {
            point1: Point {
                x: (self.width.cast_signed().bad_cast() * 0.005)
                    .bad_cast()
                    .cast_unsigned(),
                y: (self.height.cast_signed().bad_cast() * 0.185)
                    .bad_cast()
                    .cast_unsigned(),
            },
            point2: Point {
                x: (self.width.cast_signed().bad_cast() * 0.84)
                    .bad_cast()
                    .cast_unsigned(),
                y: (self.height.cast_signed().bad_cast() * 0.65)
                    .bad_cast()
                    .cast_unsigned(),
            },
        }
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

    // Returns corners
    #[must_use]
    pub fn corners(&self) -> [u32; 4] {
        [self.point1.x, self.point1.y, self.point2.x, self.point2.y]
    }
}
