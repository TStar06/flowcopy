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
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

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

    let mut form = reqwest::multipart::Form::new()
        .part(
            "file",
            reqwest::multipart::Part::bytes(wav)
                .file_name("audio.wav")
                .mime_str("audio/wav")
                .map_err(|e| format!("Invalid mime type: {}", e))?,
        )
        .text("model", GROQ_ASR_MODEL)
        .text("response_format", "json")
        .text("temperature", "0");

    let base_lang = language
        .split(&['-', '_'][..])
        .next()
        .unwrap_or(language)
        .to_string();
    if base_lang != "auto" && !base_lang.is_empty() {
        form = form.text("language", base_lang);
    }

    let mut builder = reqwest::Client::builder().timeout(REQUEST_TIMEOUT);
    if let Some(cert) = crate::cc_cloud_ca::certificate_for(endpoint) {
        builder = builder.add_root_certificate(cert);
    }
    let client = builder
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    let response = client
        .post(endpoint)
        .bearer_auth(api_key.trim())
        .multipart(form)
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
}
