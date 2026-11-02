//! Per-OS config, data and cache directories and the shared settings file
//! (docs/specs/PLATFORM.md).
//!
//! The rules live in pure functions ([`resolve`], [`settings_path_with`],
//! [`migrate_legacy_settings`]) that take the OS, an environment lookup and the base
//! folders, so every OS branch is testable on every host. [`dirs`] and [`settings_path`]
//! feed them the real process environment and `directories::BaseDirs`.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// The directory name below each OS base folder.
pub const APP_DIR: &str = "forge3d";
/// File name of the shared settings file (RPC.md §2).
pub const SETTINGS_FILE: &str = "settings.json";

/// The operating systems with their own layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Os {
    Linux,
    MacOs,
    Windows,
}

impl Os {
    /// The OS this binary was built for.
    pub fn current() -> Os {
        if cfg!(target_os = "windows") {
            Os::Windows
        } else if cfg!(target_os = "macos") {
            Os::MacOs
        } else {
            Os::Linux
        }
    }
}

/// The OS-level base folders (what `directories::BaseDirs` reports).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaseFolders {
    pub home: PathBuf,
    pub config: PathBuf,
    pub data: PathBuf,
    pub cache: PathBuf,
}

impl BaseFolders {
    /// From the OS, via the `directories` crate; `None` without a home folder.
    pub fn from_system() -> Option<BaseFolders> {
        let b = directories::BaseDirs::new()?;
        Some(BaseFolders {
            home: b.home_dir().to_path_buf(),
            config: b.config_dir().to_path_buf(),
            data: b.data_dir().to_path_buf(),
            cache: b.cache_dir().to_path_buf(),
        })
    }
}

/// The application's directories.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dirs {
    pub config: PathBuf,
    pub data: PathBuf,
    pub cache: PathBuf,
}

impl Dirs {
    /// `<config>/settings.json`.
    pub fn settings_file(&self) -> PathBuf {
        self.config.join(SETTINGS_FILE)
    }
}

/// An `XDG_*` value counts when non-empty and absolute (XDG Base Directory spec).
fn xdg(env: &dyn Fn(&str) -> Option<OsString>, name: &str) -> Option<PathBuf> {
    env(name)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
}

/// The application directories for `os`. Linux honours `XDG_CONFIG_HOME`, `XDG_DATA_HOME`
/// and `XDG_CACHE_HOME` over `base`; the other systems use `base` as is.
pub fn resolve(os: Os, env: &dyn Fn(&str) -> Option<OsString>, base: &BaseFolders) -> Dirs {
    match os {
        Os::Linux => {
            let pick = |var: &str, fallback: &Path| {
                xdg(env, var)
                    .unwrap_or_else(|| fallback.to_path_buf())
                    .join(APP_DIR)
            };
            Dirs {
                config: pick("XDG_CONFIG_HOME", &base.home.join(".config")),
                data: pick("XDG_DATA_HOME", &base.home.join(".local").join("share")),
                cache: pick("XDG_CACHE_HOME", &base.home.join(".cache")),
            }
        }
        Os::MacOs => Dirs {
            config: base.config.join(APP_DIR),
            data: base.data.join(APP_DIR),
            cache: base.cache.join(APP_DIR),
        },
        Os::Windows => Dirs {
            config: base.config.join(APP_DIR),
            data: base.data.join(APP_DIR),
            cache: base.cache.join(APP_DIR).join("cache"),
        },
    }
}

/// The settings file: `FORGE_SETTINGS` if set, else `<config>/settings.json`.
pub fn settings_path_with(
    os: Os,
    env: &dyn Fn(&str) -> Option<OsString>,
    base: Option<&BaseFolders>,
) -> Option<PathBuf> {
    if let Some(p) = env("FORGE_SETTINGS") {
        return Some(PathBuf::from(p));
    }
    Some(resolve(os, env, base?).settings_file())
}

/// Where every OS kept the settings file before brief 175:
/// `$XDG_CONFIG_HOME/forge3d/settings.json`, else `<home>/.config/forge3d/settings.json`.
pub fn legacy_settings_path(env: &dyn Fn(&str) -> Option<OsString>, home: &Path) -> PathBuf {
    xdg(env, "XDG_CONFIG_HOME")
        .unwrap_or_else(|| home.join(".config"))
        .join(APP_DIR)
        .join(SETTINGS_FILE)
}

