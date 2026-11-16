//! Local-only fixture for the unchanged bundled signalr.js and browser page.
use serde_json::json;
use std::{
    io::{self, BufRead, Write},
    path::PathBuf,
    sync::Arc,
};
use vixen_web::{Action, ApiError, ApiInput, ApiResponse, Server, WebApi};
struct Fixture;
impl WebApi for Fixture {
    fn invoke(&self, action: Action, _input: ApiInput) -> Result<ApiResponse, ApiError> {
        Ok(ApiResponse::Json(match action {
            Action::Elements
            | Action::SearchElements
            | Action::ChildElements
            | Action::ParentElements => {
                json!([{ "Id":"00000000-0000-0000-0000-000000000001", "Name":"Loopback Element", "Colors":["#FF0000"], "Children":[] }])
            }
            Action::Controllers => {
                json!([{ "Id":"00000000-0000-0000-0000-000000000002", "Name":"Loopback Controller", "IsRunning":false,"IsPaused":false }])
            }
            Action::Sequences => json!([{ "Name":"Loopback Sequence", "FileName":"test.tim" }]),
            Action::PlayStatus => json!([]),
            _ => json!({"Message":"Loopback command accepted", "Details":[],"IsSuccessful":true}),
        }))
    }
    fn context_states(&self) -> Vec<serde_json::Value> {
        vec![
            json!({"Message":null,"Details":[],"IsSuccessful":true,"State":1,"Sequence":{"Name":"Loopback Sequence","FileName":"test.tim"},"Position":"00:00:01"}),
        ]
    }
}
fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../app/Resources/wwwroot");
    let mut server = Server::start(
        "127.0.0.1:0".parse().unwrap(),
        root,
        Arc::new(Fixture),
        Arc::new(|line| eprintln!("{line}")),
    )
    .unwrap();
    println!("{}", server.address());
    io::stdout().flush().unwrap();
    let _ = io::stdin().lock().lines().next();
    server.stop();
}
