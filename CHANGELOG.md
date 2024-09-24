# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
