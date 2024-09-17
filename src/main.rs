use anyhow::anyhow;
use clap::Parser;
use colored::Colorize as _;
use dirs::home_dir;
use std::{
    env, fs,
    io::{self, Write as _},
};
use unspoken::{ChatClient, ChatClientConfig};

/// OpenAI chat API command line client.
#[derive(Debug, clap::Parser)]
#[command(version, about, long_about = None)]
struct Args {
    /// API url.
    #[arg(short, long, default_value_t = String::from("https://models.inference.ai.azure.com/"))]
    url: String,

    /// Model.
    #[arg(short, long, default_value_t = String::from("gpt-4o"))]
    model: String,

    /// System message to initialize the model. Example: "You are a helpful assistant."
    #[arg(short, long)]
    system: Option<String>,
}

struct Configuration {
    api_key: String,
    api_url: String,
    model: String,
    system_message: Option<String>,
}

impl Configuration {
    fn init(args: Args) -> anyhow::Result<Self> {
        let api_key = env::var("OPENAI_API_KEY")
            .or_else(|_| {
                home_dir()
                    .map(|home| {
                        fs::read_to_string(home.join(".config/unspoken.key"))
                            .map(|key| key.trim().to_owned())
                            .ok()
                    })
                    .flatten()
                    .ok_or("no home or can not read key file")
            })
            .map_err(|_| {
                anyhow!(
                    "Please set `OPENAI_API_KEY`, either via env or in $HOME/.config/unspoken.key"
                )
            })?;

        let Args { url, model, system } = args;

        Ok(Self {
            api_key,
            api_url: url,
            model,
            system_message: system,
        })
    }
}

fn main() -> anyhow::Result<()> {
    let Configuration {
        api_key,
        api_url,
        model,
        system_message,
    } = Configuration::init(Args::parse())?;

    let mut chat = ChatClient::new(
        api_key,
        ChatClientConfig {
            api_url,
            model,
            system_message,
        },
    );

    let you = "You:".bold().red();
    let assistant = "Assistant:".bold().green();

    print!("{} ", you);
    io::stdout().flush()?;

    for line in std::io::stdin().lines() {
        match chat.ask(line?) {
            Ok(response) => {
                print!("\n{} {response}\n\n{} ", assistant, you);
            }
            Err(e) => {
                eprintln!("{} {}", "Error:".yellow(), e.to_string().yellow());
                print!("{} ", you);
            }
        }
        io::stdout().flush()?;
    }

    println!("");

    Ok(())
}
