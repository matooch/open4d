//! docs/specs/ANIM.md §2.9, CORE.md §6.6 (brief 094): the pure time rules, the time
//! configuration as scene data, and the file round trip with the 6 -> 7 migration.

mod common;

use forge_core::file;
use forge_core::time::{convert_frame, format_time, frame_to_ms, ms_to_frame};
use forge_core::{
    Document, Interp, Key, Marker, PlayMode, SceneSettings, TimeConfig, TimeDisplay, Track,
};

fn marker(id: u64, frame: i64, name: &str) -> Marker {
    Marker {
        id,
        frame,
        name: name.into(),
        color: [1, 2, 3],
    }
}

#[test]
fn frames_become_whole_milliseconds_half_away_from_zero() {
    assert_eq!(frame_to_ms(1, 30), 33);
    assert_eq!(frame_to_ms(10, 30), 333);
    assert_eq!(frame_to_ms(2, 30), 67);
    assert_eq!(frame_to_ms(-1, 30), -33);
    assert_eq!(frame_to_ms(-2, 30), -67);
    assert_eq!(frame_to_ms(1, 24), 42, "41.67 rounds up");
    assert_eq!(frame_to_ms(1, 8), 125);
    assert_eq!(frame_to_ms(1, 16), 63, "62.5 rounds away from zero");
    assert_eq!(frame_to_ms(-1, 16), -63);
    assert_eq!(ms_to_frame(500, 24), 12);
    assert_eq!(ms_to_frame(625, 20), 13, "12.5 rounds up");
    assert_eq!(ms_to_frame(-625, 20), -12, "-12.5 rounds up too");
}

#[test]
fn converting_fps_keeps_the_time_of_a_frame() {
    // whole seconds are exact both ways
    for fps in [12u32, 24, 25, 30, 60, 120] {
        for sec in [0i64, 1, 2, 5, 60] {
            let f = sec * i64::from(fps);
            assert_eq!(convert_frame(f, fps, 30), sec * 30, "{fps}");
            assert_eq!(convert_frame(sec * 30, 30, fps), f, "{fps}");
        }
    }
    // frames between whole seconds follow the millisecond rule (brief 087)
    assert_eq!(convert_frame(10, 30, 24), 8, "0.333 s");
    assert_eq!(convert_frame(1, 30, 24), 1, "0.033 s is 0.79 frames");
    assert_eq!(convert_frame(90, 30, 24), 72);
    assert_eq!(convert_frame(-30, 30, 24), -24);
    assert_eq!(convert_frame(3, 30, 24), 2);
    assert_eq!(convert_frame(2, 24, 30), 2, "0.083 s is 2.49 frames");
    // converting there and back does not always restore the frame
    assert_eq!(convert_frame(convert_frame(7, 30, 24), 24, 30), 8);
    // identity and monotone
    let mut last = i64::MIN;
    for f in -200..400 {
        assert_eq!(convert_frame(f, 30, 30), f);
        let g = convert_frame(f, 30, 24);
        assert!(g >= last, "monotone at {f}");
        last = g;
    }
}

#[test]
fn time_display_formats() {
    use TimeDisplay::{Frames, Seconds, Smpte};
    assert_eq!(format_time(42, 30, Frames), "42");
    assert_eq!(format_time(-3, 30, Frames), "-3");
    assert_eq!(format_time(100, 30, Smpte), "00:00:03:10");
    assert_eq!(format_time(3600, 24, Smpte), "00:02:30:00");
    assert_eq!(format_time(0, 30, Smpte), "00:00:00:00");
    assert_eq!(format_time(-5, 30, Smpte), "-00:00:00:05");
    assert_eq!(format_time(30 * 3600 + 7, 30, Smpte), "01:00:00:07");
    assert_eq!(
        format_time(239, 240, Smpte),
        "00:00:00:239",
        "frame field widens"
    );
    assert_eq!(format_time(100, 30, Seconds), "3.333 s");
    assert_eq!(format_time(42, 30, Seconds), "1.400 s");
    assert_eq!(format_time(-1, 30, Seconds), "-0.033 s");
    assert_eq!(format_time(0, 24, Seconds), "0.000 s");
}

