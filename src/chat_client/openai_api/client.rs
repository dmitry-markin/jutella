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

//! OpenAI REST API client.

use crate::chat_client::openai_api::chat_completions::{ChatCompletions, ChatCompletionsRequest};
use eventsource_stream::{EventStream, Eventsource};
use futures::stream::Stream;
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue, InvalidHeaderValue, AUTHORIZATION},
    Client, Method, Request, RequestBuilder, StatusCode,
};
use serde::Deserialize;
use std::{fmt::Display, str::FromStr, time::Duration};
use url::{ParseError, Url};

const CHAT_COMPLETIONS_ENDPOINT: &str = "chat/completions";

/// Authorization header.
///
/// Use `HeaderMap::try_from(auth)` to convert to `reqwest` headers.
#[derive(Debug, Clone)]
pub enum Auth {
    /// Auth header `Authorization: Bearer {api_token}`.
    Token(String),
    /// Auth header `api-key: {api_key}`.
    ApiKey(String),
}

impl TryFrom<Auth> for HeaderMap {
    type Error = InvalidHeaderValue;

    fn try_from(auth: Auth) -> Result<Self, InvalidHeaderValue> {
        let headers = match auth {
            Auth::Token(token) => [(
                AUTHORIZATION,
                HeaderValue::from_str(&format!("Bearer {token}"))?,
            )],
            Auth::ApiKey(api_key) => [(
                HeaderName::from_str("api-key").expect("to be valid ASCII"),
                HeaderValue::from_str(&api_key)?,
            )],
        }
        .into_iter()
        .collect();

        Ok(headers)
    }
}

/// OpenAI REST API client config.
pub struct OpenAiClientConfig {
    /// Reqwest client.
    pub client: Client,
    /// Authentication token/key.
    pub auth: Auth,
    /// OpenAI chat API endpoint.
    pub base_url: String,
    /// API version used by Azure endpoints.
    pub api_version: Option<String>,
    /// HTTP request timeout.
    pub timeout: Duration,
}

/// OpenAI REST API client.
pub struct OpenAiClient {
    client: Client,
    endpoint: Url,
    headers: HeaderMap,
    timeout: Duration,
}

impl OpenAiClient {
    /// Create new OpenAI API client.
    pub fn new(
        OpenAiClientConfig {
            client,
            auth,
            base_url,
            api_version,
            timeout,
        }: OpenAiClientConfig,
    ) -> Result<Self, Error> {
        Ok(Self {
            client,
            endpoint: Url::parse(&build_url(base_url, api_version))?,
            headers: auth.try_into()?,
            timeout,
        })
    }

    /// Request chat completion message.
    pub async fn chat_completions(
        &mut self,
        body: ChatCompletionsRequest,
    ) -> Result<ChatCompletions, Error> {
        let response = self.build_request(body).send().await?;

        if response.status().is_success() {
            Ok(response.json().await?)
        } else {
            let status = response.status();
            let body = response
                .text()
                .await
                .unwrap_or(String::from("<invalid UTF-8>"));

            let description = serde_json::from_str::<ErrorBody>(&body)
                .map(|e| e.error.message)
                .unwrap_or(body);

            Err(ApiError {
                status,
                description,
            }
            .into())
        }
    }

    /// Request chat completion stream.
    pub async fn chat_completions_stream(
        &mut self,
        body: ChatCompletionsRequest,
    ) -> Result<EventStream<impl Stream<Item = Result<bytes::Bytes, reqwest::Error>>>, Error> {
        Ok(self
            .build_request(body)
            .send()
            .await?
            .bytes_stream()
            .eventsource())
    }

    /// Build request.
    fn build_request(&mut self, body: ChatCompletionsRequest) -> RequestBuilder {
        RequestBuilder::from_parts(
            self.client.clone(),
            Request::new(Method::POST, self.endpoint.clone()),
        )
        .headers(self.headers.clone())
        .json(&body)
        .timeout(self.timeout)
    }
}

fn build_url(base_url: String, api_version: Option<String>) -> String {
    if let Some(version) = api_version {
        format!("{base_url}{CHAT_COMPLETIONS_ENDPOINT}?api-version={version}")
    } else {
        format!("{base_url}{CHAT_COMPLETIONS_ENDPOINT}")
    }
}

/// Errors generated by [`OpenAiClient`].
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Invalid API key charcters.
    #[error("Non ASCII / non visible characters in API key")]
    InvalidCharactersInApiKey(#[from] InvalidHeaderValue),

    /// Reqwest error.
    #[error("Request error: {0}")]
    Request(reqwest::Error),

    /// API (HTTP) error.
    #[error("{0}")]
    Api(#[from] ApiError),

    /// URL parsing error.
    #[error("Invalid URL: {0}")]
    InvalidUrl(#[from] ParseError),
}

impl From<reqwest::Error> for Error {
    fn from(error: reqwest::Error) -> Self {
        // Remove potentially sensitive information.
        Self::Request(error.without_url())
    }
}

/// Error in case of HTTP status != 200 OK.
#[derive(Debug, thiserror::Error)]
pub struct ApiError {
    /// HTTP status code.
    pub status: StatusCode,
    /// Error description.
    pub description: String,
}

impl Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.status, self.description)
    }
}

/// Possible error body (might be incomplete type).
#[derive(Debug, Deserialize)]
pub struct ErrorBody {
    /// Internal `error` JSON object.
    error: OpenAiError,
}

/// Possible `error` field (fields other than `message` omitted).
#[derive(Debug, Deserialize)]
pub struct OpenAiError {
    /// Field `message` of `error` JSON object.
    message: String,
}
