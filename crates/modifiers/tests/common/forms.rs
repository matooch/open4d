//! Case-file reader of the deformer parity tests for the menu-created case form (next to the
//! R20 defaults overlay of `common/mod.rs`).
//!
//! The deformer cases come in two forms:
//!
//! * the older form: deformer settings directly under `[params]`, the object under
//!   `[params.input]` (with `type = "cube"` and our key names), splines under
//!   `[params.spline]`, `[params.left_helix]`, ... ;
//! * the menu-created form (follow-up cases): `frame` / `fps` under `[params]`, the deformer's
//!   settings under `[params.deformer.parameters]`, the object's under
//!   `[params.input.parameters]` (a plane's under `[params.source.parameters]` when the cube
//!   was made editable into a plane), linked helper splines under `[params.helpers.N]` (named
//!   by the deformer's link values) and `[params.editable_helpers.N]` (points), the FFD lattice
//!   under `[params.lattice]`.
//!
//! [`normalize`] turns the second form into the first, so each test builds its scene one way.
#![allow(dead_code)]

pub type Table = toml::map::Map<String, toml::Value>;

pub fn num(v: &toml::Value) -> f64 {
    v.as_float()
        .or(v.as_integer().map(|i| i as f64))
        .expect("number")
}

pub fn vec3(v: &toml::Value) -> [f64; 3] {
    let a = v.as_array().expect("vec3");
    [num(&a[0]), num(&a[1]), num(&a[2])]
}

/// Keys that carry the object's look, not a parameter.
const COSMETIC: &[&str] = &["icon_file_id", "icon_color", "color"];

fn table_of<'a>(t: &'a Table, key: &str) -> Option<&'a Table> {
    t.get(key).and_then(|v| v.as_table())
}

/// A `[... .parameters]` table in our key names: cosmetic keys dropped, `*_degrees` shortened
/// to `*_deg`.
fn our_names(t: &Table) -> Table {
    t.iter()
        .filter(|(k, _)| !COSMETIC.contains(&k.as_str()))
        .map(|(k, v)| {
            let key = match k.as_str() {
                // The Formula's four expression rows are labelled by their variables.
                "d_u_v_x_y_z_t" => "formula_d".to_string(),
                "x_x_y_z_t" => "formula_x".to_string(),
                "y_x_y_z_t" => "formula_y".to_string(),
                "z_x_y_z_t" => "formula_z".to_string(),
                _ => k
                    .strip_suffix("_degrees")
                    .map_or(k.clone(), |b| format!("{b}_deg")),
            };
            (key, v.clone())
        })
        .collect()
}

/// The object of the case as a `[params.input]` table of the older form.
fn object(params: &Table) -> Table {
    if let Some(src) = table_of(params, "source") {
        // A plane made editable: the source holds the plane's settings.
        let p = table_of(src, "parameters").expect("source parameters");
        let mut out = Table::new();
        out.insert("type".into(), "plane".into());
        for (k, v) in our_names(p) {
            let key = match k.as_str() {
                "width_segments" => "segments_w".to_string(),
                "height_segments" => "segments_h".to_string(),
                _ => k,
            };
            out.insert(key, v);
        }
        return out;
    }
    let input = table_of(params, "input").expect("input");
    let p = table_of(input, "parameters").expect("input parameters");
    let mut out = Table::new();
    out.insert("type".into(), "cube".into());
    for (k, v) in our_names(p) {
        match k.as_str() {
            "size" => {
                let s = vec3(&v);
                for (axis, x) in ["size_x", "size_y", "size_z"].into_iter().zip(s) {
                    out.insert(axis.into(), toml::Value::Float(x));
                }
            }
            "fillet_subdivision" => {
                out.insert("fillet_segments".into(), v);
            }
            _ => {
                out.insert(k, v);
            }
        }
    }
    out
}

