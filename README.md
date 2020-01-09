[![Build Status](https://travis-ci.com/monero-rs/base58-monero-rs.svg?branch=master)](https://travis-ci.com/monero-rs/base58-monero-rs) [![Crates.io](https://img.shields.io/crates/v/base58-monero.svg)](https://crates.io/crates/base58-monero) [![Documentation](https://docs.rs/base58-monero/badge.svg)](https://docs.rs/base58-monero) [![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Rust Monero Base58
===

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


Features
===

If you don't want to include all default features in your project:

```
[dependencies.base58-monero]
version = "0.2"
default-features = false
```

## `check`

Enables `encode_check` and `decode_check` functions. By default `check` feature is enable.

## `stream`

Enables `encode_stream` and `decode_stream` functions. By default `stream` feature is enable. This
feature enables async stream for encoding/decoding bytes. This should be used when encoding larger
amount of data or in asyncronous environment. `stream` can be used with `check` to enable
`encode_stream_check` and `decode_stream_check`.

About
===

This started as a research project sponsored by TrueLevel SA. It is now maintained by community
members.
