//! Acceptance scenario S3 (docs/roadmap/README.md, docs/specs/UI.md §28): model a chair
//! through the editor window, the way a person would, and check what they would see.
//!
//! primitives -> make editable -> extrude, bevel, loop cut -> symmetry -> subdivision
//! surface -> save, reopen -> export OBJ.
//!
//! The window is driven with egui_kittest: menus and buttons are clicked by label, shortcuts
//! are key presses, file dialogs are injected. Things that need the GPU (picking in the
//! viewport) are replaced by setting the stored selection, as in the other UI tests; the
//! scenario draws nothing, so there is no image check and no tolerance involved.
//! Everything is written to a temp directory.

mod common;

use std::cell::RefCell;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use egui::{Key, Modifiers};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use forge_core::{EvalCtx, NodeId, Value};
use forge_ui::app::{Dialogs, Editor, FileKind, Unsaved};
use forge_ui::commands::execute;
use forge_ui::cutting::{click, hover};
use serde_json::json;

#[derive(Default)]
struct Script {
    open: VecDeque<Option<PathBuf>>,
    save: VecDeque<Option<PathBuf>>,
}

struct Scripted(Rc<RefCell<Script>>);

impl Dialogs for Scripted {
    fn pick_open(&mut self, _: FileKind) -> Option<PathBuf> {
        self.0.borrow_mut().open.pop_front().expect("open dialog")
    }

    fn pick_save(&mut self, _: FileKind, _: &str) -> Option<PathBuf> {
        self.0.borrow_mut().save.pop_front().expect("save dialog")
    }

    fn confirm_unsaved(&mut self, _: &str) -> Unsaved {
        Unsaved::DontSave
    }
}

type Win = Harness<'static, Editor>;

fn window() -> (Win, Rc<RefCell<Script>>) {
    let script = Rc::new(RefCell::new(Script::default()));
    let mut editor = Editor::new(common::state());
    editor.set_dialogs(Box::new(Scripted(script.clone())));
    let mut h = Harness::builder()
        .with_size([1280.0, 800.0])
        .build_ui_state(|ui, e: &mut Editor| e.show(ui, None), editor);
    h.run();
    (h, script)
}

fn temp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("forge-s3-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn click_label(h: &mut Win, label: &str) {
    // A popup menu is the last thing in the tree. Items that show a shortcut carry it in
    // their label, so fall back to a prefix match.
    let exact: Vec<_> = h.query_all_by_label(label).collect();
    if let Some(last) = exact.last() {
        last.click();
    } else {
        h.get_all_by_label_contains(label)
            .last()
            .unwrap_or_else(|| panic!("no widget labelled {label}"))
            .click();
    }
    h.run();
}

/// Opens `menu` and clicks the nested items in order.
fn menu(h: &mut Win, path: &[&str]) {
    for label in path {
        click_label(h, label);
    }
}

fn key(h: &mut Win, mods: Modifiers, k: Key) {
    h.key_press_modifiers(mods, k);
    h.run();
}

fn st(h: &Win) -> &forge_ui::state::EditorState {
    &h.state().state
}

fn steps(h: &Win) -> usize {
    st(h).doc.history().undo_labels().len()
}

fn mesh_of(h: &Win, id: NodeId) -> std::sync::Arc<forge_core::MeshData> {
    match st(h).doc.scene().param(id, "mesh") {
        Some(Value::Mesh(m)) => m.clone(),
        other => panic!("no stored mesh: {other:?}"),
    }
}

fn counts(h: &Win, id: NodeId) -> (usize, usize) {
    let m = mesh_of(h, id);
    (m.points.len(), m.polys.len())
}

fn y_range(h: &Win, id: NodeId) -> (f64, f64) {
    let m = mesh_of(h, id);
    m.points.iter().fold((f64::MAX, f64::MIN), |(lo, hi), p| {
        (lo.min(p[1]), hi.max(p[1]))
    })
}

fn select_polys(h: &mut Win, id: NodeId, polys: Vec<u32>) {
    h.state_mut()
        .state
        .doc
        .transact("select", |tx| {
            tx.set_param(id, "selection.polys", Value::IndexList(polys))
        })
        .unwrap();
    h.run();
}

