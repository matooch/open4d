//! Timeline placeholder (docs/specs/UI.md §7). Animation arrives in Phase 11.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timeline {
    pub start: i64,
    pub end: i64,
    pub current: i64,
    pub fps: u32,
    pub playing: bool,
}

impl Default for Timeline {
    fn default() -> Self {
        Timeline {
            start: 0,
            end: 90,
            current: 0,
            fps: 30,
            playing: false,
        }
    }
}

impl Timeline {
    /// Advances one frame while playing, wrapping at the end.
    pub fn tick(&mut self) {
        if self.playing {
            self.current = if self.current >= self.end {
                self.start
            } else {
                self.current + 1
            };
        }
    }

    pub fn set_current(&mut self, frame: i64) {
        self.current = frame.clamp(self.start, self.end);
    }

    pub fn set_range(&mut self, a: i64, b: i64) {
        (self.start, self.end) = (a.min(b), a.max(b));
        self.set_current(self.current);
    }
}
