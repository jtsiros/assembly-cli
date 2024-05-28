use std::{env, fs, thread::sleep, time::Duration};

use anyhow::{anyhow, Context, Result};
use reqwest::{
    blocking::Client,
    header::{HeaderMap, HeaderValue},
};
use serde_json::{json, Value};

use crate::cli::TranscriberArgs;

/// A client for interacting with the AssemblyAI transcription service.
pub struct Transcriber<'a> {
    client: reqwest::blocking::Client,
    headers: HeaderMap,
    api_url: &'a str,
}

impl<'a> Transcriber<'a> {
    /// Creates a new `Transcriber` instance.
    pub fn new(client: reqwest::blocking::Client, token: &str, api_url: &'a str) -> Self {
        let mut headers = HeaderMap::new();
        headers.insert(
            reqwest::header::AUTHORIZATION,
            HeaderValue::from_str(token).expect("api_token as str"),
        );
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );

        Self {
            client,
            headers,
            api_url,
        }
    }
    /// Sends an audio transcription request to a specified URL and retrieves the
    // transcription ID from the response.
    pub fn transcribe(&self, audio_url: &str) -> Result<String> {
        let data = json!({
            "audio_url": audio_url,
            "iab_categories": true,
            "entity_detection": true
        });
        let response = self
            .client
            .post(self.api_url)
            .headers(self.headers.clone())
            .json(&data)
            .send()
            .context("err posting to transcript endpoint")?;

        let parsed_json = response.json::<Value>().map_err(|e| {
            eprintln!("ERROR: could not read body of response: {}", e);
            e
        })?;

        parsed_json
            .get("id")
            .and_then(|v| v.as_str())
            .map(String::from)
            .ok_or_else(|| anyhow!("'id' key not found in response body: {:?}", parsed_json))
    }

    /// Polls the transcription service endpoint until the transcription is complete.
    pub fn wait_for_transcription(&self, transcript_id: &str) -> Result<()> {
        let polling_endpoint = format!(
            "{transcript_url}/{id}",
            transcript_url = self.api_url,
            id = transcript_id
        );
        loop {
            let transcript_res = self
                .client
                .get(&polling_endpoint)
                .headers(self.headers.clone())
                .send()
                .context("err get: transcript response")?;

            let transcript_data: Value = transcript_res
                .json()
                .context("could not read body of poll request")?;

            let status = transcript_data
                .get("status")
                .context("status not present")?;
            match status.as_str().context("status as str")? {
                "completed" => return write_to_file(transcript_id.as_ref(), &transcript_data),
                "error" => {
                    return Err(anyhow!(transcript_data
                        .get("error")
                        .context("error not present")?
                        .clone()));
                }
                _ => sleep(Duration::from_secs(10)),
            };
        }
    }
}

/// Writes the transcription data to a file.
fn write_to_file(transcription_id: &str, content: &Value) -> Result<()> {
    let pretty_json = serde_json::to_string_pretty(content)?;
    let file_name = format!("{}.json", transcription_id);
    let current_dir = env::current_dir()?;

    let file_path = current_dir.join(file_name);
    fs::write(file_path, pretty_json)?;
    Ok(())
}

/// Runs the transcription process.
pub fn run(token: &str, args: TranscriberArgs) -> Result<()> {
    let transcript_url =
        env::var("TRANSCRIPT_URL").map_err(|_| anyhow!("TRANSCRIPT_URL not set."))?;

    let client = Client::new();
    let transcriber = Transcriber::new(client, token, &transcript_url);

    // transcript ID - either passed in as an arg, or
    // we need to post recording, then get transcript to continue
    let mut t_id: Option<String> = args.transcript_id;

    if let Some(audio_url) = &args.audio_url {
        t_id = transcriber.transcribe(audio_url)?.into();
    }
    if let Some(transcript_id) = t_id {
        println!("Using transcript ID: {}", transcript_id);
        println!("waiting for transcription results...");
        transcriber.wait_for_transcription(&transcript_id)?;
    }
    Ok(())
}
