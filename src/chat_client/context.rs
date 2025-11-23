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

//! Chatbot context.

use crate::chat_client::openai_api::message::{
    AssistantMessage, Content, Message, SystemMessage, UserMessage,
};
use iter_accumulate::IterAccumulate;

/// Chatbot context.
#[derive(Default, Clone)]
pub struct Context {
    system_message: Option<String>,
    system_message_tokens: usize,
    conversation: Vec<(Content, String, usize)>,
    min_history_tokens: Option<usize>,
    max_history_tokens: Option<usize>,
}

impl Context {
    /// Create a new chat context.
    pub fn new(
        system_message: Option<String>,
        system_message_tokens: usize,
        min_history_tokens: Option<usize>,
        max_history_tokens: Option<usize>,
    ) -> Self {
        Self {
            system_message,
            system_message_tokens,
            conversation: Vec::new(),
            min_history_tokens,
            max_history_tokens,
        }
    }

    /// Context so far with a new request message.
    pub fn with_request(&self, request: Content) -> impl Iterator<Item = Message> + '_ {
        self.system_message
            .iter()
            .map(|system_message| SystemMessage::new(system_message.clone()).into())
            .chain(self.conversation.iter().flat_map(|(request, response, _)| {
                [
                    UserMessage::new(request.clone()).into(),
                    AssistantMessage::new(response.clone()).into(),
                ]
                .into_iter()
            }))
            .chain(std::iter::once(UserMessage::new(request).into()))
    }

    /// Extend the context with a new pair of request and response.
    pub fn push(&mut self, request: Content, response: String, tokens: usize) {
        self.conversation.push((request, response, tokens));
        self.keep_recent();
    }

    /// Size of the context in tokens.
    pub fn tokens(&self) -> usize {
        self.system_message_tokens
            + self
                .conversation
                .iter()
                .map(|(_, _, tokens)| tokens)
                .sum::<usize>()
    }

    /// Discard old records to keep the context within the limits.
    fn keep_recent(&mut self) {
        if self.min_history_tokens.is_none() && self.max_history_tokens.is_none() {
            return;
        }

        let min_tokens = self.min_history_tokens.unwrap_or(usize::MAX);
        let max_tokens = self.max_history_tokens.unwrap_or(usize::MAX);

        let keep = self
            .conversation
            .iter()
            .rev()
            .map(|transaction| transaction.2)
            .accumulate((0, self.system_message_tokens), |(_, acc), x| {
                (acc, acc + x)
            })
            .map_while(|(prev, current)| (prev < min_tokens).then_some(current))
            .take_while(|current| *current <= max_tokens)
            .count();

        let discard = self.conversation.len() - keep;
        self.conversation.drain(0..discard);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty() {
        let context = Context::default();

        assert_eq!(
            context
                .with_request(Content::Text(String::from("req")))
                .collect::<Vec<_>>(),
            vec![UserMessage::new_from_str("req").into()],
        );
    }

    #[test]
    fn non_empty() {
        let mut context = Context::default();
        context.push(
            Content::Text(String::from("req1")),
            String::from("resp1"),
            2,
        );

        assert_eq!(
            context
                .with_request(Content::Text(String::from("req2")))
                .collect::<Vec<_>>(),
            vec![
                UserMessage::new_from_str("req1").into(),
                AssistantMessage::new(String::from("resp1")).into(),
                UserMessage::new_from_str("req2").into(),
            ],
        );
    }

    #[test]
    fn empty_with_system_message() {
        let context = Context::new(Some(String::from("system")), 1, None, None);

        assert_eq!(
            context
                .with_request(Content::Text(String::from("req")))
                .collect::<Vec<_>>(),
            vec![
                SystemMessage::new(String::from("system")).into(),
                UserMessage::new_from_str("req").into(),
            ]
        );
    }

    #[test]
    fn non_empty_with_system_message() {
        let mut context = Context::new(Some(String::from("system")), 1, None, None);
        context.push(
            Content::Text(String::from("req1")),
            String::from("resp1"),
            2,
        );

        assert_eq!(
            context
                .with_request(Content::Text(String::from("req2")))
                .collect::<Vec<_>>(),
            vec![
                SystemMessage::new(String::from("system")).into(),
                UserMessage::new_from_str("req1").into(),
                AssistantMessage::new(String::from("resp1")).into(),
                UserMessage::new_from_str("req2").into(),
            ]
        );
    }

    #[test]
    fn min_history_tokens() {
        let system = "to to to to to".to_string();
        let request = "do do do do do".to_string();
        let response = "be be be be be".to_string();

        let mut context = Context::new(Some(system.to_string()), 5, Some(20), None);
        assert!(context.conversation.is_empty());

        // 15 tokens
        context.push(Content::Text(request.clone()), response.clone(), 10);
        assert_eq!(context.conversation.len(), 1);

        // 25 tokens
        context.push(Content::Text(request.clone()), response.clone(), 10);
        assert_eq!(context.conversation.len(), 2);

        // 25 tokens again: one transaction was discarded
        context.push(Content::Text(request.clone()), response.clone(), 10);
        assert_eq!(context.conversation.len(), 2);
    }

    #[test]
    fn min_history_tokens_exact() {
        let request = "do do do do do".to_string();
        let response = "be be be be be".to_string();

        let mut context = Context::new(None, 0, Some(20), None);
        assert!(context.conversation.is_empty());

        // 10 tokens
        context.push(Content::Text(request.clone()), response.clone(), 10);
        assert_eq!(context.conversation.len(), 1);

        // 20 tokens
        context.push(Content::Text(request.clone()), response.clone(), 10);
        assert_eq!(context.conversation.len(), 2);

        // 20 tokens again: one transaction was discarded
        context.push(Content::Text(request.clone()), response.clone(), 10);
        assert_eq!(context.conversation.len(), 2);
    }

    #[test]
    fn max_history_tokens() {
        let system = "to to to to to".to_string();
        let request = "do do do do do".to_string();
        let response = "be be be be be".to_string();

        let mut context = Context::new(Some(system.to_string()), 5, None, Some(30));
        assert!(context.conversation.is_empty());

        // 15 tokens
        context.push(Content::Text(request.clone()), response.clone(), 10);
        assert_eq!(context.conversation.len(), 1);

        // 25 tokens
        context.push(Content::Text(request.clone()), response.clone(), 10);
        assert_eq!(context.conversation.len(), 2);

        // 25 tokens again: one transaction was discarded
        context.push(Content::Text(request.clone()), response.clone(), 10);
        assert_eq!(context.conversation.len(), 2);
    }

    #[test]
    fn max_history_tokens_exact() {
        let request = "do do do do do".to_string();
        let response = "be be be be be".to_string();

        let mut context = Context::new(None, 0, None, Some(30));
        assert!(context.conversation.is_empty());

        // 10 tokens
        context.push(Content::Text(request.clone()), response.clone(), 10);
        assert_eq!(context.conversation.len(), 1);

        // 20 tokens
        context.push(Content::Text(request.clone()), response.clone(), 10);
        assert_eq!(context.conversation.len(), 2);

        // 30 tokens
        context.push(Content::Text(request.clone()), response.clone(), 10);
        assert_eq!(context.conversation.len(), 3);

        // 30 tokens again: one transaction was discarded
        context.push(Content::Text(request.clone()), response.clone(), 10);
        assert_eq!(context.conversation.len(), 3);
    }
}
