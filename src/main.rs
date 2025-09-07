// Copyright (c) 2024 Dmitry Markin
//
// SPDX-License-Identifier: MIT
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

//! CLI interface for `jutella`.

mod app_config;
use app_config::{Args, Configuration};

use anyhow::{anyhow, Context as _};
use colored::Colorize as _;
use jutella::{ChatClient, ChatClientConfig};
use std::{
    io::{self, Read as _, Write as _},
    process::{Command, Stdio},
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let Configuration {
        auth,
        api_options,
        api_version,
        api_url,
        model,
        system_message,
        xclip,
        show_token_usage,
        show_reasoning,
        min_history_tokens,
        max_history_tokens,
        verbosity,
    } = Configuration::init(Args::parse())?;

    let mut chat = ChatClient::new(
        auth,
        ChatClientConfig {
            api_url,
            api_options,
            api_version,
            model,
            system_message,
            min_history_tokens,
            max_history_tokens,
            verbosity,
        },
    )
    .context("Failed to initialize the client")?;

    print_prompt()?;

    for line in io::stdin().lines() {
        if let Ok(completion) = chat
            .request_completion(line?)
            .await
            .inspect_err(|e| print_error(e))
        {
            // `trim()` is needed for reasoning, because OpenRouter returns three empty lines in
            // the end.
            show_reasoning.then(|| completion.reasoning.map(|r| print_reasoning(&r.trim())));

            print_response(&completion.response);

            if xclip {
                copy_to_clipboard(completion.response)
                    .inspect_err(|e| print_error(e))
                    .unwrap_or_default();
            }

            if show_token_usage {
                let tokens_info = format!(
                    "{} ({}) / {} ({})",
                    completion.tokens_in,
                    completion.tokens_in_cached.unwrap_or_default(),
                    completion.tokens_out,
                    completion.tokens_reasoning.unwrap_or_default(),
                );
                println!("{}\n", tokens_info.blue());
            }
        }

        print_prompt()?;
    }

    println!();

    Ok(())
}

fn print_prompt() -> Result<(), io::Error> {
    print!("{} ", "You:".bold().red());
    io::stdout().flush()
}

fn print_reasoning(reasoning: &str) {
    println!("\n{} {reasoning}", "Reasoning:".bold().blue());
}

fn print_response(response: &str) {
    println!("\n{} {response}\n", "Assistant:".bold().green());
}

fn print_error(e: impl ToString) {
    eprintln!("{} {}", "Error:".yellow(), e.to_string().yellow());
}

fn copy_to_clipboard(string: String) -> anyhow::Result<()> {
    let mut xclip = Command::new("xclip")
        .arg("-selection")
        .arg("clipboard")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to spawn `xclip`")?;

    let mut stdin = xclip.stdin.take().context("Failed to open `xclip` stdin")?;
    stdin
        .write_all(string.as_ref())
        .context("Failed to pass response via `xclip` stdin")?;
    drop(stdin);

    xclip
        .wait()
        .context("Failed to wait for `xclip`")?
        .success()
        .then_some(())
        .ok_or(())
        .or_else(|()| {
            let mut error = String::new();
            xclip
                .stderr
                .take()
                .context(anyhow!("Failed to open `xclip` stderr"))?
                .read_to_string(&mut error)?;

            Err(anyhow!("`xclip` returned an error: {}", error.trim()))
        })
}
