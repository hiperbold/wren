//! Adapter for the `Transcriber` port targeting OpenAI-compatible APIs
//! (`POST {base_url}/audio/transcriptions`). Serves the cloud (Groq, OpenAI) and
//! a local server (`base_url=localhost`) — the SAME code, as doc 03 requires.

use std::time::Duration;

use reqwest::blocking::multipart::{Form, Part};
use reqwest::blocking::Client;
use wren_core::{
    partial_transcript, AudioClip, PortError, ProviderConfig, Transcriber, Transcript,
    TranscriptionOptions,
};

/// Wait between the 1st and 2nd attempt — short enough that the user does not
/// notice, long enough for a network hiccup to pass (doc 08).
const RETRY_BACKOFF: Duration = Duration::from_millis(750);

pub struct RemoteApiTranscriber {
    client: Client,
    config: ProviderConfig,
}

#[derive(serde::Deserialize)]
struct TranscriptionResponse {
    text: String,
    #[serde(default)]
    language: Option<String>,
}

impl RemoteApiTranscriber {
    pub fn new(config: ProviderConfig) -> Result<Self, PortError> {
        let client = Client::builder()
            // Generous total timeout: transcribing long audio on a local server
            // can legitimately take tens of seconds.
            .timeout(Duration::from_secs(60))
            // But ESTABLISHING the connection must be fast: a host that does not
            // accept (e.g. a local provider restarting mid-development) fails in
            // ~5 s instead of hanging. Escape already cancels instantly; this only
            // bounds the life of the orphaned transcription thread.
            .connect_timeout(Duration::from_secs(5))
            .build()
            .map_err(|e| PortError::Other(e.to_string()))?;
        Ok(RemoteApiTranscriber { client, config })
    }

    fn endpoint(&self) -> String {
        format!("{}/audio/transcriptions", self.config.base_url.trim_end_matches('/'))
    }

    /// One complete attempt: builds the form (the multipart is not reusable, so
    /// we rebuild it from the bytes on every call), sends it and interprets the
    /// response.
    fn attempt(
        &self,
        flac: &[u8],
        options: &TranscriptionOptions,
    ) -> Result<Transcript, PortError> {
        let file = Part::bytes(flac.to_vec())
            .file_name("audio.flac")
            .mime_str("audio/flac")
            .map_err(|e| PortError::Other(e.to_string()))?;

        // temperature 0: deterministic, less hallucination in an ambiguous stretch (doc 08).
        let mut form = Form::new()
            .part("file", file)
            .text("model", self.config.model.clone())
            .text("response_format", "json")
            .text("temperature", "0");
        if let Some(language) = &options.language {
            form = form.text("language", language.clone());
        }
        if let Some(prompt) = &options.prompt {
            form = form.text("prompt", prompt.clone());
        }

        let mut request = self.client.post(self.endpoint()).multipart(form);
        if let Some(key) = &self.config.api_key {
            request = request.bearer_auth(key);
        }

        let response = request.send().map_err(|e| {
            // A timeout (connection or a hung read) is NOT transient: retrying
            // just doubles the wait against a provider that no longer responds. A
            // common connection error (host restarting) goes on as Network, which
            // still gets a second attempt (see `is_transient`).
            if e.is_timeout() {
                PortError::Other(format!("timed out talking to the provider: {e}"))
            } else {
                PortError::Network(e.to_string())
            }
        })?;

        let status = response.status();
        if !status.is_success() {
            let message = response.text().unwrap_or_default();
            return Err(PortError::ProviderRejected {
                status: status.as_u16(),
                message: message.chars().take(500).collect(),
            });
        }

        let body: TranscriptionResponse = response
            .json()
            .map_err(|e| PortError::Other(format!("unexpected response from the provider: {e}")))?;

        Ok(partial_transcript(
            body.text,
            body.language,
            &self.config.id,
            &self.config.model,
        ))
    }
}

/// Body of `GET /models` on an OpenAI-compatible API: only the `id` matters.
#[derive(serde::Deserialize)]
struct ModelsResponse {
    data: Vec<ModelEntry>,
}

#[derive(serde::Deserialize)]
struct ModelEntry {
    id: String,
}

