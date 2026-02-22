//! YAML config source backed by [`FileSource`].

use std::path::PathBuf;

use super::file_source::FileSource;
use crate::config::model::Config;

#[must_use]
pub fn new(path: PathBuf) -> FileSource {
    FileSource::new(path, "yaml", |content| {
        serde_yml::from_str::<Config>(content)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    })
}
