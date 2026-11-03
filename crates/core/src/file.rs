//! Native scene file: deterministic RON, optional binary chunks, step-by-step migrations
//! and external file references (docs/specs/CORE.md §6).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::layers::{self, Layer, Layers};
use crate::{
    Category, MeshData, Node, NodeId, Registry, Scene, SceneSettings, TimeConfig, Track, Value,
    parse_target,
};

pub const FORMAT: &str = "forge3d-scene";
/// Version 2 added `Value::Mesh`, version 3 `Value::IndexList`, version 4 keyframe
/// tracks, version 5 the scene settings, version 6 declares link lists and the hidden
/// `object.*` params and adds the chunk section and file reference params, version 7 the
/// time configuration (markers, preview range, playback mode), version 8 the expression
/// tag and its graph parameter (brief 108), version 9 the layer table and the hidden
/// `object.layer` param (brief 158). Older files still load, through [`MIGRATIONS`].
pub const VERSION: u32 = 9;

/// A value whose binary payload is at least this many bytes is written as a chunk by
/// [`save_with`] (CORE.md §6.2): 42 mesh points or 256 indices.
pub const CHUNK_MIN_BYTES: usize = 1024;

/// Separates the RON document from the chunk section.
const MARKER: &[u8] = b"\n%%FORGE-CHUNKS 1\n";
const KIND_MESH: u8 = 1;
const KIND_INDEX_LIST: u8 = 2;

/// The parsed document of a file, before it becomes a scene. Opaque; migrations rewrite it.
#[derive(Serialize, Deserialize)]
pub struct SceneFile {
    format: String,
    version: u32,
    #[serde(default)]
    settings: SceneSettings,
    #[serde(default)]
    time: TimeConfig,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    layers: Vec<Layer>,
    roots: Vec<NodeRecord>,
}

/// One migration step: turns a document of version `v` into one of version `v + 1` and
/// returns warnings for anything it had to drop.
pub type Migration = fn(&mut SceneFile) -> Vec<String>;

/// `MIGRATIONS[v - 1]` migrates version `v` to `v + 1` (CORE.md §6.3).
pub const MIGRATIONS: &[Migration] = &[
    migrate_1_to_2,
    migrate_2_to_3,
    migrate_3_to_4,
    migrate_4_to_5,
    migrate_5_to_6,
    migrate_6_to_7,
    migrate_7_to_8,
    migrate_8_to_9,
];

/// Version 2 added `Value::Mesh`: additive, nothing to rewrite.
fn migrate_1_to_2(_: &mut SceneFile) -> Vec<String> {
    Vec::new()
}

/// Version 3 added `Value::IndexList`: additive.
fn migrate_2_to_3(_: &mut SceneFile) -> Vec<String> {
    Vec::new()
}

/// Version 4 added `tracks`: a missing field is read as empty by serde.
fn migrate_3_to_4(_: &mut SceneFile) -> Vec<String> {
    Vec::new()
}

/// Version 5 added `settings`: a missing field is read as the defaults by serde.
fn migrate_4_to_5(_: &mut SceneFile) -> Vec<String> {
    Vec::new()
}

/// Version 6 declares `LinkList` values and `object.*` params that builds since brief 068
/// already wrote under version 5; missing params take their defaults on load.
fn migrate_5_to_6(_: &mut SceneFile) -> Vec<String> {
    Vec::new()
}

/// Version 7 added `time`: a missing field is read as the defaults by serde.
fn migrate_6_to_7(_: &mut SceneFile) -> Vec<String> {
    Vec::new()
}

/// Version 8 declares the `forge.expression` tag, whose graph is a string parameter:
/// nothing to rewrite.
fn migrate_7_to_8(_: &mut SceneFile) -> Vec<String> {
    Vec::new()
}

