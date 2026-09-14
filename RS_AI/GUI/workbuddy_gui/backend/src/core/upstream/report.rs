//! Chat activity reporting to upstream growth system.
//!
//! Ported from Go: internal/upstream/report.go
//!
//! A single report fires both the growth streak and the first_buddy task unlock.
//! Risk control: only 1 report per account per day.

use crate::core::auth::Auth;

use super::client::Client;
use super::errors::UpstreamError;

const REPORT_PATH: &str = "/v2/report";

/// Full-shape chat_request_send event (aligned with official client).
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ChatRequestEvent {
    event_code: String,
    timestamp: i64,
    report_delay: i32,
    mode: String,
    conversation_id: String,
    request_id: String,
    input_length: i32,
    request_model_id: String,
    request_model_name: String,
    is_plan: bool,
    is_auto_execute_terminal: bool,
    is_auto_modify: bool,
    codebase_enable: bool,
    max_token: i32,
    max_steps: i32,
    temperature: i32,
    max_retries: i32,
    mention_contexts: Vec<serde_json::Value>,
    knowledge_id: Vec<serde_json::Value>,
    knowledge_name: Vec<serde_json::Value>,
    codebase_id: String,
    mention_context_count: i32,
    command: String,
    expert_id: String,
    recommend_id: String,
    skill_id: String,
    skill_count: i32,
    total_count: i32,
    file_uri: String,
    present_at: i64,
    trace_id: String,
    root_request_id: String,
    parent_conversation_id: String,
    agent_name: String,
    agent_type: String,
    user_id: String,
}

impl Client {
    /// Send a chat activity report. `conversation_id` is caller-generated (e.g. `wb2api-<ms>`).
    pub fn report_chat_activity(
        &self,
        auth: &Auth,
        conversation_id: &str,
    ) -> Result<(), UpstreamError> {
        let now = chrono::Utc::now().timestamp_millis();
        let ev = ChatRequestEvent {
            event_code: "chat_request_send".into(),
            timestamp: now,
            report_delay: 0,
            mode: "craft".into(),
            conversation_id: conversation_id.into(),
            request_id: conversation_id.into(),
            input_length: 12,
            request_model_id: "deepseek-v4-flash".into(),
            request_model_name: "DeepSeek V4 Flash".into(),
            is_plan: false,
            is_auto_execute_terminal: false,
            is_auto_modify: false,
            codebase_enable: false,
            max_token: 0,
            max_steps: 0,
            temperature: 0,
            max_retries: 0,
            mention_contexts: vec![],
            knowledge_id: vec![],
            knowledge_name: vec![],
            codebase_id: String::new(),
            mention_context_count: 0,
            command: String::new(),
            expert_id: String::new(),
            recommend_id: String::new(),
            skill_id: String::new(),
            skill_count: 0,
            total_count: 0,
            file_uri: String::new(),
            present_at: now,
            trace_id: String::new(),
            root_request_id: conversation_id.into(),
            parent_conversation_id: conversation_id.into(),
            agent_name: "default".into(),
            agent_type: "conversation".into(),
            user_id: auth.uid.clone(),
        };
        let raw = serde_json::to_vec(&[ev])?;
        self.request_json(auth, "POST", REPORT_PATH, Some(&raw))?;
        Ok(())
    }
}