/// The polygon whose points all lie at `y` (the bottom or top face of a box).
fn face_at_y(h: &Win, id: NodeId, y: f64) -> u32 {
    let m = mesh_of(h, id);
    m.polys
        .iter()
        .position(|p| {
            p.iter()
                .all(|k| (m.points[*k as usize][1] - y).abs() < 1e-9)
        })
        .expect("a face at that height") as u32
}

/// Sets a tool setting the way typing in the attribute manager does, then presses Apply.
fn apply_with(h: &mut Win, settings: &[(&str, Value)]) {
    for (name, v) in settings {
        h.state_mut()
            .state
            .tool
            .as_mut()
            .unwrap()
            .set(name, v.clone())
            .unwrap();
    }
    h.run();
    click_label(h, "Apply");
}

fn evaluated(h: &Win, id: NodeId) -> forge_geometry::PolyMesh {
    forge_generators::evaluate(st(h).doc.scene(), id, &EvalCtx::default())
        .unwrap()
        .expect("geometry")
}

fn bounds(m: &forge_geometry::PolyMesh) -> (glam::DVec3, glam::DVec3) {
    m.points.iter().fold(
        (glam::DVec3::splat(f64::MAX), glam::DVec3::splat(f64::MIN)),
        |(lo, hi), p| (lo.min(*p), hi.max(*p)),
    )
}

