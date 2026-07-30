//! Cloud transcription via Groq's OpenAI-compatible audio API.
//!
//! Used as an optional alternative to the local models: highest quality and
//! very low latency, but requires a (free) Groq API key and a network
//! connection. Callers must fall back to the local engine on any error.

use std::io::Cursor;
use std::time::Duration;

use log::debug;

use crate::audio_toolkit::constants::WHISPER_SAMPLE_RATE;

pub const GROQ_TRANSCRIPTION_URL: &str = "https://api.groq.com/openai/v1/audio/transcriptions";
const GROQ_ASR_MODEL: &str = "whisper-large-v3-turbo";
/// Generous but hard cap: a dictation must never hang the paste pipeline.
/// Sized for multi-minute recordings (a 2-minute dictation is ~4 MB) on a
/// slow uplink; Groq itself answers in well under 2 s.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(serde::Deserialize)]
struct TranscriptionResponse {
    text: String,
}

/// Encodes f32 samples (mono, 16 kHz) as an in-memory 16-bit PCM WAV.
fn encode_wav(samples: &[f32]) -> Result<Vec<u8>, String> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: WHISPER_SAMPLE_RATE,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut writer = hound::WavWriter::new(&mut cursor, spec)
            .map_err(|e| format!("Failed to create WAV writer: {}", e))?;
        for &sample in samples {
            let clamped = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
            writer
                .write_sample(clamped)
                .map_err(|e| format!("Failed to write WAV sample: {}", e))?;
        }
        writer
            .finalize()
            .map_err(|e| format!("Failed to finalize WAV: {}", e))?;
    }
    Ok(cursor.into_inner())
}

/// Unique-enough multipart boundary without pulling in a rand dependency.
/// Only needs to never collide with the PCM payload bytes.
fn multipart_boundary() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!(
        "ccspeak{:032x}{:08x}{:04x}",
        nanos,
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    )
}

/// Builds the complete multipart/form-data body in memory. The body is sent
/// as one sized block with an exact content-length — reqwest's streaming
/// multipart ended the chunked body early now and then inside the running
/// app ("unexpected EOF" from Groq, dictation falling back to the slow local
/// model); a pre-built buffer cannot be truncated silently.
fn build_multipart_form(wav: &[u8], language: Option<&str>, boundary: &str) -> Vec<u8> {
    fn text_field(out: &mut Vec<u8>, boundary: &str, name: &str, value: &str) {
        out.extend_from_slice(
            format!(
                "--{boundary}\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n"
            )
            .as_bytes(),
        );
    }

    let mut out = Vec::with_capacity(wav.len() + 1024);
    out.extend_from_slice(
        format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"audio.wav\"\r\nContent-Type: audio/wav\r\n\r\n"
        )
        .as_bytes(),
    );
    out.extend_from_slice(wav);
    out.extend_from_slice(b"\r\n");
    text_field(&mut out, boundary, "model", GROQ_ASR_MODEL);
    text_field(&mut out, boundary, "response_format", "json");
    text_field(&mut out, boundary, "temperature", "0");
    if let Some(lang) = language {
        text_field(&mut out, boundary, "language", lang);
    }
    out.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
    out
}

async fn send_transcription(
    client: &reqwest::Client,
    endpoint: &str,
    api_key: &str,
    boundary: &str,
    body: Vec<u8>,
) -> Result<String, String> {
    let response = client
        .post(endpoint)
        .bearer_auth(api_key.trim())
        .header(
            "content-type",
            format!("multipart/form-data; boundary={boundary}"),
        )
        .body(body)
        .send()
        .await
        .map_err(|e| format!("Cloud ASR request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!(
            "Cloud ASR returned {}: {}",
            status,
            body.chars().take(300).collect::<String>()
        ));
    }

    let parsed: TranscriptionResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse cloud ASR response: {}", e))?;

    Ok(parsed.text.trim().to_string())
}

