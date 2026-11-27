//! Key map: chords to registry commands, defaults, user overrides, conflicts
//! (docs/specs/UI.md §14). No drawing here; the editor window is `keymap_editor`.

use std::fmt;
use std::path::{Path, PathBuf};

use egui::{Event, Key, Modifiers};
use forge_command::CommandRegistry;
use serde::Deserialize;
use serde_json::{Map, Value, json};

/// The shipped defaults (observed, G2 item 1; UI.md §14.2).
pub const DEFAULT_TOML: &str = include_str!("../keymap/default.toml");

/// Why a chord text or a key map file could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyMapError(pub String);

impl fmt::Display for KeyMapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for KeyMapError {}

fn err<T>(msg: impl Into<String>) -> Result<T, KeyMapError> {
    Err(KeyMapError(msg.into()))
}

/// One key press with modifiers. `command` is egui's `Modifiers::COMMAND`: Cmd on macOS,
/// Ctrl elsewhere.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Chord {
    pub command: bool,
    pub alt: bool,
    pub shift: bool,
    pub key: Key,
}

impl Chord {
    pub const fn new(key: Key) -> Chord {
        Chord {
            command: false,
            alt: false,
            shift: false,
            key,
        }
    }

    /// Parses `Mod+Shift+Z`. `Ctrl`, `Cmd` and `Command` are other spellings of `Mod`;
    /// words are case-insensitive.
    pub fn parse(text: &str) -> Result<Chord, KeyMapError> {
        let parts: Vec<&str> = text.split('+').map(str::trim).collect();
        let Some((key_text, mods)) = parts.split_last() else {
            return err("empty chord");
        };
        if key_text.is_empty() {
            return err(format!("chord '{text}' has no key"));
        }
        let mut chord = Chord::new(parse_key(key_text)?);
        for m in mods {
            let flag = match m.to_ascii_lowercase().as_str() {
                "mod" | "ctrl" | "cmd" | "command" => &mut chord.command,
                "alt" | "option" => &mut chord.alt,
                "shift" => &mut chord.shift,
                other => return err(format!("unknown modifier '{other}' in '{text}'")),
            };
            if *flag {
                return err(format!("modifier repeated in '{text}'"));
            }
            *flag = true;
        }
        Ok(chord)
    }

    /// The chord of a key press, if `key` is not just a modifier.
    pub fn from_press(modifiers: &Modifiers, key: Key) -> Chord {
        Chord {
            command: modifiers.command,
            alt: modifiers.alt,
            shift: modifiers.shift,
            key,
        }
    }

    /// True for exactly this key and exactly these modifiers.
    pub fn matches(&self, modifiers: &Modifiers, key: Key) -> bool {
        self.key == key
            && self.command == modifiers.command
            && self.alt == modifiers.alt
            && self.shift == modifiers.shift
    }

    /// The egui modifiers for this chord (`COMMAND` is Cmd on macOS, Ctrl elsewhere).
    pub fn modifiers(&self) -> Modifiers {
        let mut m = Modifiers::NONE;
        if self.command {
            m |= Modifiers::COMMAND;
        }
        if self.alt {
            m |= Modifiers::ALT;
        }
        if self.shift {
            m |= Modifiers::SHIFT;
        }
        m
    }

    /// The chord as the platform spells it (`Ctrl+Z`, `⌘Z`), for menus and the editor.
    pub fn display(&self, ctx: &egui::Context) -> String {
        ctx.format_shortcut(&egui::KeyboardShortcut::new(self.modifiers(), self.key))
    }
}

/// How long a prefix key waits for its second key, in seconds (UI.md §14.7).
pub const SEQUENCE_TIMEOUT: f64 = 1.5;

/// Parses the text of a binding's keys: one chord (`Mod+Z`) or two chords separated by
/// white space (`M T`). Spaces next to a `+` belong to the chord (`Mod + A`). At most two
/// keys (UI.md §14.7).
pub fn parse_keys(text: &str) -> Result<(Chord, Option<Chord>), KeyMapError> {
    let mut t = text.trim().to_string();
    while t.contains(" +") || t.contains("+ ") {
        t = t.replace(" +", "+").replace("+ ", "+");
    }
    let parts: Vec<&str> = t.split_whitespace().collect();
    match parts.as_slice() {
        [] => err("empty chord"),
        [one] => Ok((Chord::parse(one)?, None)),
        [a, b] => Ok((Chord::parse(a)?, Some(Chord::parse(b)?))),
        _ => err(format!("'{text}': a key sequence has at most two keys")),
    }
}

