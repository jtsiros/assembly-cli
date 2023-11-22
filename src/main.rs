use std::{env, fs, thread::sleep, time::Duration};

use anyhow::{anyhow, Context, Result};
use dotenv::dotenv;
use reqwest::{
    blocking::Client,
    header::{HeaderMap, HeaderValue},
};
use serde_json::{json, Value};
use structopt::StructOpt;

#[derive(StructOpt)]
struct Args {
    #[structopt(long = "recording-url")]
    audio_url: Option<String>,

    #[structopt(long = "transcript-id")]
    transcript_id: Option<String>,
}

fn main() -> Result<()> {
    dotenv().ok();
    // get audio_url argument to pass to assembly API
    let args = Args::from_args();

    let api_token = env::var("API_TOKEN").expect("API_TOKEN expected");
    let transcript_url = env::var("TRANSCRIPT_URL").expect("TRANSCRIPT_URL expected");

    let mut headers = HeaderMap::new();
    headers.insert(
        reqwest::header::AUTHORIZATION,
        HeaderValue::from_str(&api_token).expect("api_token as str"),
    );
    headers.insert(
        reqwest::header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );

    let client = Client::new();

    // transcript ID - either passed in as an arg, or
    // we need to post recording, then get transcript to continue
    let mut t_id: Option<String> = args.transcript_id;

    if let Some(audio_url) = &args.audio_url {
        t_id = Some(transcribe(&client, &headers, audio_url, &transcript_url)?);
    }
    if let Some(transcript_id) = t_id {
        println!("waiting for transcription to finish...");
        poll_for_completion(&client, transcript_id, &headers, transcript_url)?;
    }
    Ok(())
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
fn transcribe<S>(
    client: &reqwest::blocking::Client,
    headers: &HeaderMap,
    audio_url: S,
    url: S,
) -> Result<String>
where
    S: AsRef<str> + serde::ser::Serialize + reqwest::IntoUrl,
{
    let data = json!({
        "audio_url": audio_url,
        "iab_categories": true,
        "entity_detection": true
    });
    let response = client
        .post(url)
        .headers(headers.clone())
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

fn poll_for_completion<S: AsRef<str>>(
    client: &reqwest::blocking::Client,
    transcript_id: S,
    headers: &HeaderMap,
    transcript_url: S,
) -> Result<()> {
    let polling_endpoint = format!(
        "{transcript_url}/{id}",
        transcript_url = transcript_url.as_ref(),
        id = transcript_id.as_ref()
    );
    loop {
        let transcript_res = client
            .get(&polling_endpoint)
            .headers(headers.clone())
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

fn write_to_file(transcription_id: &str, content: &Value) -> Result<()> {
    let pretty_json = serde_json::to_string_pretty(content)?;
    let file_name = format!("{}.json", transcription_id);
    let current_dir = env::current_dir()?;

    let file_path = current_dir.join(file_name);
    fs::write(file_path, pretty_json)?;
    Ok(())
}
