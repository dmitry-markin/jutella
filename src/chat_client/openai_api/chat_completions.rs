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

//! OpenAI API Chat Completions request & response types.

use crate::chat_client::openai_api::message::GenericMessage;
use serde::{Deserialize, Serialize};
use serde_json::value::Value;
use std::collections::HashMap;

/// OpenAI API Chat Completions request body.
///
/// Given a list of messages comprising a conversation, the model will return a response.
/// See https://platform.openai.com/docs/api-reference/chat/create.
///
/// JSON example:
/// ```json
/// {
///   "model": "gpt-4o",
///   "messages": [
///     {
///       "role": "system",
///       "content": "You are a helpful assistant."
///     },
///     {
///       "role": "user",
///       "content": "Hello!"
///     }
///   ]
/// }
/// ```
#[derive(Debug, Default, Clone, PartialEq, Serialize)]
pub struct ChatCompletionsBody {
    /// A list of messages comprising the conversation so far.
    pub messages: Vec<GenericMessage>,

    /// ID of the model to use. See the [model endpoint compatibility]
    /// (https://platform.openai.com/docs/models/model-endpoint-compatibility)
    /// table for details on which models work with the Chat API.
    pub model: String,

    /// Number between -2.0 and 2.0. Positive values penalize new tokens based on their existing
    /// frequency in the text so far, decreasing the model's likelihood to repeat the same line
    /// verbatim.
    ///
    /// [See more information about frequency and presence penalties.]
    /// (https://platform.openai.com/docs/guides/text-generation/parameter-details)
    ///
    /// Defaults to `0`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,

    /// Modify the likelihood of specified tokens appearing in the completion.
    /// Accepts a JSON object that maps tokens (specified by their token ID in the tokenizer)
    /// to an associated bias value from -100 to 100. Mathematically, the bias is added to the
    /// logits generated by the model prior to sampling. The exact effect will vary per model,
    /// but values between -1 and 1 should decrease or increase likelihood of selection;
    /// values like -100 or 100 should result in a ban or exclusive selection of the relevant token.
    ///
    /// Defaults to `null`.
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub logit_bias: HashMap<String, f32>,

    /// Whether to return log probabilities of the output tokens or not. If true, returns the log
    /// probabilities of each output token returned in the `content` of `message`.
    ///
    /// Defaults to `false`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<bool>,

    /// An integer between 0 and 20 specifying the number of most likely tokens to return at each
    /// token position, each with an associated log probability. `logprobs` must be set to `true`
    /// if this parameter is used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_logprobs: Option<u8>,

    /// An upper bound for the number of tokens that can be generated for a completion,
    /// including visible output tokens and reasoning tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_completion_tokens: Option<usize>,

    /// How many chat completion choices to generate for each input message.Note that you will be
    /// charged based on the number of generated tokens across all of the choices.
    /// Keep `n` as `1` to minimize costs.
    ///
    /// Defaults to `1`.
    #[serde(rename = "n")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion_choices: Option<usize>,

    /// Number between -2.0 and 2.0. Positive values penalize new tokens based on whether they
    /// appear in the text so far, increasing the model's likelihood to talk about new topics.
    ///
    /// [See more information about frequency and presence penalties.]
    /// (https://platform.openai.com/docs/guides/text-generation/parameter-details)
    ///
    /// Defaults to 0.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,

    /// An object specifying the format that the model must output. Compatible with GPT-4o,
    /// GPT-4o mini, GPT-4 Turbo and all GPT-3.5 Turbo models newer than `gpt-3.5-turbo-1106`.
    ///
    /// Setting to `{ "type": "json_schema", "json_schema": {...} }` enables Structured Outputs
    /// which ensures the model will match your supplied JSON schema. Learn more in the
    /// [Structured Outputs guide](https://platform.openai.com/docs/guides/structured-outputs).
    ///
    /// Setting to `{ "type": "json_object" }` enables JSON mode, which ensures the message the
    /// model generates is valid JSON.
    ///
    /// Important: when using JSON mode, you must also instruct the model to produce JSON yourself
    /// via a system or user message. Without this, the model may generate an unending stream of
    /// whitespace until the generation reaches the token limit, resulting in a long-running and
    /// seemingly "stuck" request. Also note that the message content may be partially cut off if
    /// `finish_reason="length"`, which indicates the generation exceeded `max_tokens` or the
    /// conversation exceeded the max context length.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<Value>,

    /// This feature is in Beta. If specified, our system will make a best effort to sample
    /// deterministically, such that repeated requests with the same `seed` and parameters should
    /// return the same result. Determinism is not guaranteed, and you should refer to the
    /// `system_fingerprint` response parameter to monitor changes in the backend.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i64>,

    /// Specifies the latency tier to use for processing the request. This parameter is relevant
    /// for customers subscribed to the scale tier service:
    ///
    /// - If set to 'auto', and the Project is Scale tier enabled, the system will utilize scale
    ///   ier credits until they are exhausted.
    /// - If set to 'auto', and the Project is not Scale tier enabled, the request will be processed
    ///   using the default service tier with a lower uptime SLA and no latency guarentee.
    /// - If set to 'default', the request will be processed using the default service tier with a
    ///   lower uptime SLA and no latency guarentee.
    /// - When not set, the default behavior is 'auto'.
    ///
    /// When this parameter is set, the response body will include the `service_tier` utilized.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<String>,

