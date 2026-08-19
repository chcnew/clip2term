use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub save_dir: PathBuf,
    pub keep_last: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            save_dir: app_dir().join("shots"),
            keep_last: 50,
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let path = config_path();
        if !path.exists() {
            let cfg = Config::default();
            cfg.save()?;
            return Ok(cfg);
        }
        let text = fs::read_to_string(&path)
            .with_context(|| format!("读取配置失败: {}", path.display()))?;
        let mut cfg: Config = toml::from_str(&text).context("解析 config.toml 失败")?;
        if cfg.keep_last == 0 {
            cfg.keep_last = 50;
        }
        if cfg.save_dir.as_os_str().is_empty() {
            cfg.save_dir = Config::default().save_dir;
        }
        Ok(cfg)
    }

    pub fn save(&self) -> Result<()> {
        let path = config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let text = toml::to_string_pretty(self)?;
        fs::write(&path, text)?;
        Ok(())
    }
}

pub fn app_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("clip2term")
}

pub fn config_path() -> PathBuf {
    app_dir().join("config.toml")
}
