//! docs/specs/SCENE-OBJECTS.md S9: the Null's display (brief 046, observation O20b in
//! docs/parity/generators-3.md). Measured extents carry about 1-2 units of line width, so each
//! shape is checked against the analytic rule exactly and against the measurement within 2.

use std::sync::Arc;

use forge_core::{Document, EvalCtx, NodeId, Registry, Value};
use forge_generators::null_display::*;
use forge_generators::{Primitives, evaluate};
use glam::DVec3;

const R: f64 = 100.0;
const FRONT: ViewFrame = ViewFrame::FRONT;
/// Top view: X to the right, world Z up the screen.
const TOP: ViewFrame = ViewFrame {
    right: DVec3::X,
    up: DVec3::Z,
    toward: DVec3::Y,
};

fn lines(shape: NullShape, r: f64, a: f64, o: NullOrientation, v: &ViewFrame) -> Vec<Segment> {
    null_display_lines(shape, r, a, o, v)
}

/// (min, max) of the lines' screen coordinates in a view.
fn bounds(ls: &[Segment], v: &ViewFrame) -> ([f64; 2], [f64; 2]) {
    let mut lo = [f64::MAX; 2];
    let mut hi = [f64::MIN; 2];
    for p in ls.iter().flatten() {
        for (k, axis) in [v.right, v.up].into_iter().enumerate() {
            lo[k] = lo[k].min(p.dot(axis));
            hi[k] = hi[k].max(p.dot(axis));
        }
    }
    (lo, hi)
}

fn size(ls: &[Segment], v: &ViewFrame) -> [f64; 2] {
    let (lo, hi) = bounds(ls, v);
    [hi[0] - lo[0], hi[1] - lo[1]]
}

fn near(a: f64, b: f64, tol: f64, what: &str) {
    assert!((a - b).abs() <= tol, "{what}: {a} vs {b} (tol {tol})");
}

fn front(shape: NullShape) -> Vec<Segment> {
    lines(shape, R, 1.0, NullOrientation::Z, &FRONT)
}

fn is_loop(ls: &[Segment]) -> bool {
    ls.iter().enumerate().all(|(i, s)| {
        let next = &ls[(i + 1) % ls.len()];
        s[1].distance(next[0]) < 1e-9
    })
}

#[test]
fn shapes_match_the_measured_extents() {
    use NullShape::*;
    // (shape, analytic size, measured size, analytic min y, measured min y, analytic max y, measured max y)
    let table: [(NullShape, [f64; 2], [f64; 2]); 11] = [
        (Locator, [200.0, 200.0], [198.7, 198.7]),
        (Circle, [200.0, 200.0], [201.3, 201.3]),
        (
            Rectangle,
            [R * 2f64.sqrt(), R * 2f64.sqrt()],
            [142.7, 142.7],
        ),
        (Diamond, [200.0, 200.0], [198.7, 198.7]),
        (Triangle, [R * 3f64.sqrt(), 150.0], [173.2, 150.3]),
        (
            Pentagon,
            [
                2.0 * R * 72f64.to_radians().sin(),
                R * (1.0 + 36f64.to_radians().cos()),
            ],
            [191.1, 180.9],
        ),
        (Hexagon, [200.0, R * 3f64.sqrt()], [198.7, 173.2]),
        (
            Octagon,
            [2.0 * R * 22.5f64.to_radians().cos(); 2],
            [186.0, 186.0],
        ),
        (Star, [200.0, 200.0], [198.7, 198.7]),
        (Cube, [200.0, 200.0], [201.3, 201.3]),
        (Sphere, [200.0, 200.0], [201.3, 201.3]),
    ];
    for (shape, analytic, measured) in table {
        let s = size(&front(shape), &FRONT);
        for k in 0..2 {
            near(s[k], analytic[k], 1e-9, &format!("{shape:?} analytic {k}"));
            near(s[k], measured[k], 2.0, &format!("{shape:?} measured {k}"));
        }
    }
    // pyramid: the square (and its diagonals) of side 2R
    let s = size(&front(Pyramid), &FRONT);
    near(s[0], 201.3, 2.0, "pyramid x");
    near(s[1], 201.3, 2.0, "pyramid y");
}

