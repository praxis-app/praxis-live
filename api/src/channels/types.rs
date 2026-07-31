use entity::enums::ChannelType;
use serde::{Deserialize, Serialize};

pub(crate) use crate::servers::types::ServerPath;
use sea_orm::prelude::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ChannelPath {
    pub(crate) server_id: Uuid,
    pub(crate) channel_id: Uuid,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ChannelRequest {
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) channel_type: Option<ChannelType>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ChannelServer {
    pub(crate) id: String,
    pub(crate) slug: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ChannelResponse {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) channel_type: ChannelType,
    pub(crate) server: ChannelServer,
}

#[derive(Debug, Serialize)]
pub(crate) struct ChannelPayload {
    pub(crate) channel: ChannelResponse,
}

#[derive(Debug, Serialize)]
pub(crate) struct ChannelsPayload {
    pub(crate) channels: Vec<ChannelResponse>,
}
