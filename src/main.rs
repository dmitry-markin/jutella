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
use base64::prelude::{Engine, BASE64_STANDARD};
use colored::Colorize as _;
use futures::stream::StreamExt;
use jutella::{
    ChatClient, ChatClientConfig, Content, ContentPart, Delta, FilePart, ImagePart, TokenUsage,
};
use std::{
    io::{self, Read as _, Write as _},
    path::Path,
    process::{Command, Stdio},
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let Configuration {
        auth,
        api_options,
        api_version,
        api_url,
        http_timeout,
        model,
        system_message,
        stream,
        xclip,
        xdg_open,
        show_token_usage,
        show_reasoning,
        min_history_tokens,
        max_history_tokens,
        verbosity,
        sanitize_links,
        extra_params,
    } = Configuration::init(Args::parse())?;

    let client = ChatClient::new(ChatClientConfig {
        auth,
        api_url,
        api_options,
        api_version,
        http_timeout,
        model,
        system_message,
        min_history_tokens,
        max_history_tokens,
        verbosity,
        sanitize_links,
        extra_params,
    })
    .context("Failed to initialize the client")?;

    let mut chat = Chat {
        client,
        show_reasoning,
        xclip,
        xdg_open,
        show_token_usage,
        stream,
        pending_attachments: Vec::new(),
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
    xdg_open: bool,
    show_token_usage: bool,
    stream: bool,
    pending_attachments: Vec<ContentPart>,
}

impl Chat {
    async fn handle_line(&mut self, line: String) -> anyhow::Result<()> {
        if let Some(path) = line.strip_prefix("#file:") {
            match attach_file(path) {
                Ok(attachment) => {
                    self.pending_attachments.push(attachment);
                    let message = format!("File attached: {path}");
                    println!("{}", message.blue());
                }
                Err(e) => {
                    print_error(e);
                }
            }

            print_prompt()?;
            return Ok(());
        }

        let content = {
            if self.pending_attachments.is_empty() {
                Content::Text(line)
            } else {
                let mut parts = std::mem::take(&mut self.pending_attachments);
                parts.push(ContentPart::Text(line));

                Content::ContentParts(parts)
            }
        };

        if self.stream {
            self.handle_completion_streaming(content).await
        } else {
            self.handle_completion(content).await
        }
    }

    async fn handle_completion(&mut self, request: Content) -> anyhow::Result<()> {
        if let Ok(completion) = self
            .client
            .request_completion(request)
            .await
            .inspect_err(|e| print_error(e))
        {
            // `trim()` is needed for reasoning, because OpenRouter returns three empty lines in
            // the end.
            self.show_reasoning
                .then(|| completion.reasoning.map(|r| print_reasoning(r.trim())));

            match completion.response {
                Content::Text(response) => {
                    print_response(&response);

                    if self.xclip {
                        copy_to_clipboard(response)
                            .inspect_err(|e| print_error(e))
                            .unwrap_or_default();
                    }
                }
                Content::ContentParts(parts) => {
                    let mut needs_leading_newline = true;
                    let mut needs_trailing_newline = false;

                    for part in parts {
                        match part {
                            ContentPart::Text(text) => {
                                print_response(&text);
                                needs_leading_newline = false;
                                needs_trailing_newline = false;
                            }
                            ContentPart::Image(ImagePart { url, detail: _ }) => {
                                if needs_leading_newline {
                                    println!();
                                    needs_leading_newline = false;
                                }

                                if let Err(e) = save_and_show_image(url, self.xdg_open) {
                                    print_error(e)
                                }

                                needs_trailing_newline = true;
                            }
                            ContentPart::File(_) => {
                                print_error("files in the response not supported, ignoring");
                            }
                        }
                    }

                    if needs_trailing_newline {
                        println!();
                    }
                }
            }

            if self.show_token_usage {
                print_token_usage(completion.token_usage);
                println!("\n");
            }
        }

        print_prompt()?;

        Ok(())
    }

    async fn handle_completion_streaming(&mut self, request: Content) -> anyhow::Result<()> {
        if let Ok(mut stream) = self
            .client
            .stream_completion(request)
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
            self.handle_line(line?).await?;
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

fn xdg_open(path: String) -> anyhow::Result<()> {
    let mut xdg_open = Command::new("xdg-open")
        .arg(path)
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to spawn `xdg-open`")?;

    xdg_open
        .wait()
        .context("Failed to wait for `xdg-open`")?
        .success()
        .then_some(())
        .ok_or(())
        .or_else(|()| {
            let mut error = String::new();
            xdg_open
                .stderr
                .take()
                .context(anyhow!("Failed to open `xdg-open` stderr"))?
                .read_to_string(&mut error)?;

            Err(anyhow!("`xdg-open` returned an error: {}", error.trim()))
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

fn attach_file(path: &str) -> anyhow::Result<ContentPart> {
    let (mime_type, is_pdf) = if path.ends_with(".pdf") {
        ("application/pdf", true)
    } else if path.ends_with(".jpg") || path.ends_with(".jpeg") {
        ("image/jpeg", false)
    } else if path.ends_with(".png") {
        ("image/png", false)
    } else if path.ends_with(".gif") {
        ("image/gif", false)
    } else if path.ends_with(".webp") {
        ("image/webp", false)
    } else {
        return Err(anyhow!("unsupported file extension"));
    };

    let filename = Path::new(path)
        .file_name()
        .and_then(|filename| filename.to_str().map(ToOwned::to_owned));
    let binary = std::fs::read(path).context("failed to read file")?;
    let base64_string = BASE64_STANDARD.encode(binary);
    let encoded_data = format!("data:{mime_type};base64,{base64_string}");

    if is_pdf {
        Ok(ContentPart::File(FilePart {
            file_data: encoded_data,
            filename,
        }))
    } else {
        Ok(ContentPart::Image(ImagePart {
            url: encoded_data,
            detail: None,
        }))
    }
}

fn extract_mime_type_and_base64(encoded_data: &str) -> Option<(&str, &str)> {
    let tail = encoded_data.strip_prefix("data:")?;
    let index = tail.find(';')?;
    let (mime_type, tail) = tail.split_at(index);
    let b64_data = tail.strip_prefix(";base64,")?;

    Some((mime_type, b64_data))
}

fn save_and_show_image(encoded_data: String, open: bool) -> anyhow::Result<()> {
    let Some((mime_type, base64_data)) = extract_mime_type_and_base64(&encoded_data) else {
        return Err(anyhow!("invalid image encoded data"));
    };

    let extension = match mime_type {
        "image/jpeg" => ".jpg",
        "image/png" => ".png",
        "image/gif" => ".gif",
        "image/webp" => ".webp",
        mime_type => return Err(anyhow!("unsupported image MIME-type `{}`", mime_type)),
    };

    let binary = BASE64_STANDARD
        .decode(base64_data)
        .context("invalid base64 data")?;

    let mut file = tempfile::Builder::new()
        .prefix("jutella-")
        .rand_bytes(5)
        .suffix(extension)
        .tempfile()
        .context("failed to create temporary file for image")?;

    file.write_all(&binary)?;
    let (_, path) = file.keep().context("failed to keep temporary file")?;

    let message = format!("File saved: {}", path.display());
    println!("{}", message.bold().blue());

    if open {
        xdg_open(
            path.into_os_string()
                .into_string()
                .map_err(|_| anyhow!("saved file path is not a utf-8 string"))?,
        )?;
    }

    Ok(())
}