/// Version 9 added `layers` and the hidden `object.layer` link: a missing field is read as
/// no layers by serde and a missing param takes its default on load.
fn migrate_8_to_9(_: &mut SceneFile) -> Vec<String> {
    Vec::new()
}

/// Runs the steps from `file.version` up to `target`; shared by `load_with` and the tests.
fn migrate(file: &mut SceneFile, steps: &[Migration], target: u32) -> Vec<String> {
    let mut warnings = Vec::new();
    while file.version < target {
        let step = steps[(file.version - 1) as usize];
        warnings.extend(step(file));
        file.version += 1;
    }
    warnings
}

/// `Value` as it appears in the RON text: the same variants (so the text of every value is
/// unchanged) plus references to binary chunks.
#[derive(Serialize, Deserialize)]
enum StoredValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    Vec3([f64; 3]),
    Color([f64; 3]),
    Enum(String),
    String(String),
    Link(Option<NodeId>),
    LinkList(Vec<NodeId>),
    Mesh(Arc<MeshData>),
    IndexList(Vec<u32>),
    MeshChunk(u32),
    IndexListChunk(u32),
}

impl From<Value> for StoredValue {
    fn from(v: Value) -> Self {
        match v {
            Value::Bool(x) => Self::Bool(x),
            Value::Int(x) => Self::Int(x),
            Value::Float(x) => Self::Float(x),
            Value::Vec3(x) => Self::Vec3(x),
            Value::Color(x) => Self::Color(x),
            Value::Enum(x) => Self::Enum(x),
            Value::String(x) => Self::String(x),
            Value::Link(x) => Self::Link(x),
            Value::LinkList(x) => Self::LinkList(x),
            Value::Mesh(x) => Self::Mesh(x),
            Value::IndexList(x) => Self::IndexList(x),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct NodeRecord {
    id: NodeId,
    type_id: String,
    name: String,
    params: BTreeMap<String, StoredValue>,
    #[serde(default)]
    tracks: BTreeMap<String, Track>,
    tags: Vec<NodeRecord>,
    children: Vec<NodeRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LoadError {
    #[error("not a valid scene file: {0}")]
    Parse(String),
    #[error("unexpected format {0:?}")]
    WrongFormat(String),
    #[error("unsupported scene version {0}")]
    UnsupportedVersion(u32),
    #[error("node id {0} appears more than once")]
    DuplicateId(NodeId),
    #[error("invalid structure: {0}")]
    Structure(String),
    #[error("damaged chunk data: {0}")]
    Chunk(String),
}

/// Recoverable issues found while loading.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LoadReport {
    pub warnings: Vec<String>,
}

/// Options for [`save_with`].
#[derive(Debug, Clone)]
pub struct SaveOptions {
    /// The folder of the scene file: absolute file reference params under it or beside it
    /// are written relative to it (CORE.md §6.5).
    pub base_dir: Option<PathBuf>,
    /// Values with at least this many payload bytes become chunks (`usize::MAX`: none).
    pub chunk_min_bytes: usize,
}

impl Default for SaveOptions {
    fn default() -> Self {
        SaveOptions {
            base_dir: None,
            chunk_min_bytes: CHUNK_MIN_BYTES,
        }
    }
}

/// Options for [`load_with`].
#[derive(Debug, Clone, Default)]
pub struct LoadOptions {
    /// The folder of the scene file; relative file references are resolved against it.
    pub base_dir: Option<PathBuf>,
    /// Warn about referenced files that do not exist (needs `base_dir`; reads the file system).
    pub check_external: bool,
}

/// The text form: no chunks, no path rewriting (CORE.md §6.1).
pub fn save(scene: &Scene) -> String {
    let opts = SaveOptions {
        base_dir: None,
        chunk_min_bytes: usize::MAX,
    };
    String::from_utf8(save_with(scene, &opts)).expect("RON text is UTF-8")
}

/// Text form, or text plus a chunk section when a value reaches `chunk_min_bytes`.
pub fn save_with(scene: &Scene, opts: &SaveOptions) -> Vec<u8> {
    struct Writer<'a> {
        scene: &'a Scene,
        opts: &'a SaveOptions,
        chunks: Vec<(u8, Vec<u8>)>,
    }
    impl Writer<'_> {
        fn value(&mut self, v: Value) -> StoredValue {
            let (kind, size) = match &v {
                Value::Mesh(m) => (
                    KIND_MESH,
                    8 + 24 * m.points.len()
                        + m.polys.iter().map(|p| 4 + 4 * p.len()).sum::<usize>(),
                ),
                Value::IndexList(l) => (KIND_INDEX_LIST, 4 + 4 * l.len()),
                _ => return v.into(),
            };
            if size < self.opts.chunk_min_bytes {
                return v.into();
            }
            let payload = match &v {
                Value::Mesh(m) => encode_mesh(m),
                Value::IndexList(l) => encode_index_list(l),
                _ => unreachable!("only chunkable kinds get here"),
            };
            let at = self.chunks.len() as u32;
            self.chunks.push((kind, payload));
            if kind == KIND_MESH {
                StoredValue::MeshChunk(at)
            } else {
                StoredValue::IndexListChunk(at)
            }
        }

        fn record(&mut self, id: NodeId) -> NodeRecord {
            let n = self.scene.node(id).expect("listed node exists");
            let ty = self.scene.registry().get(&n.type_id).cloned();
            let mut params = BTreeMap::new();
            for (key, value) in &n.params {
                let mut value = value.clone();
                if let (Some(ty), Some(base), Value::String(s)) =
                    (&ty, &self.opts.base_dir, &mut value)
                {
                    let is_ref = ty.find_param(key).is_some_and(|d| d.ui.file_ref);
                    if is_ref && let Some(rel) = relativize(s, base) {
                        *s = rel;
                    }
                }
                params.insert(key.clone(), self.value(value));
            }
            NodeRecord {
                id,
                type_id: n.type_id.clone(),
                name: n.name.clone(),
                params,
                tracks: n.tracks.clone(),
                tags: n.tags.iter().map(|t| self.record(*t)).collect(),
                children: n.children.iter().map(|c| self.record(*c)).collect(),
            }
        }
    }
    let mut w = Writer {
        scene,
        opts,
        chunks: Vec::new(),
    };
    let roots = scene.roots().iter().map(|r| w.record(*r)).collect();
    let file = SceneFile {
        format: FORMAT.to_string(),
        version: VERSION,
        settings: scene.settings(),
        time: scene.time().clone(),
        layers: scene.layers().as_slice().to_vec(),
        roots,
    };
    let text = ron::ser::to_string_pretty(
        &file,
        ron::ser::PrettyConfig::default().new_line("\n".to_string()),
    )
    .expect("scene data always serializes");
    let mut out = text.into_bytes();
    if !w.chunks.is_empty() {
        out.extend_from_slice(MARKER);
        out.extend_from_slice(&(w.chunks.len() as u32).to_le_bytes());
        for (kind, payload) in &w.chunks {
            out.push(*kind);
            out.extend_from_slice(&(payload.len() as u64).to_le_bytes());
            out.extend_from_slice(&fnv1a(payload).to_le_bytes());
            out.extend_from_slice(payload);
        }
    }
    out
}

/// Loads the text form; a reference to a chunk is a `LoadError::Chunk`.
pub fn load(registry: Arc<Registry>, text: &str) -> Result<(Scene, LoadReport), LoadError> {
    load_with(registry, text.as_bytes(), &LoadOptions::default())
}

/// Loads a text or chunked file with the default options. This is the fuzz entry point
/// (CORE.md §6.1): a pure, total function of its bytes.
pub fn load_bytes(registry: Arc<Registry>, bytes: &[u8]) -> Result<(Scene, LoadReport), LoadError> {
    load_with(registry, bytes, &LoadOptions::default())
}

pub fn load_with(
    registry: Arc<Registry>,
    bytes: &[u8],
    opts: &LoadOptions,
) -> Result<(Scene, LoadReport), LoadError> {
    let (text, section) = match bytes.windows(MARKER.len()).position(|w| w == MARKER) {
        Some(at) => (&bytes[..at], Some(&bytes[at + MARKER.len()..])),
        None => (bytes, None),
    };
    let text = std::str::from_utf8(text).map_err(|e| LoadError::Parse(e.to_string()))?;
    let mut file: SceneFile = match ron::from_str(text) {
        Ok(f) => f,
        Err(e) => {
            // a newer file may use syntax this build does not know: say so
            #[derive(Deserialize)]
            struct Header {
                format: String,
                version: u32,
            }
            return Err(match ron::from_str::<Header>(text) {
                Ok(h) if h.format == FORMAT && h.version > VERSION => {
                    LoadError::UnsupportedVersion(h.version)
                }
                _ => LoadError::Parse(e.to_string()),
            });
        }
    };
    if file.format != FORMAT {
        return Err(LoadError::WrongFormat(file.format));
    }
    if file.version == 0 || file.version > VERSION {
        return Err(LoadError::UnsupportedVersion(file.version));
    }
    let chunks = match section {
        Some(s) => parse_section(s)?,
        None => Vec::new(),
    };
    let mut loader = Loader {
        scene: Scene::new(registry),
        report: LoadReport::default(),
        seen: BTreeSet::new(),
        chunks,
        opts,
    };
    loader.report.warnings = migrate(&mut file, MIGRATIONS, VERSION);
    for r in file.roots {
        loader.node(r, Place::Child(None))?;
    }
    let settings = match file.settings.check() {
        Ok(()) => file.settings,
        Err(e) => {
            loader
                .report
                .warnings
                .push(format!("scene settings: {e}; reset to the defaults"));
            SceneSettings::default()
        }
    };
    loader.scene.init_settings(settings);
    match file.time.check(&settings) {
        Ok(()) => {
            let mut time = file.time;
            time.sort_markers();
            loader.scene.init_time(time);
        }
        Err(e) => loader
            .report
            .warnings
            .push(format!("scene time: {e}; reset to the defaults")),
    }
    loader.layers(file.layers);
    let next = loader.seen.last().map_or(1, |id| id.0 + 1);
    loader.scene.set_next_id(next);
    let top = loader.scene.layers().iter().map(|l| l.id.0 + 1).max();
    loader.scene.set_next_id(top.unwrap_or(1));
    Ok((loader.scene, loader.report))
}

/// One file reference of a scene (CORE.md §6.5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalRef {
    pub node: NodeId,
    pub param: String,
    pub path: String,
}

