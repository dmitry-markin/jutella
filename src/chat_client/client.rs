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

//! Chatbot API client.

use crate::chat_client::{
    context::Context,
    openai_api::{
        chat_completions::{ChatCompletionsBody, OpenRouterReasoning},
        client::{Auth, Error as OpenAiClientError, OpenAiClient, OpenAiClientConfig},
        message::{self, AssistantMessage},
    },
};
use std::time::Duration;

/// OpenRouter reasoning settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReasoningSettings {
    /// Rasoning effort. Typically one of `minimal`, `low`, `medium`, or `high`.
    Effort(String),
    /// Reasoning budget in tokens.
    Budget(i64),
}

/// API specific options.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApiOptions {
    /// OpenAI API.
    OpenAi {
        /// Reasoning effort. Typically one of `minimal`, `low`, `medium`, or `high`.
        reasoning_effort: Option<String>,
    },
    /// OpenRouter API.
    OpenRouter {
        /// Reasoning settings.
        reasoning: Option<ReasoningSettings>,
    },
}

impl ApiOptions {
    /// Check if the API type is OpenAI.
    pub fn as_openai_reasoning_effort(&self) -> Option<String> {
        match self {
            ApiOptions::OpenAi { reasoning_effort } => reasoning_effort.clone(),
            _ => None,
        }
    }
    /// Check if the API type is OpenRouter.
    pub fn as_openrouter_reasoning_settings(&self) -> Option<OpenRouterReasoning> {
        match self {
            ApiOptions::OpenRouter { reasoning } => reasoning.as_ref().map(|r| match r {
                ReasoningSettings::Effort(e) => OpenRouterReasoning::from_effort(e.clone()),
                ReasoningSettings::Budget(b) => OpenRouterReasoning::from_budget(*b),
            }),
            _ => None,
        }
    }
}

/// Configuration for [`ChatClient`].
#[derive(Debug)]
pub struct ChatClientConfig {
    /// Authentication token/key.
    pub auth: Auth,
    /// OpenAI chat API endpoint.
    pub api_url: String,
    /// API type.
    pub api_options: ApiOptions,
    /// API version.
    pub api_version: Option<String>,
    /// HTTP request timeout.
    pub timeout: Duration,
    /// Model.
    pub model: String,
    /// System message to initialize the model.
    pub system_message: Option<String>,
    /// Min history tokens to keep in the conversation context.
    ///
    /// The context will be truncated to keep at least `min_history_tokens`, but
    /// no more than one request-response above this threshold, and under
    /// no circumstances more than `max_history_tokens`.
    /// This method of context truncation ensures that at least the latest
    /// round of messages is always kept (unless `max_history_tokens` kicks in).
    pub min_history_tokens: Option<usize>,
    /// Max history tokens to keep in the conversation context.
    pub max_history_tokens: Option<usize>,
    /// Verbosity of the answers. Passed as is to the API.
    ///
    /// Typical values are: `low`, `medium`, and `high`.
    pub verbosity: Option<String>,
}

impl ChatClientConfig {
    /// Create default config with given authentication parameters.
    pub fn default_with_auth(auth: Auth) -> Self {
        Self {
            auth,
            api_url: String::from("https://api.openai.com/v1/"),
            api_options: ApiOptions::OpenAi {
                reasoning_effort: None,
            },
            api_version: None,
            timeout: Duration::from_secs(300),
            model: String::from("gpt-4o-mini"),
            system_message: None,
            min_history_tokens: None,
            max_history_tokens: None,
            verbosity: None,
        }
    }
}

/// Generated completion.
#[derive(Debug)]
pub struct Completion {
    /// Generated response.
    pub response: String,
    /// Reasoning performed by the model.
    pub reasoning: Option<String>,
    /// Input tokens used.
    pub tokens_in: usize,
    /// Cached input tokens, if returned by the API.
    pub tokens_in_cached: Option<usize>,
    /// Output tokens used.
    pub tokens_out: usize,
    /// Reasoning tokens used, if returned by the API.
    pub tokens_reasoning: Option<usize>,
}

