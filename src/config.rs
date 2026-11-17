use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, ensure};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub icons_dir: PathBuf,
    pub apps: BTreeMap<PathBuf, PathBuf>,
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let text = fs::read_to_string(path)
            .with_context(|| format!("could not read {}", path.display()))?;
        toml::from_str(&text).with_context(|| format!("could not parse {}", path.display()))
    }

    pub fn icons_dir(&self, config_path: &Path) -> Result<PathBuf> {
        let dir = expand_home(&self.icons_dir)?;
        ensure!(!dir.as_os_str().is_empty(), "icons_dir is empty");
        let parent = config_path.parent().unwrap_or(Path::new(""));
        Ok(parent.join(dir))
    }
}

pub fn app_path(path: &Path) -> Result<PathBuf> {
    let resolved = expand_home(path)?;
    ensure!(
        resolved.is_absolute(),
        "application path must be absolute or start with ~/"
    );
    let resolved = fs::canonicalize(&resolved)
        .with_context(|| format!("could not resolve {}", resolved.display()))?;
    ensure!(
        resolved
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("app"))
            && resolved.join("Contents/Info.plist").is_file(),
        "expected a macOS .app bundle at {}",
        resolved.display()
    );
    ensure!(resolved.to_str().is_some(), "application path is not UTF-8");
    Ok(resolved)
}

fn expand_home(path: &Path) -> Result<PathBuf> {
    match path.strip_prefix("~") {
        Ok(relative) => {
            let home = env::var_os("HOME").context("HOME is not set")?;
            let home = PathBuf::from(home);
            ensure!(home.is_absolute(), "HOME is not an absolute path");
            Ok(home.join(relative))
        }
        Err(_) => Ok(path.to_owned()),
    }
}
