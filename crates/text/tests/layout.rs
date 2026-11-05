//! Outline and layout rules of docs/specs/TEXT.md on the bundled font.

use forge_text::{Align, Contour, Font, Layout};

fn font() -> Font {
    Font::bundled()
}

fn layout(text: &str, f: impl FnOnce(&mut Layout)) -> Vec<Contour> {
    let mut l = Layout::default();
    f(&mut l);
    font().layout(text, &l)
}

/// Piece end points of a contour (its polygon).
fn polygon(c: &Contour) -> Vec<[f64; 2]> {
    c.pieces.iter().map(|p| p[0]).collect()
}

fn area(c: &Contour) -> f64 {
    let pts = polygon(c);
    (0..pts.len())
        .map(|i| {
            let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
            a[0] * b[1] - b[0] * a[1]
        })
        .sum::<f64>()
        / 2.0
}

fn bbox(cs: &[Contour]) -> [f64; 4] {
    let mut b = [f64::MAX, f64::MAX, f64::MIN, f64::MIN];
    for p in cs.iter().flat_map(polygon) {
        b = [
            b[0].min(p[0]),
            b[1].min(p[1]),
            b[2].max(p[0]),
            b[3].max(p[1]),
        ];
    }
    b
}

#[test]
fn contours_are_closed_chains() {
    for c in layout("Forge g&@", |_| {}) {
        let n = c.pieces.len();
        assert!(n >= 2);
        for i in 0..n {
            assert_eq!(c.pieces[i][3], c.pieces[(i + 1) % n][0]);
        }
    }
}

#[test]
fn loops_per_glyph() {
    // O48: A 2, O 2, T 1, e 2, x 1, t 1, F 1, o 2, r 1, g 2, B 3
    for (ch, loops) in [
        ('A', 2),
        ('O', 2),
        ('T', 1),
        ('e', 2),
        ('x', 1),
        ('t', 1),
        ('F', 1),
        ('o', 2),
        ('r', 1),
        ('g', 2),
        ('B', 3),
        ('S', 1),
    ] {
        assert_eq!(layout(&ch.to_string(), |_| {}).len(), loops, "{ch}");
    }
}

#[test]
fn outer_clockwise_hole_counter_clockwise() {
    for ch in ["A", "O"] {
        let c = layout(ch, |_| {});
        assert!(area(&c[0]) < 0.0, "{ch} outer");
        assert!(area(&c[1]) > 0.0, "{ch} hole");
    }
}

#[test]
fn baseline_is_zero_and_pen_starts_at_zero() {
    let b = bbox(&layout("A", |_| {}));
    assert!(b[1].abs() < 1e-9, "A sits on the baseline");
    assert!(b[0] > 0.0 && b[0] < 20.0, "left bearing only");
}

#[test]
fn height_scales_everything() {
    let big = bbox(&layout("AB", |l| l.height = 200.0));
    let small = bbox(&layout("AB", |l| l.height = 50.0));
    for i in 0..4 {
        assert!((small[i] - big[i] / 4.0).abs() < 1e-9, "{i}");
    }
}

#[test]
fn spacing_is_added_per_gap() {
    let plain = layout("ABA", |_| {});
    let spaced = layout("ABA", |l| l.horizontal_spacing = 20.0);
    assert_eq!(plain.len(), spaced.len());
    // A (2 loops), B (3), A (2): shifts 0, 20, 40
    let shifts = [0.0, 20.0, 40.0];
    let owners = [2usize, 3, 2];
    let mut at = 0;
    for (n, shift) in owners.iter().zip(shifts) {
        for k in at..at + n {
            for (p, q) in plain[k].pieces.iter().zip(&spaced[k].pieces) {
                for i in 0..4 {
                    assert!((q[i][0] - p[i][0] - shift).abs() < 1e-9);
                    assert!((q[i][1] - p[i][1]).abs() < 1e-9);
                }
            }
        }
        at += n;
    }
}

#[test]
fn middle_centres_the_ink_and_right_ends_at_zero() {
    let left = bbox(&layout("AB", |_| {}));
    let mid = bbox(&layout("AB", |l| l.align = Align::Middle));
    let right = bbox(&layout("AB", |l| l.align = Align::Right));
    assert!((mid[0] + mid[2]).abs() < 1e-9, "symmetric: {mid:?}");
    assert!((mid[0] - (left[0] - (left[0] + left[2]) / 2.0)).abs() < 1e-9);
    assert!(right[2].abs() < 1e-9);
    // y is never touched by alignment
    assert_eq!((left[1], left[3]), (mid[1], mid[3]));
}

#[test]
fn newline_starts_a_lower_line_at_x_zero() {
    let f = font();
    let one = layout("A", |_| {});
    let two = layout("A\nB", |_| {});
    let b_alone = layout("B", |_| {});
    assert_eq!(two.len(), one.len() + b_alone.len());
    assert_eq!(&two[..one.len()], &one[..]);
    let step = f.line_height_units() * 200.0 / f.units_per_em();
    for (c, d) in two[one.len()..].iter().zip(&b_alone) {
        for (p, q) in c.pieces.iter().zip(&d.pieces) {
            for i in 0..4 {
                assert!((p[i][0] - q[i][0]).abs() < 1e-9);
                assert!((p[i][1] - q[i][1] + step).abs() < 1e-9);
            }
        }
    }
    // the step scales with the height and takes the vertical spacing
    let half = layout("A\nB", |l| l.height = 100.0);
    let low = bbox(&half[..])[1];
    assert!((low - (-step / 2.0)).abs() < 1.0);
    let spaced = layout("A\nB", |l| l.vertical_spacing = 10.0);
    assert!((bbox(&spaced)[1] - (bbox(&two)[1] - 10.0)).abs() < 1e-9);
    // a CRLF is one newline
    assert_eq!(layout("A\r\nB", |_| {}), two);
}

#[test]
fn alignment_is_per_line() {
    let c = layout("A\nBBBB", |l| l.align = Align::Middle);
    let first = bbox(&c[..2]);
    assert!((first[0] + first[2]).abs() < 1e-9);
    let rest = bbox(&c[2..]);
    assert!((rest[0] + rest[2]).abs() < 1e-9);
}

#[test]
fn nothing_to_draw() {
    assert!(layout("", |_| {}).is_empty());
    assert!(layout("   \n ", |_| {}).is_empty());
}

#[test]
fn resolve_falls_back_to_the_bundled_font() {
    let bundled = Font::bundled().layout("A", &Layout::default());
    for name in ["default", "Default", "", "No Such Family 12345"] {
        assert_eq!(Font::resolve(name).layout("A", &Layout::default()), bundled);
    }
    assert_eq!(Font::bundled().family().as_deref(), Some("DejaVu Sans"));
}

#[test]
fn a_missing_character_uses_notdef() {
    let c = layout("\u{4e2d}", |_| {});
    assert!(!c.is_empty());
}