#[test]
fn vertical_extents_of_the_triangle_pentagon_hexagon_and_axis() {
    use NullShape::*;
    // triangle y -50.3..98.7, pentagon y -80.9..98.7, hexagon y +-86.0, axis x 0..100.6 y -0.6..98.7
    let (lo, hi) = bounds(&front(Triangle), &FRONT);
    near(lo[1], -R / 2.0, 1e-9, "triangle base");
    near(lo[1], -50.3, 2.0, "triangle base measured");
    near(hi[1], R, 1e-9, "triangle apex");
    near(hi[1], 98.7, 2.0, "triangle apex measured");
    let (lo, hi) = bounds(&front(Pentagon), &FRONT);
    near(lo[1], -R * 36f64.to_radians().cos(), 1e-9, "pentagon base");
    near(lo[1], -80.9, 2.0, "pentagon base measured");
    near(hi[1], R, 1e-9, "pentagon apex");
    let (lo, hi) = bounds(&front(Hexagon), &FRONT);
    near(hi[1], R * 3f64.sqrt() / 2.0, 1e-9, "hexagon top");
    near(hi[1], 86.0, 2.0, "hexagon top measured");
    near(lo[1], -hi[1], 1e-9, "hexagon symmetric");
    let (lo, hi) = bounds(&front(Axis), &FRONT);
    near(lo[0], 0.0, 1e-9, "axis starts at the origin in x");
    near(lo[1], 0.0, 1e-9, "axis starts at the origin in y");
    near(hi[0], R, 1e-9, "axis +X arm");
    near(hi[0], 100.6, 2.0, "axis +X arm measured");
    near(hi[1], R, 1e-9, "axis +Y arm");
    near(hi[1], 98.7, 2.0, "axis +Y arm measured");
}

#[test]
fn octagon_has_flat_sides_on_the_axes_and_hexagon_vertices_left_right() {
    let (_, hi) = bounds(&front(NullShape::Octagon), &FRONT);
    // vertices on the axes would reach R = 100 (198.7 measured would match 2R); 186.0 does not
    assert!(hi[0] < 95.0);
    let hex = front(NullShape::Hexagon);
    assert!(
        hex.iter()
            .flatten()
            .any(|p| p.distance(DVec3::new(R, 0.0, 0.0)) < 1e-9)
    );
    assert!(
        hex.iter()
            .flatten()
            .any(|p| p.distance(DVec3::new(-R, 0.0, 0.0)) < 1e-9)
    );
    // triangle and pentagon: a vertex straight up
    for s in [NullShape::Triangle, NullShape::Pentagon] {
        assert!(
            front(s)
                .iter()
                .flatten()
                .any(|p| p.distance(DVec3::new(0.0, R, 0.0)) < 1e-9)
        );
    }
}

#[test]
fn segment_counts_follow_the_observation() {
    use NullShape::*;
    let expected = [
        (None, 0),
        (Dot, 0),
        (Locator, 2),
        (Circle, CIRCLE_SEGMENTS),
        (Rectangle, 4),
        (Diamond, 4),
        (Triangle, 3),
        (Pentagon, 5),
        (Hexagon, 6),
        (Octagon, 8),
        (Star, 16),
        (Axis, 2),
        (Cube, 12),
        (Pyramid, 8),
        (Sphere, 3 * CIRCLE_SEGMENTS),
    ];
    for (shape, n) in expected {
        assert_eq!(front(shape).len(), n, "{shape:?}");
    }
    for s in [
        Circle, Rectangle, Diamond, Triangle, Pentagon, Hexagon, Octagon, Star,
    ] {
        assert!(is_loop(&front(s)), "{s:?} is a closed outline");
    }
    // the Axis arms are open lines that start at the origin
    for seg in front(Axis) {
        assert_eq!(seg[0], DVec3::ZERO);
    }
    assert_eq!(null_display_points(Dot), vec![DVec3::ZERO]);
    assert!(null_display_points(Circle).is_empty());
}

