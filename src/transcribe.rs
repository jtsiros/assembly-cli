use std::{env, fs, thread::sleep, time::Duration};

use anyhow::{anyhow, Context, Result};
use reqwest::{
    blocking::Client,
    header::{HeaderMap, HeaderValue},
};
use serde_json::{json, Value};

use crate::cli::TranscriberArgs;

/// A client for interacting with the AssemblyAI transcription service.
pub struct Transcriber<S>
where
    S: AsRef<str>,
{
    client: reqwest::blocking::Client,
    headers: HeaderMap,
    api_url: S,
}

impl<S> Transcriber<S>
where
    S: AsRef<str>,
{
    /// Creates a new `Transcriber` instance.
    pub fn new(client: reqwest::blocking::Client, token: S, api_url: S) -> Self {
        let mut headers = HeaderMap::new();
        headers.insert(
            reqwest::header::AUTHORIZATION,
            HeaderValue::from_str(token.as_ref()).expect("api_token as str"),
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
    /// Sends an audio transcription request to a specified URL and retrieves the transcription ID from the response.
    ///
    /// # Parameters
    /// - `client`: A reference to a `reqwest::blocking::Client` used to make the HTTP request.
    /// - `headers`: A reference to a `HeaderMap` containing HTTP headers for the request.
    /// - `audio_url`: A URL (as a string or a type that can be converted into a URL) pointing to the audio file to be transcribed.
    /// - `url`: The URL of the transcription service endpoint.
    ///
    /// # Type Parameters
    /// - `S`: A generic type parameter constrained to types that can be referenced as a string (`AsRef<str>`),
    ///        serialized with `serde`, and converted into a URL (`reqwest::IntoUrl`).
    ///
    /// # Returns
    /// - `Result<String>`: On success, contains a `String` representing the transcription ID. On failure,
    ///                     returns an error in line with `reqwest` and standard I/O error handling.
    ///
    /// # Errors
    /// - Network or server-related errors encountered by `reqwest`.
    /// - JSON parsing errors when processing the response.
    /// - An error if the response body cannot be read or if the expected `id` key is not found in the JSON response.
    ///   The specific error is an `io::Error` with `ErrorKind::NotFound` and a custom error message detailing
    ///   the missing `id`.
    ///
    pub fn transcribe(&self, audio_url: S, url: S) -> Result<String>
    where
        S: AsRef<str> + serde::ser::Serialize + reqwest::IntoUrl,
    {
        let data = json!({
            "audio_url": audio_url,
            "iab_categories": true,
            "entity_detection": true
        });
        let response = self
            .client
            .post(url)
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
    pub fn wait_for_transcription(&self, transcript_id: S) -> Result<()> {
        let polling_endpoint = format!(
            "{transcript_url}/{id}",
            transcript_url = self.api_url.as_ref(),
            id = transcript_id.as_ref()
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
pub fn run<S: AsRef<str>>(token: S, args: TranscriberArgs) -> Result<()> {
    let transcript_url = env::var("TRANSCRIPT_URL").expect("TRANSCRIPT_URL expected");

    let client = Client::new();
    let transcriber = Transcriber::new(client, token.as_ref(), &transcript_url);

    // transcript ID - either passed in as an arg, or
    // we need to post recording, then get transcript to continue
    let mut t_id: Option<String> = args.transcript_id;

    if let Some(audio_url) = &args.audio_url {
        t_id = transcriber.transcribe(audio_url, &transcript_url)?.into();
    }
    if let Some(transcript_id) = t_id {
        println!("Using transcript ID: {}", transcript_id);
        println!("waiting for transcription results...");
        transcriber.wait_for_transcription(&transcript_id)?;
    }
    Ok(())
}