/// Every non-empty file reference param, in document order (tags before children).
pub fn external_refs(scene: &Scene) -> Vec<ExternalRef> {
    fn walk(scene: &Scene, id: NodeId, out: &mut Vec<ExternalRef>) {
        let Some(n) = scene.node(id) else { return };
        if let Some(ty) = scene.registry().get(&n.type_id) {
            for desc in ty.params.iter().filter(|d| d.ui.file_ref) {
                if let Some(Value::String(s)) = n.params.get(&desc.id)
                    && !s.is_empty()
                {
                    out.push(ExternalRef {
                        node: id,
                        param: desc.id.clone(),
                        path: s.clone(),
                    });
                }
            }
        }
        for t in &n.tags {
            walk(scene, *t, out);
        }
        for c in &n.children {
            walk(scene, *c, out);
        }
    }
    let mut out = Vec::new();
    for r in scene.roots() {
        walk(scene, *r, &mut out);
    }
    out
}

// --- paths ---

/// Removes `.` and `x/..` segments without touching the file system.
fn normalize(p: &Path) -> PathBuf {
    let mut out: Vec<Component> = Vec::new();
    for c in p.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => match out.last() {
                Some(Component::Normal(_)) => {
                    out.pop();
                }
                Some(Component::RootDir | Component::Prefix(_)) => {}
                _ => out.push(c),
            },
            _ => out.push(c),
        }
    }
    out.iter().map(|c| c.as_os_str()).collect()
}

