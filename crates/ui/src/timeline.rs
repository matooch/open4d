//! Current frame and playback (docs/specs/ANIM.md §2.6, UI.md §7). The frame rate and the
//! project range live in the scene (`SceneSettings`); this holds only editor state.

use forge_core::SceneSettings;

/// The current frame and the playback clock anchor.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Timeline {
    pub current: i64,
    pub playing: bool,
    /// `(clock time, frame)` when playback (re)started; `None` until the next `update`.
    anchor: Option<(f64, i64)>,
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
    /// never delayed, and the range loops. True if the frame changed.
    pub fn update(&mut self, now: f64, settings: &SceneSettings) -> bool {
        if !self.playing {
            return false;
        }
        self.clamp_to(settings);
        let Some((t0, f0)) = self.anchor else {
            self.anchor = Some((now, self.current));
            return false;
        };
        let elapsed = ((now - t0) * f64::from(settings.fps) + 1e-9)
            .floor()
            .max(0.0) as i64;
        let len = settings.end - settings.start + 1;
        let next = settings.start + (f0 - settings.start + elapsed).rem_euclid(len);
        let changed = next != self.current;
        self.current = next;
        changed
    }
}
