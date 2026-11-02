//! Current frame and playback (docs/specs/ANIM.md §2.6, §2.6a, UI.md §7). The frame rate,
//! the project range and the time configuration live in the scene; this holds only editor
//! state and the pure stepping rules.

use forge_core::{PlayMode, SceneSettings, TimeConfig};

/// How playback steps: the range, the clock rate, the end-of-range mode and the timing
/// option (ANIM.md §2.6a).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Playback {
    pub lo: i64,
    pub hi: i64,
    pub fps: u32,
    pub mode: PlayMode,
    /// true: real-time with frame dropping; false: every frame.
    pub drop_frames: bool,
}

impl Playback {
    /// The playback of a scene: the preview range when set, else the project range.
    pub fn of(settings: &SceneSettings, time: &TimeConfig) -> Playback {
        let (lo, hi) = time.preview.unwrap_or((settings.start, settings.end));
        Playback {
            lo,
            hi,
            fps: settings.fps,
            mode: time.mode,
            drop_frames: time.drop_frames,
        }
    }

    /// The frame `steps` frames after `from` (inside `lo..=hi`) and whether playback has
    /// finished (`Once` reached `hi`).
    pub fn frame_after(&self, from: i64, steps: i64) -> (i64, bool) {
        let len = i128::from(self.hi) - i128::from(self.lo) + 1;
        let u = i128::from(from.clamp(self.lo, self.hi)) - i128::from(self.lo) + i128::from(steps);
        let at = |p: i128| (i128::from(self.lo) + p) as i64;
        match self.mode {
            PlayMode::Once => (at(u.min(len - 1)), steps > 0 && u >= len - 1),
            PlayMode::Loop => (at(u.rem_euclid(len)), false),
            PlayMode::PingPong => {
                if len == 1 {
                    return (self.lo, false);
                }
                let period = 2 * (len - 1);
                let p = u.rem_euclid(period);
                (at(if p < len { p } else { period - p }), false)
            }
        }
    }
}

/// Where the clock started and how many steps have been taken since.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Anchor {
    t0: f64,
    from: i64,
    steps: i64,
    /// Time of the last step (every-frame timing).
    last: f64,
}

/// The current frame and the playback clock anchor.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Timeline {
    pub current: i64,
    pub playing: bool,
    /// Set by the first `update` after playback (re)started.
    anchor: Option<Anchor>,
}

impl Timeline {
    /// Sets the frame, clamped to the project range. While playing, playback continues
    /// from the new frame.
    pub fn set_current(&mut self, frame: i64, settings: &SceneSettings) {
        self.current = frame.clamp(settings.start, settings.end);
        self.anchor = None;
    }

    /// Pulls the frame back into the range (after the range changed).
    pub fn clamp_to(&mut self, settings: &SceneSettings) {
        let clamped = self.current.clamp(settings.start, settings.end);
        if clamped != self.current {
            self.set_current(clamped, settings);
        }
    }

    pub fn play(&mut self) {
        self.playing = true;
        self.anchor = None;
    }

    /// Starts playback of `playback`; in `Once` mode at the last frame it restarts at the
    /// first (ANIM.md §2.6a).
    pub fn play_in(&mut self, playback: &Playback) {
        if playback.mode == PlayMode::Once && self.current >= playback.hi {
            self.current = playback.lo;
        }
        self.play();
    }

    pub fn stop(&mut self) {
        self.playing = false;
        self.anchor = None;
    }

    pub fn toggle(&mut self) {
        if self.playing {
            self.stop();
        } else {
            self.play();
        }
    }

    /// Forgets the clock anchor (fps or range changed): playback continues from `current`.
    pub fn reanchor(&mut self) {
        self.anchor = None;
    }

    /// Moves the frame to where the clock `now` (seconds) says it is. Frames are dropped,
    /// never delayed, and the project range loops. True if the frame changed.
    pub fn update(&mut self, now: f64, settings: &SceneSettings) -> bool {
        if !self.playing {
            return false;
        }
        self.clamp_to(settings);
        let playback = Playback::of(settings, &TimeConfig::default());
        self.update_with(now, &playback)
    }

    /// The general stepper (ANIM.md §2.6a): any range, mode and timing option. `now` is the
    /// injected clock in seconds. True if the frame changed.
    pub fn update_with(&mut self, now: f64, playback: &Playback) -> bool {
        if !self.playing {
            return false;
        }
        let pulled = self.current.clamp(playback.lo, playback.hi);
        if pulled != self.current {
            self.current = pulled;
            self.anchor = None;
        }
        let Some(mut a) = self.anchor else {
            self.anchor = Some(Anchor {
                t0: now,
                from: self.current,
                steps: 0,
                last: now,
            });
            return false;
        };
        let fps = f64::from(playback.fps.max(1));
        if playback.drop_frames {
            a.steps = ((now - a.t0) * fps + 1e-9).floor().max(0.0) as i64;
        } else {
            let due = a.last + 1.0 / fps;
            if now >= due - 1e-9 {
                a.steps += 1;
                a.last = if now - due < 1.0 / fps { due } else { now };
            }
        }
        self.anchor = Some(a);
        let (next, finished) = playback.frame_after(a.from, a.steps);
        let changed = next != self.current;
        self.current = next;
        if finished {
            self.stop();
        }
        changed
    }
}
