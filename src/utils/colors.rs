use image::Rgb;

pub struct ColorTarget {
    pub color: Rgb<u8>,
    pub variation: u8,
}

impl ColorTarget {
    #[must_use]
    pub fn matches(&self, pixel: &Rgb<u8>) -> bool {
        let Rgb([tr, tg, tb]) = self.color;
        let v = i16::from(self.variation);

        (i16::from(pixel[0]) - i16::from(tr)).abs() <= v
            && (i16::from(pixel[1]) - i16::from(tg)).abs() <= v
            && (i16::from(pixel[2]) - i16::from(tb)).abs() <= v
    }

    /// Simple luminance approximation
    #[must_use]
    pub fn brightness(pixel: &Rgb<u8>) -> i32 {
        (i32::from(pixel[0]) * 299 + i32::from(pixel[1]) * 587 + i32::from(pixel[2]) * 114) / 1000
    }
}
