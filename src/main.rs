use std::env;

use anyhow::Result;
use clap::Parser;
use cli::AssemblyCLI;
use dotenv::dotenv;

mod cli;
mod question_answer;
mod transcribe;

fn main() -> Result<()> {
    dotenv().ok();
    let api_token = env::var("API_TOKEN").expect("API_TOKEN expected");

    match AssemblyCLI::parse() {
        AssemblyCLI::Transcribe(args) => transcribe::run(api_token, args),
        AssemblyCLI::Question(args) => question_answer::run(api_token, args),
    }
}
