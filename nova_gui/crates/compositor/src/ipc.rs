//=============================================
// nova_compositor/src/ipc.rs
//=============================================
// Author: Nova GUI Team
// License: MIT
// Goal: Stub JSON-RPC router for compositor commands
// Objective: Provide method registry and dispatch scaffolding (socket path aware)
//=============================================

/// Router responsible for mapping method names to placeholders.
#[derive(Debug, Clone)]
pub struct IpcRouter {
    /// Path to the Unix domain socket used for IPC.
    pub socket_path: String,
}

impl IpcRouter {
    /// Create a new router pointing at the given socket path.
    pub fn new(socket_path: &str) -> Self {
        Self {
            socket_path: socket_path.into(),
        }
    }

    /// Handle a JSON-RPC method and return a dummy payload (placeholder).
    pub fn handle(&self, request: RpcRequest) -> RpcResponse {
        RpcResponse {
            jsonrpc: "2.0".into(),
            result: Some(format!("ok:{}", request.method)),
            error: None,
            id: request.id,
        }
    }

    /// Convenience helper used during logging when no request is queued.
    pub fn handle_default(&self) -> RpcResponse {
        self.handle(RpcRequest {
            jsonrpc: "2.0".into(),
            method: "noop".into(),
            params: None,
            id: 0,
        })
    }
}

/// Simplified JSON-RPC request envelope.
#[derive(Debug, Clone)]
pub struct RpcRequest {
    /// JSON-RPC version.
    pub jsonrpc: String,
    /// Method name.
    pub method: String,
    /// Optional params payload.
    pub params: Option<String>,
    /// Request identifier.
    pub id: i64,
}

/// Simplified JSON-RPC response envelope.
#[derive(Debug, Clone)]
pub struct RpcResponse {
    /// JSON-RPC version.
    pub jsonrpc: String,
    /// Success payload.
    pub result: Option<String>,
    /// Error payload.
    pub error: Option<String>,
    /// Correlation id.
    pub id: i64,
}

impl RpcResponse {
    /// Render the response as a JSON string.
    pub fn to_json(&self) -> String {
        let result_fragment = match &self.result {
            Some(value) => format!("\"{}\"", value),
            None => "null".to_string(),
        };
        let error_fragment = match &self.error {
            Some(value) => format!("\"{}\"", value),
            None => "null".to_string(),
        };
        format!(
            "{{\"jsonrpc\":\"{}\",\"result\":{},\"error\":{},\"id\":{}}}",
            self.jsonrpc, result_fragment, error_fragment, self.id
        )
    }
}
