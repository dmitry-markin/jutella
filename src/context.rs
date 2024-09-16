use openai_api_rust::apis::{Message, Role};

/// Chat context.
#[derive(Debug, Default, Clone)]
pub struct Context {
    system: Option<String>,
    conversation: Vec<(String, String)>,
}

impl Context {
    pub fn new(system: Option<String>) -> Self {
        Self {
            system,
            conversation: Vec::new(),
        }
    }

    pub fn with_request(&self, request: String) -> Vec<Message> {
        self.system
            .iter()
            .map(|system| Message {
                role: Role::System,
                content: system.clone(),
            })
            .chain(
                self.conversation
                    .iter()
                    .map(|(request, response)| {
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
                    })
                    .flatten(),
            )
            .chain(std::iter::once(Message {
                role: Role::User,
                content: request,
            }))
            .collect()
    }

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
