//! Integration tests for config loading across all file formats.

use switchboard::config::model::Config;
use switchboard::config::sources::parse_config_str;
use switchboard::config::validation::validate;

fn load_example(name: &str) -> String {
    let path = format!("example/{name}");
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {path}: {e}"))
}

#[test]
fn yaml_example_loads_and_validates() {
    let content = load_example("switchboard.yaml");
    let config = parse_config_str("yaml", &content, "switchboard.yaml").unwrap();
    validate(&config).unwrap();
    assert!(!config.routes.is_empty());
    assert!(config.total_targets() > 0);
}

#[test]
fn yaml_full_example_loads_and_validates() {
    let content = load_example("full.yaml");
    let config = parse_config_str("yaml", &content, "full.yaml").unwrap();
    validate(&config).unwrap();
    assert!(config.routes.len() >= 3);
}

#[cfg(feature = "json")]
#[test]
fn json_example_loads_and_validates() {
    let content = load_example("switchboard.json");
    let config = parse_config_str("json", &content, "switchboard.json").unwrap();
    validate(&config).unwrap();
    assert!(!config.routes.is_empty());
}

#[cfg(feature = "toml")]
#[test]
fn toml_example_loads_and_validates() {
    let content = load_example("switchboard.toml");
    let config = parse_config_str("toml", &content, "switchboard.toml").unwrap();
    validate(&config).unwrap();
    assert!(!config.routes.is_empty());
}

#[cfg(all(feature = "json", feature = "toml"))]
#[test]
fn all_formats_produce_equivalent_configs() {
    let yaml_content = load_example("switchboard.yaml");
    let json_content = load_example("switchboard.json");
    let toml_content = load_example("switchboard.toml");

    let yaml_config = parse_config_str("yaml", &yaml_content, "yaml").unwrap();
    let json_config = parse_config_str("json", &json_content, "json").unwrap();
    let toml_config = parse_config_str("toml", &toml_content, "toml").unwrap();

    // All should have the same number of routes and targets
    assert_eq!(yaml_config.routes.len(), json_config.routes.len());
    assert_eq!(yaml_config.routes.len(), toml_config.routes.len());
    assert_eq!(yaml_config.total_targets(), json_config.total_targets());
    assert_eq!(yaml_config.total_targets(), toml_config.total_targets());

    // First route path should match across formats
    assert_eq!(yaml_config.routes[0].path, json_config.routes[0].path);
    assert_eq!(yaml_config.routes[0].path, toml_config.routes[0].path);
}

#[test]
fn unsupported_format_returns_error() {
    let result = parse_config_str("xml", "{}", "test.xml");
    assert!(result.is_err());
}

#[test]
fn invalid_config_fails_validation() {
    let empty = r#"{"routes": []}"#;
    let config: Config = serde_json::from_str(empty).unwrap();
    assert!(validate(&config).is_err());
}

#[test]
fn config_total_targets_counts_correctly() {
    let json = r#"{
        "routes": [
            {"path": "/a", "targets": [{"url": "http://a:80"}, {"url": "http://b:80"}]},
            {"path": "/b", "targets": [{"url": "http://c:80"}]}
        ]
    }"#;
    let config: Config = serde_json::from_str(json).unwrap();
    assert_eq!(config.total_targets(), 3);
}
