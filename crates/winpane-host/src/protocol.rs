use serde::{Deserialize, Serialize};

// JSON-RPC 2.0 error codes
pub const PARSE_ERROR: i32 = -32700;
pub const INVALID_REQUEST: i32 = -32600;
pub const METHOD_NOT_FOUND: i32 = -32601;
pub const INVALID_PARAMS: i32 = -32602;
pub const INTERNAL_ERROR: i32 = -32603;

#[derive(Debug, Deserialize)]
pub struct Request {
    pub jsonrpc: String,
    pub method: String,
    #[serde(default)]
    pub params: serde_json::Value,
    pub id: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct Response {
    pub jsonrpc: &'static str,
    pub result: serde_json::Value,
    pub id: serde_json::Value,
}

impl Response {
    pub fn ok(id: serde_json::Value, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0",
            result,
            id,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub jsonrpc: &'static str,
    pub error: RpcError,
    pub id: serde_json::Value,
}

impl ErrorResponse {
    pub fn new(id: serde_json::Value, code: i32, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0",
            error: RpcError {
                code,
                message: message.into(),
            },
            id,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct RpcError {
    pub code: i32,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct Notification {
    pub jsonrpc: &'static str,
    pub method: &'static str,
    pub params: serde_json::Value,
}

impl Notification {
    pub fn event(params: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0",
            method: "event",
            params,
        }
    }
}

/// Serializes a value to compact JSON (no trailing newline).
pub fn serialize_line(value: &impl Serialize) -> String {
    serde_json::to_string(value).expect("JSON serialization should not fail")
}

/// Parses a JSON-RPC 2.0 request from a line of text.
///
/// Returns an `ErrorResponse` (with null id) if the line is not valid JSON
/// or does not conform to JSON-RPC 2.0.
pub fn parse_request(line: &str) -> Result<Request, ErrorResponse> {
    let req: Request = serde_json::from_str(line).map_err(|e| {
        ErrorResponse::new(
            serde_json::Value::Null,
            PARSE_ERROR,
            format!("parse error: {e}"),
        )
    })?;

    if req.jsonrpc != "2.0" {
        return Err(ErrorResponse::new(
            req.id,
            INVALID_REQUEST,
            "jsonrpc must be \"2.0\"",
        ));
    }

    Ok(req)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_request() {
        let line = r#"{"jsonrpc":"2.0","method":"create","params":{},"id":1}"#;
        let req = parse_request(line).unwrap();
        assert_eq!(req.method, "create");
        assert_eq!(req.id, serde_json::json!(1));
    }

    #[test]
    fn parse_request_missing_params() {
        let line = r#"{"jsonrpc":"2.0","method":"destroy","id":2}"#;
        let req = parse_request(line).unwrap();
        assert_eq!(req.method, "destroy");
        assert!(req.params.is_null());
    }

    #[test]
    fn parse_request_bad_json() {
        let err = parse_request("not json").unwrap_err();
        assert_eq!(err.error.code, PARSE_ERROR);
        assert!(err.id.is_null());
    }

    #[test]
    fn parse_request_wrong_version() {
        let line = r#"{"jsonrpc":"1.0","method":"test","id":1}"#;
        let err = parse_request(line).unwrap_err();
        assert_eq!(err.error.code, INVALID_REQUEST);
        assert_eq!(err.id, serde_json::json!(1));
    }

    #[test]
    fn response_ok_serializes() {
        let resp = Response::ok(serde_json::json!(1), serde_json::json!({"ok": true}));
        let json = serialize_line(&resp);
        assert!(json.contains(r#""jsonrpc":"2.0""#));
        assert!(json.contains(r#""result":{"ok":true}"#));
    }

    #[test]
    fn error_response_serializes() {
        let resp = ErrorResponse::new(serde_json::json!(null), PARSE_ERROR, "bad");
        let json = serialize_line(&resp);
        assert!(json.contains(r#""code":-32700"#));
        assert!(json.contains(r#""message":"bad""#));
    }

    #[test]
    fn notification_serializes() {
        let n = Notification::event(serde_json::json!({"type": "click"}));
        let json = serialize_line(&n);
        assert!(json.contains(r#""method":"event""#));
        assert!(!json.contains(r#""id""#));
    }
}
