use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeItem {
    pub id: Option<i64>,
    pub item_type: String,
    pub code: Option<String>,
    pub name: String,
    pub summary: Option<String>,
    pub data_status: String,
}
