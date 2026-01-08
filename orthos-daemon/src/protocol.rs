use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct Request {
    pub method: String,
    #[serde(default)]
    pub params: Option<serde_json::Value>,
    pub id: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct Response {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub core: Option<Vec<String>>,
}

impl Response {
    pub fn success(id: Option<u64>, result: serde_json::Value) -> Self {
        Self {
            result: Some(result),
            error: None,
            id,
        }
    }
    
    pub fn error(id: Option<u64>, code: i32, message: String, core: Option<Vec<String>>) -> Self {
        Self {
            result: None,
            error: Some(ErrorResponse { code, message, core }),
            id,
        }
    }
}
