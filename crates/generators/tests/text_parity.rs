//! The Text spline (docs/specs/TEXT.md, brief 039) against tests/golden/text (O48).
//!
//! The reference meshes were made with the reference app's default font, which cannot be
//! changed by script and which we must not copy (CLEANROOM.md). Our glyphs come from the
//! bundled font, so no case can match vertex for vertex. This test checks the rules that do
//! not depend on the font (TEXT.md section 0), on the golden mesh and on ours, case by case.
//! Glyph shapes are parity-pending until the observer re-observes `a`, `o` and `word` with
//! a freely licensed font.

use std::path::Path;
use std::sync::Arc;

mod common;

use common::apply;
use forge_core::{Document, EvalCtx, Registry, Value};
use forge_generators::splines::build_curve;
use forge_generators::text_spline::TYPE;
use forge_generators::{Primitives, evaluate};
use forge_parity_harness::{GoldenCase, Mesh, discover_cases};
use forge_text::Font;

const EM: f64 = 200.0;
/// One font unit of the reference font (2048 per em) at height 200, the resolution of every
/// golden coordinate (they are whole font units) and so the bound on the spacing check.
const UNIT: f64 = EM / 2048.0;
/// The golden files hold six decimals.
const TOL: f64 = 1e-4;

type Loop = Vec<[f64; 3]>;

fn registry() -> Arc<Registry> {
    let mut reg = Registry::with_builtins();
    reg.install(&Primitives).unwrap();
    Arc::new(reg)
}

fn cases() -> Vec<GoldenCase> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
    let cases: Vec<_> = discover_cases(&root)
        .unwrap()
        .into_iter()
        .filter(|c| c.feature == "text")
        .collect();
    assert_eq!(cases.len(), 9, "the nine O48 cases");
    cases
}

fn case(name: &str) -> GoldenCase {
    cases().into_iter().find(|c| c.name == name).unwrap()
}

fn input_of(case: &GoldenCase) -> toml::map::Map<String, toml::Value> {
    let m = case.manifest().unwrap();
    m.params["input"].as_table().unwrap().clone()
}

fn offset_of(case: &GoldenCase) -> f64 {
    case.manifest().unwrap().params["extrude_offset"]
        .as_float()
        .unwrap()
}

/// The extrude (caps off) of the Text spline a case describes, with the case's own table
/// edited by `edit`. The direction is the guess of `spline_tools_parity.rs`: negative Z when
/// the loops' signed areas add up clockwise (brief 044 owns the real Auto rule).
fn extrude(
    table: &toml::map::Map<String, toml::Value>,
    offset: f64,
) -> Result<(Mesh, forge_generators::splines::Curve), forge_generators::BuildError> {
    let reg = registry();
    let mut doc = Document::new(reg.clone());
    let (ex, child) = doc
        .transact("case", |tx| {
            let ex = tx.create("forge.extrude", None, None)?;
            for cap in ["cap_start", "cap_end"] {
                tx.set_param(ex, cap, Value::Enum("none".into()))?;
            }
            let child = tx.create(TYPE, Some(ex), None)?;
            apply(tx, &reg, child, TYPE, table);
            Ok((ex, child))
        })
        .unwrap();
    let values = doc.scene().node(child).unwrap().params.clone();
    let curve = build_curve(TYPE, &values)?;
    let total: f64 = curve
        .loops()
        .iter()
        .map(|l| area(&l.iter().map(|p| p.to_array()).collect::<Vec<_>>()))
        .sum();
    let sign = if total < 0.0 { -1.0 } else { 1.0 };
    doc.transact("direction", |tx| {
        tx.set_param(ex, "movement", Value::Vec3([0.0, 0.0, sign * offset]))
    })
    .unwrap();
    let mesh = evaluate(doc.scene(), ex, &EvalCtx::default())?.expect("geometry");
    Ok((
        Mesh {
            positions: mesh.points.iter().map(|p| p.to_array()).collect(),
            faces: mesh.polys.clone(),
        },
        curve,
    ))
}

/// Ours, with `edits` applied to the case's input table.
fn ours_with(case: &GoldenCase, edits: &[(&str, toml::Value)]) -> (Mesh, Vec<Loop>) {
    let mut table = input_of(case);
    for (k, v) in edits {
        table.insert((*k).into(), v.clone());
    }
    let (mesh, _) = extrude(&table, offset_of(case)).unwrap();
    let loops = loops_of(&mesh);
    (mesh, loops)
}