#[test]
fn keys_keep_their_seconds_when_only_the_frame_rate_changes() {
    // Keys are stored in seconds; the settings change touches no node.
    let mut doc = Document::new(common::registry());
    let id = doc
        .transact("make", |tx| {
            let id = tx.create("test.box", None, None)?;
            tx.set_track(
                id,
                "size",
                Some(Track {
                    keys: vec![
                        Key {
                            time: 1.0 / 3.0,
                            value: 1.0,
                            interp: Interp::Spline,
                        },
                        Key {
                            time: 2.0,
                            value: 5.0,
                            interp: Interp::Linear,
                        },
                    ],
                }),
            )?;
            Ok(id)
        })
        .unwrap();
    let before = doc.scene().node(id).unwrap().tracks.clone();
    doc.transact("fps", |tx| {
        tx.set_settings_and_time(
            SceneSettings {
                fps: 24,
                start: 0,
                end: 72,
            },
            TimeConfig::default(),
        )
    })
    .unwrap();
    assert_eq!(doc.scene().node(id).unwrap().tracks, before);
    assert_eq!(doc.scene().settings().fps, 24);
}

#[test]
fn time_config_is_validated_sorted_and_undoable() {
    let mut doc = Document::new(common::registry());
    let rev = doc.scene().revision();
    doc.transact("t", |tx| {
        tx.set_time(TimeConfig {
            markers: vec![marker(2, 30, "b"), marker(1, 30, "a"), marker(3, 5, "c")],
            preview: Some((10, 20)),
            ..TimeConfig::default()
        })
    })
    .unwrap();
    assert!(doc.scene().revision() > rev);
    let ids: Vec<u64> = doc.scene().time().markers.iter().map(|m| m.id).collect();
    assert_eq!(ids, [3, 1, 2], "sorted by (frame, id)");
    let bad = |time: TimeConfig| doc.clone().transact("t", |tx| tx.set_time(time)).is_err();
    assert!(bad(TimeConfig {
        preview: Some((20, 10)),
        ..TimeConfig::default()
    }));
    assert!(bad(TimeConfig {
        preview: Some((-1, 10)),
        ..TimeConfig::default()
    }));
    assert!(bad(TimeConfig {
        preview: Some((10, 91)),
        ..TimeConfig::default()
    }));
    assert!(bad(TimeConfig {
        markers: vec![marker(1, 1, "a"), marker(1, 2, "b")],
        ..TimeConfig::default()
    }));
    doc.undo();
    assert_eq!(doc.scene().time(), &TimeConfig::default());
}

#[test]
fn shrinking_the_project_range_clips_the_preview_in_the_same_step() {
    let mut doc = Document::new(common::registry());
    doc.transact("t", |tx| {
        tx.set_time(TimeConfig {
            preview: Some((10, 80)),
            markers: vec![marker(1, 85, "late")],
            ..TimeConfig::default()
        })
    })
    .unwrap();
    let set = |doc: &mut Document, end: i64, start: i64| {
        doc.transact("s", |tx| {
            tx.set_settings(SceneSettings {
                fps: 30,
                start,
                end,
            })
        })
        .unwrap()
    };
    set(&mut doc, 50, 0);
    assert_eq!(doc.scene().time().preview, Some((10, 50)));
    assert_eq!(doc.scene().time().markers.len(), 1, "markers are kept");
    set(&mut doc, 90, 60);
    assert_eq!(doc.scene().time().preview, None, "nothing left of 10..50");
    doc.undo();
    assert_eq!(doc.scene().time().preview, Some((10, 50)), "one undo step");
    assert_eq!(doc.scene().settings().start, 0);
}

