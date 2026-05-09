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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CallUserResponse {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) display_name: Option<String>,
    pub(crate) profile_picture: Option<crate::users::UserImageRef>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CallSummaryResponse {
    pub(crate) messages: u64,
    pub(crate) proposals: u64,
    pub(crate) polls: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CallArtifactResponse {
    #[serde(rename = "type")]
    pub(crate) kind: &'static str,
    pub(crate) id: String,
    pub(crate) server_id: String,
    pub(crate) channel_id: String,
    pub(crate) room_name: String,
    pub(crate) status: String,
    pub(crate) started_by: CallUserResponse,
    pub(crate) participants: Vec<CallUserResponse>,
    pub(crate) participant_count: usize,
    pub(crate) duration_seconds: i64,
    pub(crate) summary: CallSummaryResponse,
    pub(crate) created_at: String,
    pub(crate) ended_at: Option<String>,
}
