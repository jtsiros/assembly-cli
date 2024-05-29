use std::path::Path;
use std::{env, fs, io};

use crate::cli::QuestionArgs;
use anyhow::anyhow;
use anyhow::Result;
use reqwest::{
    blocking::Client,
    header::{HeaderMap, HeaderValue},
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Deserialize, Serialize)]
pub struct Question {
    question: String,
    answer_format: Option<String>,
    answer_options: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct Answer {
    question: String,
    answer: String,
}

#[derive(Debug)]
/// A client for interacting with the AssemblyAI question and answer service.
pub struct QuestionAnswer<'a> {
    client: reqwest::blocking::Client,
    headers: HeaderMap,
    api_url: &'a str,
}

impl<'a> QuestionAnswer<'a> {
    /// Creates ja new `QuestionAnswer` instance.
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

    /// Sends a series of questions to and retrieves the answers.
    pub fn ask(&self, transcript_ids: Vec<String>, questions: Vec<Question>) -> Result<()> {
        let data = json!({
            "transcript_ids": transcript_ids,
            "questions": questions,
            "final_model": "basic",
        });

        let response = self
            .client
            .post(self.api_url)
            .headers(self.headers.clone())
            .json(&data)
            .send()?;

        let parsed_json = response.json::<Value>().map_err(|e| {
            eprintln!("Error: could not read body of response: {}", e);
            e
        })?;
        let answers = parsed_json
            .get("response")
            .ok_or_else(|| {
                anyhow!(
                    "Error: 'id' key not found in response body: {:?}",
                    parsed_json
                )
            })
            .and_then(|r| {
                serde_json::from_value::<Vec<Answer>>(r.clone()).map_err(|e| {
                    anyhow!("Failed to parse 'response' into slice of Responses: {}", e)
                })
            })?;

        for a in answers {
            println!("Question: {}", a.question);
            println!("Answer: {}", a.answer);
        }
        Ok(())
    }
}

pub fn run(token: &str, args: QuestionArgs) -> Result<()> {
    let api_url = env::var("QUESTION_URL").map_err(|_| anyhow!("QUESTION_URL not set."))?;

    let client = Client::new();
    let qa = QuestionAnswer::new(client, token, &api_url);
    let questions = read_questions_from_file(args.questions_file_path)?;
    qa.ask(args.transcript_id, questions)
}

/// Reads a series of questions from a JSON file.
fn read_questions_from_file(file_path: impl AsRef<Path>) -> Result<Vec<Question>, io::Error> {
    let file_content = fs::read_to_string(file_path.as_ref()).map_err(|e| {
        io::Error::new(
            e.kind(),
            format!("failed to open file {:#?}: {}", file_path.as_ref(), e),
        )
    })?;
    serde_json::from_str(&file_content).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("failed to parse JSON: {}", e),
        )
    })
}
