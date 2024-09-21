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

//! OpenAI API Message types.

use serde::{Deserialize, Serialize};
use serde_json::value::Value;

/// The role of the message author.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// System message.
    System,
    /// User message.
    User,
    /// Assistant message.
    Assistant,
    /// Tool message.
    Tool,
}

/// Conversation message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Message {
    /// System message.
    System(SystemMessage),
    /// User message.
    User(UserMessage),
    /// Assistant message.
    Assistant(AssistantMessage),
    /// Tool message.
    Tool(ToolMessage),
}

/// System message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemMessage {
    /// The contents of the message.
    pub content: String,
    /// An optional name for the participant. Provides the model information
    /// to differentiate between participants of the same role.
    pub name: Option<String>,
}

impl SystemMessage {
    pub fn new(content: String) -> Self {
        Self {
            content,
            name: None,
        }
    }
}

/// User message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserMessage {
    /// The contents of the message.
    pub content: String,
    /// An optional name for the participant. Provides the model information
    /// to differentiate between participants of the same role.
    pub name: Option<String>,
}

impl UserMessage {
    pub fn new(content: String) -> Self {
        Self {
            content,
            name: None,
        }
    }
}

/// Assistant message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssistantMessage {
    /// The contents of the message.
    pub content: Option<String>,
    /// An optional name for the participant. Provides the model information
    /// to differentiate between participants of the same role.
    pub name: Option<String>,
    /// The refusal message by the assistant.
    pub refusal: Option<String>,
    /// The tool calls generated by the model, such as function calls.
    pub tool_calls: Option<Value>,
}

impl AssistantMessage {
    pub fn new(content: String) -> Self {
        Self {
            content: Some(content),
            name: None,
            refusal: None,
            tool_calls: None,
        }
    }
}

/// Tool message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolMessage {
    /// The contents of the message.
    pub content: String,
    /// Tool call that this message is responding to.
    pub tool_call_id: String,
}

impl From<SystemMessage> for Message {
    fn from(message: SystemMessage) -> Self {
        Self::System(message)
    }
}

impl From<UserMessage> for Message {
    fn from(message: UserMessage) -> Self {
        Self::User(message)
    }
}

impl From<AssistantMessage> for Message {
    fn from(message: AssistantMessage) -> Self {
        Self::Assistant(message)
    }
}

impl From<ToolMessage> for Message {
    fn from(message: ToolMessage) -> Self {
        Self::Tool(message)
    }
}

/// Generic message.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GenericMessage {
    /// The role of the message author.
    role: Role,
    /// The contents of the message.
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    /// An optional name for the participant. Provides the model information
    /// to differentiate between participants of the same role.
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    /// The refusal message by the assistant.
    #[serde(skip_serializing_if = "Option::is_none")]
    refusal: Option<String>,
    /// The tool calls generated by the model, such as function calls.
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Value>,
    /// Tool call that this message is responding to.
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
}

impl From<Message> for GenericMessage {
    fn from(message: Message) -> Self {
        match message {
            Message::System(m) => m.into(),
            Message::User(m) => m.into(),
            Message::Assistant(m) => m.into(),
            Message::Tool(m) => m.into(),
        }
    }
}

impl From<SystemMessage> for GenericMessage {
    fn from(SystemMessage { content, name }: SystemMessage) -> Self {
        Self {
            role: Role::System,
            content: Some(content),
            name,
            refusal: None,
            tool_calls: None,
            tool_call_id: None,
        }
    }
}

impl From<UserMessage> for GenericMessage {
    fn from(UserMessage { content, name }: UserMessage) -> Self {
        Self {
            role: Role::User,
            content: Some(content),
            name,
            refusal: None,
            tool_calls: None,
            tool_call_id: None,
        }
    }
}

impl From<AssistantMessage> for GenericMessage {
    fn from(
        AssistantMessage {
            content,
            name,
            refusal,
            tool_calls,
        }: AssistantMessage,
    ) -> Self {
        Self {
            role: Role::Assistant,
            content,
            name,
            refusal,
            tool_calls,
            tool_call_id: None,
        }
    }
}

impl From<ToolMessage> for GenericMessage {
    fn from(
        ToolMessage {
            content,
            tool_call_id,
        }: ToolMessage,
    ) -> Self {
        Self {
            role: Role::Tool,
            content: Some(content),
            name: None,
            refusal: None,
            tool_calls: None,
            tool_call_id: Some(tool_call_id),
        }
    }
}

impl TryFrom<GenericMessage> for Message {
    type Error = Error;

    fn try_from(message: GenericMessage) -> Result<Self, Error> {
        Ok(match message.role {
            Role::System => Message::System(SystemMessage::try_from(message)?),
            Role::User => Message::User(UserMessage::try_from(message)?),
            Role::Assistant => Message::Assistant(AssistantMessage::try_from(message)?),
            Role::Tool => Message::Tool(ToolMessage::try_from(message)?),
        })
    }
}

/// Error when converting messages
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Missing mandatory field
    #[error("missing mandatory field `{0}`")]
    MissingField(&'static str),
    /// Invalid role
    #[error("expected role {0:?}, got {1:?}")]
    RoleMismatch(Role, Role),
}

impl TryFrom<GenericMessage> for SystemMessage {
    type Error = Error;

    fn try_from(m: GenericMessage) -> Result<Self, Error> {
        if m.role == Role::System {
            Ok(Self {
                content: m.content.ok_or(Error::MissingField("content"))?,
                name: m.name,
            })
        } else {
            Err(Error::RoleMismatch(Role::System, m.role))
        }
    }
}

impl TryFrom<GenericMessage> for UserMessage {
    type Error = Error;

    fn try_from(m: GenericMessage) -> Result<Self, Error> {
        if m.role == Role::User {
            Ok(Self {
                content: m.content.ok_or(Error::MissingField("content"))?,
                name: m.name,
            })
        } else {
            Err(Error::RoleMismatch(Role::User, m.role))
        }
    }
}

impl TryFrom<GenericMessage> for AssistantMessage {
    type Error = Error;

    fn try_from(m: GenericMessage) -> Result<Self, Error> {
        if m.role == Role::Assistant {
            Ok(Self {
                content: m.content,
                name: m.name,
                refusal: m.refusal,
                tool_calls: m.tool_calls,
            })
        } else {
            Err(Error::RoleMismatch(Role::Assistant, m.role))
        }
    }
}

impl TryFrom<GenericMessage> for ToolMessage {
    type Error = Error;

    fn try_from(m: GenericMessage) -> Result<Self, Error> {
        if m.role == Role::Tool {
            Ok(Self {
                content: m.content.ok_or(Error::MissingField("content"))?,
                tool_call_id: m.tool_call_id.ok_or(Error::MissingField("tool_call_id"))?,
            })
        } else {
            Err(Error::RoleMismatch(Role::Tool, m.role))
        }
    }
}
