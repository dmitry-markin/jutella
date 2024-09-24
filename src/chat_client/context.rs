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

//! Chatbot context.

use crate::chat_client::openai_api::message::{
    AssistantMessage, Message, SystemMessage, UserMessage,
};

/// Chatbot context.
#[derive(Debug, Default, Clone)]
pub struct Context {
    system_message: Option<String>,
    conversation: Vec<(String, String)>,
    tokenizer: Option<tiktoken_rs::CoreBPE>,
}

impl Context {
    /// Create a new chat context.
    pub fn new(system_message: Option<String>) -> Self {
        Self {
            system_message,
            conversation: Vec::new(),
            tokenizer: None,
        }
    }

    /// Create a new chat context wth tokenizer.
    pub fn new_with_tokenizer(
        system_message: Option<String>,
        tokenizer: tiktoken_rs::CoreBPE,
    ) -> Self {
        Self {
            system_message,
            conversation: Vec::new(),
            tokenizer: Some(tokenizer),
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
    }

    /// Discard old records to keep `max_tokens` tokens.
    pub fn keep_recent(&mut self, max_tokens: usize) -> Result<(), ()> {
        let Some(ref tokenizer) = self.tokenizer else {
            return Err(());
        };

        let num_tokens = |m| tokenizer.encode_with_special_tokens(m).len();

        let mut tokens = self
            .system_message
            .as_ref()
            .map(|m| num_tokens(m))
            .unwrap_or_default();
        let mut keep = 0;

        for transaction in self.conversation.iter().rev() {
            tokens += num_tokens(&transaction.0) + num_tokens(&transaction.1);

            if tokens > max_tokens {
                break;
            }

            keep += 1;
        }

        let discard = self.conversation.len() - keep;
        self.conversation.drain(0..discard);

        Ok(())
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
}
