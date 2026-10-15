//! Where the Codex executable is (spec 005, D2): `ROUDEX_CODEX_BIN`, then
//! `codex_path` in `router.toml`, then the `PATH`, then the places each system
//! installs it in. A graphical app on macOS does not inherit the terminal's
//! `PATH`, and the Codex desktop app ships its own copy, so the `PATH` alone is
//! not enough.

use crate::tiers::RouterConfig;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Os {
    Windows,
    MacOs,
    Linux,
}

impl Os {
    pub fn current() -> Self {
        if cfg!(windows) {
            Os::Windows
        } else if cfg!(target_os = "macos") {
            Os::MacOs
        } else {
            Os::Linux
        }
    }

    fn executables(self) -> &'static [&'static str] {
        match self {
            Os::Windows => &["codex.exe", "codex.cmd"],
            Os::MacOs | Os::Linux => &["codex"],
        }
    }
}

/// Inside the Codex app on macOS, by app version, newest layout first.
const MAC_APP_LAYOUTS: [&str; 3] = [
    "Contents/Resources/codex-cli/CodexCLI.app/Contents/MacOS/codex",
    "Contents/Resources/codex-cli/bin/codex",
    "Contents/Resources/codex",
];

/// What a search for Codex looks at. [`CodexSearch::from_env`] fills it from
/// the environment; tests fill it by hand.
#[derive(Debug, Clone)]
pub struct CodexSearch {
    pub os: Os,
    /// `ROUDEX_CODEX_BIN`, for tests and development: used as given.
    pub explicit: Option<PathBuf>,
    /// `codex_path` in `router.toml`.
    pub configured: Option<PathBuf>,
    pub path_env: Option<OsString>,
    pub home: Option<PathBuf>,
    /// `%LOCALAPPDATA%`, where Codex Desktop keeps its copies on Windows.
    pub local_app_data: Option<PathBuf>,
    /// The root that fixed locations such as `/Applications` hang from.
    pub root: PathBuf,
}

/// Codex is nowhere RouDex looked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodexNotFound {
    /// Every place looked at, in order.
    pub tried: Vec<String>,
    /// `codex_path` was set and points to nothing.
    pub configured: Option<PathBuf>,
    pub config_path: PathBuf,
}

impl std::fmt::Display for CodexNotFound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.configured {
            Some(path) => write!(
                f,
                "codex_path in {} points to {}, which is not a file",
                self.config_path.display(),
                path.display()
            ),
            None => write!(
                f,
                "Codex was not found. Looked in: {}. Install Codex or set codex_path in {} to its executable",
                self.tried.join(", "),
                self.config_path.display()
            ),
        }
    }
}

impl std::error::Error for CodexNotFound {}

impl CodexSearch {
    pub fn from_env() -> Self {
        let var = |name: &str| std::env::var_os(name).filter(|value| !value.is_empty());
        let configured = RouterConfig::load_or_default(&RouterConfig::default_path())
            .ok()
            .and_then(|config| config.codex_path);
        let home = if cfg!(windows) {
            var("USERPROFILE").or_else(|| var("HOME"))
        } else {
            var("HOME").or_else(|| var("USERPROFILE"))
        };
        Self {
            os: Os::current(),
            explicit: var("ROUDEX_CODEX_BIN").map(PathBuf::from),
            configured,
            path_env: var("PATH"),
            home: home.map(PathBuf::from),
            local_app_data: var("LOCALAPPDATA").map(PathBuf::from),
            root: PathBuf::from("/"),
        }
    }