/// `path` relative to `base` with `/` separators; `None` unless both are absolute and share
/// a root. Relative `path` values are returned unchanged by the caller.
fn relativize(path: &str, base: &Path) -> Option<String> {
    let path = Path::new(path);
    if !path.is_absolute() || !base.is_absolute() {
        return None;
    }
    let (p, b) = (normalize(path), normalize(base));
    let (p, b): (Vec<_>, Vec<_>) = (p.components().collect(), b.components().collect());
    let common = p.iter().zip(&b).take_while(|(x, y)| x == y).count();
    if common == 0 {
        return None;
    }
    let mut parts: Vec<String> = vec!["..".to_string(); b.len() - common];
    parts.extend(
        p[common..]
            .iter()
            .map(|c| c.as_os_str().to_string_lossy().into_owned()),
    );
    Some(if parts.is_empty() {
        ".".to_string()
    } else {
        parts.join("/")
    })
}

fn resolve(rel: &str, base: &Path) -> Option<String> {
    let rel = Path::new(rel);
    // `has_root` also catches drive-less rooted paths such as `/abs.png` on Windows.
    if rel.is_absolute() || rel.has_root() || !base.is_absolute() {
        return None;
    }
    Some(to_slash(&normalize(&base.join(rel))))
}

/// A path with `/` separators on every OS (Windows accepts them), so stored and resolved
/// paths compare equal across platforms. A Windows prefix such as `C:` is kept.
fn to_slash(path: &Path) -> String {
    use std::path::Component;
    let mut prefix = String::new();
    let mut rooted = false;
    let mut parts = Vec::new();
    for c in path.components() {
        match c {
            Component::Prefix(p) => prefix = p.as_os_str().to_string_lossy().into_owned(),
            Component::RootDir => rooted = true,
            other => parts.push(other.as_os_str().to_string_lossy().into_owned()),
        }
    }
    format!(
        "{prefix}{}{}",
        if rooted { "/" } else { "" },
        parts.join("/")
    )
}

