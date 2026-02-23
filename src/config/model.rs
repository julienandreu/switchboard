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

fn is_default_timeout(v: &u64) -> bool {
    *v == default_timeout()
}

fn is_true(v: &bool) -> bool {
    *v
}

fn is_false(v: &bool) -> bool {
    !*v
}

fn is_default_methods(v: &[String]) -> bool {
    v.len() == 1 && v[0] == "*"
}

fn is_default_actuator(v: &ActuatorConfig) -> bool {
    !v.enabled && v.auth.username.is_none() && v.auth.password.is_none()
}

fn is_default_defaults(v: &Defaults) -> bool {
    v.timeout == default_timeout()
        && v.forward_headers
        && v.proxy_headers
        && v.strip_hop_by_hop
        && v.headers.is_default()
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default, skip_serializing_if = "is_default_actuator")]
    pub actuator: ActuatorConfig,

    #[serde(default, skip_serializing_if = "is_default_defaults")]
    pub defaults: Defaults,

    pub routes: Vec<Route>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ActuatorConfig {
    #[serde(default, skip_serializing_if = "is_false")]
    pub enabled: bool,

    #[serde(default, skip_serializing_if = "ActuatorAuth::is_default")]
    pub auth: ActuatorAuth,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ActuatorAuth {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
}

impl ActuatorAuth {
    fn is_default(&self) -> bool {
        self.username.is_none() && self.password.is_none()
    }
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
    #[serde(
        default = "default_timeout",
        skip_serializing_if = "is_default_timeout"
    )]
    pub timeout: u64,

    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub forward_headers: bool,

    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub proxy_headers: bool,

    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub strip_hop_by_hop: bool,

    #[serde(default, skip_serializing_if = "HeaderRules::is_default")]
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

    #[serde(
        default = "default_methods",
        skip_serializing_if = "is_default_methods"
    )]
    pub methods: Vec<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,

    #[serde(default, skip_serializing_if = "HeaderRules::is_default")]
    pub headers: HeaderRules,

    pub targets: Vec<Target>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Target {
    pub url: String,

    #[serde(default, skip_serializing_if = "is_false")]
    pub primary: bool,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HeaderRules {
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub add: HashMap<String, String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub strip: Vec<String>,
}

impl HeaderRules {
    fn is_default(&self) -> bool {
        self.add.is_empty() && self.strip.is_empty()
    }
}
