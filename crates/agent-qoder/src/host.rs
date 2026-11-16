use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::time::Duration;

use nodavideo_agent_runtime::{
    AgentConfig, AgentEvent, AgentRuntimeError, AgentTool, PermissionMode, ToolCallAuthorization,
};
use serde_json::{Value, json};
use tokio::sync::{broadcast, oneshot, watch};

const HOST_TIMEOUT: Duration = Duration::from_secs(120);
pub const MCP_SERVER: &str = "nodavideo";

type ToolReply = (Value, bool);

pub struct SessionHost {
    pub(crate) events: broadcast::Sender<AgentEvent>,
    pub(crate) stopped: watch::Sender<bool>,
    tools: Vec<AgentTool>,
    mode: PermissionMode,
    approvals: Mutex<HashMap<String, oneshot::Sender<bool>>>,
    results: Mutex<HashMap<String, (oneshot::Sender<ToolReply>, ToolCallAuthorization)>>,
    cancelled: Mutex<HashSet<String>>,
}

// Public APIs only
impl SessionHost {
    pub(crate) fn new(config: &AgentConfig, events: broadcast::Sender<AgentEvent>) -> Self {
        Self {
            events,
            stopped: watch::channel(false).0,
            tools: config.editor_tools.clone(),
            mode: config.permission_mode,
            approvals: Mutex::new(HashMap::new()),
            results: Mutex::new(HashMap::new()),
            cancelled: Mutex::new(HashSet::new()),
        }
    }

    pub(crate) fn stop(&self) {
        self.stopped.send_replace(true);
        for (request_id, _) in self.approvals.lock().unwrap().drain() {
            let _ = self.events.send(AgentEvent::ApprovalDecision {
                request_id,
                approved: false,
            });
        }
        for (_, (_, authorization)) in self.results.lock().unwrap().drain() {
            authorization.revoke();
        }
    }

    pub(crate) fn cancel_request(&self, id: &str) {
        let mut cancelled = self.cancelled.lock().unwrap();
        if cancelled.len() >= 32 && !cancelled.contains(id) {
            drop(cancelled);
            self.stop();
            let _ = self.events.send(AgentEvent::SessionError {
                code: "QODER_CANCELLATION_OVERFLOW".into(),
                message: "Too many pending Qoder cancellations; the session was stopped safely"
                    .into(),
            });
            return;
        }
        cancelled.insert(id.into());
        if self.approvals.lock().unwrap().remove(id).is_some() {
            let _ = self.events.send(AgentEvent::ApprovalDecision {
                request_id: id.into(),
                approved: false,
            });
        }
        let mut pending = self.results.lock().unwrap();
        if let Some((_, authorization)) = pending.get(id)
            && authorization.revoke()
        {
            pending.remove(id);
        }
    }

    pub(crate) fn approve(&self, id: &str, approved: bool) -> Result<(), AgentRuntimeError> {
        let sender = self.approvals.lock().unwrap().remove(id).ok_or_else(|| {
            AgentRuntimeError::Other("This Qoder approval has expired or was cancelled".into())
        })?;
        sender
            .send(approved)
            .map_err(|_| AgentRuntimeError::Other("Qoder is no longer waiting for approval".into()))
    }

    pub(crate) fn tool_result(
        &self,
        id: &str,
        result: Value,
        is_error: bool,
    ) -> Result<(), AgentRuntimeError> {
        let (sender, authorization) = self.results.lock().unwrap().remove(id).ok_or_else(|| {
            AgentRuntimeError::Other(
                "This Qoder editor tool call has expired or was cancelled".into(),
            )
        })?;
        authorization.revoke();
        sender.send((result, is_error)).map_err(|_| {
            AgentRuntimeError::Other("Qoder is no longer waiting for the tool result".into())
        })
    }