// --- chunks ---

fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325_u64, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x0100_0000_01b3)
    })
}

fn encode_mesh(m: &MeshData) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&(m.points.len() as u32).to_le_bytes());
    out.extend_from_slice(&(m.polys.len() as u32).to_le_bytes());
    for c in m.points.iter().flatten() {
        out.extend_from_slice(&c.to_le_bytes());
    }
    for poly in &m.polys {
        out.extend_from_slice(&(poly.len() as u32).to_le_bytes());
        for i in poly {
            out.extend_from_slice(&i.to_le_bytes());
        }
    }
    out
}

fn encode_index_list(l: &[u32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(4 + 4 * l.len());
    out.extend_from_slice(&(l.len() as u32).to_le_bytes());
    for i in l {
        out.extend_from_slice(&i.to_le_bytes());
    }
    out
}

/// Bounds-checked reader over a byte slice.
struct Cursor<'a> {
    bytes: &'a [u8],
}

impl<'a> Cursor<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], LoadError> {
        if n > self.bytes.len() {
            return Err(LoadError::Chunk("data ends too early".into()));
        }
        let (head, tail) = self.bytes.split_at(n);
        self.bytes = tail;
        Ok(head)
    }

    fn u8(&mut self) -> Result<u8, LoadError> {
        Ok(self.take(1)?[0])
    }

    fn u32(&mut self) -> Result<u32, LoadError> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn u64(&mut self) -> Result<u64, LoadError> {
        let b = self.take(8)?;
        let mut a = [0u8; 8];
        a.copy_from_slice(b);
        Ok(u64::from_le_bytes(a))
    }

    fn f64(&mut self) -> Result<f64, LoadError> {
        Ok(f64::from_bits(self.u64()?))
    }

    /// A count of items that each need at least `item` bytes: refuses counts the
    /// remaining bytes cannot hold, before anything is allocated.
    fn count(&mut self, item: usize) -> Result<usize, LoadError> {
        let n = self.u32()? as usize;
        if n.saturating_mul(item) > self.bytes.len() {
            return Err(LoadError::Chunk("count larger than the data".into()));
        }
        Ok(n)
    }
}