fn parse_key(text: &str) -> Result<Key, KeyMapError> {
    if let Some(k) = Key::from_name(text) {
        return Ok(k);
    }
    Key::ALL
        .iter()
        .copied()
        .find(|k| k.name().eq_ignore_ascii_case(text))
        .map_or_else(|| err(format!("unknown key '{text}'")), Ok)
}

/// Canonical text: `Mod+Alt+Shift+<key name>`.
impl fmt::Display for Chord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.command {
            f.write_str("Mod+")?;
        }
        if self.alt {
            f.write_str("Alt+")?;
        }
        if self.shift {
            f.write_str("Shift+")?;
        }
        f.write_str(self.key.name())
    }
}

/// Where a binding is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    #[default]
    Global,
    /// Only while the pointer was over the viewport.
    Viewport,
}

impl Scope {
    pub fn name(self) -> &'static str {
        match self {
            Scope::Global => "global",
            Scope::Viewport => "viewport",
        }
    }

    fn parse(text: &str) -> Result<Scope, KeyMapError> {
        match text {
            "global" => Ok(Scope::Global),
            "viewport" => Ok(Scope::Viewport),
            other => err(format!("unknown scope '{other}'")),
        }
    }

    /// Two scopes overlap when some situation activates both.
    pub fn overlaps(self, other: Scope) -> bool {
        self == Scope::Global || other == Scope::Global || self == other
    }
}

/// One chord (or a two-key sequence) running one registry command with JSON arguments.
/// `chord` is the only key of a single binding; with `then` it is the prefix key and `then`
/// the key that runs the command (UI.md §14.7).
#[derive(Debug, Clone, PartialEq)]
pub struct Binding {
    pub chord: Chord,
    pub then: Option<Chord>,
    pub command: String,
    /// A JSON object (empty for none).
    pub args: Value,
    pub scope: Scope,
}

impl Binding {
    pub fn new(chord: Chord, command: impl Into<String>, args: Value) -> Binding {
        Binding {
            chord,
            then: None,
            command: command.into(),
            args: normalize_args(args),
            scope: Scope::Global,
        }
    }

    /// A two-key sequence binding.
    pub fn sequence(chord: Chord, then: Chord, command: impl Into<String>, args: Value) -> Binding {
        Binding {
            then: Some(then),
            ..Binding::new(chord, command, args)
        }
    }

    /// The canonical text of the keys: `Mod+Z`, or `M T` for a sequence.
    pub fn keys_text(&self) -> String {
        keys_text(self.chord, self.then)
    }

    /// The keys as the platform spells them, joined by a space.
    pub fn display(&self, ctx: &egui::Context) -> String {
        match self.then {
            Some(t) => format!("{} {}", self.chord.display(ctx), t.display(ctx)),
            None => self.chord.display(ctx),
        }
    }

    /// True when the bindings are on the same keys.
    pub fn same_keys(&self, other: &Binding) -> bool {
        self.chord == other.chord && self.then == other.then
    }

    /// True when this binding and `chord` `then` in `scope` would both claim a key press
    /// (UI.md §14.4, §14.7): the same keys, or a single key and a sequence it starts.
    pub fn collides(&self, chord: Chord, then: Option<Chord>, scope: Scope) -> bool {
        self.chord == chord
            && self.scope.overlaps(scope)
            && (self.then == then || self.then.is_none() || then.is_none())
    }

    /// A short text for the arguments (`mode=wire`), empty for none.
    pub fn args_summary(&self) -> String {
        match self.args.as_object() {
            Some(o) if !o.is_empty() => o
                .iter()
                .map(|(k, v)| match v.as_str() {
                    Some(s) => format!("{k}={s}"),
                    None => format!("{k}={v}"),
                })
                .collect::<Vec<_>>()
                .join(", "),
            _ => String::new(),
        }
    }

    fn to_json(&self) -> Value {
        let mut o = Map::new();
        o.insert("command".into(), json!(self.command));
        o.insert("args".into(), self.args.clone());
        if self.scope != Scope::Global {
            o.insert("scope".into(), json!(self.scope.name()));
        }
        Value::Object(o)
    }