    pub(crate) async fn handle(&self, frame: &Value) -> Value {
        let id = frame
            .get("request_id")
            .and_then(Value::as_str)
            .unwrap_or("");
        let session = frame.get("session_id").cloned().unwrap_or(Value::Null);
        let request = &frame["request"];
        let kind = request
            .get("subtype")
            .or_else(|| request.get("type"))
            .and_then(Value::as_str);
        let result = if self.cancelled.lock().unwrap().remove(id) || *self.stopped.borrow() {
            Err("Request cancelled".to_string())
        } else {
            match kind {
                Some("can_use_tool") => self.permission(id, request).await,
                Some("mcp_message") => self.mcp(id, request).await,
                Some("elicitation" | "elicitation_response") => Ok(json!({"action":"cancel"})),
                _ => Err("Unsupported Qoder host request".into()),
            }
        };
        self.cancelled.lock().unwrap().remove(id);
        let response = match result {
            Ok(payload) => json!({"subtype":"success","request_id":id,"response":payload}),
            Err(message) => json!({"subtype":"error","request_id":id,"error":message}),
        };
        json!({"type":"control_response","session_id":session,"response":response})
    }
}

// Private APIs only
impl SessionHost {
    async fn ask(
        &self,
        id: &str,
        tool: &str,
        input: &Value,
        title: &str,
        description: &str,
    ) -> bool {
        let (sender, receiver) = oneshot::channel();
        {
            let mut cancelled = self.cancelled.lock().unwrap();
            let mut approvals = self.approvals.lock().unwrap();
            if *self.stopped.borrow() || cancelled.remove(id) {
                return false;
            }
            approvals.insert(id.into(), sender);
        }
        let summary = input
            .get("command")
            .or_else(|| input.get("file_path"))
            .and_then(Value::as_str);
        let command_summary = match summary {
            Some(summary) => summary.to_string(),
            None => input.to_string(),
        };
        let event = AgentEvent::ApprovalRequested {
            request_id: id.into(),
            title: title.into(),
            description: description.into(),
            tool_name: tool.into(),
            command_summary,
            parameters: input.clone(),
        };
        let approved = if self.events.send(event).is_err() {
            false
        } else {
            matches!(
                tokio::time::timeout(HOST_TIMEOUT, receiver).await,
                Ok(Ok(true))
            ) && !*self.stopped.borrow()
                && !self.cancelled.lock().unwrap().contains(id)
        };
        self.approvals.lock().unwrap().remove(id);
        let _ = self.events.send(AgentEvent::ApprovalDecision {
            request_id: id.into(),
            approved,
        });
        approved
    }

    async fn permission(&self, id: &str, request: &Value) -> Result<Value, String> {
        let tool = request
            .get("tool_name")
            .and_then(Value::as_str)
            .ok_or("Missing permission tool name")?;
        let input = request
            .get("input")
            .filter(|input| input.is_object())
            .ok_or("Missing permission input")?;
        let use_id = request
            .get("tool_use_id")
            .and_then(Value::as_str)
            .ok_or("Missing permission tool id")?;
        let title = request.get("title").and_then(Value::as_str).unwrap_or(tool);
        let description = request
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or("Qoder needs permission to run this tool.");
        // SDK MCP access only enters the bridge; call_tool owns the actual editor approval.
        let editor_bridge = tool
            .strip_prefix("mcp__nodavideo__")
            .is_some_and(|name| self.tools.iter().any(|entry| entry.name == name));
        let approved = editor_bridge
            || (self.mode != PermissionMode::ReadOnly
                && self.ask(id, tool, input, title, description).await);
        if approved {
            Ok(
                json!({"behavior":"allow","permissionScope":"once","updatedInput":input,"toolUseID":use_id,"decisionClassification":"user_temporary"}),
            )
        } else {
            Ok(
                json!({"behavior":"deny","message":"Denied, cancelled, or approval timed out in NodaVideo","toolUseID":use_id,"decisionClassification":"user_reject"}),
            )
        }
    }