#[test]
fn jump_target_skips_markers_outside_the_range_and_duplicates() {
    let range = SceneSettings {
        fps: 30,
        start: 0,
        end: 90,
    };
    let t = TimeConfig {
        markers: vec![
            marker(1, -5, "out"),
            marker(2, 10, "a"),
            marker(3, 10, "same frame"),
            marker(4, 40, "b"),
            marker(5, 120, "out"),
        ],
        ..TimeConfig::default()
    };
    assert_eq!(t.jump_target(0, true, &range), Some(10));
    assert_eq!(t.jump_target(10, true, &range), Some(40));
    assert_eq!(t.jump_target(40, true, &range), None);
    assert_eq!(t.jump_target(40, false, &range), Some(10));
    assert_eq!(t.jump_target(10, false, &range), None);
    assert_eq!(t.jump_target(25, false, &range), Some(10));
}

fn doc_with_time() -> Document {
    let mut doc = Document::new(common::registry());
    doc.transact("t", |tx| {
        tx.set_settings(SceneSettings {
            fps: 24,
            start: 0,
            end: 48,
        })?;
        tx.set_time(TimeConfig {
            preview: Some((4, 40)),
            mode: PlayMode::PingPong,
            drop_frames: false,
            display: TimeDisplay::Smpte,
            markers: vec![
                Marker {
                    id: 7,
                    frame: 12,
                    name: "Hit \"hard\"\nnext".into(),
                    color: [224, 160, 48],
                },
                marker(9, 30, ""),
            ],
        })
    })
    .unwrap();
    doc
}

#[test]
fn markers_and_the_time_config_survive_a_save_and_load() {
    let doc = doc_with_time();
    let text = file::save(doc.scene());
    assert!(
        text.contains(&format!("version: {}", forge_core::file::VERSION)),
        "{text}"
    );
    assert!(text.contains("preview: Some((4, 40))"), "{text}");
    let (loaded, report) = file::load(common::registry(), &text).unwrap();
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
    assert_eq!(loaded.time(), doc.scene().time());
    assert_eq!(loaded.settings(), doc.scene().settings());
    assert_eq!(file::save(&loaded), text, "deterministic");
}

#[test]
fn files_without_a_time_block_load_with_the_defaults_and_a_new_scene_has_them() {
    assert_eq!(
        Document::new(common::registry()).scene().time(),
        &TimeConfig::default()
    );
    let t = TimeConfig::default();
    assert_eq!(
        (t.preview, t.mode, t.drop_frames, t.display, t.markers.len()),
        (None, PlayMode::Loop, true, TimeDisplay::Frames, 0)
    );
    // version 6 file, as the previous build wrote it
    let v6 = r#"(format: "forge3d-scene", version: 6, settings: (fps: 25, start: 0, end: 50), roots: [])"#;
    let (scene, report) = file::load(common::registry(), v6).unwrap();
    assert!(report.warnings.is_empty());
    assert_eq!(scene.time(), &TimeConfig::default());
    assert_eq!(scene.settings().fps, 25);
    let (again, _) = file::load(common::registry(), &file::save(&scene)).unwrap();
    assert_eq!(again.time(), &TimeConfig::default());
}

#[test]
fn an_invalid_time_block_loads_as_the_defaults_with_a_warning() {
    for bad in [
        "(preview: Some((30, 20)))",
        "(preview: Some((0, 500)))",
        "(markers: [(id: 1, frame: 1, name: \"a\", color: (0, 0, 0)), (id: 1, frame: 2, name: \"b\", color: (0, 0, 0))])",
    ] {
        let text = format!(
            r#"(format: "forge3d-scene", version: 8, settings: (fps: 30, start: 0, end: 90), time: {bad}, roots: [])"#
        );
        let (scene, report) = file::load(common::registry(), &text).unwrap();
        assert_eq!(scene.time(), &TimeConfig::default(), "{bad}");
        assert!(
            report.warnings.iter().any(|w| w.contains("scene time")),
            "{bad}: {:?}",
            report.warnings
        );
    }
    // fields missing inside a time block take their defaults (drop_frames stays true)
    let text = r#"(format: "forge3d-scene", version: 8, settings: (fps: 30, start: 0, end: 90), time: (mode: Once), roots: [])"#;
    let (scene, report) = file::load(common::registry(), text).unwrap();
    assert!(report.warnings.is_empty());
    assert_eq!(scene.time().mode, PlayMode::Once);
    assert!(scene.time().drop_frames);
}
