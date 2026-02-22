//! Serde data structures for the Switchboard configuration file.
//!
//! Contains [`Config`] (the root), [`Route`], [`Target`], [`Defaults`],
//! and [`HeaderRules`]. All types derive `Serialize` and `Deserialize`
//! with `deny_unknown_fields` for strict parsing.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

const fn default_timeout() -> u64 {
    5000
}

const fn default_true() -> bool {
    true
}

fn default_methods() -> Vec<String> {
    vec!["*".to_string()]
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub defaults: Defaults,

    pub routes: Vec<Route>,
}

impl Config {
    #[must_use]
    pub fn total_targets(&self) -> usize {
        self.routes.iter().map(|r| r.targets.len()).sum()
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Defaults {
    #[serde(default = "default_timeout")]
    pub timeout: u64,

    #[serde(default = "default_true")]
    pub forward_headers: bool,

    #[serde(default = "default_true")]
    pub proxy_headers: bool,

    #[serde(default = "default_true")]
    pub strip_hop_by_hop: bool,

    #[serde(default)]
    pub headers: HeaderRules,
}

impl Default for Defaults {
    fn default() -> Self {
        Self {
            timeout: default_timeout(),
            forward_headers: default_true(),
            proxy_headers: default_true(),
            strip_hop_by_hop: default_true(),
            headers: HeaderRules::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Route {
    pub path: String,

    #[serde(default = "default_methods")]
    pub methods: Vec<String>,

    pub timeout: Option<u64>,

    #[serde(default)]
    pub headers: HeaderRules,

    pub targets: Vec<Target>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Target {
    pub url: String,

    #[serde(default)]
    pub primary: bool,

    pub timeout: Option<u64>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HeaderRules {
    #[serde(default)]
    pub add: HashMap<String, String>,

    #[serde(default)]
    pub strip: Vec<String>,
}