#[test]
fn model_a_chair_save_reopen_and_export() {
    let dir = temp_dir();
    let (mut h, script) = window();
    assert_eq!(h.state().window_title(), "Untitled - Forge3D");

    // 1. A primitive from the Create menu: half of the seat, centred on x = 100 so that the
    //    symmetry plane (x = 0) runs along its inner side.
    menu(&mut h, &["Create", "Generator", "Cube"]);
    let seat = st(&h).selection.active.expect("the new cube is selected");
    assert_eq!(st(&h).doc.scene().node(seat).unwrap().type_id, "forge.cube");
    for (param, v) in [("size_x", 200.0), ("size_y", 40.0), ("size_z", 200.0)] {
        execute(
            &mut h.state_mut().state,
            "param.set",
            json!({"node": seat.0, "param": param, "value": v}),
        )
        .unwrap();
    }
    execute(
        &mut h.state_mut().state,
        "transform.set",
        json!({"node": seat.0, "pos": [100.0, 0.0, 0.0]}),
    )
    .unwrap();
    h.run();
    assert!(h.state().is_dirty());

    // 2. Edit > Make Editable: the cube becomes a polygon object with 8 points and 6 polygons.
    menu(&mut h, &["Edit", "Make Editable"]);
    let seat = st(&h)
        .selection
        .active
        .expect("the polygon object is selected");
    assert_eq!(
        st(&h).doc.scene().node(seat).unwrap().type_id,
        "forge.polygon"
    );
    assert_eq!(counts(&h, seat), (8, 6));
    assert_eq!(y_range(&h, seat), (-20.0, 20.0));

    // 3. A leg: inset the bottom face, then pull the new cap down (Mesh menu tools).
    let bottom = face_at_y(&h, seat, -20.0);
    select_polys(&mut h, seat, vec![bottom]);
    let before = steps(&h);
    menu(&mut h, &["Mesh", "Extrude Inner"]);
    assert!(st(&h).tool.is_some());
    apply_with(&mut h, &[("offset", Value::Float(60.0))]);
    assert_eq!(counts(&h, seat), (12, 10));
    assert_eq!(steps(&h), before + 1, "Apply is one undo step");
    menu(&mut h, &["Mesh", "Extrude"]);
    apply_with(&mut h, &[("offset", Value::Float(180.0))]);
    assert_eq!(counts(&h, seat), (16, 14));
    assert_eq!(steps(&h), before + 2);
    assert_eq!(y_range(&h, seat), (-200.0, 20.0), "the leg reaches down");
    click_label(&mut h, "Done");
    assert!(st(&h).tool.is_none());

    // 4. Bevel the outer top edge of the seat (edge mode, Alt+3).
    key(&mut h, Modifiers::ALT, Key::Num3);
    assert_eq!(
        format!("{:?}", st(&h).mode),
        "Edges",
        "Alt+3 selects the edge mode"
    );
    let edge = {
        let pm = forge_generators::to_poly_mesh(&mesh_of(&h, seat));
        let top: Vec<u32> = (0..pm.points.len() as u32)
            .filter(|i| (pm.points[*i as usize].y - 20.0).abs() < 1e-9)
            .collect();
        let max_x = top
            .iter()
            .map(|i| pm.points[*i as usize].x)
            .fold(f64::MIN, f64::max);
        let outer: Vec<u32> = top
            .into_iter()
            .filter(|i| (pm.points[*i as usize].x - max_x).abs() < 1e-9)
            .collect();
        assert_eq!(outer.len(), 2);
        pm.edge_ids()[&forge_geometry::Edge::new(outer[0], outer[1])]
    };
    h.state_mut()
        .state
        .doc
        .transact("select", |tx| {
            tx.set_param(seat, "selection.polys", Value::IndexList(Vec::new()))?;
            tx.set_param(seat, "selection.edges", Value::IndexList(vec![edge]))
        })
        .unwrap();
    h.run();
    let before = counts(&h, seat);
    menu(&mut h, &["Mesh", "Bevel"]);
    apply_with(&mut h, &[("offset", Value::Float(8.0))]);
    let after = counts(&h, seat);
    assert!(
        after.0 > before.0 && after.1 > before.1,
        "{before:?} -> {after:?}"
    );
    click_label(&mut h, "Done");

    // 5. Loop cut: hover an edge, click once.
    let before = counts(&h, seat);
    let steps_before = steps(&h);
    menu(&mut h, &["Mesh", "Loop/Path Cut"]);
    {
        let state = &mut h.state_mut().state;
        let pm = match state.doc.scene().param(seat, "mesh") {
            Some(Value::Mesh(m)) => forge_generators::to_poly_mesh(m),
            _ => unreachable!(),
        };
        let (w, hh) = (800.0, 600.0);
        let mut hovered = false;
        for e in pm.edges() {
            let mid = (pm.points[e.0 as usize] + pm.points[e.1 as usize]) * 0.5;
            if let Some(p) = state.camera.project(mid, w, hh) {
                hover(state, p.x, p.y, w, hh);
                if state.tool.as_ref().unwrap().interaction.hover.is_some() {
                    hovered = true;
                    break;
                }
            }
        }
        assert!(hovered, "an edge of the leg can be hovered");
        click(state).unwrap();
    }
    h.run();
    let after = counts(&h, seat);
    assert!(after.0 > before.0 && after.1 > before.1);
    assert_eq!(steps(&h), steps_before + 1, "a loop cut is one undo step");
    click_label(&mut h, "Done");

    // 6. Undo and redo with the keys: the cut disappears and comes back.
    key(&mut h, Modifiers::COMMAND, Key::Z);
    assert_eq!(counts(&h, seat), before);
    key(&mut h, Modifiers::COMMAND | Modifiers::SHIFT, Key::Z);
    assert_eq!(counts(&h, seat), after);

    // 7. Symmetry and a subdivision surface from the Create menu; the object manager shows
    //    the chain once the objects are nested (seat < Symmetry < Subdivision).
    key(&mut h, Modifiers::ALT, Key::Num1); // back to object mode
    // Symmetry and Subdivision are at the bottom of the long Generator list, below the
    // bottom edge of an 800 px window (open question, see the brief's report), so they are
    // created with the same command the menu runs.
    let create = |h: &mut Win, ty: &str| {
        menu(h, &["Edit", "Deselect All"]);
        execute(
            &mut h.state_mut().state,
            "object.create",
            json!({"type": ty}),
        )
        .unwrap();
        h.run();
        st(h).selection.active.unwrap()
    };
    let sym = create(&mut h, "forge.symmetry");
    let sds = create(&mut h, "forge.subdivision");
    execute(
        &mut h.state_mut().state,
        "object.move",
        json!({"nodes": [seat.0], "parent": sym.0}),
    )
    .unwrap();
    execute(
        &mut h.state_mut().state,
        "object.move",
        json!({"nodes": [sym.0], "parent": sds.0}),
    )
    .unwrap();
    h.run();
    let scene = st(&h).doc.scene();
    assert_eq!(scene.roots(), [sds]);
    assert_eq!(scene.node(sym).unwrap().parent, Some(sds));
    assert_eq!(scene.node(seat).unwrap().parent, Some(sym));

    // The chair as evaluated: a closed surface, mirrored about x = 0, smoother than the cage.
    let symmetric_only = {
        let s = st(&h).doc.scene();
        forge_generators::evaluate(s, sym, &EvalCtx::default())
            .unwrap()
            .unwrap()
    };
    let (lo, hi) = bounds(&symmetric_only);
    assert!((lo.x + hi.x).abs() < 1e-6, "mirrored: {lo} {hi}");
    assert!(hi.x > 190.0, "the seat is two halves wide: {hi}");
    let (cage_pts, cage_polys) = counts(&h, seat);
    assert!(symmetric_only.polys.len() >= 2 * cage_polys - 4);
    let chair = evaluated(&h, sds);
    assert!(chair.polys.len() > 4 * symmetric_only.polys.len());
    let (clo, chi) = bounds(&chair);
    assert!((clo.x + chi.x).abs() < 1e-6, "still mirrored: {clo} {chi}");
    assert!(
        clo.y > lo.y && chi.y < hi.y,
        "the surface shrinks into the cage"
    );
    assert!(cage_pts > 8);

    // 8. Save with Ctrl+S (the dialog is injected), then reopen with Ctrl+O in a new window.
    let path = dir.join("chair.forge");
    script.borrow_mut().save.push_back(Some(path.clone()));
    key(&mut h, Modifiers::COMMAND, Key::S);
    assert!(path.exists());
    assert!(!h.state().is_dirty());
    assert_eq!(h.state().window_title(), "chair.forge - Forge3D");
    let saved = forge_core::file::save(st(&h).doc.scene());

    let (mut again, script2) = window();
    script2.borrow_mut().open.push_back(Some(path.clone()));
    key(&mut again, Modifiers::COMMAND, Key::O);
    assert_eq!(again.state().file_path(), Some(path.as_path()));
    assert!(st(&again).status.is_empty(), "{}", st(&again).status);
    assert_eq!(forge_core::file::save(st(&again).doc.scene()), saved);
    let reopened = evaluated(&again, sds);
    assert_eq!(reopened, chair, "the reopened chair evaluates identically");
    assert!(st(&again).doc.history().undo_labels().is_empty());

    // 9. File > Export OBJ... from the reopened window.
    let obj = dir.join("chair.obj");
    script2.borrow_mut().save.push_back(Some(obj.clone()));
    menu(&mut again, &["File", "Export OBJ\u{2026}"]);
    // the options window follows the file chooser; nothing is written before its button
    assert!(again.state().io_dialog().is_some());
    assert!(!obj.exists());
    click_label(&mut again, "Export");
    assert!(again.state().io_dialog().is_none());
    assert!(st(&again).status.is_empty(), "{}", st(&again).status);
    let text = std::fs::read_to_string(&obj).unwrap();
    assert_eq!(text.lines().filter(|l| l.starts_with("o ")).count(), 1);
    let faces = text.lines().filter(|l| l.starts_with("f ")).count();
    let verts = text.lines().filter(|l| l.starts_with("v ")).count();
    assert_eq!(faces, chair.polys.len(), "one face per polygon");
    assert!(verts > 0 && verts <= chair.points.len());

    // The file imports into an empty scene as a polygon object of the same size.
    let (mut third, _) = window();
    third.state_mut().import_obj_path(&obj);
    third.run();
    let imported = st(&third).selection.active.unwrap();
    assert_eq!(counts(&third, imported).1, faces);

    // Every step of the session is undoable: Ctrl+Z all the way back leaves an empty scene,
    // and the saved file on disk is untouched.
    let mut guard = 0;
    while !st(&h).doc.history().undo_labels().is_empty() {
        key(&mut h, Modifiers::COMMAND, Key::Z);
        guard += 1;
        assert!(guard < 200, "undo does not drain the history");
    }
    assert_eq!(st(&h).doc.scene().len(), 0);
    assert!(h.state().is_dirty(), "the title shows the unsaved change");
    assert_files_only(&dir, &["chair.forge", "chair.obj"]);
    std::fs::remove_dir_all(dir).ok();
}

fn assert_files_only(dir: &Path, names: &[&str]) {
    let mut found: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    found.sort();
    assert_eq!(found, names);
}