    async fn mcp(&self, id: &str, request: &Value) -> Result<Value, String> {
        if request.get("server_name").and_then(Value::as_str) != Some(MCP_SERVER) {
            return Err("Unknown editor MCP server".into());
        }
        let message = &request["message"];
        let Some(rpc_id) = message.get("id").filter(|id| !id.is_null()) else {
            return Ok(json!({"mcp_response":{"jsonrpc":"2.0"}}));
        };
        let result = match message.get("method").and_then(Value::as_str) {
            Some("initialize") => Ok(json!({
                "protocolVersion":"2024-11-05", "capabilities":{"tools":{}},
                "serverInfo":{"name":MCP_SERVER,"version":env!("CARGO_PKG_VERSION")}
            })),
            Some("ping") => Ok(json!({})),
            Some("tools/list") => Ok(json!({"tools":self.tools.iter().map(|tool| json!({
                "name":tool.name,"description":tool.description,"inputSchema":tool.input_schema
            })).collect::<Vec<_>>()})),
            Some("tools/call") => Ok(self.call_tool(id, &message["params"]).await),
            _ => Err(json!({"code":-32601,"message":"Unsupported MCP method"})),
        };
        let response = match result {
            Ok(result) => json!({"jsonrpc":"2.0","id":rpc_id,"result":result}),
            Err(error) => json!({"jsonrpc":"2.0","id":rpc_id,"error":error}),
        };
        Ok(json!({"mcp_response":response}))
    }

    async fn call_tool(&self, id: &str, params: &Value) -> Value {
        let name = params.get("name").and_then(Value::as_str).unwrap_or("");
        let Some(tool) = self.tools.iter().find(|tool| tool.name == name) else {
            return tool_response(&json!({"error":"Unknown editor tool"}), true);
        };
        let arguments = params
            .get("arguments")
            .cloned()
            .unwrap_or_else(|| json!({}));
        if !arguments.is_object() {
            return tool_response(&json!({"error":"Tool arguments must be an object"}), true);
        }
        if self.mode == PermissionMode::ReadOnly && (tool.is_mutation || tool.requires_approval) {
            return tool_response(
                &json!({"error":"Editor writes are disabled in read-only mode"}),
                true,
            );
        }
        if (tool.requires_approval || tool.is_mutation)
            && self.mode != PermissionMode::FullAccess
            && !self
                .ask(id, name, &arguments, name, &tool.description)
                .await
        {
            return tool_response(
                &json!({"error":"Editor action denied or approval expired"}),
                true,
            );
        }
        if *self.stopped.borrow() || self.cancelled.lock().unwrap().remove(id) {
            return tool_response(&json!({"error":"Editor action cancelled"}), true);
        }
        let (sender, mut receiver) = oneshot::channel();
        let authorization = ToolCallAuthorization::approved();
        {
            let mut cancelled = self.cancelled.lock().unwrap();
            let mut pending = self.results.lock().unwrap();
            if *self.stopped.borrow() || cancelled.remove(id) {
                return tool_response(&json!({"error":"Editor action cancelled"}), true);
            }
            pending.insert(id.into(), (sender, authorization.clone()));
        }
        if self
            .events
            .send(AgentEvent::EditorToolCallRequested {
                call_id: id.into(),
                tool_name: name.into(),
                arguments,
                authorization: authorization.clone(),
            })
            .is_err()
        {
            let pending = self.results.lock().unwrap().remove(id);
            if let Some((_, authorization)) = pending {
                authorization.revoke();
            }
            return tool_response(&json!({"error":"Editor is not connected"}), true);
        }
        let waited = tokio::time::timeout(HOST_TIMEOUT, &mut receiver).await;
        // A commit that won the authorization lock must return its real outcome, not a timeout denial.
        let committed = !authorization.revoke();
        let result = if waited.is_err() && committed {
            tokio::time::timeout(HOST_TIMEOUT, receiver).await
        } else {
            waited
        };
        self.results.lock().unwrap().remove(id);
        match result {
            Ok(Ok((result, is_error))) => tool_response(&result, is_error),
            _ if committed => tool_response(
                &json!({"error":"Editor committed the action, but its result was not delivered"}),
                true,
            ),
            _ => tool_response(&json!({"error":"Editor tool cancelled or timed out"}), true),
        }
    }
}

fn tool_response(result: &Value, is_error: bool) -> Value {
    json!({"content":[{"type":"text","text":result.to_string()}],"isError":is_error})
}

