use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ChannelRequest {
    pub(crate) name: String,
    pub(crate) description: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ChannelServer {
    pub(crate) id: String,
    pub(crate) slug: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ChannelResponse {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) server: ChannelServer,
}
