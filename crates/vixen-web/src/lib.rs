//! Vixen's HTTP and SignalR wire protocol, independent of execution and UI state.

use serde_json::Value;
mod http;
mod server;
mod websocket;
pub use server::Server;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Elements,
    SearchElements,
    ChildElements,
    ParentElements,
    ElementOn,
    GroupOn,
    GroupOff,
    ElementOff,
    ClearElements,
    ElementEffect,
    PlayStatus,
    Sequences,
    PlaySequence,
    PauseSequence,
    StopSequence,
    Controllers,
    Controller,
    SetControllerState,
    SetAllControllersState,
    SaveSystem,
}

/// MVC binding sources in precedence order: form, route, then query.
/// JSON is exposed only for the two explicitly body-bound group actions.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ApiInput {
    pub fields: Vec<(String, String)>,
    pub json: Option<Value>,
}

impl ApiInput {
    pub fn get(&self, name: &str) -> Option<&str> {
        self.fields
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    /// Dictionary keys preserve spelling: Options[PropName] and Color[#FF0000].
    pub fn dictionary(&self, prefix: &str) -> Vec<(&str, &str)> {
        let mut found = Vec::new();
        for (key, value) in &self.fields {
            if let Some((head, tail)) = key.split_once('[') {
                if head.eq_ignore_ascii_case(prefix) {
                    if let Some(name) = tail.strip_suffix(']') {
                        if !found.iter().any(|(previous, _)| *previous == name) {
                            found.push((name, value.as_str()));
                        }
                    }
                }
            }
        }
        found
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ApiResponse {
    Json(Value),
    NoContent,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ApiError {
    BadRequest(String),
    Internal(String),
}

pub trait WebApi: Send + Sync + 'static {
    fn invoke(&self, action: Action, input: ApiInput) -> Result<ApiResponse, ApiError>;
    /// Long-running application work cooperates with server shutdown. Existing
    /// adapters can keep their synchronous implementation when their work is brief.
    fn invoke_cancellable(
        &self,
        action: Action,
        input: ApiInput,
        stopping: &std::sync::atomic::AtomicBool,
    ) -> Result<ApiResponse, ApiError> {
        let _ = stopping;
        self.invoke(action, input)
    }
    fn context_states(&self) -> Vec<Value>;
}

pub type Logger = std::sync::Arc<dyn Fn(&str) + Send + Sync>;

#[cfg(test)]
mod tests;
