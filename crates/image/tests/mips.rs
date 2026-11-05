mod common;
use common::*;
use forge_image::*;

fn premult_mean(img: &Image) -> [f64; 4] {
    let mut s = [0.0f64; 4];
    for p in img.data.chunks(4) {
        let a = match img.alpha {
            AlphaMode::Straight => p[3] as f64,
            AlphaMode::Premultiplied => 1.0,
        };
        for c in 0..3 {
            s[c] += p[c] as f64 * a;
        }
        s[3] += p[3] as f64;
    }
    let n = (img.width * img.height) as f64;
    s.map(|v| v / n)
}

#[test]
fn level_sizes() {
    let m = MipChain::build(&gradient(5, 3));
    let dims: Vec<_> = m.levels.iter().map(|l| (l.width, l.height)).collect();
    assert_eq!(dims, vec![(5, 3), (2, 1), (1, 1)]);
    let m = MipChain::build(&gradient(16, 16));
    assert_eq!(m.levels.len(), 5);
    assert_eq!(m.levels.last().unwrap().width, 1);
    assert_eq!(MipChain::build(&gradient(1, 1)).levels.len(), 1);
}

#[test]
fn two_by_two_box() {
    let img = Image::from_fn(2, 2, AlphaMode::Straight, |x, y| {
        [(x + 2 * y) as f32, 0.0, 1.0, 1.0]
    });
    let m = MipChain::build(&img);
    assert_eq!(m.levels[1].pixel(0, 0), [1.5, 0.0, 1.0, 1.0]);
}

#[test]
fn four_by_four_box_values() {
    let img = Image::from_fn(4, 4, AlphaMode::Straight, |x, y| {
        [(x + 4 * y) as f32, 0.0, 0.0, 1.0]
    });
    let l1 = &MipChain::build(&img).levels[1];
    assert_eq!(l1.pixel(0, 0)[0], (0.0 + 1.0 + 4.0 + 5.0) / 4.0);
    assert_eq!(l1.pixel(1, 1)[0], (10.0 + 11.0 + 14.0 + 15.0) / 4.0);
}

#[test]
fn energy_is_preserved_for_odd_sizes_and_alpha() {
    let mut rng = Lcg(5);
    for (w, h) in [(13, 7), (8, 8), (9, 1), (1, 6), (31, 17)] {
        let img = Image::from_fn(w, h, AlphaMode::Straight, |_, _| {
            [
                rng.next_f32() * 4.0,
                rng.next_f32(),
                rng.next_f32(),
                rng.next_f32(),
            ]
        });
        let want = premult_mean(&img);
        let chain = MipChain::build(&img);
        for (i, l) in chain.levels.iter().enumerate() {
            let got = premult_mean(l);
            for c in 0..4 {
                assert!(
                    (got[c] - want[c]).abs() <= 1e-5 * want[c].abs().max(1.0),
                    "{w}x{h} level {i} ch {c}: {} vs {}",
                    got[c],
                    want[c]
                );
            }
        }
    }
}

#[test]
fn transparent_pixels_do_not_bleed_colour() {
    // left pixel opaque red, right pixel transparent green: the mip is red at alpha 0.5
    let img = Image::from_fn(2, 1, AlphaMode::Straight, |x, _| {
        if x == 0 {
            [1.0, 0.0, 0.0, 1.0]
        } else {
            [0.0, 1.0, 0.0, 0.0]
        }
    });
    let p = MipChain::build(&img).levels[1].pixel(0, 0);
    assert!((p[0] - 1.0).abs() < 1e-6 && p[1].abs() < 1e-6, "{p:?}");
    assert!((p[3] - 0.5).abs() < 1e-6);
}

#[test]
fn premultiplied_input_stays_premultiplied() {
    let mut img = gradient(6, 6);
    for p in img.data.chunks_mut(4) {
        p[3] = 0.5;
    }
    let mut pre = img.clone();
    pre.set_alpha_mode(AlphaMode::Premultiplied);
    let a = MipChain::build(&img);
    let b = MipChain::build(&pre);
    assert!(b.levels.iter().all(|l| l.alpha == AlphaMode::Premultiplied));
    let mut a1 = a.levels[2].clone();
    a1.set_alpha_mode(AlphaMode::Premultiplied);
    assert!(max_abs_diff(&a1, &b.levels[2]) < 1e-6);
}

#[test]
fn linear_space_average_differs_from_encoded_average() {
    // a black/white checker: the linear mean is 0.5, which is sRGB 0.735
    let img = Image::from_fn(2, 2, AlphaMode::Straight, |x, y| {
        let v = ((x + y) % 2) as f32;
        [v, v, v, 1.0]
    });
    let p = MipChain::build(&img).levels[1].pixel(0, 0);
    assert!((p[0] - 0.5).abs() < 1e-6);
    assert!((linear_to_srgb(p[0]) - 0.735_357).abs() < 1e-4);
}
