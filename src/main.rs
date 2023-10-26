use dotenv::dotenv;
use serde_json::{json, Value};
use std::{env, process, thread::sleep, time::Duration};
use structopt::StructOpt;

use reqwest::{
    blocking::Client,
    header::{HeaderMap, HeaderValue},
};

#[derive(StructOpt)]
struct Args {
    #[structopt(long = "recording-url")]
    audio_url: Option<String>,

    #[structopt(long = "transcript-id")]
    transcript_id: Option<String>,
}

fn main() {
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
        let data = json!({
            "audio_url": audio_url,
            "iab_categories": true,
            "entity_detection": true
        });
        let response = client
            .post(&transcript_url)
            .headers(headers.clone())
            .json(&data)
            .send()
            .expect("err posting to transcript endpoint");

        t_id = response
            .json::<Value>()
            .expect("could not read body of response")
            .get("id")
            .and_then(|v| v.as_str())
            .map(String::from);
    }
    if let Some(transcript_id) = t_id {
        poll_for_completion(&client, transcript_id, &headers, transcript_url);
    } else {
        eprintln!("no transcript id present for request");
        process::exit(1);
    }
}

fn poll_for_completion<S: AsRef<str>>(
    client: &reqwest::blocking::Client,
    transcript_id: S,
    headers: &HeaderMap,
    transcript_url: S,
) {
    let polling_endpoint = format!("{}/{}", transcript_url.as_ref(), transcript_id.as_ref());
    loop {
        let transcript_res = client
            .get(&polling_endpoint)
            .headers(headers.clone())
            .send()
            .expect("err get: transcript response");

        let transcript_data: Value = transcript_res
            .json()
            .expect("could not read body of poll request");

        let status = transcript_data.get("status").expect("status not present");
        match status.as_str().expect("status as str") {
            "completed" => {
                println!("{}", transcript_data);
                break;
            }
            "error" => {
                eprintln!(
                    "error response: {}",
                    transcript_data.get("error").expect("error not present")
                );
                break;
            }
            _ => sleep(Duration::from_secs(10)),
        };
    }
}
