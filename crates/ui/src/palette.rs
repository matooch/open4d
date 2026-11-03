//! The command palette: fuzzy search over every registry command, an argument form generated
//! from the command's schema, and the window that shows them (docs/specs/UI.md §18).
//!
//! The matcher, the entry list, the search order and the form model are pure and unit-tested
//! here; [`Palette::show`] only draws them.

use egui::{Align2, Key, Modifiers, RichText};
use forge_command::{CommandRegistry, schema};
use serde_json::{Value, json};

use crate::keymap::{Chord, KeyMap};
use crate::theme::Theme;

/// How many recent keys the palette keeps.
pub const MAX_RECENT: usize = 20;
/// Rows of the result list visible at once.
const VISIBLE_ROWS: usize = 12;

// ------------------------------------------------------------------ fuzzy matcher

fn is_word_break(c: char) -> bool {
    matches!(c, ' ' | '.' | '_' | '-' | '/' | ':' | '=' | ',')
}

fn lower(c: char) -> char {
    c.to_lowercase().next().unwrap_or(c)
}

/// Case-insensitive subsequence match of `query` in `text`; the score of the best alignment
/// (UI.md §18.2), `None` when some query character cannot be placed. An empty query is
/// `Some(0)`.
pub fn fuzzy_score(query: &str, text: &str) -> Option<i32> {
    let q: Vec<char> = query.chars().map(lower).collect();
    if q.is_empty() {
        return Some(0);
    }
    let original: Vec<char> = text.chars().collect();
    let t: Vec<char> = original.iter().copied().map(lower).collect();
    let n = t.len();
    if q.len() > n {
        return None;
    }
    let word_start = |j: usize| j == 0 || is_word_break(original[j - 1]);
    // best[j]: the best score with the current query character placed at text index j.
    let mut prev: Vec<Option<i32>> = vec![None; n];
    for (i, qc) in q.iter().enumerate() {
        let mut cur: Vec<Option<i32>> = vec![None; n];
        for j in 0..n {
            if t[j] != *qc {
                continue;
            }
            let base = 10 + if word_start(j) { 20 } else { 0 };
            let link = if i == 0 {
                Some(if j == 0 { 15 } else { -(j.min(5) as i32) })
            } else {
                (0..j)
                    .filter_map(|k| {
                        prev[k].map(|p| {
                            p + if k + 1 == j {
                                15
                            } else {
                                -((j - k - 1) as i32)
                            }
                        })
                    })
                    .max()
            };
            cur[j] = link.map(|l| base + l);
        }
        prev = cur;
    }
    prev.into_iter().flatten().max().map(|s| s - (n as i32) / 4)
}

// ------------------------------------------------------------------ entries and search

/// One line of the palette: a command, or a command with the arguments of a key binding.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub command: String,
    pub label: String,
    /// The arguments to run with (`{}` for a generic entry).
    pub args: Value,
    /// `mode=wire` for a preset entry, else empty.
    pub args_text: String,
    /// Made from a key binding with arguments: runs without a form.
    pub preset: bool,
    /// The command has required arguments, so a generic entry opens a form.
    pub needs_form: bool,
    pub chord: Option<Chord>,
    /// The chord as the platform spells it.
    pub shown: String,
}

impl Entry {
    /// The recency key (UI.md §18.4).
    pub fn key(&self) -> String {
        run_key(&self.command, &self.args)
    }

    fn fields(&self) -> Vec<String> {
        let mut f = vec![self.command.clone(), self.label.clone()];
        if !self.args_text.is_empty() {
            f.push(self.args_text.clone());
        }
        if let Some(c) = self.chord {
            f.push(c.to_string());
            f.push(self.shown.clone());
        }
        f
    }
}

fn has_args(args: &Value) -> bool {
    match args {
        Value::Null => false,
        Value::Object(o) => !o.is_empty(),
        _ => true,
    }
}

/// `id`, or `id <compact json>` when there are arguments.
pub fn run_key(command: &str, args: &Value) -> String {
    if has_args(args) {
        format!("{command} {args}")
    } else {
        command.to_string()
    }
}