struct RawChunk<'a> {
    kind: u8,
    payload: &'a [u8],
}

fn parse_section(section: &[u8]) -> Result<Vec<RawChunk<'_>>, LoadError> {
    let mut c = Cursor { bytes: section };
    let n = c.count(17)?;
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        let kind = c.u8()?;
        if kind != KIND_MESH && kind != KIND_INDEX_LIST {
            return Err(LoadError::Chunk(format!("unknown chunk kind {kind}")));
        }
        let len = c.u64()?;
        let hash = c.u64()?;
        let len = usize::try_from(len)
            .ok()
            .filter(|l| *l <= c.bytes.len())
            .ok_or_else(|| LoadError::Chunk("chunk runs past the end".into()))?;
        let payload = c.take(len)?;
        if fnv1a(payload) != hash {
            return Err(LoadError::Chunk("hash mismatch".into()));
        }
        out.push(RawChunk { kind, payload });
    }
    if !c.bytes.is_empty() {
        return Err(LoadError::Chunk("bytes after the last chunk".into()));
    }
    Ok(out)
}

fn decode_mesh(payload: &[u8]) -> Result<MeshData, LoadError> {
    let mut c = Cursor { bytes: payload };
    let np = c.count(24)?;
    let nq = c.u32()? as usize;
    if np.saturating_mul(24).saturating_add(nq.saturating_mul(4)) > c.bytes.len() {
        return Err(LoadError::Chunk("mesh counts larger than the data".into()));
    }
    let mut points = Vec::with_capacity(np);
    for _ in 0..np {
        points.push([c.f64()?, c.f64()?, c.f64()?]);
    }
    let mut polys = Vec::with_capacity(nq);
    for _ in 0..nq {
        let k = c.count(4)?;
        let mut poly = Vec::with_capacity(k);
        for _ in 0..k {
            poly.push(c.u32()?);
        }
        polys.push(poly);
    }
    if !c.bytes.is_empty() {
        return Err(LoadError::Chunk("bytes after the mesh".into()));
    }
    Ok(MeshData { points, polys })
}

fn decode_index_list(payload: &[u8]) -> Result<Vec<u32>, LoadError> {
    let mut c = Cursor { bytes: payload };
    let n = c.count(4)?;
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        out.push(c.u32()?);
    }
    if !c.bytes.is_empty() {
        return Err(LoadError::Chunk("bytes after the index list".into()));
    }
    Ok(out)
}

#[derive(Clone, Copy)]
enum Place {
    Child(Option<NodeId>),
    Tag(NodeId),
}

struct Loader<'a> {
    scene: Scene,
    report: LoadReport,
    seen: BTreeSet<NodeId>,
    chunks: Vec<RawChunk<'a>>,
    opts: &'a LoadOptions,
}