/// Lists the transcription models of an OpenAI-compatible provider via
/// `GET {base_url}/models`. Used by the UI to populate the model selector when
/// configuring a provider — it saves the user from guessing the exact name.
///
/// Heuristic FILTER: Groq's endpoint returns ALL models (including LLMs), so we
/// keep only the `id`s whose lowercase contains "whisper" or "transcribe".
/// Result sorted and without duplicates.
pub fn list_models(base_url: &str, api_key: Option<&str>) -> Result<Vec<String>, PortError> {
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| PortError::Other(e.to_string()))?;

    // Same trailing-slash trim as `endpoint()`.
    let url = format!("{}/models", base_url.trim_end_matches('/'));
    let mut request = client.get(url);
    if let Some(key) = api_key {
        request = request.bearer_auth(key);
    }

    let response = request
        .send()
        .map_err(|e| PortError::Network(e.to_string()))?;

    let status = response.status();
    if !status.is_success() {
        let message = response.text().unwrap_or_default();
        return Err(PortError::ProviderRejected {
            status: status.as_u16(),
            message: message.chars().take(500).collect(),
        });
    }

    let body: ModelsResponse = response
        .json()
        .map_err(|e| PortError::Other(format!("unexpected response from the provider: {e}")))?;

    let mut models: Vec<String> = body
        .data
        .into_iter()
        .map(|m| m.id)
        .filter(|id| {
            let lower = id.to_lowercase();
            lower.contains("whisper") || lower.contains("transcribe")
        })
        .collect();
    models.sort();
    models.dedup();
    Ok(models)
}

/// Failures worth a second attempt (doc 08): the network dropped or the provider
/// is overloaded (429/5xx). Other 4xx (wrong key, malformed request) are
/// permanent — retrying only wastes the quota.
fn is_transient(error: &PortError) -> bool {
    match error {
        PortError::Network(_) => true,
        PortError::ProviderRejected { status, .. } => *status == 429 || *status >= 500,
        _ => false,
    }
}

