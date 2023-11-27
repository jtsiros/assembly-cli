use clap::Parser;

#[derive(Parser)]
#[command(bin_name = "assembly-cli")]
pub enum AssemblyCLI {
    Transcribe(TranscriberArgs),
    Question(QuestionArgs),
}

#[derive(Debug, clap::Args)]
#[command(
    name = "transcribe",
    author,
    about,
    long_about = "Sends an audio transcription request to a specified URL and retrieves the transcription ID from the response."
)]
pub struct TranscriberArgs {
    #[arg(short, long)]
    pub audio_url: Option<String>,
    #[arg(short, long)]
    pub transcript_id: Option<String>,
}

#[derive(Debug, clap::Args)]
#[command(
    name = "question",
    about,
    long_about = "Sends a series of questions to and retrieves the answers."
)]
pub struct QuestionArgs {
    #[arg(short, long)]
    pub questions_file_path: std::path::PathBuf,
    #[arg(short, long)]
    pub transcript_id: Vec<String>,
}