    fn from_json(chord: Chord, then: Option<Chord>, v: &Value) -> Result<Binding, KeyMapError> {
        let command = v["command"]
            .as_str()
            .filter(|c| !c.is_empty())
            .ok_or_else(|| KeyMapError("missing \"command\"".into()))?;
        let args = match &v["args"] {
            Value::Null => json!({}),
            a if a.is_object() => a.clone(),
            _ => return err("\"args\" must be an object"),
        };
        let scope = match v.get("scope") {
            None | Some(Value::Null) => Scope::Global,
            Some(s) => Scope::parse(s.as_str().unwrap_or_default())?,
        };
        Ok(Binding {
            chord,
            then,
            command: command.to_string(),
            args,
            scope,
        })
    }
}

/// Canonical text of a key sequence: the chords joined by a space.
pub fn keys_text(chord: Chord, then: Option<Chord>) -> String {
    match then {
        Some(t) => format!("{chord} {t}"),
        None => chord.to_string(),
    }
}

fn normalize_args(args: Value) -> Value {
    if args.is_null() { json!({}) } else { args }
}

/// Two bindings on one chord whose scopes overlap.
#[derive(Debug, Clone, PartialEq)]
pub struct Conflict {
    pub chord: Chord,
    /// The second key of the conflicting sequences; `None` for single keys and for a
    /// single key against the sequences it starts.
    pub then: Option<Chord>,
    /// Indices into [`KeyMap::bindings`].
    pub bindings: Vec<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileEntry {
    chord: String,
    command: String,
    #[serde(default)]
    args: Option<toml::Table>,
    #[serde(default)]
    scope: Scope,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileData {
    version: u32,
    #[serde(default)]
    bind: Vec<FileEntry>,
}

/// The bindings, in order. The first matching binding runs.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct KeyMap {
    bindings: Vec<Binding>,
}

impl KeyMap {
    pub fn new(bindings: Vec<Binding>) -> KeyMap {
        KeyMap { bindings }
    }

    /// The shipped default map.
    pub fn defaults() -> KeyMap {
        KeyMap::from_toml(DEFAULT_TOML).expect("the shipped default key map is valid")
    }

    /// Reads a key map file (UI.md §14.2).
    pub fn from_toml(text: &str) -> Result<KeyMap, KeyMapError> {
        let data: FileData = toml::from_str(text).map_err(|e| KeyMapError(e.to_string()))?;
        if data.version != 1 {
            return err(format!("unsupported key map version {}", data.version));
        }
        let mut bindings = Vec::new();
        for e in data.bind {
            let args = match e.args {
                None => json!({}),
                Some(t) => serde_json::to_value(t).map_err(|e| KeyMapError(e.to_string()))?,
            };
            let (chord, then) = parse_keys(&e.chord)?;
            bindings.push(Binding {
                chord,
                then,
                command: e.command,
                args,
                scope: e.scope,
            });
        }
        Ok(KeyMap { bindings })
    }

    pub fn bindings(&self) -> &[Binding] {
        &self.bindings
    }

    /// The first single-key binding for a key press, if any, in `viewport_hot` or not.
    /// Sequences are not looked up here; see [`KeyMap::step`].
    pub fn lookup(&self, modifiers: &Modifiers, key: Key, viewport_hot: bool) -> Option<&Binding> {
        self.bindings.iter().find(|b| {
            b.then.is_none()
                && b.chord.matches(modifiers, key)
                && (b.scope == Scope::Global || viewport_hot)
        })
    }

    /// True when some sequence binding active in `viewport_hot` starts with `chord`.
    pub fn is_prefix(&self, chord: Chord, viewport_hot: bool) -> bool {
        self.bindings.iter().any(|b| {
            b.then.is_some() && b.chord == chord && (b.scope == Scope::Global || viewport_hot)
        })
    }

    /// One key press through the sequence rules (UI.md §14.7). `armed` is the prefix
    /// waiting for its second key; `now` is the time in seconds.
    pub fn step(
        &self,
        press: Chord,
        viewport_hot: bool,
        armed: &mut Option<Armed>,
        now: f64,
    ) -> Step<'_> {
        if armed.is_some_and(|a| now - a.at > SEQUENCE_TIMEOUT) {
            *armed = None;
        }
        if let Some(a) = armed.take() {
            if press == Chord::new(Key::Escape) {
                return Step::Cancelled;
            }
            if let Some(b) = self.bindings.iter().find(|b| {
                b.chord == a.chord
                    && b.then == Some(press)
                    && (b.scope == Scope::Global || viewport_hot)
            }) {
                return Step::Run(b);
            }
            // not a continuation: the key is an ordinary press
        }
        if self.is_prefix(press, viewport_hot) {
            *armed = Some(Armed {
                chord: press,
                at: now,
            });
            return Step::Armed;
        }
        let single = self.bindings.iter().find(|b| {
            b.then.is_none() && b.chord == press && (b.scope == Scope::Global || viewport_hot)
        });
        single.map_or(Step::Pass, Step::Run)
    }

