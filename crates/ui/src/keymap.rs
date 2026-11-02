//! Key map: chords to registry commands, defaults, user overrides, conflicts
//! (docs/specs/UI.md §14). No drawing here; the editor window is `keymap_editor`.

use std::fmt;
use std::path::{Path, PathBuf};

use egui::{Event, Key, Modifiers};
use forge_command::CommandRegistry;
use serde::Deserialize;
use serde_json::{Map, Value, json};

/// The shipped defaults (own design, parity-pending G2).
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

/// One chord running one registry command with JSON arguments.
#[derive(Debug, Clone, PartialEq)]
pub struct Binding {
    pub chord: Chord,
    pub command: String,
    /// A JSON object (empty for none).
    pub args: Value,
    pub scope: Scope,
}

impl Binding {
    pub fn new(chord: Chord, command: impl Into<String>, args: Value) -> Binding {
        Binding {
            chord,
            command: command.into(),
            args: normalize_args(args),
            scope: Scope::Global,
        }
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

    fn from_json(chord: Chord, v: &Value) -> Result<Binding, KeyMapError> {
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
            command: command.to_string(),
            args,
            scope,
        })
    }
}

fn normalize_args(args: Value) -> Value {
    if args.is_null() { json!({}) } else { args }
}

/// Two bindings on one chord whose scopes overlap.
#[derive(Debug, Clone, PartialEq)]
pub struct Conflict {
    pub chord: Chord,
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
            bindings.push(Binding {
                chord: Chord::parse(&e.chord)?,
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

    /// The first binding for a key press, if any, in `viewport_hot` or not.
    pub fn lookup(&self, modifiers: &Modifiers, key: Key, viewport_hot: bool) -> Option<&Binding> {
        self.bindings
            .iter()
            .find(|b| b.chord.matches(modifiers, key) && (b.scope == Scope::Global || viewport_hot))
    }

    /// Consumes the key presses of this frame that have a binding and returns what they run,
    /// in press order. One command per key press.
    pub fn take_presses(
        &self,
        input: &mut egui::InputState,
        viewport_hot: bool,
    ) -> Vec<(String, Value)> {
        let mut out = Vec::new();
        let events = std::mem::take(&mut input.events);
        for event in events {
            if let Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } = &event
                && let Some(b) = self.lookup(modifiers, *key, viewport_hot)
            {
                out.push((b.command.clone(), b.args.clone()));
                continue;
            }
            input.events.push(event);
        }
        out
    }

    /// Bindings of `command`, in map order.
    pub fn for_command<'a>(&'a self, command: &'a str) -> impl Iterator<Item = &'a Binding> {
        self.bindings.iter().filter(move |b| b.command == command)
    }

    /// The first chord running `command` with exactly `args`.
    pub fn chord_of(&self, command: &str, args: &Value) -> Option<Chord> {
        let args = normalize_args(args.clone());
        self.for_command(command)
            .find(|b| b.args == args)
            .map(|b| b.chord)
    }

    /// Groups of bindings on one chord with overlapping scopes (UI.md §14.4).
    pub fn conflicts(&self) -> Vec<Conflict> {
        let mut out: Vec<Conflict> = Vec::new();
        for (i, a) in self.bindings.iter().enumerate() {
            for (j, b) in self.bindings.iter().enumerate().skip(i + 1) {
                if a.chord == b.chord && a.scope.overlaps(b.scope) {
                    match out.iter_mut().find(|c| c.chord == a.chord) {
                        Some(c) => {
                            for k in [i, j] {
                                if !c.bindings.contains(&k) {
                                    c.bindings.push(k);
                                }
                            }
                        }
                        None => out.push(Conflict {
                            chord: a.chord,
                            bindings: vec![i, j],
                        }),
                    }
                }
            }
        }
        out
    }

    /// The bindings that `chord` in `scope` would collide with.
    pub fn conflicts_with(&self, chord: Chord, scope: Scope) -> Vec<&Binding> {
        self.bindings
            .iter()
            .filter(|b| b.chord == chord && b.scope.overlaps(scope))
            .collect()
    }

    /// Adds `binding` after removing every binding it conflicts with; returns the removed.
    pub fn bind(&mut self, binding: Binding) -> Vec<Binding> {
        let (chord, scope) = (binding.chord, binding.scope);
        let mut removed = Vec::new();
        let mut kept = Vec::with_capacity(self.bindings.len() + 1);
        for b in std::mem::take(&mut self.bindings) {
            if b.chord == chord && b.scope.overlaps(scope) {
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
            let chord = match Chord::parse(text) {
                Ok(c) => c,
                Err(e) => {
                    warnings.push(format!("keymap '{text}': {e}"));
                    continue;
                }
            };
            let binding = match entry {
                Value::Null => None,
                v => match Binding::from_json(chord, v) {
                    Ok(b) => Some(b),
                    Err(e) => {
                        warnings.push(format!("keymap '{text}': {e}"));
                        continue;
                    }
                },
            };
            self.bindings.retain(|b| b.chord != chord);
            self.bindings.extend(binding);
        }
        warnings
    }

    /// The smallest `keymap` object that turns `defaults` into `self`.
    pub fn overrides(&self, defaults: &KeyMap) -> Value {
        let mut out = Map::new();
        let mut chords: Vec<Chord> = Vec::new();
        for b in self.bindings.iter().chain(&defaults.bindings) {
            if !chords.contains(&b.chord) {
                chords.push(b.chord);
            }
        }
        for chord in chords {
            let mine: Vec<&Binding> = self.bindings.iter().filter(|b| b.chord == chord).collect();
            let theirs: Vec<&Binding> = defaults
                .bindings
                .iter()
                .filter(|b| b.chord == chord)
                .collect();
            if mine == theirs {
                continue;
            }
            let value = mine.first().map_or(Value::Null, |b| b.to_json());
            out.insert(chord.to_string(), value);
        }
        Value::Object(out)
    }
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
}

impl Default for KeyMapState {
    fn default() -> Self {
        let defaults = KeyMap::defaults();
        KeyMapState {
            map: defaults.clone(),
            defaults,
            path: None,
            warnings: Vec::new(),
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
            let file = read_settings(p);
            self.warnings = self.map.apply_overrides(&file["keymap"]);
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
