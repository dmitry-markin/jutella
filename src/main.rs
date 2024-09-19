// Copyright (c) 2024 `jutella` chatbot API client developers
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

use anyhow::Context as _;
use colored::Colorize as _;
use jutella::{ChatClient, ChatClientConfig};
use std::{
    io::{self, Write as _},
    process::{Command, Stdio},
};

fn main() -> anyhow::Result<()> {
    let Configuration {
        api_key,
        api_url,
        model,
        system_message,
        xclip,
    } = Configuration::init(Args::parse())?;

    let mut chat = ChatClient::new(
        api_key,
        ChatClientConfig {
            api_url,
            model,
            system_message,
        },
    );

    for line in io::stdin().lines() {
        print_prompt()?;

        if let Ok(response) = chat.ask(line?).inspect_err(|e| print_error(e)) {
            print_response(&response);

            if xclip {
                copy_to_clipboard(response)
                    .inspect_err(|e| print_error(e))
                    .unwrap_or_default();
            }
        }
    }

    println!();

    Ok(())
}

fn print_prompt() -> Result<(), io::Error> {
    print!("{} ", "You:".bold().red());
    io::stdout().flush()
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
        .stderr(Stdio::null())
        .spawn()
        .context("Failed to spawn `xclip`")?;

    let mut stdin = xclip.stdin.take().context("Failed to open `xclip` stdin")?;
    stdin
        .write_all(string.as_ref())
        .context("Failed to pass response via `xclip` stdin")?;
    drop(stdin);
    xclip.wait().context("`xclip` returned an error")?;

    Ok(())
}