#[cfg(test)]
mod tests {
    use crate::host::SessionHost;
    use nodavideo_agent_runtime::{AgentConfig, AgentEvent, AgentTool, PermissionMode};
    use serde_json::{Value, json};
    use std::sync::Arc;
    use tokio::sync::broadcast;

    fn host(mode: PermissionMode) -> (Arc<SessionHost>, broadcast::Receiver<AgentEvent>) {
        let (events, receiver) = broadcast::channel(32);
        let config = AgentConfig {
            permission_mode: mode,
            editor_tools: vec![
                AgentTool {
                    name: "edit".into(),
                    description: "Edit timeline".into(),
                    input_schema: json!({"type":"object"}),
                    is_mutation: true,
                    requires_approval: true,
                },
                AgentTool {
                    name: "inspect".into(),
                    description: "Inspect timeline".into(),
                    input_schema: json!({"type":"object"}),
                    is_mutation: false,
                    requires_approval: false,
                },
            ],
            ..AgentConfig::default()
        };
        (Arc::new(SessionHost::new(&config, events)), receiver)
    }

    fn permission(tool: &str) -> Value {
        json!({"type":"control_request","request_id":"approval","session_id":"native",
            "request":{"subtype":"can_use_tool","tool_name":tool,"tool_use_id":"use-1","input":{"command":"test"}}})
    }

    fn mcp(method: &str, params: Value) -> Value {
        let mut frame = json!({"type":"control_request","request_id":"call","session_id":"native",
            "request":{"subtype":"mcp_message","server_name":"nodavideo",
                "message":{"jsonrpc":"2.0","id":7,"method":method}}});
        frame["request"]["message"]["params"] = params;
        frame
    }

    #[tokio::test]
    async fn native_permission_waits_for_host_and_preserves_input() {
        for allowed in [true, false] {
            let (host, mut events) = host(PermissionMode::Supervised);
            let worker = host.clone();
            let task = tokio::spawn(async move { worker.handle(&permission("Bash")).await });
            assert!(matches!(
                events.recv().await.unwrap(),
                AgentEvent::ApprovalRequested { .. }
            ));
            assert!(!task.is_finished());
            host.approve("approval", allowed).unwrap();
            let response = task.await.unwrap();
            let decision = &response["response"]["response"];
            assert_eq!(
                decision["behavior"],
                if allowed {
                    "allow"
                } else {
                    "deny"
                }
            );
            assert_eq!(decision["toolUseID"], "use-1");
            if allowed {
                assert_eq!(decision["permissionScope"], "once");
                assert_eq!(decision["updatedInput"], json!({"command":"test"}));
            }
            assert!(host.approve("approval", true).is_err());
        }
    }

    #[tokio::test(start_paused = true)]
    async fn abandoned_permission_times_out_closed() {
        let (host, mut events) = host(PermissionMode::Supervised);
        let worker = host.clone();
        let task = tokio::spawn(async move { worker.handle(&permission("Bash")).await });
        events.recv().await.unwrap();
        let response = task.await.unwrap();
        assert_eq!(response["response"]["response"]["behavior"], "deny");
        assert!(host.approve("approval", true).is_err());
    }

    #[tokio::test]
    async fn cancellation_and_disconnect_reject_pending_approvals() {
        for stop in [false, true] {
            let (host, mut events) = host(PermissionMode::Supervised);
            let worker = host.clone();
            let task = tokio::spawn(async move { worker.handle(&permission("Bash")).await });
            events.recv().await.unwrap();
            if stop {
                host.stop();
            } else {
                host.cancel_request("approval");
            }
            assert_eq!(
                task.await.unwrap()["response"]["response"]["behavior"],
                "deny"
            );
            assert!(host.approve("approval", true).is_err());
        }
    }

    #[tokio::test]
    async fn read_only_blocks_native_commands_and_editor_mutations() {
        let (host, mut events) = host(PermissionMode::ReadOnly);
        assert_eq!(
            host.handle(&permission("Bash")).await["response"]["response"]["behavior"],
            "deny"
        );
        let result = host
            .handle(&mcp("tools/call", json!({"name":"edit"})))
            .await;
        assert_eq!(
            result["response"]["response"]["mcp_response"]["result"]["isError"],
            true
        );
        assert!(events.try_recv().is_err());
    }

