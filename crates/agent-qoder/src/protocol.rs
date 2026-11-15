//! Qoder CLI SDK daemon wire protocol: control envelopes and response parsing.
//!
//! The daemon speaks NDJSON over stdin/stdout. Management calls are
//! `control_request`/`control_response` pairs whose inner request is dispatched
//! on a `daemon.`-prefixed subtype; the inner payload carries both `type` and
//! `subtype`. Auth is a JSON payload file referenced by `QODER_SDK_AUTH_PAYLOAD_FILE`;
//! the `{type:"qodercli"}` payload reuses the local login so NodaVideo never
//! handles the user's token.

use nodavideo_agent_runtime::AgentModel;
use serde_json::{Value, json};

/// SDK wire version presented during the daemon handshake.
pub const SDK_VERSION: &str = "1.0.59";
/// Daemon protocol version this adapter speaks.
pub const DAEMON_PROTOCOL_VERSION: u64 = 2;
/// Capability the daemon must advertise for multi-session routing.
pub const MULTI_SESSION_CAPABILITY: &str = "multi_session_v2";

pub const ENV_SDK_DAEMON: &str = "QODER_AGENT_SDK_DAEMON";
pub const ENV_SDK_VERSION: &str = "QODER_AGENT_SDK_VERSION";
pub const ENV_SDK_ENTRYPOINT: &str = "QODER_AGENT_SDK_ENTRYPOINT";
pub const SDK_ENTRYPOINT: &str = "sdk-ts";
/// Env var pointing at the JSON auth payload file the daemon reads at initialize.
pub const ENV_SDK_AUTH_PAYLOAD_FILE: &str = "QODER_SDK_AUTH_PAYLOAD_FILE";
/// Auth payload that reuses the local qodercli login read-only; carries no token.
pub const AUTH_PAYLOAD_QODERCLI: &str = r#"{"type":"qodercli"}"#;

/// A decoded daemon `control_response`.
#[derive(Debug)]
pub enum ControlResponse {
    Success { response: Value },
    Error { code: String, message: String },
}

/// One persisted Qoder conversation, as reported by `daemon.list_sessions`.
#[derive(Debug)]
pub struct QoderSession {
    pub id: String,
    pub title: String,
    pub state: String,
}

/// The signed-in account, as reported by `daemon.account_info`.
#[derive(Debug)]
pub struct QoderAccount {
    pub name: String,
    pub email: String,
    pub subscription_type: String,
}

/// Daemon handshake result.
#[derive(Debug)]
pub struct InitializeInfo {
    pub daemon_version: String,
    pub capabilities: Vec<String>,
}

/// Build a daemon `control_request` envelope for `subtype` (e.g. `daemon.initialize`).
pub fn control_request(request_id: &str, subtype: &str, params: Value) -> Value {
    let mut request = params;
    request["type"] = json!(subtype);
    request["subtype"] = json!(subtype);
    json!({
        "type": "control_request",
        "request_id": request_id,
        "request": request,
    })
}

/// Decode a `control_response`; returns None for frames that are not responses.
pub fn parse_control_response(value: &Value) -> Option<ControlResponse> {
    if value.get("type").and_then(Value::as_str) != Some("control_response") {
        return None;
    }
    let response = value.get("response")?;
    match response.get("subtype").and_then(Value::as_str)? {
        "success" => Some(ControlResponse::Success {
            response: response.get("response").cloned().unwrap_or(Value::Null),
        }),
        "error" => Some(ControlResponse::Error {
            code: response
                .get("code")
                .and_then(Value::as_str)
                .unwrap_or("DAEMON_REQUEST_FAILED")
                .to_string(),
            message: response
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("unknown daemon error")
                .to_string(),
        }),
        other => {
            log::debug!("Unrecognized Qoder control_response subtype '{other}'");
            None
        }
    }
}

/// The request id a `control_response` answers, used to route it to its caller.
pub fn response_request_id(value: &Value) -> Option<&str> {
    value
        .get("response")
        .and_then(|response| response.get("request_id"))
        .and_then(Value::as_str)
}

pub fn parse_initialize(value: &Value) -> Option<InitializeInfo> {
    let daemon_version = value
        .get("daemon_version")
        .and_then(Value::as_str)?
        .to_string();
    let capabilities = match value.get("capabilities").and_then(Value::as_array) {
        Some(caps) => caps
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect(),
        None => Vec::new(),
    };
    Some(InitializeInfo {
        daemon_version,
        capabilities,
    })
}

/// Parse the live model catalog from `daemon.get_models`.
pub fn parse_models(value: &Value) -> Vec<AgentModel> {
    let Some(models) = value.get("models").and_then(Value::as_array) else {
        return Vec::new();
    };
    models
        .iter()
        .filter(|model| model.get("isEnabled").and_then(Value::as_bool) != Some(false))
        .filter_map(|model| {
            let id = model
                .get("value")
                .and_then(Value::as_str)
                .or_else(|| model.get("modelId").and_then(Value::as_str))?;
            let display_name = model
                .get("displayName")
                .and_then(Value::as_str)
                .unwrap_or(id);
            let efforts = match model.get("efforts").and_then(Value::as_array) {
                Some(options) => options
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect(),
                None => Vec::new(),
            };
            Some(AgentModel {
                id: id.to_string(),
                display_name: display_name.to_string(),
                description: model
                    .get("description")
                    .and_then(Value::as_str)
                    .filter(|description| !description.is_empty())
                    .map(str::to_string),
                efforts,
                default_effort: model
                    .get("defaultEffort")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                is_default: model
                    .get("isDefault")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            })
        })
        .collect()
}

