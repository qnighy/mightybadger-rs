use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Default)]
pub struct ErrorData {
    pub errors: Vec<Payload>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Payload {
    #[serde(default)]
    pub error: ErrorPayload,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ErrorPayload {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token: Option<Uuid>,
}
