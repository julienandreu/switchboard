//! Integration tests for route matching.

use switchboard::config::model::{Defaults, HeaderRules, Route, Target};
use switchboard::proxy::routing::match_route;

fn make_route(path: &str, methods: &[&str]) -> Route {
    Route {
        path: path.into(),
        methods: methods.iter().map(|s| (*s).to_string()).collect(),
        timeout: None,
        headers: HeaderRules::default(),
        targets: vec![Target {
            url: "http://localhost:8080".into(),
            primary: false,
            timeout: None,
        }],
    }
}

#[test]
fn specificity_ordering_comprehensive() {
    let routes = vec![
        make_route("/*", &["*"]),             // catch-all (specificity 0)
        make_route("/api/*", &["*"]),         // prefix wildcard (specificity 10)
        make_route("/api/users/:id", &["*"]), // parameterized (specificity 15)
        make_route("/api/users/me", &["*"]),  // exact (specificity 20)
    ];

    // Exact match wins
    let (idx, _) = match_route(&routes, "/api/users/me", "GET").unwrap();
    assert_eq!(idx, 3);

    // Parameterized beats wildcard
    let (idx, params) = match_route(&routes, "/api/users/42", "GET").unwrap();
    assert_eq!(idx, 2);
    assert_eq!(params.get("id").unwrap(), "42");

    // Wildcard prefix matches deep paths
    let (idx, _) = match_route(&routes, "/api/other/deep/path", "GET").unwrap();
    assert_eq!(idx, 1);

    // Catch-all matches anything else
    let (idx, _) = match_route(&routes, "/something/else", "GET").unwrap();
    assert_eq!(idx, 0);
}

#[test]
fn method_filtering_restricts_matches() {
    let routes = vec![
        make_route("/orders", &["GET"]),
        make_route("/orders", &["POST"]),
    ];

    let (idx, _) = match_route(&routes, "/orders", "GET").unwrap();
    assert_eq!(idx, 0);

    let (idx, _) = match_route(&routes, "/orders", "POST").unwrap();
    assert_eq!(idx, 1);

    assert!(match_route(&routes, "/orders", "DELETE").is_none());
}

#[test]
fn wildcard_method_matches_all() {
    let routes = vec![make_route("/api/*", &["*"])];

    assert!(match_route(&routes, "/api/anything", "GET").is_some());
    assert!(match_route(&routes, "/api/anything", "POST").is_some());
    assert!(match_route(&routes, "/api/anything", "DELETE").is_some());
}

#[test]
fn multi_segment_params() {
    let routes = vec![make_route(
        "/users/:user_id/orders/:order_id/items/:item_id",
        &["*"],
    )];

    let (_, params) = match_route(&routes, "/users/1/orders/2/items/3", "GET").unwrap();
    assert_eq!(params.len(), 3);
    assert_eq!(params["user_id"], "1");
    assert_eq!(params["order_id"], "2");
    assert_eq!(params["item_id"], "3");
}

#[test]
fn empty_routes_returns_none() {
    let routes: Vec<Route> = vec![];
    assert!(match_route(&routes, "/anything", "GET").is_none());
}

#[test]
fn defaults_are_sensible() {
    let defaults = Defaults::default();
    assert_eq!(defaults.timeout, 5000);
    assert!(defaults.forward_headers);
    assert!(defaults.proxy_headers);
    assert!(defaults.strip_hop_by_hop);
}
