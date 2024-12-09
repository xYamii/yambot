use serde::{ Deserialize, Serialize };
use std::fs;
use std::path::Path;

use crate::ui::{ ChatbotConfig, Config };

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppConfig {
    pub chatbot: ChatbotConfig,
    pub sfx: Config,
    pub tts: Config,
}

impl AppConfig {
    fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let config: AppConfig = toml::from_str(&content)?;
        Ok(config)
    }

    fn to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error>> {
        let content = toml::to_string(self)?;
        fs::write(path, content)?;
        Ok(())
    }
}

pub fn load_config() -> AppConfig {
    let project_root = project_root::get_project_root().unwrap();
    let config_path = project_root.join("config.toml");
    let config: AppConfig = AppConfig::from_file(config_path).unwrap();

    return config;
}

pub fn save_config(config: &AppConfig) {
    let project_root = project_root::get_project_root().unwrap();
    let config_path = project_root.join("config.toml");
    config.to_file(config_path).unwrap();
}
