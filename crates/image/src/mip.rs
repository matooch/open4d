//! Mip chains: area-weighted box filter on linear premultiplied RGBA
//! (docs/specs/IMAGE.md §4).

use crate::buffer::Image;
use crate::colour::AlphaMode;

#[derive(Clone, Debug)]
pub struct MipChain {
    /// Level 0 is the source; the last level is 1x1.
    pub levels: Vec<Image>,
}

impl MipChain {
    /// The source must be in the working (linear) space; this does not look at the colour
    /// space, it only averages.
    pub fn build(src: &Image) -> MipChain {
        let mut levels = vec![src.clone()];
        if src.width <= 1 && src.height <= 1 {
            return MipChain { levels };
        }
        let mut cur = src.clone();
        cur.set_alpha_mode(AlphaMode::Premultiplied);
        while cur.width > 1 || cur.height > 1 {
            let next = halve(&cur);
            let mut out = next.clone();
            out.set_alpha_mode(src.alpha);
            levels.push(out);
            cur = next;
        }
        MipChain { levels }
    }

    pub fn bytes(&self) -> usize {
        self.levels.iter().map(Image::bytes).sum()
    }
}

/// `(source index, weight)` pairs covering destination cell `d` of `dn` over `sn` source cells.
fn spans(sn: u32, dn: u32) -> Vec<Vec<(usize, f64)>> {
    (0..dn)
        .map(|d| {
            let x0 = d as f64 * sn as f64 / dn as f64;
            let x1 = (d + 1) as f64 * sn as f64 / dn as f64;
            let first = x0.floor() as usize;
            let last = (x1.ceil() as usize).min(sn as usize);
            (first..last)
                .map(|i| {
                    let w = (i as f64 + 1.0).min(x1) - (i as f64).max(x0);
                    (i, w.max(0.0))
                })
                .collect()
        })
        .collect()
}

fn halve(src: &Image) -> Image {
    let dw = (src.width / 2).max(1);
    let dh = (src.height / 2).max(1);
    let xs = spans(src.width, dw);
    let ys = spans(src.height, dh);
    let norm = (src.width as f64 / dw as f64) * (src.height as f64 / dh as f64);
    let mut out = Image::new(dw, dh, AlphaMode::Premultiplied);
    for (dy, yspan) in ys.iter().enumerate() {
        for (dx, xspan) in xs.iter().enumerate() {
            let mut acc = [0.0f64; 4];
            for &(sy, wy) in yspan {
                for &(sx, wx) in xspan {
                    let i = (sy * src.width as usize + sx) * 4;
                    let w = wx * wy;
                    for (c, a) in acc.iter_mut().enumerate() {
                        *a += src.data[i + c] as f64 * w;
                    }
                }
            }
            let o = (dy * dw as usize + dx) * 4;
            for (c, a) in acc.iter().enumerate() {
                out.data[o + c] = (a / norm) as f32;
            }
        }
    }
    out
}
