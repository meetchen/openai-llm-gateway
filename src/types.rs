use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct ChatReq {
    model: String,
    messages: Vec<Message>,
    #[serde(default)]
    pub stream: bool,
    #[serde(default)]
    max_tokens: Option<u32>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Message {
    role: String,
    content: String,
}

#[derive(Serialize)]
pub struct ErrResp {
    pub error: String,
}
