use crate::engine_supervisor::{EngineError, LoadedInfo};
use crate::managers::model::{ModelInfo, ModelManager, ModelSource};
use crate::settings::{get_settings, write_settings};
use anyhow::{anyhow, Result};
use base64::Engine;
use hound::{SampleFormat, WavSpec, WavWriter};
use log::debug;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use specta::Type;
use std::collections::HashMap;
use std::fmt;
use std::io::Cursor;
use tauri::AppHandle;
use transcribe_cpp::{Capabilities, RunOptions, Transcript};

pub const GEMINI_CLOUD_MODEL_ID: &str = "google/gemini-3.5-transcribe";

pub fn is_cloud_model(model_id: &str) -> bool {
    model_id == GEMINI_CLOUD_MODEL_ID
}

pub fn default_model_name() -> String {
    "gemini-3.5-transcribe".to_string()
}

#[derive(Clone, Serialize, Deserialize, Type, Default, PartialEq, Eq)]
#[serde(transparent)]
pub struct SecretString(Option<String>);

impl fmt::Debug for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            Some(v) if !v.is_empty() => write!(f, "Some(\"[REDACTED]\")"),
            Some(_) => write!(f, "Some(\"\")"),
            None => write!(f, "None"),
        }
    }
}

impl std::ops::Deref for SecretString {
    type Target = Option<String>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for SecretString {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub fn seed_model(available_models: &mut HashMap<String, ModelInfo>) {
    let gemini_id = GEMINI_CLOUD_MODEL_ID.to_string();
    available_models
        .entry(gemini_id.clone())
        .or_insert(ModelInfo {
        id: gemini_id,
        name: "Gemini 3.5 Transcribe (Cloud)".to_string(),
        description:
            "Google Gemini cloud speech-to-text API. Fast, smart punctuation & self-corrections."
                .to_string(),
        filename: String::new(),
        source: ModelSource::Local,
        size_mb: 0,
        is_downloaded: true,
        is_downloading: false,
        partial_size: 0,
        accuracy_score: 0.98,
        speed_score: 0.95,
        supports_translation: false,
        is_recommended: false,
        supported_languages: vec![
            "en".to_string(),
            "es".to_string(),
            "fr".to_string(),
            "de".to_string(),
            "zh".to_string(),
            "ja".to_string(),
        ],
        supports_language_selection: true,
        is_custom: false,
        supports_streaming: false,
        supports_language_detection: true,
    });
}

pub fn loaded_info(model_manager: &ModelManager) -> LoadedInfo {
    let languages = model_manager
        .get_model_info(GEMINI_CLOUD_MODEL_ID)
        .map(|m| m.supported_languages)
        .unwrap_or_default();
    LoadedInfo {
        arch: "gemini-cloud".to_string(),
        variant: "gemini-3.5-transcribe".to_string(),
        backend: "Gemini Cloud API".to_string(),
        device: "Google Cloud".to_string(),
        on_gpu: false,
        capabilities: Capabilities {
            native_sample_rate: 16_000,
            languages,
            translate_target_languages: Vec::new(),
            max_timestamp_kind: Default::default(),
            supports_language_detect: true,
            supports_translate: false,
            supports_streaming: false,
            supports_spec_decode: false,
            max_audio_ms: 0,
        },
        supports_initial_prompt: false,
        supports_long_form: true,
        supports_vocabulary: false,
    }
}

pub fn run_batch(
    app_handle: &AppHandle,
    pcm: Vec<f32>,
    run_options: &RunOptions,
) -> std::result::Result<Transcript, EngineError> {
    let settings = get_settings(app_handle);
    let api_key = settings
        .gemini_stt_api_key
        .as_deref()
        .unwrap_or_default()
        .to_string();
    let model_name = if settings.gemini_stt_model.trim().is_empty() {
        default_model_name()
    } else {
        settings.gemini_stt_model.clone()
    };
    let custom_words = settings.custom_words.clone();
    let text = std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| anyhow!("Failed to create runtime for Gemini: {}", e))?;
        rt.block_on(transcribe_gemini_cloud(
            &api_key,
            &model_name,
            &pcm,
            16_000,
            &custom_words,
        ))
    })
    .join()
    .map_err(|_| EngineError::Failed("Gemini transcription thread panicked".to_string()))?
    .map_err(|e| EngineError::Failed(e.to_string()))?;

    Ok(Transcript {
        raw_text: text.clone(),
        text,
        language: run_options.language.clone(),
        ..Default::default()
    })
}

#[tauri::command]
#[specta::specta]
pub async fn change_gemini_stt_api_key(app: AppHandle, key: String) -> Result<(), String> {
    let mut settings = get_settings(&app);
    let trimmed = key.trim().to_string();
    *settings.gemini_stt_api_key = if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    };
    write_settings(&app, settings);
    Ok(())
}

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
