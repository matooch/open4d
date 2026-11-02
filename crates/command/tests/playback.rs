//! docs/specs/ANIM.md §2.6a (brief 094): playback modes, the preview range and the two
//! timing options, stepped with an injected clock (no wall clock, no window).

use forge_command::{Playback, Timeline};
use forge_core::{PlayMode, SceneSettings, TimeConfig};

fn pb(lo: i64, hi: i64, mode: PlayMode, drop_frames: bool) -> Playback {
    Playback {
        lo,
        hi,
        fps: 10,
        mode,
        drop_frames,
    }
}

/// Plays from `from`, sampling the frame at `t = 0, 0.1, ... ` (one tick per frame).
fn run(p: &Playback, from: i64, ticks: usize) -> Vec<i64> {
    let mut t = Timeline::default();
    t.current = from;
    t.play();
    (0..ticks)
        .map(|i| {
            t.update_with(i as f64 / 10.0, p);
            t.current
        })
        .collect()
}

#[test]
fn frame_after_for_each_mode() {
    let once = pb(2, 5, PlayMode::Once, true);
    assert_eq!(once.frame_after(2, 0), (2, false));
    assert_eq!(once.frame_after(2, 2), (4, false));
    assert_eq!(once.frame_after(2, 3), (5, true));
    assert_eq!(once.frame_after(2, 40), (5, true));
    assert_eq!(once.frame_after(5, 0), (5, false), "no step, not finished");
    let lp = pb(2, 5, PlayMode::Loop, true);
    assert_eq!(lp.frame_after(2, 4), (2, false));
    assert_eq!(lp.frame_after(4, 7), (3, false));
    let pp = pb(2, 5, PlayMode::PingPong, true);
    let seq: Vec<i64> = (0..10).map(|n| pp.frame_after(2, n).0).collect();
    assert_eq!(seq, [2, 3, 4, 5, 4, 3, 2, 3, 4, 5], "ends are shown once");
    assert_eq!(
        pp.frame_after(4, 3),
        (3, false),
        "starts forward from 4: 5 4 3"
    );
    let one = pb(7, 7, PlayMode::PingPong, true);
    assert_eq!(one.frame_after(7, 9), (7, false));
    let one = pb(7, 7, PlayMode::Once, true);
    assert_eq!(one.frame_after(7, 1), (7, true));
}

#[test]
fn modes_step_with_the_clock() {
    // the first update only anchors; then one frame per 0.1 s
    assert_eq!(
        run(&pb(0, 3, PlayMode::Loop, true), 0, 10),
        [0, 1, 2, 3, 0, 1, 2, 3, 0, 1]
    );
    assert_eq!(
        run(&pb(0, 3, PlayMode::PingPong, true), 0, 10),
        [0, 1, 2, 3, 2, 1, 0, 1, 2, 3]
    );
    let mut t = Timeline::default();
    t.play();
    let p = pb(0, 3, PlayMode::Once, true);
    let mut seen = Vec::new();
    for i in 0..8 {
        t.update_with(f64::from(i) / 10.0, &p);
        seen.push((t.current, t.playing));
    }
    assert_eq!(
        seen,
        [
            (0, true),
            (1, true),
            (2, true),
            (3, false),
            (3, false),
            (3, false),
            (3, false),
            (3, false)
        ],
        "Once stops on the last frame and stays"
    );
}

#[test]
fn real_time_drops_frames_and_every_frame_does_not() {
    // a stall: ticks at 0, 0.1, then 1.0 s later
    let times = [0.0, 0.1, 1.1, 1.2];
    let play = |drop: bool| {
        let p = pb(0, 99, PlayMode::Loop, drop);
        let mut t = Timeline::default();
        t.play();
        times
            .iter()
            .map(|now| {
                t.update_with(*now, &p);
                t.current
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(
        play(true),
        [0, 1, 11, 12],
        "the clock rules: frames are skipped"
    );
    assert_eq!(
        play(false),
        [0, 1, 2, 3],
        "one frame per due step, none skipped"
    );
}

#[test]
fn every_frame_never_runs_faster_than_fps_and_never_bursts() {
    let p = pb(0, 99, PlayMode::Loop, false);
    let mut t = Timeline::default();
    t.play();
    // 4 ticks per frame period: still one step per 0.1 s
    let mut frames = Vec::new();
    for i in 0..=20 {
        t.update_with(f64::from(i) * 0.025, &p);
        frames.push(t.current);
    }
    assert_eq!(*frames.last().unwrap(), 5, "0.5 s at 10 fps is 5 frames");
    assert!(frames.windows(2).all(|w| w[1] - w[0] <= 1), "{frames:?}");
}

#[test]
fn playback_stays_inside_the_preview_range() {
    let settings = SceneSettings {
        fps: 10,
        start: 0,
        end: 100,
    };
    let time = TimeConfig {
        preview: Some((20, 23)),
        mode: PlayMode::Loop,
        ..TimeConfig::default()
    };
    let p = Playback::of(&settings, &time);
    assert_eq!((p.lo, p.hi, p.fps), (20, 23, 10));
    // started outside the range, the frame is pulled to the nearest end first
    assert_eq!(run(&p, 5, 6), [20, 21, 22, 23, 20, 21]);
    assert_eq!(
        run(&p, 80, 3),
        [23, 20, 21].map(|f| f),
        "after the end: pulled to hi, then wraps"
    );
    let none = Playback::of(&settings, &TimeConfig::default());
    assert_eq!(
        (none.lo, none.hi),
        (0, 100),
        "no preview: the project range"
    );
}

#[test]
fn play_in_once_mode_at_the_end_restarts_at_the_beginning() {
    let p = pb(4, 8, PlayMode::Once, true);
    let mut t = Timeline::default();
    t.current = 8;
    t.play_in(&p);
    assert_eq!((t.current, t.playing), (4, true));
    let lp = pb(4, 8, PlayMode::Loop, true);
    let mut t = Timeline::default();
    t.current = 8;
    t.play_in(&lp);
    assert_eq!(t.current, 8, "other modes continue from where they are");
}

#[test]
fn a_manual_frame_change_re_anchors_the_clock() {
    let p = pb(0, 99, PlayMode::Loop, true);
    let s = SceneSettings {
        fps: 10,
        start: 0,
        end: 99,
    };
    let mut t = Timeline::default();
    t.play();
    t.update_with(0.0, &p);
    t.update_with(0.5, &p);
    assert_eq!(t.current, 5);
    t.set_current(50, &s);
    assert!(!t.update_with(0.6, &p), "anchors");
    t.update_with(0.8, &p);
    assert_eq!(t.current, 52);
}