#[test]
fn a_cube_seen_from_the_front_is_a_square_and_the_pyramid_has_diagonals() {
    let key = |p: DVec3| ((p.x * 1e6).round() as i64, (p.y * 1e6).round() as i64);
    let distinct = |ls: &[Segment]| {
        let mut set = std::collections::BTreeSet::new();
        for s in ls {
            let (a, b) = (key(s[0]), key(s[1]));
            if a != b {
                set.insert(if a < b { (a, b) } else { (b, a) });
            }
        }
        set.len()
    };
    assert_eq!(distinct(&front(NullShape::Cube)), 4, "no inner lines");
    // square outline (4) plus the four apex lines, which lie on the two diagonals
    let pyr = front(NullShape::Pyramid);
    assert_eq!(distinct(&pyr), 8);
    let apex_lines = pyr.iter().filter(|s| key(s[1]) == (0, 0)).count();
    assert_eq!(apex_lines, 4);
    for s in pyr.iter().filter(|s| key(s[1]) == (0, 0)) {
        near(s[0].x.abs(), R, 1e-9, "diagonal corner x");
        near(s[0].y.abs(), R, 1e-9, "diagonal corner y");
    }
    // the sphere: a circle plus a horizontal and a vertical diameter (the edge-on circles)
    let sph = front(NullShape::Sphere);
    let flat = |axis: DVec3| {
        sph.iter()
            .filter(|s| s[0].dot(axis).abs() < 1e-9 && s[1].dot(axis).abs() < 1e-9)
            .count()
    };
    assert_eq!(flat(DVec3::Z), CIRCLE_SEGMENTS, "the (u, v) circle");
    assert_eq!(flat(DVec3::Y), CIRCLE_SEGMENTS, "edge-on horizontal");
    assert_eq!(flat(DVec3::X), CIRCLE_SEGMENTS, "edge-on vertical");
}

#[test]
fn every_shape_is_centred_on_the_null() {
    use NullShape::*;
    // the circumcentre is the origin: vertices of the regular polygons lie on the radius circle
    for s in [Rectangle, Diamond, Triangle, Pentagon, Hexagon, Octagon] {
        for p in front(s).iter().flatten() {
            near(p.length(), R, 1e-9, &format!("{s:?} vertex radius"));
        }
    }
    // centrally symmetric shapes have their bounding box centred (the measurement: within 1.9)
    for s in [
        Locator, Circle, Rectangle, Diamond, Hexagon, Octagon, Star, Cube, Pyramid, Sphere,
    ] {
        let (lo, hi) = bounds(&front(s), &FRONT);
        near(lo[0] + hi[0], 0.0, 1e-9, &format!("{s:?} x centre"));
        near(lo[1] + hi[1], 0.0, 1e-9, &format!("{s:?} y centre"));
    }
    // the triangle's bounding box centre is +24.7 up (measured +24.2): it hangs from the origin
    let (lo, hi) = bounds(&front(Triangle), &FRONT);
    near((lo[1] + hi[1]) / 2.0, 25.0, 1e-9, "triangle bbox centre");
    near(
        (lo[1] + hi[1]) / 2.0,
        24.2,
        2.0,
        "triangle bbox centre measured",
    );
}

#[test]
fn star_outer_tips_sit_on_the_radius_and_alternate_with_inner_points() {
    let star = front(NullShape::Star);
    for (k, s) in star.iter().enumerate() {
        let expect = if k % 2 == 0 { R } else { R * STAR_INNER };
        near(s[0].length(), expect, 1e-9, "star vertex");
    }
    // 8 outer tips: one straight up
    assert!(
        star.iter()
            .any(|s| s[0].distance(DVec3::new(0.0, R, 0.0)) < 1e-9)
    );
}

