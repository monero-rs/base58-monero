[![Crates.io](https://img.shields.io/crates/v/base58-monero.svg)](https://crates.io/crates/base58-monero) [![Build Status](https://travis-ci.com/monero-rs/base58-monero-rs.svg?branch=master)](https://travis-ci.com/monero-rs/base58-monero-rs) [![Documentation](https://docs.rs/base58-monero/badge.svg)](https://docs.rs/base58-monero) [![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Rust Monero Base58
===

Library with support for encoding/decoding Monero base58 strings, with and without checksum verification.


## Bitcoin base58 vs Monero base58

Monero base58 is not like Bitcoin base58, bytes are converted in 8-byte blocks. The last block
can have less than 8 bytes, but at least 1 byte. Eight bytes converts to 11 or less Base58
characters; if a particular block converts to <11 characters, the conversion pads it with "1"s
(1 is 0 in Base58). Likewise, the final block can convert to 11 or less Base58 digits.

Due to the conditional padding, the 69-byte string will always convert to 95 Base58 characters
(8 * 11 + 7); where 7 is length of the last block of 5 bytes.
