use std::path::PathBuf;

use crate::config::Config;

pub struct AppState {
    pub enabled: bool,
    pub last_path: Option<PathBuf>,
    pub config: Config,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        Self {
            enabled: true,
            last_path: None,
            config,
        }
    }
}