/// Copies the legacy settings file to `new` when `new` is missing and `legacy` exists.
/// Returns true when it copied. Nothing to do when both are the same path. The legacy file
/// stays so an older build keeps working. Errors mean "not migrated".
pub fn migrate_legacy_settings(legacy: &Path, new: &Path) -> bool {
    if legacy == new || new.exists() || !legacy.is_file() {
        return false;
    }
    if let Some(parent) = new.parent()
        && std::fs::create_dir_all(parent).is_err()
    {
        return false;
    }
    std::fs::copy(legacy, new).is_ok()
}

fn process_env(name: &str) -> Option<OsString> {
    std::env::var_os(name)
}

/// The application directories of this process (`None` without a home folder).
pub fn dirs() -> Option<Dirs> {
    Some(resolve(
        Os::current(),
        &process_env,
        &BaseFolders::from_system()?,
    ))
}

/// The settings file of this process, migrating a pre-175 file on first use (silently).
pub fn settings_path() -> Option<PathBuf> {
    let base = BaseFolders::from_system();
    let path = settings_path_with(Os::current(), &process_env, base.as_ref())?;
    if process_env("FORGE_SETTINGS").is_none()
        && let Some(base) = &base
    {
        migrate_legacy_settings(&legacy_settings_path(&process_env, &base.home), &path);
    }
    Some(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn env_of(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<OsString> {
        let m: HashMap<String, OsString> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), OsString::from(v)))
            .collect();
        move |k| m.get(k).cloned()
    }

    fn p(s: &str) -> PathBuf {
        PathBuf::from(s)
    }

    fn linux_base() -> BaseFolders {
        BaseFolders {
            home: p("/home/u"),
            config: p("/home/u/.config"),
            data: p("/home/u/.local/share"),
            cache: p("/home/u/.cache"),
        }
    }

    fn mac_base() -> BaseFolders {
        BaseFolders {
            home: p("/Users/u"),
            config: p("/Users/u/Library/Application Support"),
            data: p("/Users/u/Library/Application Support"),
            cache: p("/Users/u/Library/Caches"),
        }
    }

    fn win_base() -> BaseFolders {
        BaseFolders {
            home: p("/C/Users/u"),
            config: p("/C/Users/u/AppData/Roaming"),
            data: p("/C/Users/u/AppData/Roaming"),
            cache: p("/C/Users/u/AppData/Local"),
        }
    }

    #[test]
    fn linux_defaults() {
        let d = resolve(Os::Linux, &env_of(&[]), &linux_base());
        assert_eq!(d.config, p("/home/u/.config/forge3d"));
        assert_eq!(d.data, p("/home/u/.local/share/forge3d"));
        assert_eq!(d.cache, p("/home/u/.cache/forge3d"));
        assert_eq!(
            d.settings_file(),
            p("/home/u/.config/forge3d/settings.json")
        );
    }

    #[test]
    fn linux_xdg_overrides() {
        let env = env_of(&[
            ("XDG_CONFIG_HOME", "/x/cfg"),
            ("XDG_DATA_HOME", "/x/data"),
            ("XDG_CACHE_HOME", "/x/cache"),
        ]);
        let d = resolve(Os::Linux, &env, &linux_base());
        assert_eq!(d.config, p("/x/cfg/forge3d"));
        assert_eq!(d.data, p("/x/data/forge3d"));
        assert_eq!(d.cache, p("/x/cache/forge3d"));
    }

    #[test]
    fn linux_ignores_empty_and_relative_xdg() {
        let env = env_of(&[("XDG_CONFIG_HOME", ""), ("XDG_DATA_HOME", "rel/data")]);
        let d = resolve(Os::Linux, &env, &linux_base());
        assert_eq!(d.config, p("/home/u/.config/forge3d"));
        assert_eq!(d.data, p("/home/u/.local/share/forge3d"));
    }

    #[test]
    fn macos_layout_ignores_xdg() {
        let env = env_of(&[("XDG_CONFIG_HOME", "/x/cfg")]);
        let d = resolve(Os::MacOs, &env, &mac_base());
        assert_eq!(d.config, p("/Users/u/Library/Application Support/forge3d"));
        assert_eq!(d.data, p("/Users/u/Library/Application Support/forge3d"));
        assert_eq!(d.cache, p("/Users/u/Library/Caches/forge3d"));
    }

    #[test]
    fn windows_layout() {
        let d = resolve(Os::Windows, &env_of(&[]), &win_base());
        assert_eq!(d.config, p("/C/Users/u/AppData/Roaming/forge3d"));
        assert_eq!(d.data, p("/C/Users/u/AppData/Roaming/forge3d"));
        assert_eq!(d.cache, p("/C/Users/u/AppData/Local/forge3d/cache"));
    }

    #[test]
    fn forge_settings_beats_everything() {
        let env = env_of(&[("FORGE_SETTINGS", "/my/s.json"), ("XDG_CONFIG_HOME", "/x")]);
        for (os, base) in [
            (Os::Linux, linux_base()),
            (Os::MacOs, mac_base()),
            (Os::Windows, win_base()),
        ] {
            assert_eq!(
                settings_path_with(os, &env, Some(&base)),
                Some(p("/my/s.json"))
            );
        }
        // even without a home folder
        assert_eq!(
            settings_path_with(Os::Linux, &env, None),
            Some(p("/my/s.json"))
        );
    }

    #[test]
    fn xdg_beats_default_and_no_home_is_none() {
        let env = env_of(&[("XDG_CONFIG_HOME", "/x")]);
        assert_eq!(
            settings_path_with(Os::Linux, &env, Some(&linux_base())),
            Some(p("/x/forge3d/settings.json"))
        );
        assert_eq!(settings_path_with(Os::Linux, &env_of(&[]), None), None);
    }

    #[test]
    fn legacy_path_follows_the_old_rule() {
        assert_eq!(
            legacy_settings_path(&env_of(&[]), &p("/Users/u")),
            p("/Users/u/.config/forge3d/settings.json")
        );
        assert_eq!(
            legacy_settings_path(&env_of(&[("XDG_CONFIG_HOME", "/x")]), &p("/Users/u")),
            p("/x/forge3d/settings.json")
        );
    }

    #[test]
    fn migration_copies_once_and_keeps_the_old_file() {
        let t = tempfile::tempdir().unwrap();
        let legacy = t.path().join("old/forge3d/settings.json");
        let new = t.path().join("new/forge3d/settings.json");
        std::fs::create_dir_all(legacy.parent().unwrap()).unwrap();
        std::fs::write(&legacy, r#"{"rpc":{"port":9}}"#).unwrap();
        assert!(migrate_legacy_settings(&legacy, &new));
        assert_eq!(
            std::fs::read_to_string(&new).unwrap(),
            r#"{"rpc":{"port":9}}"#
        );
        assert!(legacy.exists());
        // a newer file is never overwritten
        std::fs::write(&new, "{}").unwrap();
        assert!(!migrate_legacy_settings(&legacy, &new));
        assert_eq!(std::fs::read_to_string(&new).unwrap(), "{}");
    }

    #[test]
    fn migration_no_ops() {
        let t = tempfile::tempdir().unwrap();
        let a = t.path().join("a.json");
        std::fs::write(&a, "{}").unwrap();
        // same path (Linux): nothing to do
        assert!(!migrate_legacy_settings(&a, &a));
        // no legacy file
        assert!(!migrate_legacy_settings(
            &t.path().join("none.json"),
            &t.path().join("n.json")
        ));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn this_os_is_linux_and_matches_xdg() {
        assert_eq!(Os::current(), Os::Linux);
        let base = BaseFolders::from_system().expect("home");
        let d = resolve(Os::current(), &env_of(&[]), &base);
        assert!(
            d.config.ends_with(".config/forge3d") || std::env::var_os("XDG_CONFIG_HOME").is_some()
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn this_os_is_macos() {
        assert_eq!(Os::current(), Os::MacOs);
        let d = dirs().expect("home");
        assert!(d.config.ends_with("Library/Application Support/forge3d"));
        assert!(d.cache.ends_with("Library/Caches/forge3d"));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn this_os_is_windows() {
        assert_eq!(Os::current(), Os::Windows);
        let d = dirs().expect("home");
        assert!(d.config.ends_with("forge3d"));
        assert!(d.config.to_string_lossy().contains("AppData"));
    }
}