/// Transcribes samples via an OpenAI-compatible audio endpoint (Groq directly
/// or the Car-Controlling proxy). `language` follows the app setting ("auto"
/// lets the model detect; anything else is sent as ISO code). The embedded
/// company CA is applied automatically when `endpoint` targets the proxy host.
pub async fn transcribe_cloud(
    samples: &[f32],
    language: &str,
    endpoint: &str,
    api_key: &str,
) -> Result<String, String> {
    if api_key.trim().is_empty() {
        return Err("No cloud API key configured".to_string());
    }

    let wav = encode_wav(samples)?;
    debug!(
        "Cloud transcription: sending {:.1}s of audio",
        samples.len() as f32 / WHISPER_SAMPLE_RATE as f32
    );

    let base_lang = language
        .split(&['-', '_'][..])
        .next()
        .unwrap_or(language)
        .to_string();
    let lang_opt = if base_lang != "auto" && !base_lang.is_empty() {
        Some(base_lang.as_str())
    } else {
        None
    };

    let boundary = multipart_boundary();
    let body = build_multipart_form(&wav, lang_opt, &boundary);

    // Standard TLS against the proxy's public (Let's Encrypt) certificate.
    // The app talks to the proxy via the domain, so no embedded root
    // certificate is applied — that path (rustls + our custom IP-SAN CA)
    // corrupted large upload bodies in transit, making Groq reject them with
    // "unexpected EOF" so every long dictation fell back to the slow local
    // model. certificate_for() only returns a cert for the bare proxy IP,
    // which is no longer used.
    let mut builder = reqwest::Client::builder().timeout(REQUEST_TIMEOUT);
    if let Some(cert) = crate::cc_cloud_ca::certificate_for(endpoint) {
        builder = builder.add_root_certificate(cert);
    }
    let client = builder
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    // One quick retry before giving up: a transient failure costs ~1 s here,
    // while the local-model fallback costs 5–35 s of waiting.
    let mut last_err = String::new();
    for attempt in 0..2 {
        if attempt > 0 {
            debug!("Cloud ASR retry after: {}", last_err);
            tokio::time::sleep(Duration::from_millis(300)).await;
        }
        match send_transcription(&client, endpoint, api_key, &boundary, body.clone()).await {
            Ok(text) => return Ok(text),
            Err(e) => last_err = e,
        }
    }
    Err(last_err)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_wav_produces_valid_header() {
        let samples = vec![0.0f32; 1600]; // 100ms of silence
        let wav = encode_wav(&samples).unwrap();
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        // 1600 samples * 2 bytes + 44 byte header
        assert_eq!(wav.len(), 1600 * 2 + 44);
    }

    #[test]
    fn multipart_form_is_terminated_and_complete() {
        let wav = vec![0u8; 64];
        let boundary = "testboundary123";
        let body = build_multipart_form(&wav, None, boundary);
        let text = String::from_utf8_lossy(&body);
        // Groq rejected requests whose body ended before the closing
        // boundary ("unexpected EOF") — the built body must always carry it.
        assert!(text.ends_with("--testboundary123--\r\n"));
        assert!(text.contains("name=\"file\""));
        assert!(text.contains("name=\"model\""));
        assert!(text.contains(GROQ_ASR_MODEL));
        assert!(text.contains("name=\"temperature\""));
        assert!(!text.contains("name=\"language\""));
    }

    #[test]
    fn multipart_form_includes_language_when_given() {
        let body = build_multipart_form(&[0u8; 8], Some("de"), "b0undary");
        let text = String::from_utf8_lossy(&body);
        assert!(text.contains("name=\"language\""));
        assert!(text.contains("\r\n\r\nde\r\n"));
    }

    #[test]
    fn multipart_form_embeds_payload_verbatim() {
        // PCM audio is binary — the payload must survive byte-for-byte.
        let wav: Vec<u8> = (0..=255u8).collect();
        let body = build_multipart_form(&wav, None, "xyz");
        assert!(body
            .windows(wav.len())
            .any(|window| window == wav.as_slice()));
    }

    #[test]
    fn multipart_boundaries_are_unique() {
        let a = multipart_boundary();
        let b = multipart_boundary();
        assert_ne!(a, b);
        assert!(a.starts_with("ccspeak"));
    }
}
