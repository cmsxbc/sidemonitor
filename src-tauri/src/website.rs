use serde;
use serde_json;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct WebSite {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct WebSiteInfo {
    pub websites: Vec<WebSite>,
    pub default: String,
    pub slider: Option<u64>,
}

impl WebSiteInfo {
    pub fn from_json(path: PathBuf) -> Result<Self, Box<dyn Error>> {
        Ok(serde_json::from_str::<Self>(
            fs::read_to_string(path)?.as_str(),
        )?)
    }
}