/// Every registry command plus one preset per key binding with arguments (UI.md §18.3).
pub fn entries(
    registry: &CommandRegistry,
    keys: &KeyMap,
    shown: &dyn Fn(Chord) -> String,
) -> Vec<Entry> {
    let mut out = Vec::new();
    for h in registry.iter() {
        let needs_form = schema::validate(&h.args_schema(), &json!({})).is_err();
        let generic = keys.for_command(h.id()).find(|b| !has_args(&b.args));
        out.push(Entry {
            command: h.id().to_string(),
            label: h.label().to_string(),
            args: json!({}),
            args_text: String::new(),
            preset: false,
            needs_form,
            chord: generic.map(|b| b.chord),
            shown: generic.map(|b| shown(b.chord)).unwrap_or_default(),
        });
        for b in keys.for_command(h.id()).filter(|b| has_args(&b.args)) {
            let summary = b.args_summary();
            out.push(Entry {
                command: h.id().to_string(),
                label: format!("{}: {summary}", h.label()),
                args: b.args.clone(),
                args_text: summary,
                preset: true,
                needs_form: false,
                chord: Some(b.chord),
                shown: shown(b.chord),
            });
        }
    }
    out
}

/// Indices into `entries`, best first (UI.md §18.4). `recent` holds run keys, newest first.
pub fn search(entries: &[Entry], query: &str, recent: &[String]) -> Vec<usize> {
    let tokens: Vec<&str> = query.split_whitespace().collect();
    let recency = |e: &Entry| recent.iter().position(|k| *k == e.key());
    let mut scored: Vec<(usize, i32)> = Vec::new();
    for (i, e) in entries.iter().enumerate() {
        if tokens.is_empty() {
            scored.push((i, 0));
            continue;
        }
        let fields = e.fields();
        let mut total = 0;
        let mut all = true;
        for tok in &tokens {
            match fields.iter().filter_map(|f| fuzzy_score(tok, f)).max() {
                Some(s) => total += s,
                None => {
                    all = false;
                    break;
                }
            }
        }
        if all {
            scored.push((i, total));
        }
    }
    scored.sort_by(|(a, sa), (b, sb)| {
        let (ea, eb) = (&entries[*a], &entries[*b]);
        // Recent first; among recent ones the newest. With a query the score decides first.
        let rank = |e: &Entry| recency(e).unwrap_or(usize::MAX);
        let by_recent = rank(ea).cmp(&rank(eb));
        let by_text = ea
            .label
            .to_lowercase()
            .cmp(&eb.label.to_lowercase())
            .then_with(|| ea.command.cmp(&eb.command))
            .then_with(|| ea.args_text.cmp(&eb.args_text));
        if tokens.is_empty() {
            by_recent.then(by_text)
        } else {
            sb.cmp(sa).then(by_recent).then(by_text)
        }
    });
    scored.into_iter().map(|(i, _)| i).collect()
}

// ------------------------------------------------------------------ argument form

/// What kind of widget a property needs.
#[derive(Debug, Clone, PartialEq)]
pub enum FieldKind {
    Text,
    Integer,
    Number,
    /// A fixed set of values (enum, constants, boolean).
    Choice(Vec<Value>),
    /// Any other JSON, typed as text.
    Json,
}

/// One property of the argument object (an empty `name` is the whole argument value).
#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    pub name: String,
    pub required: bool,
    pub kind: FieldKind,
    pub description: String,
    pub default: Option<Value>,
}

/// The text a choice shows.
pub fn choice_text(v: &Value) -> String {
    match v.as_str() {
        Some(s) => s.to_string(),
        None => v.to_string(),
    }
}

fn resolve<'a>(root: &'a Value, s: &'a Value) -> &'a Value {
    if let Some(r) = s.get("$ref").and_then(Value::as_str)
        && let Some(t) = r.strip_prefix('#').and_then(|p| root.pointer(p))
    {
        return resolve(root, t);
    }
    if let Some([one]) = s.get("allOf").and_then(Value::as_array).map(Vec::as_slice) {
        return resolve(root, one);
    }
    s
}

fn is_null_schema(s: &Value) -> bool {
    s.get("type").and_then(Value::as_str) == Some("null")
}

