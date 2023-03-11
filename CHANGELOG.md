# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html) as described in [The Cargo Book](https://doc.rust-lang.org/cargo/reference/manifest.html#the-version-field).

## [Unreleased]

### Added

- Implement no_std environment support for all features except streaming

### Removed

- Set default-features to false
- Removed unnecessary thiserror package

### Changed

- Refactored custom Error enum to not depend on thiserror and to only use std for the stream feature
- Use alloc and core in place of std where possible

## [1.0.0] - 2022-06-27

### Removed

- Feature `stream` is no longer part of `default` features

### Changed

- Bump MSRV from 1.45.2 to 1.49.0
- Updating dprint-plugin-markdown 0.11.2 to 0.13.3
- Updating dprint-plugin-toml 0.5.3 to 0.5.4

## [0.3.2] - 2021-11-15

### Added

- Shared workflow to automate release management and publication on [crates.io](https://crates.io) ([#14](https://github.com/monero-rs/base58-monero/pull/14))

### Fixed

- Fix a bug in the condition that validates inputs in `encode_block` and `decode_block` ([#13](https://github.com/monero-rs/base58-monero/pull/13))

## [0.3.1] - 2021-09-27

### Changed

- CI migrated to GitHub Actions with more tests and build
- `hex` dependency bumped from `0.3` to `0.4` ([#2](https://github.com/monero-rs/base58-monero/pull/2))

### Added

- Minimum Stable Rust Version of `1.45.2`
- Changelog tracking past and futur release
- Benchmarks and results in README ([#6](https://github.com/monero-rs/base58-monero/pull/6))
- Code coverage ([#7](https://github.com/monero-rs/base58-monero/pull/7))

## [0.3.0] - 2021-04-09

### Changed

- Update to `tokio` version `"1"` ([#1](https://github.com/monero-rs/base58-monero/pull/1))
- Improve async doc code example tests runtime with `tokio_test`
- Add more documentation about `check` and `stream` features

## [0.2.1] - 2021-03-19

### Changed

- Use `thiserror` to handle display, from and error implementation on `base58::Error`

## [0.2.0] - 2020-01-09

### Added

- New `stream` feature for asynchronous streams
- More test vectors

### Changed

- Improved documentation with examples

## [0.1.1] - 2019-03-09

### Added

- File header
- Trait `std::error::Error` on `Error`

## [0.1.0] - 2019-03-06

### Added

- Initial release of the library

[Unreleased]: https://github.com/monero-rs/base58-monero/compare/v1.0.0...HEAD
[1.0.0]: https://github.com/monero-rs/base58-monero/compare/v0.3.2...v1.0.0
[0.3.2]: https://github.com/monero-rs/base58-monero/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/monero-rs/base58-monero/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/monero-rs/base58-monero/compare/v0.2.1...v0.3.0
[0.2.1]: https://github.com/monero-rs/base58-monero/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/monero-rs/base58-monero/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/monero-rs/base58-monero/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/monero-rs/base58-monero/releases/tag/v0.1.0
