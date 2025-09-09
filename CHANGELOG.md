# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.6.0] - 2025-09-09

This release adds support for getting reasoning summaries and setting `reasoning_budget` when using OpenRouter API.

### Added

- Option to show reasoning summary (only OpenRouter API) ([#17](https://github.com/dmitry-markin/jutella/pull/17))
- Support `reasoning_budget` with OpenRouter API ([#16](https://github.com/dmitry-markin/jutella/pull/16))

## [0.5.0] - 2025-09-06

This relase adds support for [OpenRouter](https://openrouter.ai/) API, exposes options for `reasonning_effort` and response `verbosity`, extends token usage reporting, and adds token usage display to the CLI client.

### Added

- Allow disabling system message via CLI ([#12](https://github.com/dmitry-markin/jutella/pull/12))
- Support OpenRouter API ([#11](https://github.com/dmitry-markin/jutella/pull/11))
- Allow setting `reasoning_effort` & `verbosity` and return detailed token usage ([#10](https://github.com/dmitry-markin/jutella/pull/10))

### Changed

- Increase HTTP timeout from 2 min to 5 min ([#14](https://github.com/dmitry-markin/jutella/pull/14))
- Bump dependencies ([#13](https://github.com/dmitry-markin/jutella/pull/13))

### Fixed

- Fix error on null `system_fingerprint` with `gpt-4.5-preview` ([commit](https://github.com/dmitry-markin/jutella/commit/44f241c1c108effe79340bcab5b4f2ba99834662))

## [0.4.0] - 2024-11-30

This release adds `min_history_tokens` context window rolling strategy. It can be handy to keep the last big response in the context. Additionally, the API now provides token usage info.

### Added

- Extend API to report tokens used ([#8](https://github.com/dmitry-markin/jutella/pull/8))
- Add `min_history_tokens` rolling context window strategy ([#7](https://github.com/dmitry-markin/jutella/pull/7))

### Fixed

- Fix loading config file passed as CLI option ([commit](https://github.com/dmitry-markin/jutella/commit/be668dcfb3f082e54e437088d64234af7e5f650e))
- Remove impossible `Error::NoTokenizer` and update docs ([commit](https://github.com/dmitry-markin/jutella/commit/4aef26a43024f0390775da07b26c4ae7a5c378aa))

## [0.3.1] - 2024-09-24

This is a bugfix release fixing compilation of the library with `default-features = false`.

### Changed

- Fix compilation of library with `default-features = false` ([commit](https://github.com/dmitry-markin/jutella/commit/3e9493f5ec67fea0cbc35467aa0789d3d5914add))

## [0.3.0] - 2024-09-24

This release introduces several new features and improvements. Key updates are:

- Execution is now async, based on custom OpenAI API client implementation with proper error handling.
- Added the possibility to discard old messages in the context to keep it below allowed max token limit.
- Added support for Azure endpoints.
- The binary dependencies made optional in the library. Use `default-features = false` when depending on the library.
- CLI can now copy every response to clipboard via `xclip` on X11.

### Added

- Support Azure endpoints ([#4](https://github.com/dmitry-markin/jutella/pull/4))
- Implement rolling context window ([#3](https://github.com/dmitry-markin/jutella/pull/3))
- cli: Support copying every response to clipboard with `xclip` ([commit](https://github.com/dmitry-markin/jutella/commit/88e5ea633fca541edd140cd5c9c2941d8e2862ed))

### Changed

- Replace `openai_api_rust` with custom async OpenAI API client ([#2](https://github.com/dmitry-markin/jutella/pull/2))
- cli: Print `xclip` stderr on invocation failure ([commit](https://github.com/dmitry-markin/jutella/commit/06f5431a2f9fca4ca0babab24a37b9644f3e82c4))
- Make bin dependencies optional for lib ([commit](https://github.com/dmitry-markin/jutella/commit/ff76ba787df8739930cab43759c8903c48b326da))

## [0.2.0] - 2024-09-19

The project was renamed to `jutella`.

### Changed

- Use "mini" model by default
- Improve docs
- Rename `unspoken` -> `jutella`

## [0.1.1] - 2024-09-18

Improved documentation and README.

### Added

- Improve README
- Improve help
- Improve docs

## [0.1.0] - 2024-09-17

Initial release.

### Added

- Add README
- Introduce a config file
- Add command line arguments
- Make `ChatClientConfig` public
- Support setting API key in a config
- Report recoverable errors
- Initial commit
