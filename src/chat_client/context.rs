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
    AssistantMessage, Message, SystemMessage, UserMessage,
};
use iter_accumulate::IterAccumulate;

/// Chatbot context.
#[derive(Debug, Default, Clone)]
pub struct Context {
    system_message: Option<String>,
    conversation: Vec<(String, String)>,
    tokenizer: Option<tiktoken_rs::CoreBPE>,
    min_history_tokens: Option<usize>,
    max_history_tokens: Option<usize>,
}

impl Context {
    /// Create a new chat context.
    pub fn new(system_message: Option<String>) -> Self {
        Self {
            system_message,
            conversation: Vec::new(),
            tokenizer: None,
            min_history_tokens: None,
            max_history_tokens: None,
        }
    }

    /// Create a new chat context wth tokenizer.
    pub fn new_with_rolling_window(
        system_message: Option<String>,
        tokenizer: tiktoken_rs::CoreBPE,
        min_history_tokens: Option<usize>,
        max_history_tokens: Option<usize>,
    ) -> Self {
        debug_assert!(min_history_tokens.is_some() || max_history_tokens.is_some());

        Self {
            system_message,
            conversation: Vec::new(),
            tokenizer: Some(tokenizer),
            min_history_tokens,
            max_history_tokens,
        }
    }