    pub fn find(&self) -> Result<PathBuf, CodexNotFound> {
        let not_found = |tried: Vec<String>, configured: Option<PathBuf>| CodexNotFound {
            tried,
            configured,
            config_path: RouterConfig::default_path(),
        };
        if let Some(explicit) = &self.explicit {
            return Ok(explicit.clone());
        }
        if let Some(configured) = &self.configured {
            return if configured.is_file() {
                Ok(configured.clone())
            } else {
                Err(not_found(Vec::new(), Some(configured.clone())))
            };
        }
        let names = self.os.executables();
        if let Some(found) = self
            .path_env
            .iter()
            .flat_map(std::env::split_paths)
            .flat_map(|dir| names.iter().map(move |name| dir.join(name)))
            .find(|candidate| candidate.is_file())
        {
            return Ok(found);
        }
        let mut tried = vec!["PATH".to_owned()];
        if self.os == Os::Windows {
            if let Some(found) = self.newest_desktop_copy() {
                return Ok(found);
            }
            if let Some(root) = &self.local_app_data {
                tried.push(
                    desktop_bin(root)
                        .join("*")
                        .join(names[0])
                        .display()
                        .to_string(),
                );
            }
        }
        for candidate in self.known_locations() {
            if candidate.is_file() {
                return Ok(candidate);
            }
            tried.push(candidate.display().to_string());
        }
        Err(not_found(tried, None))
    }

    /// Fixed places per system, in the order they are tried.
    fn known_locations(&self) -> Vec<PathBuf> {
        let home = |relative: &str| self.home.as_ref().map(|home| home.join(relative));
        let root = |relative: &str| self.root.join(relative);
        match self.os {
            Os::Windows => Vec::new(),
            Os::MacOs => {
                let apps = [Some(root("Applications")), home("Applications")];
                let bundled = apps.into_iter().flatten().flat_map(|apps| {
                    MAC_APP_LAYOUTS.map(|layout| apps.join("Codex.app").join(layout))
                });
                bundled
                    .chain([root("opt/homebrew/bin/codex"), root("usr/local/bin/codex")])
                    .chain(home(".local/bin/codex"))
                    .collect()
            }
            Os::Linux => home(".local/bin/codex")
                .into_iter()
                .chain([root("usr/local/bin/codex")])
                .chain(home(".npm-global/bin/codex"))
                .collect(),
        }
    }

    /// Codex Desktop on Windows keeps a copy per version in a folder named
    /// after a hash, so the newest one wins.
    fn newest_desktop_copy(&self) -> Option<PathBuf> {
        let bin = desktop_bin(self.local_app_data.as_ref()?);
        std::fs::read_dir(bin)
            .ok()?
            .filter_map(Result::ok)
            .map(|entry| entry.path().join(self.os.executables()[0]))
            .filter_map(|candidate| {
                let modified = candidate.metadata().ok()?.modified().ok()?;
                Some((modified, candidate))
            })
            .max()
            .map(|(_, candidate)| candidate)
    }
}

fn desktop_bin(local_app_data: &Path) -> PathBuf {
    local_app_data.join("OpenAI").join("Codex").join("bin")
}

/// The program to start: what the search finds, or plain `codex` so that
/// starting it fails with [`start_error`]'s message.
pub fn program() -> PathBuf {
    CodexSearch::from_env()
        .find()
        .unwrap_or_else(|_| PathBuf::from("codex"))
}

