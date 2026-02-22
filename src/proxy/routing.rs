//! Specificity-based route matching for incoming HTTP requests.
//!
//! [`match_route`] scores each configured route against the request
//! path and method using a specificity system: exact segments score
//! highest, parameterized segments (`:param`) score lower, and
//! wildcard prefixes (`/prefix/*`) and catch-all (`/*`) score lowest.
//! The highest-scoring match wins, with captured parameters returned.

use std::collections::HashMap;

use crate::config::model::Route;

#[must_use]
#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
pub fn match_route(
    routes: &[Route],
    path: &str,
    method: &str,
) -> Option<(usize, HashMap<String, String>)> {
    let request_segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

    let mut best_match: Option<(usize, HashMap<String, String>)> = None;
    let mut best_specificity: i32 = -1;

    for (idx, route) in routes.iter().enumerate() {
        if !method_matches(&route.methods, method) {
            continue;
        }

        let route_path = &route.path;

        // Catch-all: "/*" or "*"
        if route_path == "/*" || route_path == "*" {
            if best_specificity < 0 {
                best_match = Some((idx, HashMap::new()));
                best_specificity = 0;
            }
            continue;
        }

        // Wildcard prefix: "/qa/*" matches "/qa/anything/deep"
        if route_path.ends_with("/*") {
            let prefix = &route_path[..route_path.len() - 2];
            let prefix_segments: Vec<&str> = prefix.split('/').filter(|s| !s.is_empty()).collect();

            if request_segments.len() >= prefix_segments.len()
                && segments_match_exact(
                    &prefix_segments,
                    &request_segments[..prefix_segments.len()],
                )
            {
                let specificity = prefix_segments.len() as i32 * 10;
                if specificity > best_specificity {
                    best_match = Some((idx, HashMap::new()));
                    best_specificity = specificity;
                }
            }
            continue;
        }

        // Exact or parameterized match
        let route_segments: Vec<&str> = route_path.split('/').filter(|s| !s.is_empty()).collect();

        if route_segments.len() != request_segments.len() {
            continue;
        }

        let mut params = HashMap::new();
        let mut matched = true;
        let mut specificity: i32 = 0;

        for (rs, qs) in route_segments.iter().zip(request_segments.iter()) {
            if let Some(param_name) = rs.strip_prefix(':') {
                params.insert(param_name.to_string(), (*qs).to_string());
                specificity += 5;
            } else if *rs == *qs {
                specificity += 10;
            } else {
                matched = false;
                break;
            }
        }

        if matched && specificity > best_specificity {
            best_match = Some((idx, params));
            best_specificity = specificity;
        }
    }

    best_match
}

fn method_matches(methods: &[String], method: &str) -> bool {
    methods
        .iter()
        .any(|m| m == "*" || m.eq_ignore_ascii_case(method))
}

fn segments_match_exact(route: &[&str], request: &[&str]) -> bool {
    route.iter().zip(request.iter()).all(|(r, q)| *r == *q)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::model::{Route, Target};

    fn route(path: &str, methods: &[&str]) -> Route {
        Route {
            path: path.into(),
            methods: methods.iter().map(|s| s.to_string()).collect(),
            timeout: None,
            headers: Default::default(),
            targets: vec![Target {
                url: "http://localhost:8080".into(),
                primary: false,
                timeout: None,
            }],
        }
    }

    #[test]
    fn exact_match() {
        let routes = vec![route("/orders", &["*"])];
        let result = match_route(&routes, "/orders", "GET");
        assert!(result.is_some());
        let (idx, params) = result.unwrap();
        assert_eq!(idx, 0);
        assert!(params.is_empty());
    }

    #[test]
    fn parameterized_match() {
        let routes = vec![route("/orders/:id", &["*"])];
        let result = match_route(&routes, "/orders/42", "GET");
        assert!(result.is_some());
        let (idx, params) = result.unwrap();
        assert_eq!(idx, 0);
        assert_eq!(params.get("id").unwrap(), "42");
    }

    #[test]
    fn wildcard_prefix_match() {
        let routes = vec![route("/qa/*", &["*"])];
        let result = match_route(&routes, "/qa/anything/deep", "GET");
        assert!(result.is_some());
        assert_eq!(result.unwrap().0, 0);
    }

    #[test]
    fn catch_all_match() {
        let routes = vec![route("/*", &["*"])];
        let result = match_route(&routes, "/anything/at/all", "POST");
        assert!(result.is_some());
    }

    #[test]
    fn exact_beats_wildcard() {
        let routes = vec![route("/*", &["*"]), route("/orders", &["*"])];
        let result = match_route(&routes, "/orders", "GET");
        assert!(result.is_some());
        assert_eq!(result.unwrap().0, 1);
    }

    #[test]
    fn parameterized_beats_wildcard() {
        let routes = vec![route("/*", &["*"]), route("/orders/:id", &["*"])];
        let result = match_route(&routes, "/orders/42", "GET");
        assert!(result.is_some());
        assert_eq!(result.unwrap().0, 1);
    }

    #[test]
    fn method_filter() {
        let routes = vec![route("/orders", &["POST"])];
        let result = match_route(&routes, "/orders", "GET");
        assert!(result.is_none());

        let result = match_route(&routes, "/orders", "POST");
        assert!(result.is_some());
    }

    #[test]
    fn no_match() {
        let routes = vec![route("/orders", &["*"])];
        let result = match_route(&routes, "/products", "GET");
        assert!(result.is_none());
    }

    #[test]
    fn multi_param() {
        let routes = vec![route("/users/:user_id/orders/:order_id", &["*"])];
        let result = match_route(&routes, "/users/1/orders/2", "GET");
        assert!(result.is_some());
        let (_, params) = result.unwrap();
        assert_eq!(params.get("user_id").unwrap(), "1");
        assert_eq!(params.get("order_id").unwrap(), "2");
    }
}
