//! Prints the OpenAPI document: `cargo run -p users-server --bin openapi`.

fn main() {
    let document = users_server::api().openapi();
    println!("{}", serde_json::to_string_pretty(&document).unwrap());
}