#[test]
fn aspect_scales_the_vertical_axis_of_flat_shapes() {
    use NullShape::*;
    // Rectangle R50 AR2: X 71.3 (50 sqrt 2 = 70.7), Y 142.7 (2 x 70.7)
    let s = size(
        &lines(Rectangle, 50.0, 2.0, NullOrientation::Z, &FRONT),
        &FRONT,
    );
    near(s[0], 50.0 * 2f64.sqrt(), 1e-9, "x unchanged");
    near(s[1], 100.0 * 2f64.sqrt(), 1e-9, "y doubled");
    near(s[0], 71.3, 2.0, "x measured");
    near(s[1], 142.7, 2.0, "y measured");
    for shape in [
        Locator, Circle, Diamond, Triangle, Pentagon, Hexagon, Octagon, Star, Axis,
    ] {
        let one = size(&lines(shape, R, 1.0, NullOrientation::Z, &FRONT), &FRONT);
        let two = size(&lines(shape, R, 2.0, NullOrientation::Z, &FRONT), &FRONT);
        near(two[0], one[0], 1e-9, &format!("{shape:?} x"));
        near(two[1], 2.0 * one[1], 1e-9, &format!("{shape:?} y"));
    }
    // aspect 0 flattens to a line, and radius 0 to a point
    let flat = size(
        &lines(Rectangle, R, 0.0, NullOrientation::Z, &FRONT),
        &FRONT,
    );
    near(flat[1], 0.0, 1e-9, "aspect 0");
    let point = size(&lines(Circle, 0.0, 1.0, NullOrientation::Z, &FRONT), &FRONT);
    assert_eq!(point, [0.0, 0.0]);
}

#[test]
fn aspect_does_not_change_the_solids() {
    for shape in [NullShape::Cube, NullShape::Pyramid, NullShape::Sphere] {
        for o in [
            NullOrientation::Camera,
            NullOrientation::X,
            NullOrientation::Y,
            NullOrientation::Z,
        ] {
            assert_eq!(
                lines(shape, 50.0, 2.0, o, &FRONT),
                lines(shape, 50.0, 1.0, o, &FRONT),
                "{shape:?} {o:?}"
            );
        }
    }
    // R50 AR2 pyramid, Front and Top: 101.9 x 101.9 measured
    let p = lines(
        NullShape::Pyramid,
        50.0,
        2.0,
        NullOrientation::Camera,
        &FRONT,
    );
    for v in [&FRONT, &TOP] {
        let s = size(&p, v);
        near(s[0], 101.9, 2.0, "pyramid x");
        near(s[1], 101.9, 2.0, "pyramid y");
    }
}

#[test]
fn each_orientation_lies_in_its_plane() {
    use NullOrientation::*;
    let circle = |o, v: &ViewFrame| lines(NullShape::Circle, R, 1.0, o, v);
    let dims = |o, v: &ViewFrame| size(&circle(o, v), v);
    // (orientation, normal axis, front view size, top view size): lines are 0 wide
    let line_v = [0.0, 200.0];
    let line_h = [200.0, 0.0];
    let full = [200.0, 200.0];
    let cases = [
        (X, DVec3::X, line_v, line_v),
        (Y, DVec3::Y, line_h, full),
        (Z, DVec3::Z, full, line_h),
    ];
    for (o, normal, f, t) in cases {
        for p in circle(o, &FRONT).iter().flatten() {
            near(p.dot(normal), 0.0, 1e-9, &format!("{o:?} plane"));
        }
        let (got_f, got_t) = (dims(o, &FRONT), dims(o, &TOP));
        for k in 0..2 {
            near(got_f[k], f[k], 1e-9, &format!("{o:?} front {k}"));
            near(got_t[k], t[k], 1e-9, &format!("{o:?} top {k}"));
        }
    }
    // measured: X front 2.5 x 198.7, Top 1.3 x 198.7; Y front 198.7 x 2.5; Z top 198.7 x 1.3
    near(dims(X, &FRONT)[1], 198.7, 2.0, "x front measured");
    near(dims(Y, &FRONT)[0], 198.7, 2.0, "y front measured");
    near(dims(Z, &TOP)[0], 198.7, 2.0, "z top measured");
}

