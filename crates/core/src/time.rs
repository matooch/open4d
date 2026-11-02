//! Document time: markers, preview range, playback mode and display (docs/specs/ANIM.md
//! §2.9). Pure data and integer arithmetic; own design, parity-pending G5.

use serde::{Deserialize, Serialize};

use crate::SceneSettings;

/// What playback does at the end of the playback range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum PlayMode {
    Once,
    #[default]
    Loop,
    PingPong,
}

impl PlayMode {
    pub fn id(self) -> &'static str {
        match self {
            PlayMode::Once => "once",
            PlayMode::Loop => "loop",
            PlayMode::PingPong => "ping_pong",
        }
    }

    pub fn from_id(id: &str) -> Option<PlayMode> {
        Some(match id {
            "once" => PlayMode::Once,
            "loop" => PlayMode::Loop,
            "ping_pong" => PlayMode::PingPong,
            _ => return None,
        })
    }
}

/// How times are written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum TimeDisplay {
    #[default]
    Frames,
    Smpte,
    Seconds,
}

impl TimeDisplay {
    pub fn id(self) -> &'static str {
        match self {
            TimeDisplay::Frames => "frames",
            TimeDisplay::Smpte => "smpte",
            TimeDisplay::Seconds => "seconds",
        }
    }

    pub fn from_id(id: &str) -> Option<TimeDisplay> {
        Some(match id {
            "frames" => TimeDisplay::Frames,
            "smpte" => TimeDisplay::Smpte,
            "seconds" => TimeDisplay::Seconds,
            _ => return None,
        })
    }
}

/// A named, coloured frame in the timeline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Marker {
    pub id: u64,
    pub frame: i64,
    pub name: String,
    pub color: [u8; 3],
}

/// The default marker colour, `#e0a030`.
pub const DEFAULT_MARKER_COLOR: [u8; 3] = [0xe0, 0xa0, 0x30];

/// `#rrggbb` for a colour.
pub fn color_hex(c: [u8; 3]) -> String {
    format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2])
}

/// Parses `#rrggbb` (case insensitive).
pub fn parse_color(text: &str) -> Option<[u8; 3]> {
    let hex = text.strip_prefix('#')?;
    if hex.len() != 6 || !hex.is_ascii() {
        return None;
    }
    let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
    Some([byte(0)?, byte(2)?, byte(4)?])
}

/// Everything about time that is saved with the scene besides fps and the project range.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct TimeConfig {
    pub preview: Option<(i64, i64)>,
    pub mode: PlayMode,
    pub drop_frames: bool,
    pub display: TimeDisplay,
    pub markers: Vec<Marker>,
}

/// The state of a new scene: loop, real-time, frames, no preview, no markers.
impl Default for TimeConfig {
    fn default() -> Self {
        TimeConfig {
            preview: None,
            mode: PlayMode::Loop,
            drop_frames: true,
            display: TimeDisplay::Frames,
            markers: Vec::new(),
        }
    }
}

impl TimeConfig {
    /// Why this configuration is not valid inside `range`, if it is not.
    pub fn check(&self, range: &SceneSettings) -> Result<(), String> {
        if let Some((a, b)) = self.preview {
            if a > b {
                return Err(format!("preview start {a} is after preview end {b}"));
            }
            if a < range.start || b > range.end {
                return Err(format!(
                    "preview range {a}..{b} is outside the project range {}..{}",
                    range.start, range.end
                ));
            }
        }
        for (i, m) in self.markers.iter().enumerate() {
            if self.markers[..i].iter().any(|o| o.id == m.id) {
                return Err(format!("duplicate marker id {}", m.id));
            }
        }
        Ok(())
    }

    /// Restores the marker order `(frame, id)`.
    pub fn sort_markers(&mut self) {
        self.markers.sort_by_key(|m| (m.frame, m.id));
    }

    /// The id a new marker gets.
    pub fn next_marker_id(&self) -> u64 {
        self.markers.iter().map(|m| m.id).max().map_or(1, |m| m + 1)
    }

    pub fn marker(&self, id: u64) -> Option<&Marker> {
        self.markers.iter().find(|m| m.id == id)
    }

    /// The preview range clipped to `range` (`None` when nothing is left).
    pub fn clipped_preview(&self, range: &SceneSettings) -> Option<(i64, i64)> {
        let (a, b) = self.preview?;
        let (a, b) = (a.max(range.start), b.min(range.end));
        (a <= b).then_some((a, b))
    }

    /// The frame of the next (`forward`) or previous marker inside `range`, strictly after
    /// or before `current`.
    pub fn jump_target(&self, current: i64, forward: bool, range: &SceneSettings) -> Option<i64> {
        let inside = self
            .markers
            .iter()
            .map(|m| m.frame)
            .filter(|f| (range.start..=range.end).contains(f));
        if forward {
            inside.filter(|f| *f > current).min()
        } else {
            inside.filter(|f| *f < current).max()
        }
    }
}

/// The time of `frame` at `fps` as whole milliseconds, half away from zero: the evaluation
/// resolution of ANIM.md §1.4 (brief 087).
pub fn frame_to_ms(frame: i64, fps: u32) -> i64 {
    let (n, d) = (i128::from(frame) * 1000, i128::from(fps.max(1)));
    let q = (2 * n.abs() + d) / (2 * d);
    (if n < 0 { -q } else { q }) as i64
}

/// The frame a time in whole milliseconds lands on at `fps`, half rounding up.
pub fn ms_to_frame(ms: i64, fps: u32) -> i64 {
    (i128::from(ms) * i128::from(fps) + 500).div_euclid(1000) as i64
}

/// Moves a frame number from one frame rate to another so it keeps its time (ANIM.md §2.9).
pub fn convert_frame(frame: i64, old_fps: u32, new_fps: u32) -> i64 {
    ms_to_frame(frame_to_ms(frame, old_fps), new_fps)
}

/// `frame` written in `display` (non-drop-frame).
pub fn format_time(frame: i64, fps: u32, display: TimeDisplay) -> String {
    let fps = i64::from(fps.max(1));
    match display {
        TimeDisplay::Frames => frame.to_string(),
        TimeDisplay::Seconds => {
            let ms = frame_to_ms(frame, fps as u32);
            let sign = if ms < 0 { "-" } else { "" };
            format!("{sign}{}.{:03} s", ms.abs() / 1000, ms.abs() % 1000)
        }
        TimeDisplay::Smpte => {
            let sign = if frame < 0 { "-" } else { "" };
            let f = frame.unsigned_abs() as i64;
            let width = ((fps - 1).to_string().len()).max(2);
            let (secs, ff) = (f / fps, f % fps);
            format!(
                "{sign}{:02}:{:02}:{:02}:{ff:0width$}",
                secs / 3600,
                secs / 60 % 60,
                secs % 60
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colour_text_round_trips() {
        assert_eq!(color_hex(DEFAULT_MARKER_COLOR), "#e0a030");
        assert_eq!(parse_color("#E0A030"), Some(DEFAULT_MARKER_COLOR));
        assert_eq!(parse_color("e0a030"), None);
        assert_eq!(parse_color("#e0a03"), None);
        assert_eq!(parse_color("#gggggg"), None);
        assert_eq!(parse_color("#éé"), None);
    }
}
