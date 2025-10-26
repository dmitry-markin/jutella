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

//! Streaming of chatbot response.

use crate::chat_client::{
    client::{ChatClient, TokenUsage},
    error::Error,
    openai_api::chat_completions::StreamingChunk,
};
use eventsource_stream::{Event, EventStreamError};
use futures::{
    ready,
    stream::{FusedStream, Stream, StreamExt},
    task::Poll,
};
use std::pin::Pin;

/// Chat completion delta event.
pub enum Delta {
    /// Reasoning delta. Returned before the content.
    Reasoning(String),
    /// Assistant response delta.
    Content(String),
    /// Token usage info. Always the last event.
    Usage(TokenUsage),
}

/// Stream state.
#[derive(Debug)]
enum State {
    WaitingForData,
    ReceivingReasoning,
    ReceivingContent { partial_response: String },
    WaitingForDone,
    WaitingForEndOfStream,
    Terminated,
}

impl State {
    /// Transition to further state getting the response accumulated.
    fn finalize(&mut self, new_state: Self) -> Option<String> {
        let old_state = std::mem::replace(self, new_state);

        match old_state {
            Self::ReceivingContent { partial_response } => {
                (!partial_response.is_empty()).then_some(partial_response)
            }
            _ => None,
        }
    }
}

/// Stream returned by [`ChatClient::stream_completion`].
pub struct CompletionStream<'a, S> {
    client: &'a mut ChatClient,
    stream: S,
    state: State,
    request: String,
}

impl<'a, S> CompletionStream<'a, S> {
    pub(crate) fn new(client: &'a mut ChatClient, stream: S, request: String) -> Self {
        Self {
            client,
            stream,
            state: State::WaitingForData,
            request,
        }
    }
}

impl<'a, S> Stream for CompletionStream<'a, S>
where
    S: Stream<Item = Result<Event, EventStreamError<reqwest::Error>>> + Unpin,
{
    type Item = Result<Delta, Error>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut futures::task::Context,
    ) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();

        if matches!(this.state, State::Terminated) {
            return Poll::Ready(None);
        }

        loop {
            let event = match ready!(this.stream.poll_next_unpin(cx)) {
                Some(Ok(event)) => {
                    if event.data == "[DONE]" {
                        if let Some(response) = this.state.finalize(State::WaitingForEndOfStream) {
                            this.client.extend_context(this.request.clone(), response);
                        }
                        continue;
                    }

                    event
                }
                Some(Err(e)) => {
                    if let Some(response) = this.state.finalize(State::Terminated) {
                        this.client.extend_context(this.request.clone(), response);
                    }

                    return Poll::Ready(Some(Err(Error::from(e))));
                }
                None => {
                    if let Some(response) = this.state.finalize(State::Terminated) {
                        this.client.extend_context(this.request.clone(), response);
                    }

                    return Poll::Ready(None);
                }
            };

            let delta = match parse_stream_chunk(&event.data) {
                Ok(Some(delta)) => delta,
                Ok(None) => continue,
                Err(e) => {
                    if let Some(response) = this.state.finalize(State::Terminated) {
                        this.client.extend_context(this.request.clone(), response);
                    }

                    return Poll::Ready(Some(Err(e)));
                }
            };

            match this.state {
                State::WaitingForData | State::ReceivingReasoning => match delta {
                    Delta::Reasoning(_) => {
                        this.state = State::ReceivingReasoning;
                    }
                    Delta::Content(ref content) => {
                        this.state = State::ReceivingContent {
                            partial_response: content.clone(),
                        };
                    }
                    Delta::Usage(_) => {
                        this.state = State::WaitingForDone;
                    }
                },
                State::ReceivingContent {
                    ref mut partial_response,
                } => match delta {
                    Delta::Reasoning(_) => {
                        if let Some(response) = this.state.finalize(State::Terminated) {
                            this.client.extend_context(this.request.clone(), response);
                        }

                        return Poll::Ready(Some(Err(Error::UnexpectedStreamEvent(
                            "reasoning after content",
                        ))));
                    }
                    Delta::Content(ref content) => {
                        partial_response.push_str(content);
                    }
                    Delta::Usage(_) => {
                        if let Some(response) = this.state.finalize(State::WaitingForDone) {
                            this.client.extend_context(this.request.clone(), response);
                        }
                    }
                },
                State::WaitingForDone => {
                    this.state = State::Terminated;
                    match delta {
                        Delta::Reasoning(_) => {
                            return Poll::Ready(Some(Err(Error::UnexpectedStreamEvent(
                                "reasoning after usage",
                            ))))
                        }
                        Delta::Content(_) => {
                            return Poll::Ready(Some(Err(Error::UnexpectedStreamEvent(
                                "content after usage",
                            ))))
                        }
                        Delta::Usage(_) => {
                            return Poll::Ready(Some(Err(Error::UnexpectedStreamEvent(
                                "duplicate usage",
                            ))))
                        }
                    }
                }
                State::WaitingForEndOfStream => {
                    // If the underlying stream errored after receiving `[DONE]` event we do not
                    // propagate this error.
                    this.state = State::Terminated;
                    return Poll::Ready(None);
                }
                State::Terminated => unreachable!("terminated state is handled by early return"),
            }

            return Poll::Ready(Some(Ok(delta)));
        }
    }
}

fn parse_stream_chunk(event: &str) -> Result<Option<Delta>, Error> {
    let mut chunk: StreamingChunk = serde_json::from_str(event)?;

    let choice = match chunk.choices.pop() {
        Some(choice) => choice,
        None => {
            if let Some(usage) = chunk.usage {
                return Ok(Some(Delta::Usage(usage.into())));
            } else {
                return Err(Error::NoChoices);
            }
        }
    };

    if let Some(reasoning) = choice.delta.reasoning {
        Ok(Some(Delta::Reasoning(reasoning)))
    } else if let Some(content) = choice.delta.content {
        Ok(Some(Delta::Content(content)))
    } else if let Some(refusal) = choice.delta.refusal {
        Err(Error::Refusal(refusal))
    } else if choice.finish_reason.is_some() {
        // Just ignore finish reason message.
        Ok(None)
    } else {
        Err(Error::NoContent)
    }
}

impl<'a, S> FusedStream for CompletionStream<'a, S>
where
    S: Stream<Item = Result<Event, EventStreamError<reqwest::Error>>> + Unpin,
{
    fn is_terminated(&self) -> bool {
        matches!(self.state, State::Terminated)
    }
}