fn ours(case: &GoldenCase) -> (Mesh, Vec<Loop>) {
    ours_with(case, &[])
}

fn golden(case: &GoldenCase) -> (Mesh, Vec<Loop>) {
    let mesh = case.expected_mesh().unwrap().unwrap();
    let loops = loops_of(&mesh);
    (mesh, loops)
}

/// The front (z = 0) points of every connected ring of side faces, in vertex order; rings
/// ordered by their first vertex.
fn loops_of(mesh: &Mesh) -> Vec<Loop> {
    let n = mesh.positions.len();
    let mut parent: Vec<usize> = (0..n).collect();
    fn find(p: &mut [usize], mut x: usize) -> usize {
        while p[x] != x {
            p[x] = p[p[x]];
            x = p[x];
        }
        x
    }
    for f in &mesh.faces {
        for v in &f[1..] {
            let (a, b) = (
                find(&mut parent, f[0] as usize),
                find(&mut parent, *v as usize),
            );
            parent[a] = b;
        }
    }
    let mut rings: Vec<(usize, Loop)> = Vec::new();
    let mut root_of = std::collections::BTreeMap::new();
    for i in 0..n {
        if mesh.positions[i][2].abs() > 1e-9 {
            continue;
        }
        let r = find(&mut parent, i);
        let k = *root_of.entry(r).or_insert_with(|| {
            rings.push((i, Vec::new()));
            rings.len() - 1
        });
        rings[k].1.push(mesh.positions[i]);
    }
    rings.sort_by_key(|(first, _)| *first);
    rings.into_iter().map(|(_, l)| l).collect()
}

fn area(l: &[[f64; 3]]) -> f64 {
    (0..l.len())
        .map(|i| {
            let (a, b) = (l[i], l[(i + 1) % l.len()]);
            a[0] * b[1] - b[0] * a[1]
        })
        .sum::<f64>()
        / 2.0
}

/// `[min x, min y, max x, max y]` of some loops.
fn bbox(loops: &[Loop]) -> [f64; 4] {
    let mut b = [f64::MAX, f64::MAX, f64::MIN, f64::MIN];
    for p in loops.iter().flatten() {
        b = [
            b[0].min(p[0]),
            b[1].min(p[1]),
            b[2].max(p[0]),
            b[3].max(p[1]),
        ];
    }
    b
}

fn s(v: &str) -> toml::Value {
    toml::Value::String(v.into())
}

fn f(v: f64) -> toml::Value {
    toml::Value::Float(v)
}

fn close(a: f64, b: f64, tol: f64, what: &str) {
    assert!((a - b).abs() <= tol, "{what}: {a} vs {b} (tol {tol})");
}

#[test]
fn every_case_builds_with_the_same_structure() {
    // loops per glyph, from the golden meshes: O48 `defaults` Text = T e x t, `word` Forge,
    // `align-center`, `spacing`, `two-lines` A B.
    let glyph_loops = |text: &str| -> usize {
        text.chars()
            .filter(|c| !c.is_whitespace())
            .map(|c| match c {
                'A' | 'O' | 'e' | 'o' | 'g' => 2,
                'B' => 3,
                _ => 1,
            })
            .sum()
    };
    for case in cases() {
        let (gm, gl) = golden(&case);
        let (om, ol) = ours(&case);
        let text = input_of(&case)["text"].as_str().unwrap().to_string();
        let name = &case.name;
        assert_eq!(gl.len(), glyph_loops(&text), "{name}: golden loops");
        assert_eq!(ol.len(), gl.len(), "{name}: loops");
        // one quad per edge, front and back rings, z in {0, -offset}
        for (mesh, loops) in [(&gm, &gl), (&om, &ol)] {
            let points: usize = loops.iter().map(Vec::len).sum();
            assert_eq!(mesh.faces.len(), points, "{name}: one side quad per edge");
            assert_eq!(mesh.positions.len(), 2 * points, "{name}");
            let off = offset_of(&case);
            for p in &mesh.positions {
                assert!(
                    p[2].abs() < 1e-9 || (p[2] + off).abs() < 1e-9,
                    "{name}: extrude must grow toward -Z, got z {}",
                    p[2]
                );
            }
        }
    }
}