    #[tokio::test]
    async fn editor_mutations_execute_only_after_approval_and_return_actual_result() {
        let (host, mut events) = host(PermissionMode::Supervised);
        let worker = host.clone();
        let task = tokio::spawn(async move {
            worker
                .handle(&mcp(
                    "tools/call",
                    json!({"name":"edit","arguments":{"revision":3}}),
                ))
                .await
        });
        assert!(matches!(
            events.recv().await.unwrap(),
            AgentEvent::ApprovalRequested { .. }
        ));
        assert!(events.try_recv().is_err());
        host.approve("call", true).unwrap();
        assert!(matches!(
            events.recv().await.unwrap(),
            AgentEvent::ApprovalDecision { approved: true, .. }
        ));
        assert!(
            matches!(events.recv().await.unwrap(), AgentEvent::EditorToolCallRequested { tool_name, arguments, .. } if tool_name == "edit" && arguments["revision"] == 3)
        );
        host.tool_result("call", json!({"revision":4}), false)
            .unwrap();
        let response = task.await.unwrap();
        let result = &response["response"]["response"]["mcp_response"]["result"];
        assert_eq!(result["isError"], false);
        assert_eq!(result["content"][0]["text"], "{\"revision\":4}");
        assert!(host.tool_result("call", json!({}), false).is_err());
    }

    #[tokio::test]
    async fn denied_editor_mutation_never_emits_execution() {
        let (host, mut events) = host(PermissionMode::Supervised);
        let worker = host.clone();
        let task = tokio::spawn(async move {
            worker
                .handle(&mcp("tools/call", json!({"name":"edit"})))
                .await
        });
        events.recv().await.unwrap();
        host.approve("call", false).unwrap();
        let response = task.await.unwrap();
        assert_eq!(
            response["response"]["response"]["mcp_response"]["result"]["isError"],
            true
        );
        while let Ok(event) = events.try_recv() {
            assert!(!matches!(event, AgentEvent::EditorToolCallRequested { .. }));
        }
    }

