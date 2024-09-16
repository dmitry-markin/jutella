use anyhow::anyhow;
use colored::Colorize as _;
use std::io::{self, Write as _};
use unspoken::ChatClient;

fn main() -> anyhow::Result<()> {
    let api_key =
        std::env::var("OPENAI_API_KEY").map_err(|_| anyhow!("Please set `OPENAI_API_KEY` env."))?;
    let mut chat = ChatClient::new(api_key, Default::default());

    let you = "You:".bold().red();
    let assistant = "Assistant:".bold().green();

    print!("{} ", you);
    io::stdout().flush()?;

    for line in std::io::stdin().lines() {
        let response = chat.ask(line?)?;

        print!("\n{} {response}\n\n{} ", assistant, you);
        io::stdout().flush()?;
    }

    println!("");

    Ok(())
}