/// Why starting Codex failed. A missing executable says where RouDex looked
/// and how to point it at Codex instead of a bare "file not found".
pub fn start_error(error: &std::io::Error) -> String {
    if error.kind() == std::io::ErrorKind::NotFound {
        if let Err(missing) = CodexSearch::from_env().find() {
            return missing.to_string();
        }
    }
    error.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, SystemTime};

    struct Layout(PathBuf);

    impl Layout {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!("roudex-codex-{}", uuid::Uuid::new_v4()));
            std::fs::create_dir_all(&root).unwrap();
            Self(root)
        }

        /// An executable file at `relative`, `age` seconds old.
        fn file(&self, relative: &str, age: u64) -> PathBuf {
            let path = self.0.join(relative);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            let file = std::fs::File::create(&path).unwrap();
            file.set_modified(SystemTime::now() - Duration::from_secs(age))
                .unwrap();
            path
        }

        fn search(&self, os: Os) -> CodexSearch {
            CodexSearch {
                os,
                explicit: None,
                configured: None,
                path_env: None,
                home: Some(self.0.join("home")),
                local_app_data: Some(self.0.join("local")),
                root: self.0.join("root"),
            }
        }
    }

    impl Drop for Layout {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn windows_uses_the_path_then_the_newest_desktop_copy() {
        let layout = Layout::new();
        let mut search = layout.search(Os::Windows);
        layout.file("local/OpenAI/Codex/bin/old-hash/codex.exe", 3600);
        let newest = layout.file("local/OpenAI/Codex/bin/new-hash/codex.exe", 10);
        assert_eq!(search.find().unwrap(), newest);

        let on_path = layout.file("tools/codex.cmd", 7200);
        search.path_env =
            Some(std::env::join_paths([layout.0.join("missing"), layout.0.join("tools")]).unwrap());
        assert_eq!(search.find().unwrap(), on_path);
    }

    #[test]
    fn macos_finds_the_codex_app_in_every_known_layout() {
        for (index, layout_path) in MAC_APP_LAYOUTS.iter().enumerate() {
            let layout = Layout::new();
            let search = layout.search(Os::MacOs);
            // An older layout next to a newer one: the newer layout wins.
            for older in &MAC_APP_LAYOUTS[index..] {
                layout.file(&format!("root/Applications/Codex.app/{older}"), 0);
            }
            let expected = layout
                .0
                .join("root/Applications/Codex.app")
                .join(layout_path);
            assert_eq!(search.find().unwrap(), expected, "{layout_path}");
        }

        let layout = Layout::new();
        let in_home = layout.file("home/Applications/Codex.app/Contents/Resources/codex", 0);
        assert_eq!(layout.search(Os::MacOs).find().unwrap(), in_home);
        let layout = Layout::new();
        let brew = layout.file("root/opt/homebrew/bin/codex", 0);
        layout.file("home/.local/bin/codex", 0);
        assert_eq!(layout.search(Os::MacOs).find().unwrap(), brew);
    }

    #[test]
    fn linux_looks_in_the_usual_user_and_system_folders() {
        let layout = Layout::new();
        let npm = layout.file("home/.npm-global/bin/codex", 0);
        assert_eq!(layout.search(Os::Linux).find().unwrap(), npm);
        let system = layout.file("root/usr/local/bin/codex", 0);
        assert_eq!(layout.search(Os::Linux).find().unwrap(), system);
        let local = layout.file("home/.local/bin/codex", 0);
        assert_eq!(layout.search(Os::Linux).find().unwrap(), local);
    }

    #[test]
    fn the_override_and_codex_path_come_first() {
        let layout = Layout::new();
        layout.file("home/.local/bin/codex", 0);
        let configured = layout.file("custom/codex", 0);
        let mut search = layout.search(Os::Linux);
        search.configured = Some(configured.clone());
        assert_eq!(search.find().unwrap(), configured);
        search.explicit = Some(PathBuf::from("from-env"));
        assert_eq!(search.find().unwrap(), PathBuf::from("from-env"));

        search.explicit = None;
        search.configured = Some(layout.0.join("custom/missing"));
        let error = search.find().unwrap_err();
        assert!(error.to_string().contains("codex_path"), "{error}");
        assert!(error.to_string().contains("not a file"), "{error}");
    }

    #[test]
    fn a_missing_codex_lists_every_place_tried() {
        let layout = Layout::new();
        for os in [Os::Windows, Os::MacOs, Os::Linux] {
            let error = layout.search(os).find().unwrap_err();
            let message = error.to_string();
            assert!(message.starts_with("Codex was not found"), "{message}");
            assert!(message.contains("PATH"), "{message}");
            assert!(message.contains("codex_path"), "{message}");
            match os {
                Os::Windows => assert!(message.contains("OpenAI"), "{message}"),
                Os::MacOs => assert!(message.contains("Codex.app"), "{message}"),
                Os::Linux => assert!(message.contains(".npm-global"), "{message}"),
            }
        }
    }
}
