use openai_api_rust::{
    chat::{ChatApi, ChatBody},
    Auth, OpenAI,
};

mod context;
use context::Context;

#[derive(Debug)]
pub struct ChatClientConfig {
    pub api_url: String,
    pub model: String,
    pub system: Option<String>,
}

impl Default for ChatClientConfig {
    fn default() -> Self {
        Self {
            api_url: String::from("https://models.inference.ai.azure.com/"),
            model: String::from("gpt-4o"),
            system: None,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("API error: {0}")]
    ApiError(String),
    #[error("Request error: {0}")]
    RequestError(String),
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

/// OpenAI chat API client.
pub struct ChatClient {
    openai: OpenAI,
    model: String,
    context: Context,
}

impl ChatClient {
    /// Create new `ChatClient` accessinf API at `api_url` with `auth_token`.
    pub fn new(auth_token: String, config: ChatClientConfig) -> Self {
        let ChatClientConfig {
            api_url,
            model,
            system,
        } = config;

        let api_url = if api_url.ends_with('/') {
            api_url
        } else {
            api_url + "/"
        };

        Self {
            openai: OpenAI::new(Auth::new(&auth_token), &api_url),
            model,
            context: Context::new(system),
        }
    }

    /// Send a `request` and receive a respone.
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
