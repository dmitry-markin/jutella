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

//! `jutella` CLI interface configuration.

use anyhow::{anyhow, Context as _};
use clap::{Parser, ValueEnum};
use dirs::home_dir;
use jutella::Auth;
use std::{fs, path::PathBuf};

const HOME_CONFIG_LOCATION: &str = ".config/jutella.toml";
const DEFAULT_ENDPOINT: &str = "https://api.openai.com/v1/";
const DEFAULT_MODEL: &str = "gpt-4o-mini";

/// API to use.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ApiType {
    /// OpenAI API (including Azure).
    #[clap(name = "openai")]
    OpenAi,
    /// OpenRouter API.
    #[clap(name = "openrouter")]
    OpenRouter,
}

impl From<ApiType> for jutella::ApiType {
    fn from(value: ApiType) -> Self {
        match value {
            ApiType::OpenAi => jutella::ApiType::OpenAi,
            ApiType::OpenRouter => jutella::ApiType::OpenRouter,
        }
    }
}

#[derive(Debug, Parser)]
#[command(version)]
#[command(about = "Chatbot API CLI. Supports OpenAI chat completions API, \
                   including OpenAI, Azure, and OpenRouter flavors.",
          long_about = None)]
#[command(after_help = "You can only set API key/token in the config. \
                        Command line options override the ones in the config.")]
pub struct Args {
    /// API flavor. Default: openai.
    #[arg(short, long, value_enum)]
    api: Option<ApiType>,

    /// Base API url. Default: "https://api.openai.com/v1/".
    #[arg(short = 'u', long)]
    api_url: Option<String>,

    /// API version GET parameter used by Azure.
    #[arg(long)]
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

    /// Show number of tokens used while generating the response.
    #[arg(short = 'g', long)]
    show_token_usage: bool,

    /// Reasoning effort. Typical values are: `minimal`, `low`, `medium`, or `high`.
    #[arg(short, long)]
    reasoning_effort: Option<String>,

    /// Verbosity of the answers. Typical values are: `low`, `medium`, or `high`.
    #[arg(short, long)]
    verbosity: Option<String>,

    /// Keep at least that many tokens in the conversation context.
    ///
    /// The context will be truncated to keep at least `min_history_tokens`, but
    /// no more than one request-response above this threshold, and under
    /// no circumstances more than `max_history_tokens`.
    /// This method of context truncation ensures that at least the latest round of
    /// messages is always kept (unless `max_history_tokens` kicks in).
    #[arg(short = 'n', long)]
    min_history_tokens: Option<usize>,

    /// Keep at most that many tokens in the conversation context.
    #[arg(short = 't', long)]
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
    api: Option<String>,
    api_version: Option<String>,
    api_key: Option<String>,
    api_token: Option<String>,
    model: Option<String>,
    system_message: Option<String>,
    min_history_tokens: Option<usize>,
    max_history_tokens: Option<usize>,
    xclip: Option<bool>,
    show_token_usage: Option<bool>,
    reasoning_effort: Option<String>,
    verbosity: Option<String>,
}

pub struct Configuration {
    pub api_url: String,
    pub api_type: jutella::ApiType,
    pub api_version: Option<String>,
    pub auth: Auth,
    pub model: String,
    pub system_message: Option<String>,
    pub min_history_tokens: Option<usize>,
    pub max_history_tokens: Option<usize>,
    pub xclip: bool,
    pub show_token_usage: bool,
    pub reasoning_effort: Option<String>,
    pub verbosity: Option<String>,
}

impl Configuration {
    pub fn init(args: Args) -> anyhow::Result<Self> {
        let Args {
            api_url,
            api,
            api_version,
            model,
            system_message,
            min_history_tokens,
            max_history_tokens,
            config,
            xclip,
            show_token_usage,
            reasoning_effort,
            verbosity,
        } = args;

        let config_path = config.ok_or(()).or_else(|()| {
            home_dir()
                .ok_or(anyhow!(
                    "Home dir missing, cannot read config from standard location"
                ))
                .map(|p| p.join(HOME_CONFIG_LOCATION))
        })?;

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

        let config_api_type = config
            .api
            .map(|api| ApiType::from_str(&api, true))
            .transpose()
            .map_err(|e| anyhow!("Invalid API type in config: {}", e))?;
        let api_type = api.or(config_api_type).unwrap_or(ApiType::OpenAi);

        let api_version = api_version.or(config.api_version);

        let model = model
            .or(config.model)
            .unwrap_or_else(|| String::from(DEFAULT_MODEL));

        let system_message = system_message.or(config.system_message);

        let min_history_tokens = min_history_tokens.or(config.min_history_tokens);
        let max_history_tokens = max_history_tokens.or(config.max_history_tokens);

        let xclip = xclip || config.xclip.unwrap_or_default();
        let show_token_usage = show_token_usage || config.show_token_usage.unwrap_or_default();

        let reasoning_effort = reasoning_effort.or(config.reasoning_effort);
        let verbosity = verbosity.or(config.verbosity);

        Ok(Self {
            api_url,
            api_type: api_type.into(),
            api_version,
            auth,
            model,
            system_message,
            min_history_tokens,
            max_history_tokens,
            xclip,
            show_token_usage,
            reasoning_effort,
            verbosity,
        })
    }
}
