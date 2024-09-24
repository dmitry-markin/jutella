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
use jutella::Auth;
use std::{fs, path::PathBuf};

const HOME_CONFIG_LOCATION: &str = ".config/jutella.toml";
const DEFAULT_ENDPOINT: &str = "https://api.openai.com/v1/";
const DEFAULT_MODEL: &str = "gpt-4o-mini";

#[derive(Debug, Parser)]
#[command(version)]
#[command(about = "Chatbot API CLI. Currently supports OpenAI chat API.", long_about = None)]
#[command(after_help = "You can only set API key/token in config. \
                        Command line options override the ones from config.")]
pub struct Args {
    /// Base API url. Default: "https://api.openai.com/v1/".
    #[arg(short = 'u', long)]
    api_url: Option<String>,

    /// API version.
    #[arg(short, long)]
    api_version: Option<String>,

    /// Model. Default: "gpt-4o-mini". You likely need to include the version date.
    #[arg(short, long)]
    model: Option<String>,

    /// Optional system message to initialize the model. Example: "You are a helpful assistant."
    #[arg(short, long)]
    system_message: Option<String>,

    /// Config file location. Default: "$HOME/.config/jutella.toml".
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// Use `xclip` to copy every response to clipboard.
    #[arg(short, long)]
    xclip: bool,

    /// Keep at most that many tokens in the conversation context.
    #[arg(short, long)]
    max_history_tokens: Option<usize>,
}

impl Args {
    pub fn parse() -> Self {
        <Args as Parser>::parse()
    }
}

#[derive(Debug, serde::Deserialize)]
struct ConfigFile {
    api_url: Option<String>,
    api_version: Option<String>,
    api_key: Option<String>,
    api_token: Option<String>,
    model: Option<String>,
    system_message: Option<String>,
    max_history_tokens: Option<usize>,
    xclip: Option<bool>,
}

pub struct Configuration {
    pub api_url: String,
    pub api_version: Option<String>,
    pub auth: Auth,
    pub model: String,
    pub system_message: Option<String>,
    pub max_history_tokens: Option<usize>,
    pub xclip: bool,
}

impl Configuration {
    pub fn init(args: Args) -> anyhow::Result<Self> {
        let Args {
            api_url,
            api_version,
            model,
            system_message,
            max_history_tokens,
            config,
            xclip,
        } = args;

        let config_path = config
            .ok_or(())
            .or_else(|()| {
                home_dir().ok_or(anyhow!(
                    "Home dir missing, cannot read config from standard location"
                ))
            })?
            .join(HOME_CONFIG_LOCATION);

        let config = fs::read_to_string(config_path.clone()).with_context(|| {
            anyhow!(
                "Failed to read config file {}",
                config_path.to_str().unwrap_or_default()
            )
        })?;

        let config: ConfigFile = toml::from_str(&config).with_context(|| {
            anyhow!(
                "failed to parse config file {}",
                config_path.to_str().unwrap_or_default()
            )
        })?;

        let auth = match (config.api_token, config.api_key) {
            (Some(token), None) => Auth::Token(token),
            (None, Some(api_key)) => Auth::ApiKey(api_key),
            _ => {
                return Err(anyhow!(
                    "Exactly one of `api_key` or `api_token` must be set in config"
                ))
            }
        };

        let api_url = api_url
            .or(config.api_url)
            .unwrap_or_else(|| String::from(DEFAULT_ENDPOINT));

        let api_version = api_version.or(config.api_version);

        let model = model
            .or(config.model)
            .unwrap_or_else(|| String::from(DEFAULT_MODEL));

        let system_message = system_message.or(config.system_message);

        let max_history_tokens = max_history_tokens.or(config.max_history_tokens);

        let xclip = if xclip {
            true
        } else {
            config.xclip.unwrap_or_default()
        };

        Ok(Self {
            api_url,
            api_version,
            auth,
            model,
            system_message,
            max_history_tokens,
            xclip,
        })
    }
}