    /// Up to 4 sequences where the API will stop generating further tokens.
    ///
    /// Defaults to `null`.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub stop: Vec<String>,

    /// If set, partial message deltas will be sent, like in ChatGPT. Tokens will be sent as
    /// data-only server-sent events as they become available, with the stream terminated by
    /// a `data: [DONE]` message.
    ///
    /// Defaults to `false`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,

    /// Options for streaming response. Only set this when you set `stream: true`.
    ///
    /// Defaults to `null`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_options: Option<Value>,

    /// What sampling temperature to use, between 0 and 2. Higher values like 0.8 will make the
    /// output more random, while lower values like 0.2 will make it more focused and deterministic.
    ///
    /// We generally recommend altering this or `top_p` but not both.
    ///
    /// Defaults to `1`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    /// An alternative to sampling with temperature, called nucleus sampling, where the model
    /// considers the results of the tokens with top_p probability mass. So 0.1 means only the
    /// tokens comprising the top 10% probability mass are considered.
    ///
    /// We generally recommend altering this or `temperature` but not both.
    ///
    /// Defaults to `1`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,

    /// A list of tools the model may call. Currently, only functions are supported as a tool.
    /// Use this to provide a list of functions the model may generate JSON inputs for.
    /// A max of 128 functions are supported.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<Value>,

    /// Controls which (if any) tool is called by the model.
    /// `none` means the model will not call any tool and instead generates a message.
    /// `auto` means the model can pick between generating a message or calling one or more tools.
    /// `required` means the model must call one or more tools.
    /// Specifying a particular tool via `{"type": "function", "function": {"name": "my_function"}}`
    /// forces the model to call that tool.
    ///
    /// `none` is the default when no tools are present. `auto` is the default if tools are present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<Value>,

    /// Whether to enable parallel function calling during tool use.
    ///
    /// Defaults to `true`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parallel_tool_calls: Option<bool>,

    /// A unique identifier representing your end-user, which can help OpenAI to monitor and detect abuse.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
}

/// OpenAI API Chat Completions response.
///
/// Represents a chat completion response returned by model, based on the provided input.
/// See https://platform.openai.com/docs/api-reference/chat/object.
///
/// JSON example:
/// ```json
/// {
///   "id": "chatcmpl-123",
///   "object": "chat.completion",
///   "created": 1677652288,
///   "model": "gpt-4o-mini",
///   "system_fingerprint": "fp_44709d6fcb",
///   "choices": [{
///     "index": 0,
///     "message": {
///       "role": "assistant",
///       "content": "\n\nHello there, how may I assist you today?",
///     },
///     "logprobs": null,
///     "finish_reason": "stop"
///   }],
///   "usage": {
///     "prompt_tokens": 9,
///     "completion_tokens": 12,
///     "total_tokens": 21,
///     "completion_tokens_details": {
///       "reasoning_tokens": 0
///     }
///   }
/// }
/// ```
#[derive(Debug, Clone, Eq, PartialEq, Deserialize)]
pub struct ChatCompletions {
    /// A unique identifier for the chat completion.
    pub id: String,

    /// A list of chat completion choices. Can be more than one if `completion_choices`
    /// (`n`) is greater than 1.
    pub choices: Vec<CompletionChoice>,

    /// The Unix timestamp (in seconds) of when the chat completion was created.
    pub created: u64,

    /// The model used for the chat completion.
    pub model: String,

    /// The service tier used for processing the request. This field is only included if the
    /// `service_tier` parameter is specified in the request.
    pub service_tier: Option<String>,

    /// This fingerprint represents the backend configuration that the model runs with.
    ///
    /// Can be used in conjunction with the `seed` request parameter to understand when
    /// backend changes have been made that might impact determinism.
    pub system_fingerprint: Option<String>,

    /// The object type, which is always `chat.completion`.
    pub object: String,

    /// Usage statistics for the completion request.
    pub usage: Usage,
}

/// Completion choice
#[derive(Debug, Clone, Eq, PartialEq, Deserialize)]
pub struct CompletionChoice {
    /// The reason the model stopped generating tokens. This will be `stop` if the model hit a
    /// natural stop point or a provided stop sequence, `length` if the maximum number of tokens
    /// specified in the request was reached, `content_filter` if content was omitted due to a flag
    /// from our content filters, `tool_calls` if the model called a tool, or `function_call`
    /// (deprecated) if the model called a function.
    pub finish_reason: String,

    /// The index of the choice in the list of choices.
    pub index: usize,

    /// A chat completion message generated by the model.
    pub message: GenericMessage,

    ///  Log probability information for the choice.
    pub logprobs: Option<Value>,
}

/// Usage details
#[derive(Debug, Clone, Eq, PartialEq, Deserialize)]
pub struct Usage {
    /// Number of tokens in the prompt.
    pub prompt_tokens: usize,

    /// Number of tokens in the generated completion.
    pub completion_tokens: usize,

    /// Total number of tokens used in the request (prompt + completion).
    pub total_tokens: usize,

    /// Breakdown of tokens used in the prompt.
    pub prompt_tokens_details: Option<Value>,

    /// Breakdown of tokens used in a completion.
    pub completion_tokens_details: Option<Value>,
}
