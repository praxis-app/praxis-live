use sea_orm::prelude::Uuid;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CallPath {
    pub(crate) server_id: Uuid,
    pub(crate) channel_id: Uuid,
    pub(crate) call_id: Uuid,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct JoinCallResponse {
    pub(crate) livekit_url: String,
    pub(crate) room_name: String,
    pub(crate) token: String,
    pub(crate) call: CallResponse,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CallResponse {
    pub(crate) id: String,
    pub(crate) server_id: String,
    pub(crate) channel_id: String,
    pub(crate) room_name: String,
    pub(crate) status: String,
}