/// Errors during interaction with a chatbot.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Error reported by the model API.
    #[error("API error: {0}")]
    OpenAiClient(#[from] OpenAiClientError),
    /// The response contains no completion choices.
    #[error("Response contains no choices")]
    NoChoices,
    /// The response contains no message.
    #[error("Response contains no message")]
    NoMessage,
    /// Message conversion error.
    #[error("Invalid message: {0}")]
    InvalidMessage(#[from] message::Error),
    /// The completion response message contains no `content`.
    #[error("Assistant message contains no `content`")]
    NoContent,
    /// Model refused the request.
    #[error("Model refused the request: \"{0}\"")]
    Refusal(String),
    /// Tokenizer initialization error.
    #[error("Failed to initialize tokenizer: {0}")]
    TokenizerInit(String),
}

/// Model configuration.
#[derive(Debug, Clone)]
pub struct ModelConfig {
    pub model: String,
    pub api_options: ApiOptions,
    pub verbosity: Option<String>,
}

/// Chatbot API client.
pub struct ChatClient {
    client: OpenAiClient,
    model_config: ModelConfig,
    context: Context,
}

impl ChatClient {
    /// Create new [`ChatClient`] accessing OpenAI chat API.
    pub fn new(config: ChatClientConfig) -> Result<Self, Error> {
        Self::new_with_client(config, reqwest::Client::new())
    }

    /// Cretae new [`ChatClient`] accessing OpenAI chat API with preconfigured [`reqwest::Client`].
    ///
    /// Make sure to setup a header `Authorization: Bearer {api_key}` if using OpenAI endpoints,
    /// or `api-key: {api_key}` header if using Azure endpoints.
    pub fn new_with_client(
        config: ChatClientConfig,
        client: reqwest::Client,
    ) -> Result<Self, Error> {
        let ChatClientConfig {
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
        } = config;

        let client = OpenAiClient::new(OpenAiClientConfig {
            client,
            auth,
            base_url: ensure_trailing_slash(api_url),
            api_version,
            timeout,
        })?;
        let context = create_context(system_message, min_history_tokens, max_history_tokens)?;

        Ok(Self {
            client,
            model_config: ModelConfig {
                model,
                api_options,
                verbosity,
            },
            context,
        })
    }

    /// Ask a new question, extending the chat context after a successful respone.
    pub async fn ask(&mut self, request: String) -> Result<String, Error> {
        self.request_completion(request).await.map(|c| c.response)
    }

    /// Request completion, extending the chat context after a successful respone.
    pub async fn request_completion(&mut self, request: String) -> Result<Completion, Error> {
        let mut completion = self
            .client
            .chat_completions(Self::body(
                self.model_config.clone(),
                &self.context,
                request.clone(),
            ))
            .await?;

        let choice = completion.choices.pop().ok_or(Error::NoChoices)?;
        let assistant_message = AssistantMessage::try_from(choice.message)?;
        let response = assistant_message.content.ok_or(
            assistant_message
                .refusal
                .map_or(Error::NoContent, Error::Refusal),
        )?;

        // TODO: we likely need to count tokens used in case of errors as well.

        self.context.push(request, response.clone());

        Ok(Completion {
            response,
            reasoning: assistant_message.reasoning,
            tokens_in: completion.usage.prompt_tokens,
            tokens_in_cached: completion
                .usage
                .prompt_tokens_details
                .and_then(|d| d.cached_tokens),
            tokens_out: completion.usage.completion_tokens,
            tokens_reasoning: completion
                .usage
                .completion_tokens_details
                .and_then(|d| d.reasoning_tokens),
        })
    }

    /// Construct a request body.
    fn body(
        ModelConfig {
            model,
            api_options,
            verbosity,
        }: ModelConfig,
        context: &Context,
        request: String,
    ) -> ChatCompletionsBody {
        ChatCompletionsBody {
            model,
            messages: context.with_request(request).map(Into::into).collect(),
            reasoning_effort: api_options.as_openai_reasoning_effort(),
            reasoning: api_options.as_openrouter_reasoning_settings(),
            verbosity,
            ..Default::default()
        }
    }
}

fn ensure_trailing_slash(url: String) -> String {
    if url.ends_with('/') {
        url
    } else {
        url + "/"
    }
}

fn create_context(
    system_message: Option<String>,
    min_history_tokens: Option<usize>,
    max_history_tokens: Option<usize>,
) -> Result<Context, Error> {
    let context = if min_history_tokens.is_some() || max_history_tokens.is_some() {
        Context::new_with_rolling_window(
            system_message,
            tiktoken_rs::o200k_base().map_err(|e| Error::TokenizerInit(format!("{e}")))?,
            min_history_tokens,
            max_history_tokens,
        )
    } else {
        Context::new(system_message)
    };

    Ok(context)
}
