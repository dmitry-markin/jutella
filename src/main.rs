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
use futures::stream::StreamExt;
use jutella::{ChatClient, ChatClientConfig, Delta, TokenUsage};
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
        timeout,
        model,
        system_message,
        stream,
        xclip,
        show_token_usage,
        show_reasoning,
        min_history_tokens,
        max_history_tokens,
        verbosity,
    } = Configuration::init(Args::parse())?;

    let client = ChatClient::new(ChatClientConfig {
        auth,
        api_url,
        api_options,
        api_version,
        timeout,
        model,
        system_message,
        min_history_tokens,
        max_history_tokens,
        verbosity,
    })
    .context("Failed to initialize the client")?;

    let mut chat = Chat {
        client,
        show_reasoning,
        xclip,
        show_token_usage,
        stream,
    };

    chat.run().await
}

#[derive(Debug, Eq, PartialEq)]
enum DeltaType {
    Nothing,
    Reasoning,
    Content,
    Usage,
}

struct Chat {
    client: ChatClient,
    show_reasoning: bool,
    xclip: bool,
    show_token_usage: bool,
    stream: bool,
}

impl Chat {
    async fn handle_line(&mut self, line: String) -> anyhow::Result<()> {
        if let Ok(completion) = self
            .client
            .request_completion(line)
            .await
            .inspect_err(|e| print_error(e))
        {
            // `trim()` is needed for reasoning, because OpenRouter returns three empty lines in
            // the end.
            self.show_reasoning
                .then(|| completion.reasoning.map(|r| print_reasoning(r.trim())));

            print_response(&completion.response);

            if self.xclip {
                copy_to_clipboard(completion.response)
                    .inspect_err(|e| print_error(e))
                    .unwrap_or_default();
            }

            if self.show_token_usage {
                print_token_usage(completion.token_usage);
                println!("\n");
            }
        }

        print_prompt()?;

        Ok(())
    }

    async fn handle_line_streaming(&mut self, line: String) -> anyhow::Result<()> {
        if let Ok(mut stream) = self
            .client
            .stream_completion(line)
            .await
            .inspect_err(|e| print_error(e))
        {
            let mut response = String::new();
            let mut last_delta = DeltaType::Nothing;
            // CR user entered is one newline.
            let mut trailing_newlines = 1;

            while let Some(event) = stream.next().await {
                if let Ok(event) = event.inspect_err(|e| {
                    println!();
                    print_error(e);
                }) {
                    match event {
                        Delta::Reasoning(reasoning) => {
                            if last_delta != DeltaType::Reasoning {
                                last_delta = DeltaType::Reasoning;

                                if self.show_reasoning {
                                    print!("{} ", "\nReasoning:".bold().blue());
                                }
                            }

                            if self.show_reasoning {
                                print!("{}", reasoning);
                                trailing_newlines = count_trailing_newlines(reasoning);
                                io::stdout().flush()?;
                            }
                        }
                        Delta::Content(content) => {
                            if last_delta != DeltaType::Content {
                                last_delta = DeltaType::Content;

                                for _ in 0..2 - trailing_newlines {
                                    println!();
                                }

                                print!("{} ", "Assistant:".bold().green());
                            }

                            print!("{}", content);
                            response.push_str(&content);
                            io::stdout().flush()?;
                        }
                        Delta::Usage(usage) => {
                            last_delta = DeltaType::Usage;

                            if self.show_token_usage {
                                println!("\n");
                                print_token_usage(usage);
                            }
                        }
                    }
                }
            }

            println!("\n");

            if self.xclip {
                copy_to_clipboard(response)
                    .inspect_err(|e| print_error(e))
                    .unwrap_or_default();
            }
        }

        print_prompt()?;

        Ok(())
    }

    async fn run(&mut self) -> anyhow::Result<()> {
        print_prompt()?;

        for line in io::stdin().lines() {
            if self.stream {
                self.handle_line_streaming(line?).await?;
            } else {
                self.handle_line(line?).await?;
            }
        }

        println!();

        Ok(())
    }
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

fn print_token_usage(usage: TokenUsage) {
    let tokens_info = format!(
        "{} ({}) / {} ({})",
        usage.tokens_in,
        usage.tokens_in_cached.unwrap_or_default(),
        usage.tokens_out,
        usage.tokens_reasoning.unwrap_or_default(),
    );
    print!("{}", tokens_info.blue());
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

// Count up to two trailing newlines.
fn count_trailing_newlines(mut string: String) -> u8 {
    if string.pop() == Some('\n') {
        if string.pop() == Some('\n') {
            2
        } else {
            1
        }
    } else {
        0
    }
}
