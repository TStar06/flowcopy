//! Anonymous usage telemetry for the Car-Controlling cloud proxy.
//!
//! Sends one fire-and-forget event per dictation so the central dashboard can
//! show daily counts and failure reasons. NO dictation content, NO audio, NO
//! user or device identifier is ever transmitted — only counters.
//!
//! Only active when the app runs against the company proxy (cc_cloud provider
//! with an embedded token); otherwise this is a no-op.

use crate::settings::{cc_cloud_app_token, AppSettings, CC_CLOUD_BASE_URL, CC_CLOUD_PROVIDER_ID};
use serde::Serialize;

#[derive(Serialize)]
pub struct TelemetryEvent {
    /// "dictation" or "translate".
    pub kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_lang: Option<String>,
    /// "ok", "rejected" or "llm_error".
    pub outcome: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reject_reason: Option<&'static str>,
    pub app_version: &'static str,
}

/// Sends the event in the background if the company proxy is the active
/// provider. Never blocks the paste pipeline and swallows all errors.
pub fn send(settings: &AppSettings, event: TelemetryEvent) {
    if settings.post_process_provider_id != CC_CLOUD_PROVIDER_ID {
        return;
    }
    let token = cc_cloud_app_token();
    if token.is_empty() {
        return;
    }

    tauri::async_runtime::spawn(async move {
        let url = format!("{CC_CLOUD_BASE_URL}/cc/telemetry");
        let mut builder = reqwest::Client::builder().timeout(std::time::Duration::from_secs(3));
        if let Some(cert) = crate::cc_cloud_ca::certificate_for(&url) {
            builder = builder.add_root_certificate(cert);
        }
        let Ok(client) = builder.build() else {
            return;
        };
        let _ = client
            .post(&url)
            .bearer_auth(token)
            .json(&event)
            .send()
            .await;
    });
}
