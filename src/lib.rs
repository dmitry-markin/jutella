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

//! Chatbot API client library.

#![warn(missing_docs)]

use openai_api_rust::{
    chat::{ChatApi, ChatBody},
    Auth, OpenAI,
};

mod context;
use context::Context;

/// Configuration for [`ChatClient`].
#[derive(Debug)]
pub struct ChatClientConfig {
    /// OpenAI chat API endpoint.
    pub api_url: String,
    /// Model.
    pub model: String,
    /// System message to initialize the model.
    pub system_message: Option<String>,
}

impl Default for ChatClientConfig {
    fn default() -> Self {
        Self {
            api_url: String::from("https://models.inference.ai.azure.com/"),
            model: String::from("gpt-4o"),
            system_message: None,
        }
    }
}

/// Errors during interaction with a chatbot.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Error reported by the model API.
    #[error("API error: {0}")]
    ApiError(String),
    /// Web request error.
    #[error("Request error: {0}")]
    RequestError(String),
    /// Unexpected/missing data in the response.
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
}

impl From<openai_api_rust::Error> for Error {
    fn from(error: openai_api_rust::Error) -> Self {
        match error {
            openai_api_rust::Error::ApiError(e) => Error::ApiError(e),
            openai_api_rust::Error::RequestError(e) => Error::RequestError(e),
        }
    }
}

/// Chatbot API client.
pub struct ChatClient {
    openai: OpenAI,
    model: String,
    context: Context,
}

impl ChatClient {
    /// Create new [`ChatClient`] accessing OpenAI chat API with `auth_token`.
    pub fn new(auth_token: String, config: ChatClientConfig) -> Self {
        let ChatClientConfig {
            api_url,
            model,
            system_message,
        } = config;

        let api_url = if api_url.ends_with('/') {
            api_url
        } else {
            api_url + "/"
        };

        Self {
            openai: OpenAI::new(Auth::new(&auth_token), &api_url),
            model,
            context: Context::new(system_message),
        }
    }

    /// Ask a new question, extending the chat context after a successful respone.
    pub fn ask(&mut self, request: String) -> Result<String, Error> {
        let response = self.openai.chat_completion_create(&Self::body(
            self.model.clone(),
            &self.context,
            request.clone(),
        ))?;

        let choice = response
            .choices
            .first()
            .ok_or(Error::InvalidResponse(String::from("No choices returned")))?;

        let answer = choice
            .message
            .as_ref()
            .ok_or(Error::InvalidResponse(String::from("No message returned")))?
            .content
            .clone();

        self.context.push(request, answer.clone());

        Ok(answer)
    }

    /// Construct a request body.
    fn body(model: String, context: &Context, request: String) -> ChatBody {
        ChatBody {
            model,
            max_tokens: None,
            temperature: None,
            top_p: None,
            n: Some(1),
            stream: Some(false),
            stop: None,
            presence_penalty: None,
            frequency_penalty: None,
            logit_bias: None,
            user: None,
            messages: context.with_request(request),
        }
    }
}
