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

//! `jutella` CLI interface configuration.

use anyhow::{anyhow, Context as _};
use clap::Parser;
use dirs::home_dir;
use std::{env, fs, io, path::PathBuf};

#[derive(Debug, Parser)]
#[command(version)]
#[command(about = "Chatbot API CLI. Currently supports OpenAI chat API.", long_about = None)]
#[command(after_help = "You can only set `api_key` in config. \
                        Command line options override the ones from config.")]
pub struct Args {
    /// API url. Default: "https://models.inference.ai.azure.com/".
    #[arg(short, long)]
    url: Option<String>,

    /// Model. Default: "gpt-4o-mini".
    #[arg(short, long)]
    model: Option<String>,

    /// Optional system message to initialize the model. Example: "You are a helpful assistant."
    #[arg(short, long)]
    system: Option<String>,

    /// Config file location. Default: "$HOME/.config/jutella.toml".
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// Use `xclip` to copy every response to clipboard.
    #[arg(short, long)]
    xclip: bool,
}

impl Args {
    pub fn parse() -> Self {
        <Args as Parser>::parse()
    }
}

#[derive(Debug, serde::Deserialize)]
struct ConfigFile {
    api_key: Option<String>,
    url: Option<String>,
    model: Option<String>,
    system_message: Option<String>,
    xclip: Option<bool>,
}

pub struct Configuration {
    pub api_key: String,
    pub api_url: String,
    pub model: String,
    pub system_message: Option<String>,
    pub xclip: bool,
}

impl Configuration {
    pub fn init(args: Args) -> anyhow::Result<Self> {
        let Args {
            url,
            model,
            system,
            config,
            xclip,
        } = args;

        let config: Option<ConfigFile> = if let Some(config_path) = config {
            // Try reading CLI-provided config file first.
            Some(
                toml::from_str(&fs::read_to_string(config_path.clone()).with_context(|| {
                    anyhow!(
                        "Failed to read config file {}",
                        config_path
                            .to_str()
                            .expect("to have only unicode characters in path")
                    )
                })?)
                .context("Failed to parse config file {config_path}")?,
            )
        } else {
            // If there is $HOME, try reading config from standard path.
            if let Some(config_path) = home_dir().map(|home| home.join(".config/jutella.toml")) {
                match fs::read_to_string(config_path.clone()) {
                    Ok(string) => Ok(toml::from_str(&string).with_context(|| {
                        anyhow!(
                            "Failed to parse config file {}",
                            config_path
                                .to_str()
                                .expect("to have only unicode characters in path")
                        )
                    })?),
                    Err(error) => match error.kind() {
                        // Missing config in $HOME is not an error.
                        io::ErrorKind::NotFound => Ok(None),
                        _ => Err(error).context("Failed to read config file {config_path}"),
                    },
                }?
            } else {
                None
            }
        };

        let api_key = env::var("OPENAI_API_KEY").or_else(|_| {
            config
                .as_ref()
                .and_then(|c| c.api_key.clone())
                .ok_or(anyhow!(
                    "Set `api_key` in config. You can also set `OPENAI_API_KEY` env \
                     if you know what you are doing."
                ))
        })?;

        let api_url = url
            .or_else(|| config.as_ref().and_then(|c| c.url.clone()))
            .unwrap_or_else(|| String::from("https://models.inference.ai.azure.com/"));

        let model = model
            .or_else(|| config.as_ref().and_then(|c| c.model.clone()))
            .unwrap_or_else(|| String::from("gpt-4o-mini"));

        let system_message =
            system.or_else(|| config.as_ref().and_then(|c| c.system_message.clone()));

        let xclip = if xclip {
            true
        } else {
            config.as_ref().and_then(|c| c.xclip).unwrap_or_default()
        };

        Ok(Self {
            api_key,
            api_url,
            model,
            system_message,
            xclip,
        })
    }
}
