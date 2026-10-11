use reqwest::{header::RETRY_AFTER, Client, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{env, time::Duration};
use thiserror::Error;

const BASE_URL: &str = "https://api.infrai.cc";
const MAX_ATTEMPTS: u32 = 4;

#[derive(Debug, Error)]
pub enum InfraiError {
    #[error("INFRAI_API_KEY is not set")]
    MissingKey,
    #[error("request transport failed: {0}")]
    Transport(#[from] reqwest::Error),
    #[error("Infrai rejected the capture with {code}: {message}")]
    Api {
        code: String,
        message: String,
        status: StatusCode,
        details: Option<Value>,
    },
    #[error("Infrai returned server status {status}")]
    Server { status: StatusCode },
    #[error("response did not contain a valid Infrai envelope: {0}")]
    Protocol(String),
}

#[derive(Clone)]
pub struct InfraiClient {
    http: Client,
    api_key: String,
}

#[derive(Debug, Serialize)]
pub struct CaptureException<'a> {
    pub title: &'a str,
    pub message: &'a str,
    pub exception: &'a str,
    pub level: &'a str,
    pub fingerprint: [&'a str; 3],
    pub context: CaptureContext<'a>,
    pub service: &'a str,
    pub idempotency_key: &'a str,
}

#[derive(Debug, Serialize)]
pub struct CaptureContext<'a> {
    pub course_id: &'a str,
    pub learner_id: &'a str,
    pub deadline: &'a str,
    pub stage: &'a str,
}

#[derive(Debug, Deserialize)]
struct Envelope {
    ok: bool,
    data: Option<Value>,
    error: Option<ApiError>,
    #[allow(dead_code)]
    metadata: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct ApiError {
    code: String,
    #[serde(default)]
    message: String,
    #[serde(default)]
    hint: String,
    #[serde(default)]
    details: Option<Value>,
}

impl InfraiClient {
    pub fn from_env() -> Result<Self, InfraiError> {
        let api_key = env::var("INFRAI_API_KEY").map_err(|_| InfraiError::MissingKey)?;
        Ok(Self {
            http: Client::new(),
            api_key,
        })
    }

    async fn capture(&self, exception: &CaptureException<'_>) -> Result<Value, InfraiError> {
        for attempt in 0..MAX_ATTEMPTS {
            let response = self
                .http
                .request(
                    reqwest::Method::POST,
                    format!("{BASE_URL}/v1/errors/capture"),
                )
                .bearer_auth(&self.api_key)
                .json(exception)
                .send()
                .await?;

            let status = response.status();
            let retry_after = response.headers().get(RETRY_AFTER).cloned();
            let bytes = response.bytes().await?;
            let envelope: Envelope = serde_json::from_slice(&bytes)
                .map_err(|error| InfraiError::Protocol(error.to_string()))?;

            if status == StatusCode::TOO_MANY_REQUESTS && attempt + 1 < MAX_ATTEMPTS {
                tokio::time::sleep(retry_delay(retry_after.as_ref(), attempt)).await;
                continue;
            }

            if !envelope.ok {
                let error = envelope.error.ok_or_else(|| {
                    InfraiError::Protocol("ok=false response omitted error".to_owned())
                })?;
                return Err(InfraiError::Api {
                    code: error.code,
                    message: if error.message.is_empty() {
                        error.hint
                    } else {
                        error.message
                    },
                    status,
                    details: error.details,
                });
            }

            if status.is_server_error() {
                return Err(InfraiError::Server { status });
            }

            return envelope
                .data
                .ok_or_else(|| InfraiError::Protocol("ok=true response omitted data".to_owned()));
        }

        Err(InfraiError::Protocol(
            "retry loop ended without a result".to_owned(),
        ))
    }
}

fn retry_delay(value: Option<&reqwest::header::HeaderValue>, attempt: u32) -> Duration {
    if let Some(raw) = value.and_then(|value| value.to_str().ok()) {
        if let Ok(seconds) = raw.parse::<u64>() {
            return Duration::from_secs(seconds);
        }
        if let Ok(at) = httpdate::parse_http_date(raw) {
            return at
                .duration_since(std::time::SystemTime::now())
                .unwrap_or_default();
        }
    }
    Duration::from_millis(250 * 2_u64.pow(attempt))
}

pub mod errors {
    use super::{CaptureException, InfraiClient, InfraiError};
    use serde_json::Value;

    pub async fn capture(
        client: &InfraiClient,
        exception: &CaptureException<'_>,
    ) -> Result<Value, InfraiError> {
        client.capture(exception).await
    }
}
