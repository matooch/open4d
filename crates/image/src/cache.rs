//! Texture cache (docs/specs/IMAGE.md §5).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use crate::buffer::Image;
use crate::colour::{AlphaMode, ColorSpace};
use crate::decode::{ImageInfo, Limits, decode};
use crate::error::ImageError;
use crate::mip::MipChain;

/// How a texture is loaded; part of the cache key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LoadOptions {
    pub tag: ColorSpace,
    /// Alpha mode of the returned pixels.
    pub alpha: AlphaMode,
    /// Override of how the file's alpha is read (`None` = the format default).
    pub source_alpha: Option<AlphaMode>,
    pub mips: bool,
}

impl Default for LoadOptions {
    fn default() -> Self {
        LoadOptions {
            tag: ColorSpace::Srgb,
            alpha: AlphaMode::Straight,
            source_alpha: None,
            mips: true,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Texture {
    /// Level 0 is the full image; a single level when mips are off.
    pub levels: Vec<Image>,
    pub tag: ColorSpace,
    pub alpha: AlphaMode,
    pub info: ImageInfo,
    pub path: PathBuf,
}

impl Texture {
    pub fn bytes(&self) -> usize {
        self.levels.iter().map(Image::bytes).sum()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub invalidations: u64,
    pub bytes: usize,
    pub entries: usize,
}

#[derive(Clone, PartialEq, Eq, Hash)]
struct Key {
    path: PathBuf,
    opts: LoadOptions,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct Stamp {
    mtime: Option<SystemTime>,
    size: u64,
}

struct Entry {
    stamp: Stamp,
    tex: Arc<Texture>,
    bytes: usize,
    tick: u64,
}

struct Inner {
    map: HashMap<Key, Entry>,
    budget: usize,
    bytes: usize,
    tick: u64,
    stats: CacheStats,
}

impl Inner {
    fn evict_to_budget(&mut self) {
        while self.bytes > self.budget {
            let Some(k) = self
                .map
                .iter()
                .min_by_key(|(_, e)| e.tick)
                .map(|(k, _)| k.clone())
            else {
                break;
            };
            if let Some(e) = self.map.remove(&k) {
                self.bytes -= e.bytes;
                self.stats.evictions += 1;
            }
        }
    }
}

/// Thread-safe LRU cache of decoded textures with mip chains.
pub struct TextureCache {
    inner: Mutex<Inner>,
    limits: Limits,
}

fn resolve(path: &Path) -> PathBuf {
    std::fs::canonicalize(path)
        .or_else(|_| std::path::absolute(path))
        .unwrap_or_else(|_| path.to_path_buf())
}

impl TextureCache {
    pub fn new(budget_bytes: usize) -> TextureCache {
        TextureCache {
            inner: Mutex::new(Inner {
                map: HashMap::new(),
                budget: budget_bytes,
                bytes: 0,
                tick: 0,
                stats: CacheStats::default(),
            }),
            limits: Limits::default(),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn stats(&self) -> CacheStats {
        let g = self.lock();
        CacheStats {
            bytes: g.bytes,
            entries: g.map.len(),
            ..g.stats
        }
    }

    pub fn set_budget(&self, budget_bytes: usize) {
        let mut g = self.lock();
        g.budget = budget_bytes;
        g.evict_to_budget();
    }

    pub fn clear(&self) {
        let mut g = self.lock();
        g.map.clear();
        g.bytes = 0;
    }

    /// For a `ParamDesc::file_path` value: an empty string means "no texture".
    pub fn get_ref(
        &self,
        param_value: &str,
        opts: &LoadOptions,
    ) -> Result<Option<Arc<Texture>>, ImageError> {
        if param_value.is_empty() {
            return Ok(None);
        }
        self.get(Path::new(param_value), opts).map(Some)
    }

    pub fn get(&self, path: &Path, opts: &LoadOptions) -> Result<Arc<Texture>, ImageError> {
        let path = resolve(path);
        let key = Key { path, opts: *opts };
        let meta = match std::fs::metadata(&key.path) {
            Ok(m) => m,
            Err(e) => {
                let mut g = self.lock();
                let stale: Vec<Key> = g
                    .map
                    .keys()
                    .filter(|k| k.path == key.path)
                    .cloned()
                    .collect();
                for k in stale {
                    if let Some(e) = g.map.remove(&k) {
                        g.bytes -= e.bytes;
                        g.stats.invalidations += 1;
                    }
                }
                return Err(e.into());
            }
        };
        let stamp = Stamp {
            mtime: meta.modified().ok(),
            size: meta.len(),
        };
        {
            let mut g = self.lock();
            g.tick += 1;
            let tick = g.tick;
            match g.map.get_mut(&key) {
                Some(e) if e.stamp == stamp => {
                    e.tick = tick;
                    let t = e.tex.clone();
                    g.stats.hits += 1;
                    return Ok(t);
                }
                Some(_) => {
                    if let Some(e) = g.map.remove(&key) {
                        g.bytes -= e.bytes;
                    }
                    g.stats.invalidations += 1;
                }
                None => {}
            }
            g.stats.misses += 1;
        }
        // decode outside the lock
        let bytes = std::fs::read(&key.path)?;
        let tex = Arc::new(self.load(&bytes, &key)?);
        let cost = tex.bytes();
        let mut g = self.lock();
        if cost > g.budget {
            return Ok(tex);
        }
        g.tick += 1;
        let tick = g.tick;
        if let Some(e) = g.map.get_mut(&key)
            && e.stamp == stamp
        {
            // another thread finished first: share its result
            e.tick = tick;
            return Ok(e.tex.clone());
        }
        if let Some(old) = g.map.remove(&key) {
            g.bytes -= old.bytes;
        }
        g.bytes += cost;
        g.map.insert(
            key,
            Entry {
                stamp,
                tex: tex.clone(),
                bytes: cost,
                tick,
            },
        );
        g.evict_to_budget();
        Ok(tex)
    }

    fn load(&self, bytes: &[u8], key: &Key) -> Result<Texture, ImageError> {
        let mut dec = decode(bytes, &self.limits)?;
        if let Some(m) = key.opts.source_alpha {
            dec = dec.with_source_alpha(m);
        }
        let info = dec.info;
        let img = dec.into_working(key.opts.tag, key.opts.alpha);
        let alpha = img.alpha;
        let levels = if key.opts.mips {
            MipChain::build(&img).levels
        } else {
            vec![img]
        };
        Ok(Texture {
            levels,
            tag: key.opts.tag,
            alpha,
            info,
            path: key.path.clone(),
        })
    }
}
