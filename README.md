# jutella

[![License](https://img.shields.io/badge/License-MIT-blue.svg)](https://github.com/dmitry-markin/jutella/blob/master/LICENSE) [![crates.io](https://img.shields.io/crates/v/jutella.svg)](https://crates.io/crates/jutella) [![docs.rs](https://img.shields.io/docsrs/jutella.svg)](https://docs.rs/jutella/latest/jutella/)

Chatbot API client library and CLI interface. Currently supports OpenAI chat API, including OpenAI and Azure endpoints.


## Command line interface

To get started with CLI, put your API key and endpoint into `~/.config/jutella.toml`. See a config [example](https://github.com/dmitry-markin/jutella/blob/master/config/jutella.toml).

![Screenshot](doc/screenshot.png)

Invoking the CLI with `jutella -x` makes it copy every response to clipboard on X11.

### Installation

1. Install `cargo` from https://rustup.rs/.
2. Install the CLI from [crates.io](https://crates.io/crates/jutella) with `cargo install jutella`.
3. Alternatively, clone the repo and build the CLI with `cargo build --release`. The resulting executable will be `target/release/jutella`.


## Library

To use the chat API, initialize `ChatClient` with `OPENAI_API_KEY` and `ChatClientConfig`:

```rust
let mut chat = ChatClient::new(Auth::Token(api_key), ChatClientConfig::default())?;
```

Request replies via `ChatClient::ask()`:

```rust
let answer = chat.ask("What is the highest point on Earth?".to_string()).await?;
println!("{answer}");
```

`ChatClient` keeps the conversation context and uses it with every `ask()` to generate the reply.
