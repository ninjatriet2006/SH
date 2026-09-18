//! Growth and travel endpoints.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::client::Client;
use super::errors::UpstreamError;
use crate::core::auth::Auth;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TravelState {
    pub state: String,
    #[serde(default)]
    pub daily_limit_reached: bool,
    #[serde(default)]
    pub record_id: i64,
    #[serde(default)]
    pub reward_credit: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Buddy { pub id: i64, pub name: String }

impl Client {
    fn growth_call(&self, auth: &Auth, method: &str, path: &str, body: Option<Value>) -> Result<Value, UpstreamError> {
        let raw = body.map(|v| serde_json::to_vec(&v)).transpose()?;
        self.request_json(auth, method, path, raw.as_deref())
    }

    pub fn travel_status(&self, auth: &Auth) -> Result<TravelState, UpstreamError> {
        Ok(serde_json::from_value(self.growth_call(auth, "GET", "/activity/growth/buddy/travel/status", None)?)?)
    }

    pub fn travel_depart(&self, auth: &Auth, location_id: i64) -> Result<(), UpstreamError> {
        self.growth_call(auth, "POST", "/activity/growth/buddy/travel/depart", Some(serde_json::json!({"location_id": location_id})))?;
        Ok(())
    }

    pub fn travel_claim(&self, auth: &Auth, record_id: i64) -> Result<i64, UpstreamError> {
        let value = self.growth_call(auth, "POST", "/activity/growth/buddy/travel/claim", Some(serde_json::json!({"record_id": record_id})))?;
        Ok(value.get("reward_credit").and_then(Value::as_i64).unwrap_or_default())
    }

    pub fn buddy_info(&self, auth: &Auth) -> Result<Option<Buddy>, UpstreamError> {
        let value = self.growth_call(auth, "GET", "/activity/growth/buddy/info", None)?;
        if value.get("buddy").is_none() || value["buddy"].is_null() { return Ok(None); }
        Ok(Some(serde_json::from_value(value["buddy"].clone())?))
    }

    pub fn buddy_first(&self, auth: &Auth) -> Result<(), UpstreamError> {
        self.growth_call(auth, "POST", "/activity/growth/buddy/first", Some(serde_json::json!({})))?;
        Ok(())
    }

    pub fn buddy_agreement(&self, auth: &Auth) -> Result<(), UpstreamError> {
        self.growth_call(auth, "POST", "/activity/growth/buddy/agreement", Some(serde_json::json!({"agree": true})))?;
        Ok(())
    }

    pub fn growth_streak(&self, auth: &Auth) -> Result<i64, UpstreamError> {
        let value = self.growth_call(auth, "GET", "/activity/growth/streak", None)?;
        Ok(value.get("streak").and_then(|v| v.get("days")).and_then(Value::as_i64).unwrap_or_default())
    }
}
