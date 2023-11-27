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
pub struct QuestionAnswer<S>
where
    S: AsRef<str>,
{
    client: reqwest::blocking::Client,
    headers: HeaderMap,
    api_url: S,
}

impl<S> QuestionAnswer<S>
where
    S: AsRef<str>,
{
    /// Creates a new `QuestionAnswer` instance.
    pub fn new<U: AsRef<str>>(client: reqwest::blocking::Client, token: U, api_url: S) -> Self {
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

    /// Sends a series of questions to and retrieves the answers.
    pub fn ask(&self, transcript_ids: Vec<String>, questions: Vec<Question>) -> Result<()> {
        let data = json!({
            "transcript_ids": transcript_ids,
            "questions": questions,
            "final_model": "basic",
        });

        let response = self
            .client
            .post(self.api_url.as_ref())
            .headers(self.headers.clone())
            .json(&data)
            .send()?;

        let parsed_json = response.json::<Value>().map_err(|e| {
            eprintln!("ERROR: could not read body of response: {}", e);
            e
        })?;
        let answers = parsed_json
            .get("response")
            .ok_or_else(|| anyhow!("'id' key not found in response body: {:?}", parsed_json))
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

pub fn run<S: AsRef<str>>(token: S, args: QuestionArgs) -> Result<()> {
    let client = Client::new();
    let api_url = env::var("QUESTION_URL").expect("TRANSCRIPT_URL expected");
    let qa = QuestionAnswer::new(client, token, api_url);
    let questions = read_questions_from_file(args.questions_file_path)?;
    qa.ask(args.transcript_id, questions)
}

/// Reads a series of questions from a JSON file.
fn read_questions_from_file(file_path: std::path::PathBuf) -> Result<Vec<Question>, io::Error> {
    let file_content = fs::read_to_string(file_path)?;
    // Deserialize the string to Vec<Question>
    let questions: Vec<Question> = serde_json::from_str(&file_content)?;
    Ok(questions)
}