#[test]
fn per_glyph_loop_counts() {
    // golden: `a` A=2, `o` O=2, `word` F1 o2 r1 g2 e2, `defaults` T1 e2 x1 t1
    for (ch, n) in [
        ('A', 2),
        ('O', 2),
        ('F', 1),
        ('o', 2),
        ('r', 1),
        ('g', 2),
        ('e', 2),
        ('T', 1),
        ('x', 1),
        ('t', 1),
        ('B', 3),
    ] {
        let layout = Font::bundled().layout(&ch.to_string(), &Default::default());
        assert_eq!(layout.len(), n, "{ch}");
    }
}

#[test]
fn outer_clockwise_holes_counter_clockwise() {
    for name in ["a", "o"] {
        let c = case(name);
        for (who, (_, l)) in [("golden", golden(&c)), ("ours", ours(&c))] {
            assert!(area(&l[0]) < 0.0, "{name} {who}: outer is clockwise");
            assert!(area(&l[1]) > 0.0, "{name} {who}: hole is counter-clockwise");
        }
    }
}

#[test]
fn baseline_and_pen_start_at_zero() {
    for name in ["a", "two-lines"] {
        let c = case(name);
        let first_line = |l: &[Loop]| bbox(&l[..2]);
        for (who, (_, l)) in [("golden", golden(&c)), ("ours", ours(&c))] {
            let b = first_line(&l);
            close(
                b[1],
                0.0,
                TOL,
                &format!("{name} {who}: A sits on the baseline"),
            );
            assert!(
                b[0] > 0.0 && b[0] < 0.1 * EM,
                "{name} {who}: left bearing only"
            );
        }
    }
}

#[test]
fn height_scales_every_coordinate() {
    let (_, big) = golden(&case("a"));
    let (_, small) = golden(&case("height"));
    let (bb, bs) = (bbox(&big), bbox(&small));
    for i in 0..4 {
        close(bs[i], bb[i] / 4.0, TOL, &format!("golden bbox[{i}]"));
    }
    // ours: the same case table at 200 and 50
    let a = case("a");
    let (_, o200) = ours_with(&a, &[("height", f(200.0))]);
    let (_, o50) = ours_with(&a, &[("height", f(50.0))]);
    let (b200, b50) = (bbox(&o200), bbox(&o50));
    for i in 0..4 {
        close(b50[i], b200[i] / 4.0, 1e-9, &format!("ours bbox[{i}]"));
    }
    // and the loops themselves, point by point
    for (p, q) in o200.iter().flatten().zip(o50.iter().flatten()) {
        for i in 0..2 {
            close(q[i], p[i] / 4.0, 1e-9, "point");
        }
    }
}

#[test]
fn middle_centres_the_ink_of_the_line() {
    let (_, mid) = golden(&case("align-center"));
    let (_, spaced) = golden(&case("spacing"));
    let (_, one) = golden(&case("a"));
    let bm = bbox(&mid);
    close(bm[0], -bm[2], TOL, "golden bbox is symmetric");
    // plain AB is the spacing case less its spacing: the ink of AB is as wide as the centred
    // one, and the centred ink starts at -width/2
    let width = bm[2] - bm[0];
    let bs = bbox(&spaced);
    let spacing = (bs[2] - bs[0]) - width;
    // the A of the spacing case is unmoved; centring moved it by -(ink min + ink max) / 2
    let plain_max = bs[2] - spacing;
    let plain_min = bbox(&one)[0];
    let shift = mid[0][0][0] - one[0][0][0];
    close(shift, -(plain_min + plain_max) / 2.0, TOL, "golden shift");
    // ours: symmetric, and the shift is the same formula
    let ab = case("align-center");
    let (_, left) = ours_with(&ab, &[("align", s("left"))]);
    let (_, middle) = ours_with(&ab, &[("align", s("middle"))]);
    let (bl, bo) = (bbox(&left), bbox(&middle));
    close(bo[0], -bo[2], 1e-9, "ours symmetric");
    close(bo[2] - bo[0], bl[2] - bl[0], 1e-9, "ours width");
    close(
        middle[0][0][0] - left[0][0][0],
        -(bl[0] + bl[2]) / 2.0,
        1e-9,
        "ours shift",
    );
    // y never changes
    close(bo[1], bl[1], 1e-9, "y");
    close(bo[3], bl[3], 1e-9, "y");
}

