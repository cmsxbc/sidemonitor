use serde;
use serde_json;
use std::collections;
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
        let wi = serde_json::from_str::<Self>(fs::read_to_string(path)?.as_str())?;
        let mut names = collections::HashSet::new();
        let mut has_default = false;
        for website in wi.websites.clone().into_iter() {
            if website.name == wi.default {
                has_default = true
            }
            if !names.contains(&website.name) {
                names.insert(website.name);
            } else {
                return Err(format!("Duplicate names: {}", website.name).into());
            }
        }
        if has_default {
            Ok(wi)
        } else {
            Err(format!("default: {} does not exist", wi.default).into())
        }
    }
}
