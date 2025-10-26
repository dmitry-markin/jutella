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

//! Chatbot response error.

use crate::chat_client::openai_api::{client::Error as OpenAiClientError, message};
use eventsource_stream::EventStreamError;

/// Errors during interaction with a chatbot.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Error reported by the model API.
    #[error("API error: {0}")]
    OpenAiClient(#[from] OpenAiClientError),
    /// The response contains no completion choices.
    #[error("Response contains no choices")]
    NoChoices,
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
    /// Stream error.
    // TODO: decompose and extract transport error.
    #[error("Stream error: {0}")]
    StreamError(#[from] EventStreamError<reqwest::Error>),
    /// Completion delta JSON parsing error.
    #[error("Completion delta JSON parsing error: {0}")]
    DeltaJsonError(#[from] serde_json::Error),
    /// Reasoning delta after content.
    #[error("Unexpected stream event: {0}")]
    UnexpectedStreamEvent(&'static str),
}