/// One linked helper spline as a `[params.spline]` table of the older form.
fn helper(params: &Table, index: usize) -> Table {
    let helpers = table_of(params, "helpers").expect("helpers");
    let h = table_of(helpers, &index.to_string()).expect("helper");
    let mut out = Table::new();
    for key in ["position", "rotation_degrees"] {
        if let Some(v) = h.get(key) {
            let key = if key == "position" {
                "position"
            } else {
                "rotation_deg"
            };
            out.insert(key.into(), v.clone());
        }
    }
    if let Some(e) =
        table_of(params, "editable_helpers").and_then(|t| table_of(t, &index.to_string()))
    {
        out.insert("type".into(), "editable".into());
        for (k, v) in e {
            out.insert(k.clone(), v.clone());
        }
        return out;
    }
    out.extend(our_names(
        table_of(h, "parameters").expect("helper parameters"),
    ));
    // A generated spline without a `type` row is the Helix (its own rows).
    if !out.contains_key("type") {
        assert!(out.contains_key("start_radius"), "helper {index}: no type");
        out.insert("type".into(), "helix".into());
    } else {
        let lower = out["type"].as_str().unwrap().to_lowercase();
        out.insert("type".into(), lower.into());
    }
    out
}

/// The names the deformer's link values use for the helpers `1..=n`: an editable (hand-made)
/// helper is "First helper", "Second helper", ...; a generated one is its type name, `.1`,
/// `.2` for the repeats (`Helix`, `Helix.1`).
fn helper_names(params: &Table) -> Vec<String> {
    let helpers = table_of(params, "helpers").map_or(0, |t| t.len());
    let editable = table_of(params, "editable_helpers");
    let ordinal = ["First", "Second", "Third", "Fourth"];
    let (mut made, mut seen) = (0, std::collections::BTreeMap::<String, usize>::new());
    (1..=helpers)
        .map(|i| {
            if editable.is_some_and(|e| e.contains_key(&i.to_string())) {
                made += 1;
                return format!("{} helper", ordinal[made - 1]);
            }
            let t = helper(params, i)["type"].as_str().unwrap().to_string();
            let name = match t.as_str() {
                "helix" => "Helix".to_string(),
                "arc" => "Arc".to_string(),
                other => other.to_string(),
            };
            let n = seen.entry(name.clone()).or_insert(0);
            *n += 1;
            if *n == 1 {
                name
            } else {
                format!("{name}.{}", *n - 1)
            }
        })
        .collect()
}

/// Brings a menu-created case to the older form (see the module docs); an older case is
/// returned as it is.
pub fn normalize(params: &Table) -> Table {
    let Some(deformer) = table_of(params, "deformer") else {
        return params.clone();
    };
    let mut out = our_names(table_of(deformer, "parameters").expect("deformer parameters"));
    for key in ["frame", "fps"] {
        if let Some(v) = params.get(key) {
            out.insert(key.into(), v.clone());
        }
    }
    out.insert("input".into(), toml::Value::Table(object(params)));
    for key in ["lattice", "editable_helpers"] {
        if let Some(v) = params.get(key) {
            out.insert(key.into(), v.clone());
        }
    }
    // Spline links: the value is the helper's name; replace it by the helper's own table under
    // the key the older form used.
    if params.contains_key("helpers") {
        let names = helper_names(params);
        fn table_key(link: &str) -> &str {
            match link {
                "z_spline_left" => "left_helix",
                "z_spline_right" => "right_helix",
                "x_spline_top" => "x_top",
                "x_spline_bottom" => "x_bottom",
                // `spline`, `original_spline`, `modifying_spline`
                other => other,
            }
        }
        for link in [
            "spline",
            "original_spline",
            "modifying_spline",
            "z_spline_left",
            "z_spline_right",
            "x_spline_top",
            "x_spline_bottom",
        ] {
            let Some(name) = out.get(link).and_then(|v| v.as_str()).map(str::to_string) else {
                continue;
            };
            if name.is_empty() {
                continue;
            }
            let index =
                names.iter().position(|n| *n == name).unwrap_or_else(|| {
                    panic!("link {link} = {name:?}: no such helper in {names:?}")
                }) + 1;
            out.insert(
                table_key(link).into(),
                toml::Value::Table(helper(params, index)),
            );
        }
    }
    out
}
