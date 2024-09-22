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

//! Chatbot API client.

use crate::chat_client::{
    context::Context,
    openai_api::{
        chat_completions::ChatCompletionsBody,
        client::{Auth, Error as OpenAiClientError, OpenAiClient},
        message::{self, AssistantMessage},
    },
};

/// Configuration for [`ChatClient`].
#[derive(Debug)]
pub struct ChatClientConfig {
    /// OpenAI chat API endpoint.
    pub api_url: String,
    /// API version.
    pub api_version: Option<String>,
    /// Model.
    pub model: String,
    /// System message to initialize the model.
    pub system_message: Option<String>,
    /// Max history tokens to keep in the conversation context.
    pub max_history_tokens: Option<usize>,
}

impl Default for ChatClientConfig {
    fn default() -> Self {
        Self {
            api_url: String::from("https://models.inference.ai.azure.com/"),
            api_version: None,
            model: String::from("gpt-4o-mini"),
            system_message: None,
            max_history_tokens: None,
        }
    }
}

/// Errors during interaction with a chatbot.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Error reported by the model API.
    #[error("OpenAI API client error: {0}")]
    OpenAiClient(#[from] OpenAiClientError),
    /// No choices returned.
    #[error("Response contains no choices")]
    NoChoices,
    /// No message.
    #[error("Response contains no message")]
    NoMessage,
    /// Message conversion error.
    #[error("Invalid message: {0}")]
    InvalidMessage(#[from] message::Error),
    /// No content.
    #[error("Assistant message contains no `content`")]
    NoContent,
    /// Refusal by the model.
    #[error("Model refused the request: \"{0}\"")]
    Refusal(String),
    /// Tokenizer initialization error
    #[error("Failed to initialize tokenizer: {0}")]
    TokenizerInit(String),
    /// No tokenizer setup.
    #[error("No tokenizer setup. This is a bug.")]
    NoTokenizer,
}

/// Chatbot API client.
pub struct ChatClient {
    client: OpenAiClient,
    model: String,
    context: Context,
    max_history_tokens: Option<usize>,
}

impl ChatClient {
    /// Create new [`ChatClient`] accessing OpenAI chat API with `api_key`.
    pub fn new(auth: Auth, config: ChatClientConfig) -> Result<Self, Error> {
        let ChatClientConfig {
            api_url,
            api_version,
            model,
            system_message,
            max_history_tokens,
        } = config;

        let api_url = ensure_trailing_slash(api_url);
        let context = create_context(system_message, max_history_tokens.is_some())?;

        Ok(Self {
            client: OpenAiClient::new(auth, api_url, api_version)?,
            model,
            context,
            max_history_tokens,
        })
    }

    /// Cretae new [`ChatClient`] accessing OpenAI chat API with preconfigured [`reqwest::Client`].
    ///
    /// Make sure to setup `Authorization:` header to `Bearer <api_key>"`.
    pub fn new_with_client(
        client: reqwest::Client,
        config: ChatClientConfig,
    ) -> Result<Self, Error> {
        let ChatClientConfig {
            api_url,
            api_version,
            model,
            system_message,
            max_history_tokens,
        } = config;

        let api_url = ensure_trailing_slash(api_url);
        let context = create_context(system_message, max_history_tokens.is_some())?;

        Ok(Self {
            client: OpenAiClient::new_with_client(client, api_url, api_version),
            model,
            context,
            max_history_tokens,
        })
    }

    /// Ask a new question, extending the chat context after a successful respone.
    pub async fn ask(&mut self, request: String) -> Result<String, Error> {
        let mut response = self
            .client
            .chat_completions(Self::body(
                self.model.clone(),
                &self.context,
                request.clone(),
            ))
            .await?;

        let choice = response.choices.pop().ok_or(Error::NoChoices)?;
        let assistant_message = AssistantMessage::try_from(choice.message)?;
        let answer = assistant_message.content.ok_or(
            assistant_message
                .refusal
                .map_or(Error::NoContent, Error::Refusal),
        )?;

        self.context.push(request, answer.clone());

        if let Some(max_tokens) = self.max_history_tokens {
            self.context
                .keep_recent(max_tokens)
                .map_err(|_| Error::NoTokenizer)?;
        }

        Ok(answer)
    }

    /// Construct a request body.
    fn body(model: String, context: &Context, request: String) -> ChatCompletionsBody {
        ChatCompletionsBody {
            model,
            messages: context.with_request(request).map(Into::into).collect(),
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

fn create_context(system_message: Option<String>, init_tokenizer: bool) -> Result<Context, Error> {
    let context = if init_tokenizer {
        Context::new_with_tokenizer(
            system_message,
            tiktoken_rs::o200k_base().map_err(|e| Error::TokenizerInit(format!("{e}")))?,
        )
    } else {
        Context::new(system_message)
    };

    Ok(context)
}