fn kind_of_type(name: &str) -> FieldKind {
    match name {
        "string" => FieldKind::Text,
        "integer" => FieldKind::Integer,
        "number" => FieldKind::Number,
        "boolean" => FieldKind::Choice(vec![json!(true), json!(false)]),
        _ => FieldKind::Json,
    }
}

fn kind_of(root: &Value, s: &Value) -> FieldKind {
    let s = resolve(root, s);
    for key in ["anyOf", "oneOf"] {
        let Some(options) = s.get(key).and_then(Value::as_array) else {
            continue;
        };
        let real: Vec<&Value> = options.iter().filter(|o| !is_null_schema(o)).collect();
        // constants (documented enum variants) form a choice
        let consts: Vec<Value> = real
            .iter()
            .filter_map(|o| {
                o.get("const").cloned().or_else(|| {
                    o.get("enum")
                        .and_then(Value::as_array)
                        .filter(|e| e.len() == 1)
                        .map(|e| e[0].clone())
                })
            })
            .collect();
        if !real.is_empty() && consts.len() == real.len() {
            return FieldKind::Choice(consts);
        }
        if let [one] = real.as_slice() {
            return kind_of(root, one);
        }
        return FieldKind::Json;
    }
    if let Some(e) = s.get("enum").and_then(Value::as_array) {
        return FieldKind::Choice(e.clone());
    }
    match s.get("type") {
        Some(Value::String(t)) => kind_of_type(t),
        Some(Value::Array(ts)) => ts
            .iter()
            .filter_map(Value::as_str)
            .find(|t| *t != "null")
            .map_or(FieldKind::Json, kind_of_type),
        _ => FieldKind::Json,
    }
}

