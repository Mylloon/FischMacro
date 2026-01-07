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
