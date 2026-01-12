use image::Rgb;

pub struct ColorTarget {
    pub color: Rgb<u8>,
    pub variation: u8,
}

impl ColorTarget {
    #[must_use]
    pub fn matches(&self, pixel: Rgb<u8>) -> bool {
        let Rgb([tr, tg, tb]) = self.color;
        let v = i16::from(self.variation);

        (i16::from(pixel[0]) - i16::from(tr)).abs() <= v
            && (i16::from(pixel[1]) - i16::from(tg)).abs() <= v
            && (i16::from(pixel[2]) - i16::from(tb)).abs() <= v
    }
}

pub static COLOR_FISH: &[ColorTarget] = &[
    ColorTarget {
        color: Rgb([0x43, 0x4b, 0x5b]),
        variation: 3,
    },
    ColorTarget {
        color: Rgb([0x4a, 0x4a, 0x5c]),
        variation: 4,
    },
    ColorTarget {
        color: Rgb([0x47, 0x51, 0x5d]),
        variation: 4,
    },
];

pub static COLOR_WHITE: &[ColorTarget] = &[ColorTarget {
    color: Rgb([0xff, 0xff, 0xff]),
    variation: 15,
}];

pub static COLOR_HOOK: &[ColorTarget] = &[
    ColorTarget {
        color: Rgb([0x84, 0x85, 0x87]),
        variation: 4,
    },
    ColorTarget {
        color: Rgb([0x78, 0x77, 0x73]),
        variation: 4,
    },
    ColorTarget {
        color: Rgb([0x7a, 0x78, 0x73]),
        variation: 4,
    },
];

pub static COLOR_MINIGAME_ARROWS: ColorTarget = ColorTarget {
    color: Rgb([0x5f, 0x3b, 0x34]),
    variation: 4,
};
