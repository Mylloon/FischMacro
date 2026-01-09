use crate::helpers::BadCast;

pub struct Point {
    pub x: u32,
    pub y: u32,
}

/// Area from point 1 to point 2
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
        regions
            .iter()
            .map(|r| {
                let [_, _, x_max, y_max] = r.corners();
                // We substract a magic number for screen dimensions to not indicate a safe point out
                // of Roblox (magic number could be where our taskbar live for example)
                Point {
                    x: x_max.min(self.width - 50),
                    y: (y_max + 30).min(self.height - 50),
                }
            })
            .min_by_key(|p| p.x + p.y)
    }

    /// Find where is the mini-game region
    // TODO: Make this more accurate by double-checking with "anchors"
    #[must_use]
    pub fn calculate_mini_game_region(&self) -> Region {
        Region {
            point1: Point {
                x: (self.width.cast_signed().bad_cast() * 0.28)
                    .bad_cast()
                    .cast_unsigned(),
                y: (self.height.cast_signed().bad_cast() * 0.8)
                    .bad_cast()
                    .cast_unsigned(),
            },
            point2: Point {
                x: (self.width.cast_signed().bad_cast() * 0.72)
                    .bad_cast()
                    .cast_unsigned(),
                y: (self.height.cast_signed().bad_cast() * 0.85)
                    .bad_cast()
                    .cast_unsigned(),
            },
        }
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
