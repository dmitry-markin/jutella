use anyhow::anyhow;
use colored::Colorize as _;
use dirs::home_dir;
use std::{
    env, fs,
    io::{self, Write as _},
};
use unspoken::ChatClient;

fn main() -> anyhow::Result<()> {
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
            anyhow!("Please set `OPENAI_API_KEY`, either via env or in $HOME/.config/unspoken.key")
        })?;

    let mut chat = ChatClient::new(api_key, Default::default());

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