#[test]
fn camera_orientation_follows_the_view() {
    let circle = |v: &ViewFrame| lines(NullShape::Circle, R, 1.0, NullOrientation::Camera, v);
    for v in [&FRONT, &TOP] {
        let s = size(&circle(v), v);
        near(s[0], 200.0, 1e-9, "full circle across");
        near(s[1], 200.0, 1e-9, "full circle up");
        for p in circle(v).iter().flatten() {
            near(p.dot(v.toward), 0.0, 1e-9, "in the view plane");
        }
    }
    // an oblique view: the circle lies in the plane spanned by right and up
    let d = DVec3::new(1.0, 2.0, -2.0).normalize();
    let right = DVec3::Y.cross(d).normalize();
    let oblique = ViewFrame {
        right,
        up: d.cross(right),
        toward: -d,
    };
    assert_ne!(circle(&oblique), circle(&FRONT));
    for p in circle(&oblique).iter().flatten() {
        near(p.dot(d), 0.0, 1e-9, "oblique plane");
        near(p.length(), R, 1e-9, "radius");
    }
    assert!(follows_view(NullShape::Circle, NullOrientation::Camera));
    assert!(!follows_view(NullShape::Circle, NullOrientation::X));
    // solids use the world axes whatever the view
    assert!(!follows_view(NullShape::Cube, NullOrientation::Camera));
    for s in [NullShape::Cube, NullShape::Pyramid, NullShape::Sphere] {
        assert_eq!(
            lines(s, R, 1.0, NullOrientation::Camera, &oblique),
            lines(s, R, 1.0, NullOrientation::Camera, &FRONT)
        );
    }
}

#[test]
fn flat_shapes_keep_up_up_in_every_fixed_plane() {
    // the Triangle's apex is +Y in the X and Z planes and +Z in the Y plane (S9.2)
    for (o, up) in [
        (NullOrientation::X, DVec3::Y),
        (NullOrientation::Z, DVec3::Y),
        (NullOrientation::Y, DVec3::Z),
    ] {
        let t = lines(NullShape::Triangle, R, 1.0, o, &FRONT);
        assert!(
            t.iter().flatten().any(|p| p.distance(up * R) < 1e-9),
            "{o:?}"
        );
    }
}

// --- node, defaults, files ---

fn doc() -> Document {
    let mut r = Registry::with_builtins();
    r.install(&Primitives).unwrap();
    Document::new(Arc::new(r))
}

fn new_null(d: &mut Document) -> NodeId {
    let mut id = None;
    d.transact("null", |tx| {
        id = Some(tx.create(TYPE, None, None)?);
        Ok(())
    })
    .unwrap();
    id.unwrap()
}

#[test]
fn a_new_null_has_the_observed_defaults() {
    let mut d = doc();
    let n = new_null(&mut d);
    let ty = d.scene().node_type(n).unwrap().clone();
    let default = |k: &str| ty.find_param(k).unwrap().default.clone();
    assert_eq!(default("shape"), Value::Enum("dot".into()));
    assert_eq!(default("radius"), Value::Float(10.0));
    assert_eq!(default("aspect"), Value::Float(1.0));
    assert_eq!(default("orientation"), Value::Enum("camera".into()));
    assert_eq!(default("display_color"), Value::Enum("material".into()));
    assert_eq!(default("color"), Value::Color([1.0, 1.0, 1.0]));
    assert_eq!(default("icon_color"), Value::Color([1.0, 0.9, 0.4]));
    assert_eq!(default("xray"), Value::Bool(false));
    let data = null_data(d.scene(), n).unwrap();
    assert_eq!(data.shape, NullShape::Dot);
    assert_eq!(data.radius, 10.0);
    assert_eq!(data.aspect, 1.0);
    assert_eq!(data.orientation, NullOrientation::Camera);
    assert_eq!(data.color_mode, NullColorMode::Material);
    assert!(!data.xray);
}