    /// Consumes the key presses of this frame that have a binding (or arm or continue a
    /// sequence) and returns what they run, in press order. One command per key press.
    pub fn take_presses(
        &self,
        input: &mut egui::InputState,
        viewport_hot: bool,
        armed: &mut Option<Armed>,
    ) -> Vec<(String, Value)> {
        let mut out = Vec::new();
        let now = input.time;
        let events = std::mem::take(&mut input.events);
        for event in events {
            if let Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } = &event
            {
                match self.step(Chord::from_press(modifiers, *key), viewport_hot, armed, now) {
                    Step::Run(b) => {
                        out.push((b.command.clone(), b.args.clone()));
                        continue;
                    }
                    Step::Armed | Step::Cancelled => continue,
                    Step::Pass => {}
                }
            }
            input.events.push(event);
        }
        out
    }

    /// Bindings of `command`, in map order.
    pub fn for_command<'a>(&'a self, command: &'a str) -> impl Iterator<Item = &'a Binding> {
        self.bindings.iter().filter(move |b| b.command == command)
    }

    /// The first binding running `command` with exactly `args`.
    pub fn binding_of<'a>(&'a self, command: &'a str, args: &Value) -> Option<&'a Binding> {
        let args = normalize_args(args.clone());
        self.for_command(command).find(|b| b.args == args)
    }

    /// The first chord running `command` with exactly `args` (the prefix of a sequence).
    pub fn chord_of(&self, command: &str, args: &Value) -> Option<Chord> {
        let args = normalize_args(args.clone());
        self.for_command(command)
            .find(|b| b.args == args)
            .map(|b| b.chord)
    }

    /// Groups of bindings that claim the same key press with overlapping scopes (UI.md
    /// §14.4, §14.7): the same keys, or a single key and a sequence it starts.
    pub fn conflicts(&self) -> Vec<Conflict> {
        let mut out: Vec<Conflict> = Vec::new();
        for (i, a) in self.bindings.iter().enumerate() {
            for (j, b) in self.bindings.iter().enumerate().skip(i + 1) {
                if !a.collides(b.chord, b.then, b.scope) {
                    continue;
                }
                let then = if a.then == b.then { a.then } else { None };
                match out
                    .iter_mut()
                    .find(|c| c.chord == a.chord && c.then == then)
                {
                    Some(c) => {
                        for k in [i, j] {
                            if !c.bindings.contains(&k) {
                                c.bindings.push(k);
                            }
                        }
                    }
                    None => out.push(Conflict {
                        chord: a.chord,
                        then,
                        bindings: vec![i, j],
                    }),
                }
            }
        }
        out
    }

    /// The bindings that the single key `chord` in `scope` would collide with.
    pub fn conflicts_with(&self, chord: Chord, scope: Scope) -> Vec<&Binding> {
        self.conflicts_with_keys(chord, None, scope)
    }

    /// The bindings that `chord` (`then`) in `scope` would collide with.
    pub fn conflicts_with_keys(
        &self,
        chord: Chord,
        then: Option<Chord>,
        scope: Scope,
    ) -> Vec<&Binding> {
        self.bindings
            .iter()
            .filter(|b| b.collides(chord, then, scope))
            .collect()
    }

    /// Adds `binding` after removing every binding it conflicts with; returns the removed.
    pub fn bind(&mut self, binding: Binding) -> Vec<Binding> {
        let (chord, then, scope) = (binding.chord, binding.then, binding.scope);
        let mut removed = Vec::new();
        let mut kept = Vec::with_capacity(self.bindings.len() + 1);
        for b in std::mem::take(&mut self.bindings) {
            if b.collides(chord, then, scope) {
                removed.push(b);
            } else {
                kept.push(b);
            }
        }
        self.bindings = kept;
        self.bindings.push(binding);
        removed
    }

    /// Removes the binding at `index`.
    pub fn remove(&mut self, index: usize) -> Option<Binding> {
        (index < self.bindings.len()).then(|| self.bindings.remove(index))
    }

    /// Command ids that the registry does not know.
    pub fn unknown_commands(&self, registry: &CommandRegistry) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for b in &self.bindings {
            if registry.get(&b.command).is_none() && !out.contains(&b.command) {
                out.push(b.command.clone());
            }
        }
        out
    }

    /// Applies the `keymap` object of the settings file (UI.md §14.3). Entries that cannot
    /// be read are skipped; the returned texts say which and why.
    pub fn apply_overrides(&mut self, overrides: &Value) -> Vec<String> {
        let mut warnings = Vec::new();
        let obj = match overrides {
            Value::Null => return warnings,
            Value::Object(o) => o,
            _ => {
                warnings.push("keymap: expected an object".to_string());
                return warnings;
            }
        };
        for (text, entry) in obj {
            let (chord, then) = match parse_keys(text) {
                Ok(k) => k,
                Err(e) => {
                    warnings.push(format!("keymap '{text}': {e}"));
                    continue;
                }
            };
            let binding = match entry {
                Value::Null => None,
                v => match Binding::from_json(chord, then, v) {
                    Ok(b) => Some(b),
                    Err(e) => {
                        warnings.push(format!("keymap '{text}': {e}"));
                        continue;
                    }
                },
            };
            if is_reserved(chord) || then.is_some_and(is_reserved) {
                warnings.push(format!("keymap '{text}': {RESERVED_WARNING}"));
                continue;
            }
            match binding {
                // an entry replaces every binding on its keys, and a single key the
                // sequences it starts (and the other way round)
                Some(b) => {
                    self.bindings.retain(|x| {
                        !(x.chord == chord
                            && (x.then == then || x.then.is_none() || then.is_none()))
                    });
                    self.bindings.push(b);
                }
                // null unbinds exactly these keys
                None => self
                    .bindings
                    .retain(|x| !(x.chord == chord && x.then == then)),
            }
        }
        warnings
    }

    /// The smallest `keymap` object that turns `defaults` into `self`.
    pub fn overrides(&self, defaults: &KeyMap) -> Value {
        let mut out = Map::new();
        let mut keys: Vec<(Chord, Option<Chord>)> = Vec::new();
        for b in self.bindings.iter().chain(&defaults.bindings) {
            if !keys.contains(&(b.chord, b.then)) {
                keys.push((b.chord, b.then));
            }
        }
        for (chord, then) in keys {
            let on = |m: &'_ KeyMap| -> Vec<Binding> {
                m.bindings
                    .iter()
                    .filter(|b| b.chord == chord && b.then == then)
                    .cloned()
                    .collect()
            };
            let (mine, theirs) = (on(self), on(defaults));
            if mine == theirs {
                continue;
            }
            let value = mine.first().map_or(Value::Null, Binding::to_json);
            out.insert(keys_text(chord, then), value);
        }
        Value::Object(out)
    }
}

