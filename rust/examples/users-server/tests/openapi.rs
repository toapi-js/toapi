//! `openapi.json` is committed so the client can be generated from it and so
//! API changes show up in reviews. This test fails when it is out of date;
//! run it with `UPDATE_OPENAPI=1` to rewrite it.

#[test]
fn openapi_json_is_up_to_date() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/openapi.json");
    let expected = serde_json::to_string_pretty(&users_server::api().openapi()).unwrap() + "\n";
    if std::env::var_os("UPDATE_OPENAPI").is_some() {
        std::fs::write(path, &expected).unwrap();
        return;
    }
    let actual = std::fs::read_to_string(path).unwrap_or_default();
    assert!(
        actual == expected,
        "openapi.json is out of date, run `UPDATE_OPENAPI=1 cargo test -p users-server --test openapi`"
    );
}
