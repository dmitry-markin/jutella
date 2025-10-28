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
    error::Error,
    openai_api::{
        chat_completions::{ChatCompletionsBody, OpenRouterReasoning, StreamOptions, Usage},
        client::{Auth, OpenAiClient, OpenAiClientConfig},
        message::AssistantMessage,
    },
    stream::CompletionStream,
};
use eventsource_stream::{Event, EventStreamError};
use futures::stream::Stream;
use std::{sync::Arc, time::Duration};

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
    pub http_timeout: Duration,
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
            http_timeout: Duration::from_secs(300),
            model: String::from("gpt-4o-mini"),
            system_message: None,
            min_history_tokens: None,
            max_history_tokens: None,
            verbosity: None,
        }
    }
}

/// Token usage info.
#[derive(Debug)]
pub struct TokenUsage {
    /// Input tokens used.
    pub tokens_in: usize,
    /// Cached input tokens, if returned by the API.
    pub tokens_in_cached: Option<usize>,
    /// Output tokens used.
    pub tokens_out: usize,
    /// Reasoning tokens used, if returned by the API.
    pub tokens_reasoning: Option<usize>,
}

impl From<Usage> for TokenUsage {
    fn from(usage: Usage) -> Self {
        Self {
            tokens_in: usage.prompt_tokens,
            tokens_in_cached: usage.prompt_tokens_details.and_then(|d| d.cached_tokens),
            tokens_out: usage.completion_tokens,
            tokens_reasoning: usage
                .completion_tokens_details
                .and_then(|d| d.reasoning_tokens),
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
    /// Token usage.
    pub token_usage: TokenUsage,
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

    /// Create new [`ChatClient`] accessing OpenAI chat API sharing existing [`reqwest::Client`].
    pub fn new_with_client(
        config: ChatClientConfig,
        client: reqwest::Client,
    ) -> Result<Self, Error> {
        let tokenizer =
            tiktoken_rs::o200k_base().map_err(|e| Error::TokenizerInit(format!("{e}")))?;

        Self::new_with_client_and_tokenizer(config, client, Arc::new(tokenizer))
    }

    /// Create new [`ChatClient`] accessing OpenAI chat API sharing existing [`reqwest::Client`]
    /// and tokenizer.
    ///
    /// Sharing tokenizer between multiple chat instances helps reduce memory footprint (every
    /// tokenizer instance uses ~50MiB of RAM).
    pub fn new_with_client_and_tokenizer(
        config: ChatClientConfig,
        client: reqwest::Client,
        tokenizer: Arc<tiktoken_rs::CoreBPE>,
    ) -> Result<Self, Error> {
        let ChatClientConfig {
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
        } = config;

        let client = OpenAiClient::new(OpenAiClientConfig {
            client,
            auth,
            base_url: ensure_trailing_slash(api_url),
            api_version,
            timeout: http_timeout,
        })?;

        let context = if min_history_tokens.is_some() || max_history_tokens.is_some() {
            Context::new_with_rolling_window(
                system_message,
                tokenizer,
                min_history_tokens,
                max_history_tokens,
            )
        } else {
            Context::new(system_message)
        };

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
                false,
            ))
            .await?;

        let choice = completion.choices.pop().ok_or(Error::NoChoices)?;
        let assistant_message = AssistantMessage::try_from(choice.message)?;
        let response = assistant_message.content.ok_or(
            assistant_message
                .refusal
                .map_or(Error::NoContent, Error::Refusal),
        )?;

        // TODO: we likely need to report tokens used in case of errors as well.

        self.extend_context(request, response.clone());

        Ok(Completion {
            response,
            reasoning: assistant_message.reasoning,
            token_usage: completion.usage.into(),
        })
    }

    /// Stream completion, extending the chat context on success.
    pub async fn stream_completion<'a>(
        &'a mut self,
        request: String,
    ) -> Result<
        CompletionStream<'a, impl Stream<Item = Result<Event, EventStreamError<reqwest::Error>>>>,
        Error,
    > {
        let stream = self
            .client
            .chat_completions_stream(Self::body(
                self.model_config.clone(),
                &self.context,
                request.clone(),
                true,
            ))
            .await?;

        Ok(CompletionStream::new(self, stream, request))
    }

    pub(crate) fn extend_context(&mut self, request: String, response: String) {
        self.context.push(request, response);
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
        stream: bool,
    ) -> ChatCompletionsBody {
        ChatCompletionsBody {
            model,
            messages: context.with_request(request).map(Into::into).collect(),
            reasoning_effort: api_options.as_openai_reasoning_effort(),
            reasoning: api_options.as_openrouter_reasoning_settings(),
            verbosity,
            stream: Some(stream),
            stream_options: stream.then_some(StreamOptions {
                include_obfuscation: None,
                include_usage: Some(true),
            }),
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