/// The form of a command: its properties, required ones first (UI.md §18.5). A schema that
/// is not an object with `properties` but rejects `{}` becomes one JSON field for the whole
/// argument value; one that accepts `{}` has no fields.
pub fn form_fields(schema_json: &Value) -> Vec<Field> {
    let Some(props) = schema_json.get("properties").and_then(Value::as_object) else {
        if schema::validate(schema_json, &json!({})).is_err() {
            return vec![Field {
                name: String::new(),
                required: true,
                kind: FieldKind::Json,
                description: "Arguments as JSON".into(),
                default: None,
            }];
        }
        return Vec::new();
    };
    let required: Vec<&str> = schema_json
        .get("required")
        .and_then(Value::as_array)
        .map(|r| r.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    let field = |name: &str, required: bool| {
        let s = &props[name];
        let r = resolve(schema_json, s);
        let description = s
            .get("description")
            .or_else(|| r.get("description"))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        Field {
            name: name.to_string(),
            required,
            kind: kind_of(schema_json, s),
            description,
            default: s.get("default").or_else(|| r.get("default")).cloned(),
        }
    };
    let mut out: Vec<Field> = required
        .iter()
        .filter(|n| props.contains_key(**n))
        .map(|n| field(n, true))
        .collect();
    out.extend(
        props
            .keys()
            .filter(|n| !required.contains(&n.as_str()))
            .map(|n| field(n, false)),
    );
    out
}

/// The text each field starts with.
pub fn initial_values(fields: &[Field]) -> Vec<String> {
    fields
        .iter()
        .map(|f| match (&f.default, &f.kind) {
            (Some(d), _) if !d.is_null() => choice_text(d),
            (_, FieldKind::Choice(options)) if f.required => {
                options.first().map(choice_text).unwrap_or_default()
            }
            _ => String::new(),
        })
        .collect()
}

fn parse_field(f: &Field, text: &str) -> Result<Value, String> {
    let name = if f.name.is_empty() {
        "arguments"
    } else {
        &f.name
    };
    let bad = |what: &str| format!("{name}: {what}");
    match &f.kind {
        FieldKind::Text => Ok(Value::String(text.to_string())),
        FieldKind::Integer => text
            .trim()
            .parse::<i64>()
            .map(Value::from)
            .map_err(|_| bad(&format!("\"{text}\" is not a whole number"))),
        FieldKind::Number => text
            .trim()
            .parse::<f64>()
            .ok()
            .filter(|v| v.is_finite())
            .map(Value::from)
            .ok_or_else(|| bad(&format!("\"{text}\" is not a number"))),
        FieldKind::Choice(options) => options
            .iter()
            .find(|o| choice_text(o) == text)
            .cloned()
            .ok_or_else(|| bad(&format!("\"{text}\" is not one of the choices"))),
        FieldKind::Json => serde_json::from_str(text).map_err(|e| bad(&e.to_string())),
    }
}

/// The argument object for the typed `values` (parallel to `fields`): empty optional fields
/// are left out, an empty required one is an error (UI.md §18.5).
pub fn build_args(fields: &[Field], values: &[String]) -> Result<Value, String> {
    let mut obj = serde_json::Map::new();
    for (f, text) in fields.iter().zip(values) {
        if text.trim().is_empty() {
            if f.required {
                return Err(format!(
                    "{} is required",
                    if f.name.is_empty() {
                        "arguments"
                    } else {
                        &f.name
                    }
                ));
            }
            continue;
        }
        let v = parse_field(f, text)?;
        if f.name.is_empty() {
            return Ok(v);
        }
        obj.insert(f.name.clone(), v);
    }
    Ok(Value::Object(obj))
}

// ------------------------------------------------------------------ the window

struct Form {
    entry: Entry,
    schema: Value,
    fields: Vec<Field>,
    values: Vec<String>,
    error: Option<String>,
}

/// State of the palette window.
#[derive(Default)]
pub struct Palette {
    pub open: bool,
    pub query: String,
    /// Index into the current result list.
    pub selected: usize,
    /// Run keys, newest first (UI.md §18.4).
    pub recent: Vec<String>,
    entries: Option<Vec<Entry>>,
    form: Option<Form>,
    focus: bool,
}

impl Palette {
    /// Opens with an empty query and the first row selected.
    pub fn open_palette(&mut self) {
        self.open = true;
        self.query.clear();
        self.selected = 0;
        self.entries = None;
        self.form = None;
        self.focus = true;
    }

    pub fn close(&mut self) {
        self.open = false;
        self.entries = None;
        self.form = None;
    }

    /// True while the argument form replaces the list.
    pub fn form_open(&self) -> bool {
        self.form.is_some()
    }

    /// The error the form shows, if any.
    pub fn form_error(&self) -> Option<&str> {
        self.form.as_ref().and_then(|f| f.error.as_deref())
    }

    /// Records a command run from anywhere in the window (UI.md §18.4).
    pub fn note_run(&mut self, command: &str, args: &Value) {
        if command == "ui.command_palette" {
            return;
        }
        let mut keys = vec![run_key(command, args)];
        if has_args(args) {
            keys.push(command.to_string());
        }
        for k in keys.into_iter().rev() {
            self.recent.retain(|r| *r != k);
            self.recent.insert(0, k);
        }
        self.recent.truncate(MAX_RECENT);
    }

    /// The ids of the entries the current query shows, best first (tests read it).
    pub fn visible(&self) -> Vec<(String, Value)> {
        let Some(entries) = &self.entries else {
            return Vec::new();
        };
        search(entries, &self.query, &self.recent)
            .into_iter()
            .map(|i| (entries[i].command.clone(), entries[i].args.clone()))
            .collect()
    }

    /// Draws the window when open. Returns the command to run (the palette is closed then).
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        registry: &CommandRegistry,
        keys: &KeyMap,
    ) -> Option<(String, Value)> {
        if !self.open {
            return None;
        }
        if self.entries.is_none() {
            let shown = |c: Chord| c.display(ctx);
            self.entries = Some(entries(registry, keys, &shown));
        }
        let mut run = None;
        egui::Window::new("Command palette")
            .title_bar(false)
            .anchor(Align2::CENTER_TOP, [0.0, 48.0])
            .fixed_size([560.0, 0.0])
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                if self.form.is_some() {
                    self.form_ui(ui, &mut run);
                } else {
                    self.list_ui(ui, registry, &mut run);
                }
            });
        let escape = ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape));
        if escape || run.is_some() {
            self.close();
        }
        run
    }

    fn list_ui(
        &mut self,
        ui: &mut egui::Ui,
        registry: &CommandRegistry,
        run: &mut Option<(String, Value)>,
    ) {
        let theme = Theme::default();
        // Take the navigation keys first so the text field never sees them.
        let (down, up, enter) = ui.input_mut(|i| {
            (
                i.consume_key(Modifiers::NONE, Key::ArrowDown),
                i.consume_key(Modifiers::NONE, Key::ArrowUp),
                i.consume_key(Modifiers::NONE, Key::Enter),
            )
        });
        let before = self.query.clone();
        let edit = ui.add(
            egui::TextEdit::singleline(&mut self.query)
                .hint_text("Search commands")
                .desired_width(f32::INFINITY),
        );
        if self.focus {
            edit.request_focus();
            self.focus = false;
        }
        if self.query != before {
            self.selected = 0;
        }
        let hits = {
            let entries = self.entries.as_deref().unwrap_or_default();
            search(entries, &self.query, &self.recent)
        };
        if down && self.selected + 1 < hits.len() {
            self.selected += 1;
        }
        if up {
            self.selected = self.selected.saturating_sub(1);
        }
        self.selected = self.selected.min(hits.len().saturating_sub(1));
        ui.separator();
        if hits.is_empty() {
            ui.weak("No matching command");
        }
        let mut picked = enter.then(|| hits.get(self.selected).copied()).flatten();
        let row_height = ui.text_style_height(&egui::TextStyle::Body) + 8.0;
        egui::ScrollArea::vertical()
            .max_height(row_height * VISIBLE_ROWS as f32)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                let entries = self.entries.as_deref().unwrap_or_default();
                for (row, &i) in hits.iter().enumerate() {
                    let e = &entries[i];
                    let selected = row == self.selected;
                    let response = ui
                        .horizontal(|ui| {
                            let r =
                                ui.add(egui::Button::selectable(selected, RichText::new(&e.label)));
                            ui.weak(&e.command);
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if e.chord.is_some() {
                                        ui.label(
                                            RichText::new(&e.shown).monospace().color(theme.accent),
                                        );
                                    }
                                },
                            );
                            r
                        })
                        .inner;
                    if selected && (down || up) {
                        response.scroll_to_me(None);
                    }
                    if response.clicked() {
                        picked = Some(i);
                    }
                }
            });
        if let Some(i) = picked {
            let entries = self.entries.as_deref().unwrap_or_default();
            let e = entries[i].clone();
            if e.needs_form
                && !e.preset
                && let Some(h) = registry.get(&e.command)
            {
                let schema = h.args_schema();
                let fields = form_fields(&schema);
                let values = initial_values(&fields);
                self.form = Some(Form {
                    entry: e,
                    schema,
                    fields,
                    values,
                    error: None,
                });
            } else {
                *run = Some((e.command, e.args));
            }
        }
    }

    fn form_ui(&mut self, ui: &mut egui::Ui, run: &mut Option<(String, Value)>) {
        let theme = Theme::default();
        let Some(form) = self.form.as_mut() else {
            return;
        };
        ui.horizontal(|ui| {
            ui.heading(&form.entry.label);
            ui.weak(&form.entry.command);
        });
        ui.separator();
        let mut submit = false;
        egui::Grid::new("palette_form")
            .num_columns(2)
            .spacing([12.0, 6.0])
            .show(ui, |ui| {
                for (f, value) in form.fields.iter().zip(form.values.iter_mut()) {
                    let name = if f.name.is_empty() {
                        "arguments"
                    } else {
                        &f.name
                    };
                    let label = if f.required {
                        format!("{name} *")
                    } else {
                        name.to_string()
                    };
                    let l = ui.label(label);
                    if !f.description.is_empty() {
                        l.clone().on_hover_text(&f.description);
                    }
                    match &f.kind {
                        FieldKind::Choice(options) => {
                            let combo = egui::ComboBox::from_id_salt(("palette_field", name))
                                .selected_text(value.clone())
                                .show_ui(ui, |ui| {
                                    if !f.required {
                                        ui.selectable_value(value, String::new(), "(none)");
                                    }
                                    for o in options {
                                        let t = choice_text(o);
                                        ui.selectable_value(value, t.clone(), t);
                                    }
                                });
                            combo.response.labelled_by(l.id);
                        }
                        _ => {
                            let edit = ui
                                .add(
                                    egui::TextEdit::singleline(value)
                                        .id_salt(("palette_field", name))
                                        .desired_width(320.0),
                                )
                                .labelled_by(l.id);
                            if edit.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                                submit = true;
                            }
                        }
                    }
                    ui.end_row();
                }
            });
        if let Some(e) = &form.error {
            ui.colored_label(theme.error, e);
        }
        ui.horizontal(|ui| {
            if ui.button("Run").clicked() {
                submit = true;
            }
            if ui.button("Back").clicked() {
                self.form = None;
                self.focus = true;
            }
        });
        if let Some(form) = self.form.as_mut()
            && submit
        {
            match build_args(&form.fields, &form.values).and_then(|args| {
                schema::validate(&form.schema, &args)
                    .map(|()| args)
                    .map_err(|e| e.replace("$.", ""))
            }) {
                Ok(args) => *run = Some((form.entry.command.clone(), args)),
                Err(e) => form.error = Some(e),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_command::register_editor_commands;

    fn registry() -> CommandRegistry {
        let mut r = CommandRegistry::new();
        register_editor_commands(&mut r).unwrap();
        r
    }

    fn score(q: &str, t: &str) -> i32 {
        fuzzy_score(q, t).unwrap_or_else(|| panic!("{q:?} should match {t:?}"))
    }

    #[test]
    fn matches_subsequences_ignoring_case() {
        assert!(fuzzy_score("undo", "edit.undo").is_some());
        assert!(fuzzy_score("UNDO", "Undo").is_some());
        assert!(fuzzy_score("vsh", "view.shading").is_some());
        assert!(fuzzy_score("oudn", "edit.undo").is_none());
        assert!(fuzzy_score("undoo", "undo").is_none());
        assert!(fuzzy_score("x", "").is_none());
        assert_eq!(fuzzy_score("", "anything"), Some(0));
        assert_eq!(fuzzy_score("", ""), Some(0));
    }

    #[test]
    fn better_alignments_score_higher() {
        // a prefix beats a later word start
        assert!(score("ed", "edit.undo") > score("ed", "undo.edit"));
        // a word start beats the middle of a word
        assert!(score("s", "view.shading") > score("s", "viewers"));
        // a contiguous run beats scattered letters
        assert!(score("sel", "select") > score("sel", "sxxexxl"));
        // the best alignment is found even when the first hit is poor
        // (a leftmost-greedy match would take the first `a` and lose the run `ab`)
        assert!(score("ab", "xaxab") > 25);
        // a shorter text wins a tie
        assert!(score("undo", "undo") > score("undo", "undo_everything_again"));
        assert!(score("undo", "Undo") > score("undo", "edit.undo"));
    }

    #[test]
    fn unicode_and_long_text_do_not_panic() {
        assert!(fuzzy_score("ä", "Ändern").is_some());
        assert!(fuzzy_score("zz", &"a".repeat(500)).is_none());
        assert!(fuzzy_score("aaa", &"a".repeat(200)).is_some());
    }

    fn shown(c: Chord) -> String {
        c.to_string().replace("Mod", "Ctrl")
    }

    fn entries_of(r: &CommandRegistry) -> Vec<Entry> {
        entries(r, &KeyMap::defaults(), &shown)
    }

    fn find<'a>(es: &'a [Entry], id: &str, args: Value) -> &'a Entry {
        es.iter()
            .find(|e| e.command == id && e.args == args)
            .unwrap_or_else(|| panic!("no entry {id} {args}"))
    }

    #[test]
    fn entries_cover_the_registry_and_the_bindings_with_arguments() {
        let r = registry();
        let es = entries_of(&r);
        for h in r.iter() {
            let e = find(&es, h.id(), json!({}));
            assert_eq!(e.label, h.label());
            assert!(!e.preset);
        }
        // chords come from the key map
        let undo = find(&es, "edit.undo", json!({}));
        assert_eq!(undo.chord.unwrap().to_string(), "Mod+Z");
        assert_eq!(undo.shown, "Ctrl+Z");
        let palette = find(&es, "ui.command_palette", json!({}));
        assert_eq!(palette.chord.unwrap().to_string(), "Mod+Shift+P");
        // a command bound only with arguments has no chord on its generic entry
        let shading = find(&es, "view.shading", json!({}));
        assert!(shading.chord.is_none() && shading.needs_form);
        // ... but a preset per binding
        let wire = find(&es, "view.shading", json!({"mode": "wire"}));
        assert!(wire.preset && !wire.needs_form);
        assert_eq!(wire.chord.unwrap().to_string(), "3");
        assert_eq!(wire.args_text, "mode=wire");
        assert!(wire.label.ends_with("mode=wire"), "{}", wire.label);
        // needs_form follows the schema
        assert!(find(&es, "object.create", json!({})).needs_form);
        assert!(!find(&es, "edit.redo", json!({})).needs_form);
        assert!(!find(&es, "object.delete", json!({})).needs_form);
    }

    fn ids(es: &[Entry], hits: &[usize]) -> Vec<String> {
        hits.iter().map(|&i| es[i].command.clone()).collect()
    }

    #[test]
    fn search_matches_id_label_arguments_and_chord() {
        let r = registry();
        let es = entries_of(&r);
        let first = |q: &str| {
            let hits = search(&es, q, &[]);
            assert!(!hits.is_empty(), "{q}: no hits");
            es[hits[0]].clone()
        };
        assert_eq!(first("edit.undo").command, "edit.undo");
        assert_eq!(first("undo").command, "edit.undo");
        assert_eq!(first("Undo").label, "Undo");
        // by chord text, canonical and shown
        assert_eq!(first("Mod+Shift+P").command, "ui.command_palette");
        assert_eq!(first("ctrl+shift+p").command, "ui.command_palette");
        // by arguments
        let e = first("shading wire");
        assert_eq!(e.command, "view.shading");
        assert!(e.args_text.contains("mode=wire"), "{}", e.label);
        // every token must match
        assert!(search(&es, "undo zzzzqq", &[]).is_empty());
        assert!(search(&es, "   ", &[]).len() == es.len());
    }

    #[test]
    fn recent_entries_come_first_then_the_rest_by_label() {
        let r = registry();
        let es = entries_of(&r);
        // empty query: every entry, recent first (newest first), then by label
        let recent = vec!["edit.redo".to_string(), "edit.undo".to_string()];
        let hits = search(&es, "", &recent);
        assert_eq!(hits.len(), es.len());
        assert_eq!(ids(&es, &hits)[..2], ["edit.redo", "edit.undo"]);
        let labels: Vec<String> = hits[2..]
            .iter()
            .map(|&i| es[i].label.to_lowercase())
            .collect();
        let mut sorted = labels.clone();
        sorted.sort();
        assert_eq!(labels, sorted);
        // with a query the score decides first, recency breaks ties
        let hits = search(&es, "edit", &recent);
        assert!(!hits.is_empty());
        let exact = search(&es, "edit.redo", &recent);
        assert_eq!(es[exact[0]].command, "edit.redo");
        // the same score: the recent one first
        let a = search(&es, "object.rename", &[]);
        assert_eq!(es[a[0]].command, "object.rename");
    }

    #[test]
    fn note_run_keeps_a_bounded_unique_newest_first_list() {
        let mut p = Palette::default();
        p.note_run("edit.undo", &json!({}));
        p.note_run("object.create", &json!({"type": "forge.cube"}));
        p.note_run("edit.undo", &json!(null));
        p.note_run("ui.command_palette", &json!({}));
        assert_eq!(
            p.recent,
            [
                "edit.undo".to_string(),
                r#"object.create {"type":"forge.cube"}"#.to_string(),
                "object.create".to_string()
            ]
        );
        for i in 0..40 {
            p.note_run(&format!("a.b{i}"), &json!({}));
        }
        assert_eq!(p.recent.len(), MAX_RECENT);
        assert_eq!(p.recent[0], "a.b39");
    }

    fn fields_of(r: &CommandRegistry, id: &str) -> Vec<Field> {
        form_fields(&r.get(id).unwrap().args_schema())
    }

    #[test]
    fn forms_are_generated_from_the_schema() {
        let r = registry();
        let f = fields_of(&r, "object.create");
        assert_eq!(f.len(), 1);
        assert_eq!((f[0].name.as_str(), f[0].required), ("type", true));
        assert_eq!(f[0].kind, FieldKind::Text);
        // an enum (through $ref) is a choice
        let f = fields_of(&r, "view.shading");
        let mode = f.iter().find(|f| f.name == "mode").unwrap();
        assert!(mode.required);
        let FieldKind::Choice(options) = &mode.kind else {
            panic!("{:?}", mode.kind);
        };
        assert!(options.contains(&json!("smooth_wire")), "{options:?}");
        // the optional pane comes after the required mode and is a choice too
        assert_eq!(f[0].name, "mode");
        let pane = f.iter().find(|f| f.name == "pane").unwrap();
        assert!(!pane.required && matches!(pane.kind, FieldKind::Choice(_)));
        // numbers and booleans
        let f = fields_of(&r, "timeline.set_frame");
        assert_eq!(f[0].kind, FieldKind::Integer);
        let f = fields_of(&r, "view.shadows");
        assert_eq!(
            f[0].kind,
            FieldKind::Choice(vec![json!(true), json!(false)])
        );
        // lists are JSON text
        let f = fields_of(&r, "object.delete");
        assert!(f.iter().all(|f| !f.required));
        assert!(
            f.iter()
                .any(|f| f.name == "nodes" && f.kind == FieldKind::Json)
        );
        // no arguments, no fields
        assert!(fields_of(&r, "edit.undo").is_empty());
    }

    #[test]
    fn a_non_object_schema_gets_one_json_field() {
        let schema = json!({"type": "array", "items": {"type": "integer"}, "minItems": 1});
        let f = form_fields(&schema);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].name, "");
        assert_eq!(build_args(&f, &["[1, 2]".into()]).unwrap(), json!([1, 2]));
        assert!(build_args(&f, &[String::new()]).is_err());
    }

    #[test]
    fn typed_text_becomes_the_argument_object() {
        let r = registry();
        let f = fields_of(&r, "view.shading");
        let values = initial_values(&f);
        // a required choice starts at its first option; the optional pane is empty
        let args = build_args(&f, &values).unwrap();
        assert!(
            args["mode"].is_string() && args.get("pane").is_none(),
            "{args}"
        );
        let set = |name: &str, v: &str| -> Vec<String> {
            let mut vals = values.clone();
            let i = f.iter().position(|f| f.name == name).unwrap();
            vals[i] = v.to_string();
            vals
        };
        let args = build_args(&f, &set("pane", "top")).unwrap();
        assert_eq!(args["pane"], "top");
        assert!(
            build_args(&f, &set("mode", ""))
                .unwrap_err()
                .contains("mode is required")
        );
        assert!(build_args(&f, &set("mode", "bogus")).is_err());

        let f = fields_of(&r, "timeline.set_frame");
        assert_eq!(
            build_args(&f, &["12".into()]).unwrap(),
            json!({"frame": 12})
        );
        assert_eq!(
            build_args(&f, &[" 7 ".into()]).unwrap(),
            json!({"frame": 7})
        );
        assert!(
            build_args(&f, &["x".into()])
                .unwrap_err()
                .starts_with("frame:")
        );
        assert!(build_args(&f, &["1.5".into()]).is_err());
        assert_eq!(
            build_args(&f, &[String::new()]).unwrap_err(),
            "frame is required"
        );

        let f = fields_of(&r, "object.delete");
        assert_eq!(
            build_args(&f, &["[1,2]".into()]).unwrap(),
            json!({"nodes": [1, 2]})
        );
        assert_eq!(build_args(&f, &[String::new()]).unwrap(), json!({}));
        assert!(build_args(&f, &["[1,".into()]).is_err());

        // a text field keeps spaces and may be empty only when optional
        let f = fields_of(&r, "object.create");
        assert_eq!(
            build_args(&f, &["forge.cube".into()]).unwrap(),
            json!({"type": "forge.cube"})
        );
    }
}
