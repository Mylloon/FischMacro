use std::{path::Path, sync::Arc, thread::spawn};

use image::{Rgb, RgbImage};
use imageproc::{
    drawing::{draw_hollow_circle, draw_hollow_rect},
    rect::Rect,
};

use crate::utils::geometry::{Point, Region};

pub trait Drawable {
    /// Draw to image
    fn draw(&self, img: &RgbImage, path: impl AsRef<Path>);

    /// Draw asynchronously in a background thread
    fn draw_async(self, img: Arc<RgbImage>, path: impl AsRef<Path>)
    where
        Self: Sized + Send + 'static,
    {
        let path = path.as_ref().to_owned();
        spawn(move || {
            self.draw(&img, path);
        });
    }
}

impl Drawable for Region {
    fn draw(&self, img: &RgbImage, path: impl AsRef<Path>) {
        let dims = self.get_size();
        draw_hollow_rect(
            img,
            Rect::at(self.point1.x.cast_signed(), self.point1.y.cast_signed())
                .of_size(dims.width, dims.height),
            Rgb([0xff, 0, 0]),
        )
        .save(path)
        .ok();
    }
}

impl Drawable for Point {
    fn draw(&self, img: &RgbImage, path: impl AsRef<Path>) {
        draw_hollow_circle(
            img,
            (self.x.cast_signed(), self.y.cast_signed()),
            40,
            Rgb([0xff, 0, 0]),
        )
        .save(path)
        .ok();
    }
}