#[test]
fn horizontal_spacing_is_added_once_per_gap() {
    // golden: spacing 20 widens AB by 19.92 (whole font units of the reference font)
    let (_, mid) = golden(&case("align-center"));
    let (_, spaced) = golden(&case("spacing"));
    let (bm, bs) = (bbox(&mid), bbox(&spaced));
    let added = (bs[2] - bs[0]) - (bm[2] - bm[0]);
    close(added, 20.0, UNIT, "golden added width");
    // the first glyph does not move
    let (_, one) = golden(&case("a"));
    for k in 0..2 {
        assert_eq!(one[k], spaced[k], "A is unmoved");
    }
    // ours: shifts 0, 20, 40 for a three-letter word, exactly
    let abc = case("spacing");
    let (_, plain) = ours_with(&abc, &[("text", s("ABA")), ("horizontal_spacing", f(0.0))]);
    let (_, wide) = ours_with(&abc, &[("text", s("ABA")), ("horizontal_spacing", f(20.0))]);
    let mut at = 0;
    for (n, shift) in [(2usize, 0.0), (3, 20.0), (2, 40.0)] {
        for k in at..at + n {
            for (p, q) in plain[k].iter().zip(&wide[k]) {
                close(q[0] - p[0], shift, 1e-9, "x shift");
                close(q[1], p[1], 1e-9, "y");
            }
        }
        at += n;
    }
}

#[test]
fn a_newline_starts_a_lower_line_at_x_zero() {
    let c = case("two-lines");
    let (_, g) = golden(&c);
    let (_, one) = golden(&case("a"));
    assert_eq!(g[..2], one[..], "golden line 1 is the lone A");
    // golden line 2 is the B of `spacing`, moved by a constant in y (and x)
    let (_, spaced) = golden(&case("spacing"));
    let dy = g[2][0][1] - spaced[2][0][1];
    for (p, q) in g[2..].iter().flatten().zip(spaced[2..].iter().flatten()) {
        close(p[1] - q[1], dy, TOL, "constant line step");
        close(p[0] - q[0], g[2][0][0] - spaced[2][0][0], TOL, "constant x");
    }
    assert!(dy < 0.0 && g[2..].iter().flatten().all(|p| p[1] < 0.0));
    // reference font: 1.23 em; ours: the bundled font's own line height, same law
    close(dy, -1.23 * EM, UNIT, "golden step (whole font units)");
    let (_, o) = ours(&c);
    let font = Font::bundled();
    let step = font.line_height_units() / font.units_per_em() * EM;
    let alone = case("a");
    let (_, line1) = ours(&alone);
    assert_eq!(o[..2], line1[..], "ours line 1 is the lone A");
    let b = bbox(&o[2..]);
    assert!(b[3] < 0.0 && b[1] > -2.0 * step);
    // B alone, moved down by one step, is line 2
    let (_, b_alone) = ours_with(&c, &[("text", s("B"))]);
    for (p, q) in o[2..].iter().flatten().zip(b_alone.iter().flatten()) {
        close(p[0], q[0], 1e-9, "x");
        close(p[1], q[1] - step, 1e-9, "y");
    }
}

#[test]
fn intermediate_points_uniform_gives_pieces_times_number_plus_one() {
    let c = case("intermediate");
    let (_, g) = golden(&c);
    assert_eq!(g.len(), 1);
    assert_eq!(g[0].len() % 5, 0, "golden: 28 pieces * (4 + 1)");
    assert_eq!(g[0].len(), 28 * 5);
    let (_, curve) = extrude(&input_of(&c), 20.0).unwrap();
    assert_eq!(curve.extra.len(), 0);
    let loops = curve.loops();
    assert_eq!(loops[0].len(), curve.segments.len() * 5);
    let (_, o) = ours(&c);
    assert_eq!(o[0].len(), loops[0].len());
}

#[test]
fn adaptive_gives_a_straight_letter_only_its_corners() {
    // golden `a`: A has 8 outer + 3 inner points, all corners
    let (_, g) = golden(&case("a"));
    assert_eq!((g[0].len(), g[1].len()), (8, 3));
    let (_, o) = ours(&case("a"));
    let (_, curve) = extrude(&input_of(&case("a")), 20.0).unwrap();
    assert_eq!(o[0].len(), curve.segments.len());
    assert_eq!(o[1].len(), curve.extra[0].segments.len());
}

#[test]
fn the_default_text_is_text() {
    let reg = registry();
    let ty = reg.get(TYPE).unwrap();
    assert_eq!(
        ty.find_param("text").unwrap().default,
        Value::String(input_of(&case("defaults"))["text"].as_str().unwrap().into())
    );
    assert_eq!(ty.find_param("height").unwrap().default, Value::Float(EM));
}
