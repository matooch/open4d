//! The evaluation context handed to evaluation (docs/specs/MODIFIERS.md §8): the time,
//! the frame rate and the RNG seed that time-dependent and random nodes read.

/// What an evaluation may depend on besides the scene itself.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EvalCtx {
    /// Time in seconds.
    pub time: f64,
    /// Frames per second of the document.
    pub fps: f64,
    /// Seed of any random number generator the evaluation uses.
    pub seed: u64,
}

impl Default for EvalCtx {
    /// Frame 0 at 30 fps, seed 0.
    fn default() -> Self {
        EvalCtx {
            time: 0.0,
            fps: 30.0,
            seed: 0,
        }
    }
}

impl EvalCtx {
    /// The context of `frame` at `fps` (seed 0).
    pub fn at_frame(frame: i64, fps: f64) -> Self {
        EvalCtx {
            time: frame as f64 / fps,
            fps,
            seed: 0,
        }
    }

    /// The context at `time` seconds with the given `fps` (seed 0).
    pub fn at_time(time: f64, fps: f64) -> Self {
        EvalCtx { time, fps, seed: 0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_is_frame_zero_at_30_fps() {
        let c = EvalCtx::default();
        assert_eq!((c.time, c.fps, c.seed), (0.0, 30.0, 0));
    }

    #[test]
    fn at_frame_divides_by_fps() {
        assert_eq!(EvalCtx::at_frame(15, 30.0).time, 0.5);
        assert_eq!(EvalCtx::at_time(2.0, 24.0).fps, 24.0);
    }
}
