use std::env;

use anyhow::Result;
use dotenv::dotenv;
use reqwest::blocking::Client;
use structopt::StructOpt;
use transcribe::Transcriber;

mod transcribe;

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

    let client = Client::new();
    let transcriber = Transcriber::new(client, &api_token, &transcript_url);

    // transcript ID - either passed in as an arg, or
    // we need to post recording, then get transcript to continue
    let mut t_id: Option<String> = args.transcript_id;

    if let Some(audio_url) = &args.audio_url {
        t_id = Some(transcriber.transcribe(audio_url, &transcript_url)?);
    }
    if let Some(transcript_id) = t_id {
        println!("waiting for transcription to finish...");
        transcriber.wait_for_transcription(&transcript_id)?;
    }
    Ok(())
}