    /// Context so far with a new request message.
    pub fn with_request(&self, request: String) -> impl Iterator<Item = Message> + '_ {
        self.system_message
            .iter()
            .map(|system_message| SystemMessage::new(system_message.clone()).into())
            .chain(self.conversation.iter().flat_map(|(request, response)| {
                [
                    UserMessage::new(request.clone()).into(),
                    AssistantMessage::new(response.clone()).into(),
                ]
                .into_iter()
            }))
            .chain(std::iter::once(UserMessage::new(request).into()))
    }

    /// Extend the context with a new pair of request and response.
    pub fn push(&mut self, request: String, response: String) {
        self.conversation.push((request, response));
        self.keep_recent();
    }

    /// Discard old records to keep the context within the limits.
    fn keep_recent(&mut self) {
        let Some(ref tokenizer) = self.tokenizer else {
            return;
        };

        // At least one of the numbers is limited if tokenizer is set.
        debug_assert!(self.min_history_tokens.is_some() || self.max_history_tokens.is_some());
        let min_tokens = self.min_history_tokens.unwrap_or(usize::MAX);
        let max_tokens = self.max_history_tokens.unwrap_or(usize::MAX);

        let num_tokens = |m| tokenizer.encode_with_special_tokens(m).len();

        let system_tokens = self
            .system_message
            .as_ref()
            .map(|m| num_tokens(m))
            .unwrap_or_default();

        let keep = self
            .conversation
            .iter()
            .rev()
            .map(|transaction| num_tokens(&transaction.0) + num_tokens(&transaction.1))
            .accumulate((0, system_tokens), |(_, acc), x| (acc, acc + x))
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
                .with_request(String::from("req"))
                .collect::<Vec<_>>(),
            vec![UserMessage::new(String::from("req")).into()],
        );
    }

    #[test]
    fn non_empty() {
        let mut context = Context::default();
        context.push(String::from("req1"), String::from("resp1"));

        assert_eq!(
            context
                .with_request(String::from("req2"))
                .collect::<Vec<_>>(),
            vec![
                UserMessage::new(String::from("req1")).into(),
                AssistantMessage::new(String::from("resp1")).into(),
                UserMessage::new(String::from("req2")).into(),
            ],
        );
    }

    #[test]
    fn empty_with_system_message() {
        let context = Context::new(Some(String::from("system")));

        assert_eq!(
            context
                .with_request(String::from("req"))
                .collect::<Vec<_>>(),
            vec![
                SystemMessage::new(String::from("system")).into(),
                UserMessage::new(String::from("req")).into(),
            ]
        );
    }

    #[test]
    fn non_empty_with_system_message() {
        let mut context = Context::new(Some(String::from("system")));
        context.push(String::from("req1"), String::from("resp1"));

        assert_eq!(
            context
                .with_request(String::from("req2"))
                .collect::<Vec<_>>(),
            vec![
                SystemMessage::new(String::from("system")).into(),
                UserMessage::new(String::from("req1")).into(),
                AssistantMessage::new(String::from("resp1")).into(),
                UserMessage::new(String::from("req2")).into(),
            ]
        );
    }

    #[test]
    fn min_history_tokens() {
        let tokenizer = tiktoken_rs::o200k_base().unwrap();
        let num_tokens = |m| tokenizer.encode_with_special_tokens(m).len();
        let system = "to to to to to".to_string();
        let request = "do do do do do".to_string();
        let response = "be be be be be".to_string();
        assert_eq!(num_tokens(&system), 5);
        assert_eq!(num_tokens(&request), 5);
        assert_eq!(num_tokens(&response), 5);

        let mut context = Context::new_with_rolling_window(
            Some(system.to_string()),
            tokenizer.clone(),
            Some(20),
            None,
        );
        assert!(context.conversation.is_empty());

        // 15 tokens
        context.push(request.clone(), response.clone());
        assert_eq!(context.conversation.len(), 1);

        // 25 tokens
        context.push(request.clone(), response.clone());
        assert_eq!(context.conversation.len(), 2);

        // 25 tokens again: one transaction was discarded
        context.push(request.clone(), response.clone());
        assert_eq!(context.conversation.len(), 2);
    }

    #[test]
    fn min_history_tokens_exact() {
        let tokenizer = tiktoken_rs::o200k_base().unwrap();
        let num_tokens = |m| tokenizer.encode_with_special_tokens(m).len();
        let request = "do do do do do".to_string();
        let response = "be be be be be".to_string();
        assert_eq!(num_tokens(&request), 5);
        assert_eq!(num_tokens(&response), 5);

        let mut context = Context::new_with_rolling_window(None, tokenizer.clone(), Some(20), None);
        assert!(context.conversation.is_empty());

        // 10 tokens
        context.push(request.clone(), response.clone());
        assert_eq!(context.conversation.len(), 1);

        // 20 tokens
        context.push(request.clone(), response.clone());
        assert_eq!(context.conversation.len(), 2);

        // 20 tokens again: one transaction was discarded
        context.push(request.clone(), response.clone());
        assert_eq!(context.conversation.len(), 2);
    }

    #[test]
    fn max_history_tokens() {
        let tokenizer = tiktoken_rs::o200k_base().unwrap();
        let num_tokens = |m| tokenizer.encode_with_special_tokens(m).len();
        let system = "to to to to to".to_string();
        let request = "do do do do do".to_string();
        let response = "be be be be be".to_string();
        assert_eq!(num_tokens(&system), 5);
        assert_eq!(num_tokens(&request), 5);
        assert_eq!(num_tokens(&response), 5);

        let mut context = Context::new_with_rolling_window(
            Some(system.to_string()),
            tokenizer.clone(),
            None,
            Some(30),
        );
        assert!(context.conversation.is_empty());

        // 15 tokens
        context.push(request.clone(), response.clone());
        assert_eq!(context.conversation.len(), 1);

        // 25 tokens
        context.push(request.clone(), response.clone());
        assert_eq!(context.conversation.len(), 2);

        // 25 tokens again: one transaction was discarded
        context.push(request.clone(), response.clone());
        assert_eq!(context.conversation.len(), 2);
    }

    #[test]
    fn max_history_tokens_exact() {
        let tokenizer = tiktoken_rs::o200k_base().unwrap();
        let num_tokens = |m| tokenizer.encode_with_special_tokens(m).len();
        let request = "do do do do do".to_string();
        let response = "be be be be be".to_string();
        assert_eq!(num_tokens(&request), 5);
        assert_eq!(num_tokens(&response), 5);

        let mut context = Context::new_with_rolling_window(None, tokenizer.clone(), None, Some(30));
        assert!(context.conversation.is_empty());

        // 10 tokens
        context.push(request.clone(), response.clone());
        assert_eq!(context.conversation.len(), 1);

        // 20 tokens
        context.push(request.clone(), response.clone());
        assert_eq!(context.conversation.len(), 2);

        // 30 tokens
        context.push(request.clone(), response.clone());
        assert_eq!(context.conversation.len(), 3);

        // 30 tokens again: one transaction was discarded
        context.push(request.clone(), response.clone());
        assert_eq!(context.conversation.len(), 3);
    }
}
