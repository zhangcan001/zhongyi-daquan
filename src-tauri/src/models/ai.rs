use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiPlaceholderResponse {
    pub enabled: bool,
    pub message: String,
}
