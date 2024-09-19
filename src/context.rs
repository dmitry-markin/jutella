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

use openai_api_rust::apis::{Message, Role};

/// Chatbot context.
#[derive(Debug, Default, Clone)]
pub struct Context {
    system_message: Option<String>,
    conversation: Vec<(String, String)>,
}

impl Context {
    /// Create new chat context.
    pub fn new(system_message: Option<String>) -> Self {
        Self {
            system_message,
            conversation: Vec::new(),
        }
    }

    /// Context so far with a new request message.
    pub fn with_request(&self, request: String) -> Vec<Message> {
        self.system_message
            .iter()
            .map(|system_message| Message {
                role: Role::System,
                content: system_message.clone(),
            })
            .chain(self.conversation.iter().flat_map(|(request, response)| {
                [
                    Message {
                        role: Role::User,
                        content: request.clone(),
                    },
                    Message {
                        role: Role::Assistant,
                        content: response.clone(),
                    },
                ]
                .into_iter()
            }))
            .chain(std::iter::once(Message {
                role: Role::User,
                content: request,
            }))
            .collect()
    }

    /// Extend the context with a new pair of request and response.
    pub fn push(&mut self, request: String, response: String) {
        self.conversation.push((request, response));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Compare messages. [`Message`]` does not implement `Eq` :(
    fn compare_messages(left: Vec<Message>, right: Vec<Message>) -> bool {
        std::iter::zip(left.into_iter(), right.into_iter()).all(|(left, right)| {
            left.content == right.content
                && match (left.role, right.role) {
                    (Role::System, Role::System) => true,
                    (Role::User, Role::User) => true,
                    (Role::Assistant, Role::Assistant) => true,
                    _ => false,
                }
        })
    }

    #[test]
    fn empty() {
        let context = Context::default();

        assert!(compare_messages(
            context.with_request(String::from("req")),
            vec![Message {
                role: Role::User,
                content: String::from("req"),
            },]
        ));
    }

    #[test]
    fn non_empty() {
        let mut context = Context::default();
        context.push(String::from("req1"), String::from("resp1"));

        assert!(compare_messages(
            context.with_request(String::from("req2")),
            vec![
                Message {
                    role: Role::User,
                    content: String::from("req1"),
                },
                Message {
                    role: Role::Assistant,
                    content: String::from("resp1"),
                },
                Message {
                    role: Role::User,
                    content: String::from("req2"),
                },
            ]
        ));
    }

    #[test]
    fn empty_with_system_message() {
        let context = Context::new(Some(String::from("system")));

        assert!(compare_messages(
            context.with_request(String::from("req")),
            vec![
                Message {
                    role: Role::System,
                    content: String::from("system"),
                },
                Message {
                    role: Role::User,
                    content: String::from("req"),
                },
            ]
        ));
    }

    #[test]
    fn non_empty_with_system_message() {
        let mut context = Context::new(Some(String::from("system")));
        context.push(String::from("req1"), String::from("resp1"));

        assert!(compare_messages(
            context.with_request(String::from("req2")),
            vec![
                Message {
                    role: Role::System,
                    content: String::from("system"),
                },
                Message {
                    role: Role::User,
                    content: String::from("req1"),
                },
                Message {
                    role: Role::Assistant,
                    content: String::from("resp1"),
                },
                Message {
                    role: Role::User,
                    content: String::from("req2"),
                },
            ]
        ));
    }
}
