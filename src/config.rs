use anyhow::Context;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct ClientConfig {
    pub email: String,
    pub api_token: String,
    pub api_key: String,
}

pub fn get_config_path() -> anyhow::Result<std::path::PathBuf> {
    Ok(ProjectDirs::from("", "", "cloudflare-api-client")
        .context("Failed to get project directories")?
        .config_dir()
        .join("config.toml"))
}

pub fn load_config() -> anyhow::Result<Option<ClientConfig>> {
    let config_path = get_config_path()?;

    if !config_path.exists() || !config_path.is_file() {
        return Ok(None);
    }

    let config_content = std::fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read config at {config_path:?}"))?;

    let config: ClientConfig = toml::from_str(&config_content)
        .with_context(|| format!("Failed to deserialize config at {config_path:?}"))?;

    Ok(Some(config))
}