#[test]
fn the_param_options_and_ranges_are_the_observed_ones() {
    use forge_core::ParamKind;
    let d = doc();
    let ty = d.scene().registry().get(TYPE).unwrap().clone();
    let ParamKind::Enum { options } = &ty.find_param("shape").unwrap().kind else {
        panic!("shape is an enum")
    };
    let ids: Vec<&str> = options.iter().map(|o| o.id.as_str()).collect();
    let labels: Vec<&str> = options.iter().map(|o| o.label.as_str()).collect();
    assert_eq!(
        labels,
        [
            "None",
            "Dot",
            "Locator",
            "Circle",
            "Rectangle",
            "Diamond",
            "Triangle",
            "Pentagon",
            "Hexagon",
            "Octagon",
            "Star",
            "Axis",
            "Cube",
            "Pyramid",
            "Sphere"
        ]
    );
    // every option parses and round-trips to the same id
    for (id, shape) in ids.iter().zip(NullShape::ALL) {
        assert_eq!(NullShape::from_id(id), Some(shape));
        assert_eq!(shape.id(), *id);
    }
    let ParamKind::Enum { options } = &ty.find_param("orientation").unwrap().kind else {
        panic!()
    };
    assert_eq!(
        options.iter().map(|o| o.label.as_str()).collect::<Vec<_>>(),
        ["Camera", "X", "Y", "Z"]
    );
    let ParamKind::Enum { options } = &ty.find_param("display_color").unwrap().kind else {
        panic!()
    };
    assert_eq!(
        options.iter().map(|o| o.label.as_str()).collect::<Vec<_>>(),
        ["Material", "Layer", "Automatic", "Custom"]
    );
    // Radius and Aspect Ratio: min 0, max 1e20
    for k in ["radius", "aspect"] {
        let ParamKind::Float { min, max } = ty.find_param(k).unwrap().kind else {
            panic!()
        };
        assert_eq!((min, max), (0.0, 1.0e20), "{k}");
    }
    let mut d = doc();
    let n = new_null(&mut d);
    // a value below the range is clamped to the minimum (CORE.md)
    d.transact("neg", |tx| tx.set_param(n, "radius", Value::Float(-1.0)))
        .unwrap();
    assert_eq!(null_data(d.scene(), n).unwrap().radius, 0.0);
}

#[test]
fn the_null_params_round_trip_through_a_file() {
    let mut d = doc();
    let n = new_null(&mut d);
    for (k, v) in [
        ("shape", Value::Enum("star".into())),
        ("radius", Value::Float(123.5)),
        ("aspect", Value::Float(0.25)),
        ("orientation", Value::Enum("y".into())),
        ("display_color", Value::Enum("custom".into())),
        ("color", Value::Color([0.1, 0.2, 0.3])),
        ("xray", Value::Bool(true)),
    ] {
        d.transact("set", |tx| tx.set_param(n, k, v)).unwrap();
    }
    let text = forge_core::file::save(d.scene());
    let (loaded, report) = forge_core::file::load(d.scene().registry().clone(), &text).unwrap();
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
    assert_eq!(forge_core::file::save(&loaded), text);
    let data = null_data(&loaded, n).unwrap();
    assert_eq!(data.shape, NullShape::Star);
    assert_eq!(data.radius, 123.5);
    assert_eq!(data.aspect, 0.25);
    assert_eq!(data.orientation, NullOrientation::Y);
    assert_eq!(data.color_mode, NullColorMode::Custom);
    assert_eq!(data.color, [0.1, 0.2, 0.3]);
    assert!(data.xray);
}

#[test]
fn a_file_saved_before_the_params_loads_with_the_defaults() {
    // the shape of CORE.md's example: a Null with no params at all
    let old = r#"(
    format: "forge3d-scene",
    version: 7,
    settings: (fps: 30, start: 0, end: 90),
    time: (preview: None, mode: Loop, drop_frames: true, display: Frames, markers: []),
    roots: [
        (id: 1, type_id: "forge.null", name: "Null", params: {}, tracks: {}, tags: [], children: []),
    ],
)"#;
    let d = doc();
    let (scene, report) = forge_core::file::load(d.scene().registry().clone(), old).unwrap();
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
    let data = null_data(&scene, scene.roots()[0]).unwrap();
    assert_eq!(data.shape, NullShape::Dot);
    assert_eq!(data.radius, 10.0);
    assert_eq!(data.aspect, 1.0);
    assert_eq!(data.orientation, NullOrientation::Camera);
}

#[test]
fn null_never_contributes_geometry() {
    let mut d = doc();
    let n = new_null(&mut d);
    for shape in NullShape::ALL {
        d.transact("shape", |tx| {
            tx.set_param(n, "shape", Value::Enum(shape.id().into()))
        })
        .unwrap();
        assert!(
            evaluate(d.scene(), n, &EvalCtx::default())
                .unwrap()
                .is_none(),
            "{shape:?} has no mesh"
        );
    }
}
