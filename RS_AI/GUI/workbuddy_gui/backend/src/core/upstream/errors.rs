//! Upstream error classification.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrKind {
    None,
    HardCredit,
    SoftRate,
    SessionDead,
    NotFound,
    Server,
    Client,
}

pub type ErrorKind = ErrKind;

#[derive(Debug)]
pub struct UpstreamError {
    pub kind: ErrKind,
    pub status: Option<u16>,
    pub msg: String,
    pub body: Option<String>,
}

impl fmt::Display for UpstreamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.status {
            Some(status) => write!(f, "upstream {} (http {}): {}", self.kind, status, self.msg),
            None => write!(f, "upstream {}: {}", self.kind, self.msg),
        }
    }
}

impl std::error::Error for UpstreamError {}

impl fmt::Display for ErrKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::None => "none",
            Self::HardCredit => "hard_credit",
            Self::SoftRate => "soft_rate",
            Self::SessionDead => "session_dead",
            Self::NotFound => "not_found",
            Self::Server => "server",
            Self::Client => "client",
        };
        f.write_str(value)
    }
}

impl UpstreamError {
    pub fn new(kind: ErrKind, status: u16, msg: impl Into<String>) -> Self {
        Self {
            kind,
            status: (status != 0).then_some(status),
            msg: msg.into(),
            body: None,
        }
    }

    pub fn from_transport<E: fmt::Display>(error: E) -> Self {
        Self::new(ErrKind::Server, 0, error.to_string())
    }

    pub fn from_ureq(error: ureq::Error) -> Self {
        match error {
            ureq::Error::Status(code, response) => {
                let mut body = String::new();
                let _ = response.into_reader().read_to_string(&mut body);
                let kind = classify(code, &body);
                Self { kind, status: Some(code), msg: truncate_str(&body, 200), body: Some(body) }
            }
            other => Self::from_transport(other),
        }
    }
}

impl From<ureq::Error> for UpstreamError {
    fn from(value: ureq::Error) -> Self { Self::from_ureq(value) }
}

impl From<std::io::Error> for UpstreamError {
    fn from(value: std::io::Error) -> Self { Self::from_transport(value) }
}

impl From<serde_json::Error> for UpstreamError {
    fn from(value: serde_json::Error) -> Self { Self::new(ErrKind::Client, 0, value.to_string()) }
}

pub fn classify(status: u16, body: &str) -> ErrKind {
    let lower = body.to_lowercase();
    if status == 402 || ["insufficient credit", "no credit", "quota exceeded", "积分不足", "额度不足", "余额不足"]
        .iter().any(|marker| lower.contains(&marker.to_lowercase())) { return ErrKind::HardCredit; }
    if body.contains("12153") || body.contains("Offline user session not found") { return ErrKind::SessionDead; }
    match status {
        404 => ErrKind::NotFound,
        429 => ErrKind::SoftRate,
        500..=599 => ErrKind::Server,
        400..=499 => ErrKind::Client,
        _ => ErrKind::None,
    }
}

pub fn truncate_str(value: &str, max: usize) -> String {
    value.trim().chars().take(max).collect()
}

use std::io::Read;
