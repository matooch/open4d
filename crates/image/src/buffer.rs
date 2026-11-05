//! The in-memory pixel type (docs/specs/IMAGE.md §2).

use crate::colour::{AlphaMode, premultiply, unpremultiply};

/// `f32` RGBA, interleaved, row 0 at the top.
#[derive(Clone, Debug, PartialEq)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub data: Vec<f32>,
    pub alpha: AlphaMode,
}

impl Image {
    /// All zero (transparent black).
    pub fn new(width: u32, height: u32, alpha: AlphaMode) -> Image {
        Image {
            width,
            height,
            data: vec![0.0; width as usize * height as usize * 4],
            alpha,
        }
    }

    pub fn from_fn(
        width: u32,
        height: u32,
        alpha: AlphaMode,
        mut f: impl FnMut(u32, u32) -> [f32; 4],
    ) -> Image {
        let mut data = Vec::with_capacity(width as usize * height as usize * 4);
        for y in 0..height {
            for x in 0..width {
                data.extend_from_slice(&f(x, y));
            }
        }
        Image {
            width,
            height,
            data,
            alpha,
        }
    }

    pub fn pixel(&self, x: u32, y: u32) -> [f32; 4] {
        let i = (y as usize * self.width as usize + x as usize) * 4;
        [
            self.data[i],
            self.data[i + 1],
            self.data[i + 2],
            self.data[i + 3],
        ]
    }

    pub fn set_pixel(&mut self, x: u32, y: u32, p: [f32; 4]) {
        let i = (y as usize * self.width as usize + x as usize) * 4;
        self.data[i..i + 4].copy_from_slice(&p);
    }

    /// Memory used by the pixels.
    pub fn bytes(&self) -> usize {
        self.data.len() * std::mem::size_of::<f32>()
    }

    /// Converts the colour between straight and premultiplied in place.
    pub fn set_alpha_mode(&mut self, mode: AlphaMode) {
        if self.alpha == mode {
            return;
        }
        match mode {
            AlphaMode::Premultiplied => premultiply(&mut self.data),
            AlphaMode::Straight => unpremultiply(&mut self.data),
        }
        self.alpha = mode;
    }
}
