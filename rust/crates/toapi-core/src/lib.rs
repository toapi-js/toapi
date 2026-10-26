//! Shared pieces of the Toapi Rust implementation:
//!
//! - the wire protocol constants (compatible with the TypeScript packages)
//! - parsing of route templates such as `/users/{id}`
//! - the deterministic naming scheme that maps `GET /users/{id}` to
//!   `GetUsersByIdRequest` and friends

pub const TAGS_HEADER: &str = "X-TAPI-Tags";
pub const EXPIRES_AT_HEADER: &str = "X-TAPI-Expires-At";
pub const TAGS_CONTENT_TYPE: &str = "text/tapi-tags";
pub const SESSION_COOKIE_NAME: &str = "__tapi-session";
pub const INVALIDATIONS_ROUTE: &str = "/__tapi/invalidations";
pub const OPENAPI_ROUTE: &str = "/__tapi/openapi.json";

/// One segment of a route template.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Segment {
    /// A literal segment such as `users`.
    Static(String),
    /// A parameter such as `{id}`, or `{*rest}` when `wildcard` is set.
    Param { name: String, wildcard: bool },
}

/// Parses a route template such as `/users/{id}` into its segments.
///
/// Parameters must span a whole segment; `/files/{name}.txt` is rejected.
pub fn parse_path(path: &str) -> Result<Vec<Segment>, String> {
    let Some(rest) = path.strip_prefix('/') else {
        return Err(format!("route `{path}` must start with `/`"));
    };
    let mut segments = Vec::new();
    for raw in rest.split('/') {
        if raw.is_empty() {
            continue;
        }
        if let Some(inner) = raw.strip_prefix('{').and_then(|s| s.strip_suffix('}')) {
            let (name, wildcard) = match inner.strip_prefix('*') {
                Some(name) => (name, true),
                None => (inner, false),
            };
            if name.is_empty() || name.contains(['{', '}']) {
                return Err(format!("invalid parameter `{raw}` in route `{path}`"));
            }
            segments.push(Segment::Param {
                name: name.to_string(),
                wildcard,
            });
        } else if raw.contains(['{', '}']) {
            return Err(format!(
                "parameter in route `{path}` must span a whole segment, found `{raw}`"
            ));
        } else {
            segments.push(Segment::Static(raw.to_string()));
        }
    }
    Ok(segments)
}

/// The names of all parameters of a route template, in order.
pub fn path_params(path: &str) -> Result<Vec<String>, String> {
    Ok(parse_path(path)?
        .into_iter()
        .filter_map(|s| match s {
            Segment::Param { name, .. } => Some(name),
            Segment::Static(_) => None,
        })
        .collect())
}

/// The base name of an operation: `GET /users/{id}` becomes `GetUsersById`.
/// The generated types append a suffix to it (see [`request_type_name`] and
/// friends).
///
/// - the method is PascalCased (`GET` → `Get`)
/// - the path is named by [`path_type_name`]
pub fn type_name(method: &str, path: &str) -> Result<String, String> {
    Ok(pascal_case(&method.to_ascii_lowercase()) + &path_type_name(path)?)
}

/// The name of a path: `/users/{id}` becomes `UsersById`.
///
/// - static segments are PascalCased, splitting words on non-alphanumerics
///   (`/user-groups` → `UserGroups`)
/// - parameters become `By` + their PascalCased name (`{id}` → `ById`,
///   `{*rest}` → `ByRest`)
/// - the root route `/` becomes `Root`
pub fn path_type_name(path: &str) -> Result<String, String> {
    let mut name = String::new();
    let segments = parse_path(path)?;
    if segments.is_empty() {
        name.push_str("Root");
    }
    for segment in segments {
        match segment {
            Segment::Static(s) => name.push_str(&pascal_case(&s)),
            Segment::Param { name: param, .. } => {
                name.push_str("By");
                name.push_str(&pascal_case(&param));
            }
        }
    }
    Ok(name)
}

/// Name of the struct holding a whole request (path parameters, query and body).
pub fn request_type_name(type_name: &str) -> String {
    format!("{type_name}Request")
}

/// Name of the call builder of an operation.
pub fn call_type_name(type_name: &str) -> String {
    format!("{type_name}Call")
}

/// Name of the path tree node for a path: `/users/{id}` → `UsersByIdRoute`.
/// The root of the tree is always `Api`.
pub fn route_type_name(path: &str) -> Result<String, String> {
    if parse_path(path)?.is_empty() {
        return Ok("Api".into());
    }
    Ok(path_type_name(path)? + "Route")
}

/// Name of the query parameter struct of a request type.
pub fn query_type_name(type_name: &str) -> String {
    format!("{type_name}Query")
}

/// Name of the request body type of a request type.
pub fn request_body_type_name(type_name: &str) -> String {
    format!("{type_name}RequestBody")
}

/// Name of the response body type of a request type.
pub fn response_body_type_name(type_name: &str) -> String {
    format!("{type_name}ResponseBody")
}

