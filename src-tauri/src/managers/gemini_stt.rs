use anyhow::{anyhow, Result};
use base64::Engine;
use hound::{SampleFormat, WavSpec, WavWriter};
use log::debug;
use reqwest::Client;
use serde_json::Value;
use std::io::Cursor;

pub fn pcm_to_wav_bytes(samples: &[f32], sample_rate: u32) -> Result<Vec<u8>> {
    let spec = WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut writer = WavWriter::new(&mut cursor, spec)?;
        for &sample in samples {
            let sample_i16 = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
            writer.write_sample(sample_i16)?;
        }
        writer.finalize()?;
    }
    Ok(cursor.into_inner())
}

pub async fn transcribe_gemini_cloud(
    api_key: &str,
    model_name: &str,
    samples: &[f32],
    sample_rate: u32,
    _custom_words: &[String],
) -> Result<String> {
    if api_key.trim().is_empty() {
        return Err(anyhow!(
            "Gemini API key is missing. Please set your Gemini API key in Settings -> Models."
        ));
    }

    let wav_bytes = pcm_to_wav_bytes(samples, sample_rate)?;
    let base64_audio = base64::engine::general_purpose::STANDARD.encode(&wav_bytes);

    let payload = serde_json::json!({
        "contents": [{
            "role": "user",
            "parts": [{
                "inline_data": {
                    "mime_type": "audio/wav",
                    "data": base64_audio
                }
            }]
        }],
        "generationConfig": {
            "temperature": 0,
            "audioTranscriptionConfig": {
                "wordTimestamp": true,
                "diarization": false
            }
        }
    });

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent",
        model_name
    );

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .header("x-goog-api-key", api_key)
        .json(&payload)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let err_body = response.text().await.unwrap_or_default();
        return Err(anyhow!(
            "Gemini API call failed with status {}: {}",
            status,
            err_body
        ));
    }

    let json_resp: Value = response.json().await?;
    debug!("Gemini API response: {:?}", json_resp);

    if let Some(parts) = json_resp["candidates"][0]["content"]["parts"].as_array() {
        let text: String = parts
            .iter()
            .filter_map(|part| part["text"].as_str())
            .collect::<Vec<_>>()
            .join("");
        Ok(text.trim().to_string())
    } else {
        Ok(String::new())
    }
}