impl Loader<'_> {
    fn chunk(&self, at: u32, kind: u8) -> Result<&[u8], LoadError> {
        match self.chunks.get(at as usize) {
            None => Err(LoadError::Chunk(format!("chunk {at} does not exist"))),
            Some(c) if c.kind != kind => {
                Err(LoadError::Chunk(format!("chunk {at} has the wrong kind")))
            }
            Some(c) => Ok(c.payload),
        }
    }

    fn value(&self, v: StoredValue) -> Result<Value, LoadError> {
        Ok(match v {
            StoredValue::Bool(x) => Value::Bool(x),
            StoredValue::Int(x) => Value::Int(x),
            StoredValue::Float(x) => Value::Float(x),
            StoredValue::Vec3(x) => Value::Vec3(x),
            StoredValue::Color(x) => Value::Color(x),
            StoredValue::Enum(x) => Value::Enum(x),
            StoredValue::String(x) => Value::String(x),
            StoredValue::Link(x) => Value::Link(x),
            StoredValue::LinkList(x) => Value::LinkList(x),
            StoredValue::Mesh(x) => Value::Mesh(x),
            StoredValue::IndexList(x) => Value::IndexList(x),
            StoredValue::MeshChunk(at) => {
                Value::Mesh(Arc::new(decode_mesh(self.chunk(at, KIND_MESH)?)?))
            }
            StoredValue::IndexListChunk(at) => {
                Value::IndexList(decode_index_list(self.chunk(at, KIND_INDEX_LIST)?)?)
            }
        })
    }

    /// Resolves a relative file reference against the scene folder (CORE.md §6.5).
    fn external(&mut self, id: NodeId, is_ref: bool, value: Value) -> Value {
        let (true, Some(base), Value::String(s)) = (is_ref, &self.opts.base_dir, &value) else {
            return value;
        };
        if s.is_empty() {
            return value;
        }
        let path = resolve(s, base).unwrap_or_else(|| s.clone());
        if self.opts.check_external && !Path::new(&path).exists() {
            self.warn(id, format!("external file {path:?} not found"));
        }
        Value::String(path)
    }

    /// Installs the layer table after the nodes: drops layers whose id repeats or is a node
    /// id, and clears `object.layer` links to layers that do not exist (CORE.md §6.8).
    fn layers(&mut self, list: Vec<Layer>) {
        let mut ids = BTreeSet::new();
        let mut kept = Vec::new();
        for mut l in list {
            if self.seen.contains(&l.id) || !ids.insert(l.id) {
                let msg = format!("layer {} repeats an id; dropped", l.id);
                self.report.warnings.push(msg);
                continue;
            }
            l.name = l.name.trim().to_string();
            if l.name.is_empty() {
                l.name = format!("Layer {}", l.id);
            }
            l.color = l.color.map(|c| {
                if c.is_finite() {
                    c.clamp(0.0, 1.0)
                } else {
                    0.0
                }
            });
            kept.push(l);
        }
        self.scene.init_layers(Layers::from_list(kept));
        let nodes: Vec<NodeId> = self.seen.iter().copied().collect();
        for id in nodes {
            let dangling = matches!(
                self.scene.param(id, layers::LAYER),
                Some(Value::Link(Some(l))) if !self.scene.layers().contains(*l)
            );
            if dangling {
                if let Some(n) = self.scene.node_mut(id) {
                    n.params
                        .insert(layers::LAYER.to_string(), Value::Link(None));
                }
                self.warn(id, "layer link to a missing layer cleared".into());
            }
        }
    }

    fn warn(&mut self, id: NodeId, msg: String) {
        self.report.warnings.push(format!("node {id}: {msg}"));
    }

    fn node(&mut self, r: NodeRecord, place: Place) -> Result<(), LoadError> {
        let id = r.id;
        if !self.seen.insert(id) {
            return Err(LoadError::DuplicateId(id));
        }
        let ty = self.scene.registry().get(&r.type_id).cloned();
        let mut raw = BTreeMap::new();
        for (key, v) in r.params {
            raw.insert(key, self.value(v)?);
        }
        let params = match &ty {
            None => {
                self.warn(id, format!("unknown type {:?} (kept as-is)", r.type_id));
                raw
            }
            Some(ty) => {
                let ok = match place {
                    Place::Child(_) => ty.category.is_hierarchy(),
                    Place::Tag(_) => ty.category == Category::Tag,
                };
                if !ok {
                    return Err(LoadError::Structure(format!(
                        "node {id} of type {:?} ({:?}) cannot be placed here",
                        r.type_id, ty.category
                    )));
                }
                let mut stored = raw;
                let mut params = BTreeMap::new();
                for desc in &ty.params {
                    let value = match stored.remove(&desc.id) {
                        None => desc.default.clone(),
                        Some(v) => match desc.validate(v.clone()) {
                            Ok(valid) => {
                                if valid != v {
                                    self.warn(id, format!("param {:?} clamped", desc.id));
                                }
                                valid
                            }
                            Err(e) => {
                                self.warn(
                                    id,
                                    format!("param {:?}: {e}; reset to default", desc.id),
                                );
                                desc.default.clone()
                            }
                        },
                    };
                    let value = self.external(id, desc.ui.file_ref, value);
                    params.insert(desc.id.clone(), value);
                }
                for unknown in stored.keys() {
                    self.warn(id, format!("unknown param {unknown:?} dropped"));
                }
                params
            }
        };
        let mut tracks = BTreeMap::new();
        for (key, track) in r.tracks {
            let verdict = match &ty {
                None => Ok(()),
                Some(ty) => parse_target(&key, |p| ty.find_param(p))
                    .map(|_| ())
                    .and_then(|()| track.validate()),
            };
            match verdict {
                Ok(()) => {
                    tracks.insert(key, track);
                }
                Err(e) => self.warn(id, format!("track {key:?}: {e}; dropped")),
            }
        }
        let (parent, host) = match place {
            Place::Child(p) => (p, None),
            Place::Tag(h) => (None, Some(h)),
        };
        self.scene.insert_loaded(Node {
            id,
            type_id: r.type_id,
            name: r.name,
            params,
            parent,
            children: r.children.iter().map(|c| c.id).collect(),
            tags: r.tags.iter().map(|t| t.id).collect(),
            host,
            tracks,
            rev: 0,
        });
        for t in r.tags {
            self.node(t, Place::Tag(id))?;
        }
        for c in r.children {
            self.node(c, Place::Child(Some(id)))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrate_runs_every_step_in_order_from_the_file_version() {
        use std::sync::Mutex;
        static LOG: Mutex<Vec<u32>> = Mutex::new(Vec::new());
        fn step(n: u32) -> Vec<String> {
            LOG.lock().unwrap().push(n);
            vec![format!("step {n}")]
        }
        let steps: [Migration; 3] = [|_| step(1), |_| step(2), |_| step(3)];
        let mut file = SceneFile {
            format: FORMAT.into(),
            version: 2,
            settings: SceneSettings::default(),
            time: TimeConfig::default(),
            layers: Vec::new(),
            roots: Vec::new(),
        };
        let notes = migrate(&mut file, &steps, 4);
        assert_eq!(file.version, 4);
        assert_eq!(*LOG.lock().unwrap(), vec![2, 3]);
        assert_eq!(notes, ["step 2", "step 3"]);
    }

    #[test]
    fn relative_paths_are_lexical() {
        // An absolute path needs a drive on Windows.
        let abs = |p: &str| {
            if cfg!(windows) {
                format!("C:{p}")
            } else {
                p.to_string()
            }
        };
        let base = abs("/a/b");
        let b = Path::new(&base);
        assert_eq!(
            relativize(&abs("/a/b/c/d.png"), b).as_deref(),
            Some("c/d.png")
        );
        assert_eq!(relativize(&abs("/a/x.png"), b).as_deref(), Some("../x.png"));
        assert_eq!(relativize(&abs("/a/b"), b).as_deref(), Some("."));
        assert_eq!(relativize("rel.png", b), None);
        assert_eq!(resolve("../x.png", b), Some(abs("/a/x.png")));
        assert_eq!(resolve("/abs.png", b), None);
        assert_eq!(resolve(&abs("/abs.png"), b), None);
    }
}