    #[tokio::test]
    async fn mcp_catalog_notifications_and_bridge_permissions() {
        let (host, mut events) = host(PermissionMode::ReadOnly);
        let catalog = host.handle(&mcp("tools/list", json!({}))).await;
        assert_eq!(
            catalog["response"]["response"]["mcp_response"]["result"]["tools"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        let mut notification = mcp("notifications/initialized", json!({}));
        notification["request"]["message"]
            .as_object_mut()
            .unwrap()
            .remove("id");
        assert_eq!(
            host.handle(&notification).await["response"]["response"],
            json!({"mcp_response":{"jsonrpc":"2.0"}})
        );
        assert_eq!(
            host.handle(&permission("mcp__nodavideo__inspect")).await["response"]["response"]["behavior"],
            "allow"
        );
        assert!(events.try_recv().is_err());
    }

    #[tokio::test]
    async fn unknown_tools_and_servers_are_rejected() {
        let (host, mut events) = host(PermissionMode::FullAccess);
        let result = host
            .handle(&mcp("tools/call", json!({"name":"unknown"})))
            .await;
        assert_eq!(
            result["response"]["response"]["mcp_response"]["result"]["isError"],
            true
        );
        let mut request = mcp("tools/list", json!({}));
        request["request"]["server_name"] = json!("other");
        assert_eq!(host.handle(&request).await["response"]["subtype"], "error");
        assert!(events.try_recv().is_err());
    }

    #[tokio::test]
    async fn cancellation_overflow_stops_before_queued_execution() {
        let (host, mut events) = host(PermissionMode::FullAccess);
        for index in 0..32 {
            host.cancel_request(&format!("old-{index}"));
        }
        host.cancel_request("call");
        assert!(*host.stopped.borrow());
        let response = host
            .handle(&mcp("tools/call", json!({"name":"edit"})))
            .await;
        assert_eq!(response["response"]["subtype"], "error");
        assert!(matches!(
            events.recv().await.unwrap(),
            AgentEvent::SessionError { .. }
        ));
        assert!(events.try_recv().is_err());
    }

    #[tokio::test]
    async fn completed_requests_release_their_cancellation_markers() {
        let (host, mut events) = host(PermissionMode::FullAccess);
        for index in 0..64 {
            let id = format!("call-{index}");
            let mut frame = mcp("tools/call", json!({"name":"inspect"}));
            frame["request_id"] = json!(id);
            let worker = host.clone();
            let task = tokio::spawn(async move { worker.handle(&frame).await });
            assert!(matches!(
                events.recv().await.unwrap(),
                AgentEvent::EditorToolCallRequested { .. }
            ));
            host.cancel_request(&id);
            assert_eq!(
                task.await.unwrap()["response"]["response"]["mcp_response"]["result"]["isError"],
                true
            );
            assert!(host.cancelled.lock().unwrap().is_empty());
            assert!(!*host.stopped.borrow());
        }
    }

    #[tokio::test]
    async fn cancellation_before_registration_never_prompts_or_executes() {
        let (host, mut events) = host(PermissionMode::Supervised);
        host.cancel_request("approval");
        assert!(!host.ask("approval", "Bash", &json!({}), "Run", "Run").await);
        host.cancel_request("call");
        assert_eq!(
            host.call_tool("call", &json!({"name":"inspect"})).await["isError"],
            true
        );
        assert!(events.try_recv().is_err());
    }

    #[tokio::test]
    async fn cancellation_after_queued_approval_still_denies() {
        for stop in [false, true] {
            let (host, mut events) = host(PermissionMode::Supervised);
            let worker = host.clone();
            let task = tokio::spawn(async move { worker.handle(&permission("Bash")).await });
            events.recv().await.unwrap();
            host.approve("approval", true).unwrap();
            if stop {
                host.stop();
            } else {
                host.cancel_request("approval");
            }
            assert_eq!(
                task.await.unwrap()["response"]["response"]["behavior"],
                "deny"
            );
        }
    }

    #[tokio::test(start_paused = true)]
    async fn committed_action_returns_real_result_after_cancellation_or_deadline() {
        for cancel in [false, true] {
            let (host, mut events) = host(PermissionMode::FullAccess);
            let worker = host.clone();
            let task = tokio::spawn(async move {
                worker
                    .handle(&mcp("tools/call", json!({"name":"edit"})))
                    .await
            });
            let AgentEvent::EditorToolCallRequested { authorization, .. } =
                events.recv().await.unwrap()
            else {
                panic!("expected editor call");
            };
            authorization.lock_for_commit().unwrap().complete();
            if cancel {
                host.cancel_request("call");
            } else {
                tokio::time::advance(std::time::Duration::from_secs(121)).await;
                tokio::task::yield_now().await;
            }
            assert!(!task.is_finished());
            host.tool_result("call", json!({"revision":4}), false)
                .unwrap();
            let response = task.await.unwrap();
            let result = &response["response"]["response"]["mcp_response"]["result"];
            assert_eq!(result["isError"], false);
            assert_eq!(result["content"][0]["text"], "{\"revision\":4}");
        }
    }

    #[tokio::test(start_paused = true)]
    async fn editor_result_timeout_and_cancellation_reject_late_results() {
        for cancel in [false, true] {
            let (host, mut events) = host(PermissionMode::ReadOnly);
            let worker = host.clone();
            let task = tokio::spawn(async move {
                worker
                    .handle(&mcp("tools/call", json!({"name":"inspect"})))
                    .await
            });
            let AgentEvent::EditorToolCallRequested { authorization, .. } =
                events.recv().await.unwrap()
            else {
                panic!("expected editor call");
            };
            assert!(authorization.is_valid());
            if cancel {
                host.cancel_request("call");
            }
            let response = task.await.unwrap();
            assert!(!authorization.is_valid());
            assert_eq!(
                response["response"]["response"]["mcp_response"]["result"]["isError"],
                true
            );
            assert!(host.tool_result("call", json!({}), false).is_err());
        }
    }
}