/// Why a settings-file entry on a reserved chord is dropped (UI.md §14.3).
pub const RESERVED_WARNING: &str =
    "1, 2 and 3 are navigation keys; the binding was dropped (shading is on Mod+Shift+1 ..)";

/// True for the bare `1`, `2` and `3` chords: the navigation hold keys (UI.md §6.1, owner
/// note N1). They are read from keyboard state, so a binding on them could never be told
/// from a navigation gesture.
pub fn is_reserved(chord: Chord) -> bool {
    !chord.command
        && !chord.alt
        && !chord.shift
        && crate::viewport_panel::NavMode::KEYS
            .iter()
            .any(|(k, _)| *k == chord.key)
}

/// A prefix key that was pressed and waits for the second key of a sequence.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Armed {
    pub chord: Chord,
    /// When it was pressed, in seconds.
    pub at: f64,
}

/// What one key press does (UI.md §14.7).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Step<'a> {
    /// Run this binding; the press is consumed.
    Run(&'a Binding),
    /// The press is the first key of a sequence: consumed, nothing runs yet.
    Armed,
    /// Escape while a prefix was armed: consumed, the prefix is dropped.
    Cancelled,
    /// Not a binding: the press is left for the rest of the editor.
    Pass,
}

/// The key map in use, its defaults and where overrides are stored.
#[derive(Debug, Clone)]
pub struct KeyMapState {
    pub map: KeyMap,
    pub defaults: KeyMap,
    /// The settings file the overrides live in; `None` keeps them in memory only.
    pub path: Option<PathBuf>,
    /// Problems found while reading the settings file.
    pub warnings: Vec<String>,
    /// The prefix key waiting for its second key (UI.md §14.7).
    pub armed: Option<Armed>,
}

