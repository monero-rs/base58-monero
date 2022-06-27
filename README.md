[![Build Status](https://img.shields.io/github/workflow/status/monero-rs/base58-monero/Build/main)](https://github.com/monero-rs/base58-monero/actions/workflows/build.yml)
[![Codecov branch](https://img.shields.io/codecov/c/gh/monero-rs/base58-monero/main)](https://app.codecov.io/gh/monero-rs/base58-monero)
[![unsafe forbidden](https://img.shields.io/badge/unsafe-forbidden-success.svg)](https://github.com/rust-secure-code/safety-dance/)
[![Crates.io](https://img.shields.io/crates/v/base58-monero.svg)](https://crates.io/crates/base58-monero)
[![Documentation](https://docs.rs/base58-monero/badge.svg)](https://docs.rs/base58-monero)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![MSRV](https://img.shields.io/badge/MSRV-1.49.0-blue)](https://blog.rust-lang.org/2020/12/31/Rust-1.49.0.html)

# Rust Monero Base58

Library with support for encoding/decoding Monero base58 strings, with and without checksum
verification.

## Bitcoin base58 vs Monero base58

Monero base58 is not like Bitcoin base58, bytes are converted in 8-byte blocks. The last block can
have less than 8 bytes, but at least 1 byte. Eight bytes converts to 11 or less Base58 characters;
if a particular block converts to `<11` characters, the conversion pads it with "`1`"s (`1` is `0`
in Base58). Likewise, the final block can convert to 11 or less Base58 digits.

Due to the conditional padding, the 69-byte string, like Monero addresses, will always convert to 95
Base58 characters `(8 * 11 + 7)`; where 7 is length of the last block of 5 bytes.

The alphabet is composed of 58 characters visually not similar to avoid confusion, e.g. both `1` and
`l` are not part of the alphabet together, only `1` is present. The full alphabet is composed of:
`123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz`

## Features

By default both `check` and `stream` features are enabled. If you don't want to include all default features in your project:

```
[dependencies.base58-monero]
version = "0.3"
default-features = false
```

### `check`

Enables `encode_check` and `decode_check` functions. By default `check` feature is enable.

### `stream`

Enables `encode_stream` and `decode_stream` functions. By default `stream` feature is enable. This
feature enables async stream for encoding/decoding bytes. This should be used when encoding larger
amount of data or in asyncronous environment. `stream` can be used with `check` to enable
`encode_stream_check` and `decode_stream_check`.

## Benchmarks

Results obtained on an Intel(R) Core(TM) i7-7700HQ CPU @ 2.80GHz with a standard Monero address as data source.
Performances are shown in nanosecond per iteration of compute, the smaller the better:

| Operation | Regular          | `_check`           |
| --------- | ---------------- | ------------------ |
| `encode`  | 652 ns (+/- 107) | 1,272 ns (+/- 760) |
| `decode`  | 612 ns (+/- 82)  | 1,187 ns (+/- 541) |

Check versions compute or verify the checksum while encoding or decoding the data.

Benchmarks can be found under `/benches` and run with `cargo +nightly bench`.

## Releases and Changelog

See [CHANGELOG.md](CHANGELOG.md) and [RELEASING.md](RELEASING.md).

## About

This started as a research project sponsored by TrueLevel SA. It is now maintained by community members.

## Licensing

The code in this project is licensed under the [MIT License](LICENSE)
