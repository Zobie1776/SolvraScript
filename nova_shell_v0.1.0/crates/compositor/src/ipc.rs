//=============================================
// nova_compositor/src/ipc.rs
//=============================================
// Author: Nova Shell Team
// License: MIT
// Goal: Stub JSON-RPC router for compositor commands
// Objective: Provide method registry and dispatch scaffolding
//=============================================

//=============================================
// SECTION: Router
//=============================================

/// Router responsible for mapping method names to placeholders.
#[derive(Default)]
pub struct IpcRouter;

impl IpcRouter {
    /// Create a new router instance.
    pub fn new() -> Self {
        Self
    }

    /// Handle a JSON-RPC method and return a dummy payload.
    pub fn handle(&self, request: RpcRequest) -> RpcResponse {
        RpcResponse {
            jsonrpc: "2.0".to_string(),
            result: Some(format!("ok:{}", request.method)),
            error: None,
            id: request.id,
        }
    }

    /// Convenience helper used during tick logging.
    pub fn handle_default(&self) -> RpcResponse {
        self.handle(RpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "noop".to_string(),
            params: None,
            id: 0,
        })
    }
}

//=============================================
// SECTION: Message Types
//=============================================

/// Simplified JSON-RPC request envelope.
#[derive(Debug, Clone)]
pub struct RpcRequest {
    /// Version string.
    pub jsonrpc: String,
    /// Method name.
    pub method: String,
    /// Optional params.
    pub params: Option<String>,
    /// Request identifier.
    pub id: i64,
}

/// Simplified JSON-RPC response.
#[derive(Debug, Clone)]
pub struct RpcResponse {
    /// Version string.
    pub jsonrpc: String,
    /// Success payload expressed as string for scaffolding.
    pub result: Option<String>,
    /// Error value when applicable.
    pub error: Option<String>,
    /// Echoed identifier.
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