impl Default for KeyMapState {
    fn default() -> Self {
        let defaults = KeyMap::defaults();
        KeyMapState {
            map: defaults.clone(),
            defaults,
            path: None,
            warnings: Vec::new(),
            armed: None,
        }
    }
}

impl KeyMapState {
    /// Uses the settings file `path`: loads its `keymap` overrides over the defaults and
    /// stores later changes there.
    pub fn use_settings_file(&mut self, path: Option<PathBuf>) {
        self.map = self.defaults.clone();
        self.warnings.clear();
        if let Some(p) = &path {
            let mut file = read_settings(p);
            self.warnings = self.map.apply_overrides(&file["keymap"]);
            // entries on the reserved digits are removed from the file, so the warning
            // shows once; everything else in the file stays as it is
            if let Some(obj) = file["keymap"].as_object_mut() {
                let before = obj.len();
                obj.retain(|text, _| {
                    !parse_keys(text)
                        .is_ok_and(|(c, t)| is_reserved(c) || t.is_some_and(is_reserved))
                });
                if obj.len() != before {
                    if obj.is_empty() {
                        file.as_object_mut().map(|o| o.remove("keymap"));
                    }
                    if let Err(e) = std::fs::write(p, file.to_string()) {
                        self.warnings
                            .push(format!("Could not update the settings file: {e}"));
                    }
                }
            }
        }
        self.path = path;
    }

    /// Writes the overrides to the settings file (removing `keymap` when there are none).
    pub fn save(&self) -> Result<(), String> {
        match &self.path {
            Some(p) => store_overrides(p, &self.map.overrides(&self.defaults)),
            None => Ok(()),
        }
    }

    /// Back to the defaults.
    pub fn reset_all(&mut self) {
        self.map = self.defaults.clone();
        self.armed = None;
    }

    /// Consumes this frame's key presses (UI.md §14.7) and returns what they run.
    pub fn take_presses(
        &mut self,
        input: &mut egui::InputState,
        viewport_hot: bool,
    ) -> Vec<(String, Value)> {
        self.map.take_presses(input, viewport_hot, &mut self.armed)
    }

    /// The armed prefix, if it has not timed out at `now`.
    pub fn pending_prefix(&self, now: f64) -> Option<Chord> {
        self.armed
            .filter(|a| now - a.at <= SEQUENCE_TIMEOUT)
            .map(|a| a.chord)
    }

    /// Puts the default bindings of `command` with `args` back (dropping the user's for it);
    /// a chord a default needs is taken from whoever holds it now.
    pub fn reset_binding(&mut self, command: &str, args: &Value) {
        let args = normalize_args(args.clone());
        self.map
            .bindings
            .retain(|b| !(b.command == command && b.args == args));
        let defaults: Vec<Binding> = self
            .defaults
            .bindings
            .iter()
            .filter(|b| b.command == command && b.args == args)
            .cloned()
            .collect();
        for d in defaults {
            self.map.bind(d);
        }
    }

    /// True when the binding is the default one for its chord.
    pub fn is_default(&self, b: &Binding) -> bool {
        self.defaults.bindings.contains(b)
    }
}

/// The settings file as JSON; missing, unreadable or not an object gives `{}`.
pub fn read_settings(path: &Path) -> Value {
    match std::fs::read_to_string(path)
        .ok()
        .and_then(|t| serde_json::from_str::<Value>(&t).ok())
    {
        Some(v @ Value::Object(_)) => v,
        _ => json!({}),
    }
}

/// Stores `overrides` as the `keymap` key of the settings file, keeping its other keys.
pub fn store_overrides(path: &Path, overrides: &Value) -> Result<(), String> {
    let mut settings = read_settings(path);
    match overrides.as_object() {
        Some(o) if !o.is_empty() => settings["keymap"] = overrides.clone(),
        _ if !path.exists() => return Ok(()),
        _ => {
            if let Some(o) = settings.as_object_mut() {
                o.remove("keymap");
            }
        }
    }
    path.parent()
        .map_or(Ok(()), std::fs::create_dir_all)
        .and_then(|()| std::fs::write(path, settings.to_string()))
        .map_err(|e| format!("Could not store the key map: {e}"))
}