pub fn parse_sessions(value: &Value) -> Vec<QoderSession> {
    let Some(sessions) = value.get("sessions").and_then(Value::as_array) else {
        return Vec::new();
    };
    sessions
        .iter()
        .filter_map(|session| {
            let id = session
                .get("local_session_id")
                .and_then(Value::as_str)?
                .to_string();
            Some(QoderSession {
                id,
                title: session
                    .get("title")
                    .and_then(Value::as_str)
                    .unwrap_or("Untitled")
                    .to_string(),
                state: session
                    .get("state")
                    .and_then(Value::as_str)
                    .unwrap_or("inactive")
                    .to_string(),
            })
        })
        .collect()
}

pub fn parse_account(value: &Value) -> Option<QoderAccount> {
    let account = value.get("account")?;
    Some(QoderAccount {
        name: account
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        email: account
            .get("email")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        subscription_type: account
            .get("subscriptionType")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
    })
}

/// Extract a semver from `qodercli --version` output (a bare `1.1.89` line).
pub fn parse_version(output: &str) -> Option<String> {
    let token = output.split_whitespace().find(|part| {
        part.starts_with(|character: char| character.is_ascii_digit()) && part.contains('.')
    })?;
    Some(
        token
            .trim_end_matches(|character: char| !character.is_ascii_digit() && character != '.')
            .to_string(),
    )
}

#[cfg(test)]
mod tests {
    use crate::protocol::{
        ControlResponse, control_request, parse_account, parse_control_response, parse_initialize,
        parse_models, parse_sessions, parse_version, response_request_id,
    };
    use serde_json::json;

    #[test]
    fn control_request_carries_both_discriminators() {
        let envelope = control_request(
            "req-1",
            "daemon.initialize",
            json!({"sdk_version": "1.0.59"}),
        );
        assert_eq!(envelope["type"], "control_request");
        assert_eq!(envelope["request_id"], "req-1");
        assert_eq!(envelope["request"]["type"], "daemon.initialize");
        assert_eq!(envelope["request"]["subtype"], "daemon.initialize");
        assert_eq!(envelope["request"]["sdk_version"], "1.0.59");
    }

    #[test]
    fn control_response_success_and_error() {
        let success = json!({
            "type": "control_response",
            "response": {"subtype": "success", "request_id": "r1", "response": {"models": []}}
        });
        assert_eq!(response_request_id(&success), Some("r1"));
        match parse_control_response(&success) {
            Some(ControlResponse::Success { response }) => {
                assert!(response.get("models").is_some());
            }
            other => panic!("expected success, got {other:?}"),
        }

        let error = json!({
            "type": "control_response",
            "response": {"subtype": "error", "request_id": "r2", "code": "AUTH", "error": "no login"}
        });
        match parse_control_response(&error) {
            Some(ControlResponse::Error { code, message }) => {
                assert_eq!(code, "AUTH");
                assert_eq!(message, "no login");
            }
            other => panic!("expected error, got {other:?}"),
        }
    }

    #[test]
    fn non_response_frames_are_ignored() {
        assert!(parse_control_response(&json!({"type": "assistant"})).is_none());
    }

    #[test]
    fn initialize_parses_version_and_capabilities() {
        let info = parse_initialize(&json!({
            "daemon_protocol_version": 2,
            "daemon_version": "1.1.89",
            "capabilities": ["multi_session_v2", "daemon_queries_v1"]
        }))
        .unwrap();
        assert_eq!(info.daemon_version, "1.1.89");
        assert!(info.capabilities.contains(&"multi_session_v2".to_string()));
    }

    #[test]
    fn models_parse_efforts_and_default() {
        let models = parse_models(&json!({"models": [
            {"value": "auto", "displayName": "Auto", "modelId": "auto", "isDefault": true, "isEnabled": true},
            {"value": "ultimate", "displayName": "Ultimate", "isReasoning": true, "isEnabled": true,
             "efforts": ["xhigh", "high", "low"], "defaultEffort": "high"},
            {"value": "hidden", "displayName": "Hidden", "isEnabled": false}
        ]}));
        assert_eq!(models.len(), 2);
        assert_eq!(models[0].id, "auto");
        assert!(models[0].is_default);
        assert_eq!(models[1].efforts, vec!["xhigh", "high", "low"]);
        assert_eq!(models[1].default_effort.as_deref(), Some("high"));
    }

    #[test]
    fn sessions_parse_ids_and_titles() {
        let sessions = parse_sessions(&json!({"sessions": [
            {"local_session_id": "abc", "title": "Smart Bundles", "state": "inactive"},
            {"local_session_id": "def"}
        ]}));
        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0].id, "abc");
        assert_eq!(sessions[0].title, "Smart Bundles");
        assert_eq!(sessions[1].title, "Untitled");
    }

    #[test]
    fn account_parses_profile() {
        let account = parse_account(&json!({"account": {
            "userId": "u1", "name": "Elvin Ega", "email": "e@example.com", "subscriptionType": "Free"
        }}))
        .unwrap();
        assert_eq!(account.name, "Elvin Ega");
        assert_eq!(account.subscription_type, "Free");
        assert!(parse_account(&json!({})).is_none());
    }

    #[test]
    fn version_parsing_tolerates_prefixes() {
        assert_eq!(parse_version("1.1.89\n").as_deref(), Some("1.1.89"));
        assert_eq!(parse_version("qodercli 1.1.89").as_deref(), Some("1.1.89"));
        assert_eq!(parse_version("1.1.89-beta").as_deref(), Some("1.1.89"));
        assert!(parse_version("no version here").is_none());
    }
}