impl Transcriber for RemoteApiTranscriber {
    fn transcribe(
        &self,
        clip: &AudioClip,
        options: &TranscriptionOptions,
    ) -> Result<Transcript, PortError> {
        // Lossless FLAC: same audio, smaller upload than WAV (doc 08 §2).
        // Encodes ONCE; each attempt only clones the bytes into the form.
        let flac = crate::preprocess::encode_flac(&clip.samples, clip.sample_rate)?;

        // Single retry with a short backoff for a transient failure (doc 08).
        // A blocking sleep is fine: the trait is synchronous and runs on its own thread.
        match self.attempt(&flac, options) {
            Err(error) if is_transient(&error) => {
                std::thread::sleep(RETRY_BACKOFF);
                self.attempt(&flac, options)
            }
            result => result,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::mpsc;

    /// One-note HTTP server: answers each connection with the status from the
    /// list (in order) and sends the raw request captured through the channel.
    fn mock_server(statuses: Vec<u16>) -> (String, mpsc::Receiver<Vec<u8>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let (tx, rx) = mpsc::channel();

        std::thread::spawn(move || {
            for status in statuses {
                let (mut stream, _) = listener.accept().unwrap();
                let mut raw = Vec::new();
                let mut buf = [0u8; 4096];
                loop {
                    let n = stream.read(&mut buf).unwrap_or(0);
                    if n == 0 {
                        break;
                    }
                    raw.extend_from_slice(&buf[..n]);
                    let Some(headers_end) =
                        raw.windows(4).position(|w| w == b"\r\n\r\n")
                    else {
                        continue;
                    };
                    let headers = String::from_utf8_lossy(&raw[..headers_end]);
                    let content_length = headers
                        .lines()
                        .filter_map(|l| {
                            l.to_ascii_lowercase()
                                .strip_prefix("content-length:")
                                .and_then(|v| v.trim().parse::<usize>().ok())
                        })
                        .next()
                        .unwrap_or(0);
                    if raw.len() >= headers_end + 4 + content_length {
                        break;
                    }
                }

                // The 200 response depends on the route: `/models` returns a
                // catalog (whisper + LLMs, to exercise the filter); the rest, the
                // usual transcription.
                let is_models = contains(&raw, b"GET /v1/models")
                    || contains(&raw, b"GET /models");
                let (line, body) = match status {
                    200 if is_models => (
                        "200 OK",
                        r#"{"data":[{"id":"whisper-large-v3-turbo"},{"id":"llama-3.1-8b"},{"id":"whisper-large-v3"},{"id":"gpt-4o-transcribe"},{"id":"whisper-large-v3-turbo"}]}"#,
                    ),
                    200 => ("200 OK", r#"{"text":"hello from mock","language":"pt"}"#),
                    503 => ("503 Service Unavailable", "{}"),
                    _ => ("401 Unauthorized", "{}"),
                };
                let response = format!(
                    "HTTP/1.1 {line}\r\nContent-Type: application/json\r\n\
                     Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(response.as_bytes());
                let _ = tx.send(raw);
            }
        });

        (format!("http://{addr}/v1"), rx)
    }

    fn provider(base_url: String) -> ProviderConfig {
        ProviderConfig {
            id: "mock".into(),
            label: "Mock".into(),
            kind: wren_core::ProviderKind::RemoteApi,
            base_url,
            api_key: Some("test-key".into()),
            model: "whisper-test".into(),
            sends_audio_externally: false,
        }
    }

    fn clip() -> AudioClip {
        AudioClip { samples: vec![0i16; 8_000], sample_rate: 16_000, duration_ms: 500 }
    }

    fn contains(haystack: &[u8], needle: &[u8]) -> bool {
        haystack.windows(needle.len()).any(|w| w == needle)
    }

    #[test]
    fn upload_is_flac_with_temperature_zero_and_language() {
        let (base_url, rx) = mock_server(vec![200]);
        let transcriber = RemoteApiTranscriber::new(provider(base_url)).unwrap();
        let options =
            TranscriptionOptions { language: Some("pt".into()), prompt: None };

        let out = transcriber.transcribe(&clip(), &options).unwrap();
        assert_eq!(out.text, "hello from mock");

        let request = rx.recv().unwrap();
        assert!(contains(&request, b"filename=\"audio.flac\""));
        assert!(contains(&request, b"audio/flac"));
        assert!(contains(&request, b"fLaC"), "body missing the FLAC magic");
        assert!(contains(&request, b"name=\"temperature\"\r\n\r\n0"));
        assert!(contains(&request, b"name=\"language\"\r\n\r\npt"));
        assert!(!contains(&request, b"audio/wav"));
    }

    #[test]
    fn error_5xx_gets_an_extra_attempt() {
        let (base_url, rx) = mock_server(vec![503, 200]);
        let transcriber = RemoteApiTranscriber::new(provider(base_url)).unwrap();

        let out = transcriber
            .transcribe(&clip(), &TranscriptionOptions::default())
            .unwrap();
        assert_eq!(out.text, "hello from mock");
        // Two requests reached the server.
        assert!(rx.recv_timeout(Duration::from_secs(2)).is_ok());
        assert!(rx.recv_timeout(Duration::from_secs(2)).is_ok());
    }

    #[test]
    fn permanent_error_does_not_retry() {
        let (base_url, rx) = mock_server(vec![401]);
        let transcriber = RemoteApiTranscriber::new(provider(base_url)).unwrap();

        let err = transcriber
            .transcribe(&clip(), &TranscriptionOptions::default())
            .unwrap_err();
        assert!(matches!(err, PortError::ProviderRejected { status: 401, .. }));
        assert!(rx.recv_timeout(Duration::from_secs(2)).is_ok());
        // No second attempt: the channel stays empty.
        assert!(rx.recv_timeout(Duration::from_millis(300)).is_err());
    }

    #[test]
    fn list_models_filters_transcription_only_sorted_and_deduped() {
        let (base_url, _rx) = mock_server(vec![200]);
        let models = list_models(&base_url, Some("test-key")).unwrap();
        // Of the mock's 5, the LLM (llama) drops out; the whisper/transcribe ones
        // stay, sorted and without the whisper-large-v3-turbo duplicate.
        assert_eq!(
            models,
            vec![
                "gpt-4o-transcribe".to_string(),
                "whisper-large-v3".to_string(),
                "whisper-large-v3-turbo".to_string(),
            ]
        );
    }

    #[test]
    fn list_models_error_status_becomes_provider_rejected() {
        let (base_url, _rx) = mock_server(vec![401]);
        let err = list_models(&base_url, None).unwrap_err();
        assert!(matches!(err, PortError::ProviderRejected { status: 401, .. }));
    }
}
