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
use eventsource_stream::{Event, EventStream, EventStreamError};
use futures::{
    ready,
    stream::{Stream, StreamExt},
    task::Poll,
};
use std::{pin::Pin, time::Duration};

/// Chat completion delta event.
pub enum Delta {
    /// Assistant response delta.
    Content(String),
    /// Reasoning delta. Returned before the content.
    Reasoning(String),
    /// Token usage info. Always the last event.
    TokenUsage(TokenUsage),
}

/// Stream returned by [`ChatClient::stream_completion`].
pub struct CompletionStream<'a, S> {
    client: &'a mut ChatClient,
    stream: S,
    terminated: bool,
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

        if this.terminated {
            return Poll::Ready(None);
        }

        match ready!(this.stream.poll_next_unpin(cx)) {
            Some(Ok(event)) => {
                if event.data == "[DONE]" {
                    this.terminated = true;

                    return Poll::Ready(None);
                    // TODO: poll underlying stream to give it a chance to clean up.
                }

                // TODO: extend context.
                Poll::Ready(Some(parse_stream_chunk(&event.data)))
            }
            Some(Err(e)) => {
                this.terminated = true;

                Poll::Ready(Some(Err(Error::from(e))))
            }
            None => {
                this.terminated = true;

                Poll::Ready(None)
            }
        }
    }
}

fn parse_stream_chunk(event: &str) -> Result<Delta, Error> {
    let mut chunk: StreamingChunk = serde_json::from_str(event)?;

    // TODO: proper error handling.
    // TODO: reasoning parsing.
    // TODO: token usage parsing.
    Ok(Delta::Content(chunk.choices.pop().unwrap().delta.content))
}