/// PascalCases a name, splitting words on non-alphanumeric characters and
/// keeping existing inner capitals (`userGroups` → `UserGroups`).
pub fn pascal_case(s: &str) -> String {
    s.split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut chars = w.chars();
            let first = chars.next().unwrap().to_ascii_uppercase();
            std::iter::once(first).chain(chars).collect::<String>()
        })
        .collect()
}

/// The Rust field name for a parameter or property name
/// (`includePosts` / `include-posts` → `include_posts`).
///
/// Keywords are returned unchanged; use [`is_keyword`] to decide whether the
/// identifier must be emitted as a raw identifier.
pub fn field_name(s: &str) -> String {
    let mut out = String::new();
    let mut prev_lower_or_digit = false;
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            if c.is_ascii_uppercase() && prev_lower_or_digit {
                out.push('_');
            }
            prev_lower_or_digit = c.is_ascii_lowercase() || c.is_ascii_digit();
            out.push(c.to_ascii_lowercase());
        } else {
            if !out.ends_with('_') {
                out.push('_');
            }
            prev_lower_or_digit = false;
        }
    }
    let out = out.trim_matches('_').to_string();
    if out.is_empty() {
        "_".to_string()
    } else if out.starts_with(|c: char| c.is_ascii_digit()) {
        format!("_{out}")
    } else {
        out
    }
}

/// Whether `s` is a Rust keyword that has to be written as `r#s`.
pub fn is_keyword(s: &str) -> bool {
    matches!(
        s,
        "as" | "break"
            | "const"
            | "continue"
            | "crate"
            | "else"
            | "enum"
            | "extern"
            | "false"
            | "fn"
            | "for"
            | "if"
            | "impl"
            | "in"
            | "let"
            | "loop"
            | "match"
            | "mod"
            | "move"
            | "mut"
            | "pub"
            | "ref"
            | "return"
            | "self"
            | "Self"
            | "static"
            | "struct"
            | "super"
            | "trait"
            | "true"
            | "type"
            | "unsafe"
            | "use"
            | "where"
            | "while"
            | "async"
            | "await"
            | "dyn"
            | "abstract"
            | "become"
            | "box"
            | "do"
            | "final"
            | "macro"
            | "override"
            | "priv"
            | "typeof"
            | "unsized"
            | "virtual"
            | "yield"
            | "try"
            | "gen"
    )
}

/// Whether requests with this method are queries (cached, watchable) rather
/// than mutations (which invalidate tags).
pub fn is_query_method(method: &str) -> bool {
    method.eq_ignore_ascii_case("get")
}

/// Whether requests with this method carry a request body.
pub fn has_body(method: &str) -> bool {
    matches!(
        method.to_ascii_lowercase().as_str(),
        "post" | "put" | "patch"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_routes() {
        assert_eq!(type_name("GET", "/path/{id}").unwrap(), "GetPathById");
        assert_eq!(type_name("get", "/users").unwrap(), "GetUsers");
        assert_eq!(type_name("PUT", "/path/{id}").unwrap(), "PutPathById");
        assert_eq!(
            type_name("GET", "/orgs/{org}/users/{user_id}").unwrap(),
            "GetOrgsByOrgUsersByUserId"
        );
        assert_eq!(type_name("POST", "/user-groups").unwrap(), "PostUserGroups");
        assert_eq!(
            type_name("GET", "/files/{*rest}").unwrap(),
            "GetFilesByRest"
        );
        assert_eq!(type_name("GET", "/").unwrap(), "GetRoot");
        assert_eq!(
            type_name("DELETE", "/userGroups/").unwrap(),
            "DeleteUserGroups"
        );
    }

    #[test]
    fn derived_names() {
        let name = type_name("GET", "/path/{id}").unwrap();
        assert_eq!(request_type_name(&name), "GetPathByIdRequest");
        assert_eq!(call_type_name(&name), "GetPathByIdCall");
        assert_eq!(query_type_name(&name), "GetPathByIdQuery");
        assert_eq!(response_body_type_name(&name), "GetPathByIdResponseBody");
        let name = type_name("PUT", "/path/{id}").unwrap();
        assert_eq!(request_body_type_name(&name), "PutPathByIdRequestBody");
    }

    #[test]
    fn route_names() {
        assert_eq!(route_type_name("/").unwrap(), "Api");
        assert_eq!(route_type_name("/users").unwrap(), "UsersRoute");
        assert_eq!(route_type_name("/users/{id}").unwrap(), "UsersByIdRoute");
    }

    #[test]
    fn rejects_invalid_paths() {
        assert!(parse_path("users").is_err());
        assert!(parse_path("/files/{name}.txt").is_err());
        assert!(parse_path("/files/{}").is_err());
    }

    #[test]
    fn field_names() {
        assert_eq!(field_name("id"), "id");
        assert_eq!(field_name("includePosts"), "include_posts");
        assert_eq!(field_name("include-posts"), "include_posts");
        assert_eq!(field_name("user_id"), "user_id");
        assert_eq!(field_name("2fa"), "_2fa");
        assert_eq!(field_name("type"), "type");
        assert!(is_keyword("type"));
    }
}
