use std::{
    fs::create_dir_all,
    io::{Error, Result},
    path::{Path, PathBuf},
    sync::Arc,
    thread::spawn,
};

use image::{Rgb, RgbImage};
use imageproc::{
    drawing::{draw_hollow_circle, draw_hollow_circle_mut, draw_hollow_rect},
    rect::Rect,
};

use crate::utils::geometry::{Point, Region};

pub trait Drawable {
    /// Draw to image
    fn draw_logic(&self, img: &RgbImage, path: PathBuf);

    /// Draw to image
    fn draw(&self, img: &RgbImage, path: impl AsRef<Path>, overwrite: bool) {
        match Self::unique_path(&path, overwrite) {
            Ok(p) => self.draw_logic(img, p),
            Err(e) => eprintln!("Couldn't save image {}: {e}", path.as_ref().display()),
        }
    }

    /// Draw asynchronously in a background thread
    fn draw_async(self, img: Arc<RgbImage>, path: impl AsRef<Path>, overwrite: bool)
    where
        Self: Sized + Send + 'static,
    {
        let path = path.as_ref().to_owned();
        spawn(move || {
            self.draw(&img, path, overwrite);
        });
    }

    /// Assure that we get an unique path.
    /// Take care of directories
    ///
    /// # Errors
    /// If no good path found
    fn unique_path(p: impl AsRef<Path>, allow_overwrite: bool) -> Result<PathBuf> {
        let p = p.as_ref();

        p.parent()
            .filter(|d| !d.as_os_str().is_empty())
            .map(create_dir_all)
            .transpose()?;

        if allow_overwrite || !p.exists() {
            return Ok(p.to_path_buf());
        }

        let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
        let ext = p.extension().and_then(|e| e.to_str());
        let dir = p.parent().unwrap_or(Path::new(""));

        (0..10_000)
            .map(|i| {
                let name = ext.map_or_else(|| format!("{stem}{i}"), |e| format!("{stem}{i}.{e}"));
                dir.join(name)
            })
            .find(|c| !c.exists())
            .ok_or_else(|| Error::other("No free filename"))
    }
}

impl Drawable for Region {
    fn draw_logic(&self, img: &RgbImage, path: PathBuf) {
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
    fn draw_logic(&self, img: &RgbImage, path: PathBuf) {
        let mut tmp = draw_hollow_circle(
            img,
            (self.x.cast_signed(), self.y.cast_signed()),
            40,
            Rgb([0xff, 0, 0]),
        );

        draw_hollow_circle_mut(
            &mut tmp,
            (self.x.cast_signed(), self.y.cast_signed()),
            5,
            Rgb([0xff, 0, 0]),
        );

        tmp.save(path).ok();
    }
}
