use std::error::Error;
use std::path::Path;

use anyhow::anyhow;
use tokio::fs::read_to_string;

use crate::app::Config;

pub(crate) async fn load_config(path: &Path) -> Result<Config, Box<dyn Error>> {
  let contents = read_to_string(path)
    .await
    .map_err(|e| format!("'{}': {}", path.to_string_lossy(), e))?;

  let config: Config = match path.extension().unwrap().to_str().unwrap() {
    "json" => serde_json::from_str(&contents)?,
    "yaml" => serde_yaml::from_str(&contents)?,
    "toml" => toml::from_str(&contents)?,
    _ => Err(anyhow!("unknown config format"))?,
  };

  Ok(config)
}
