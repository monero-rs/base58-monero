// Rust Monero Base58 Library
// Written in 2019 by
//   h4sh3d <h4sh3d@protonmail.com>
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

//! # Monero Base58
//!
//! Monero base58 is not like Bitcoin base58, bytes are converted in 8-byte blocks. The last block
//! can have less than 8 bytes, but at least 1 byte. Eight bytes converts to 11 or less Base58
//! characters; if a particular block converts to `<11` characters, the conversion pads it with
//! "1"s (1 is 0 in Base58). Likewise, the final block can convert to 11 or less Base58 digits.
//!
//! Due to the conditional padding, the 69-byte string, like Monero addresses, will always convert
//! to 95 Base58 characters `(8 * 11 + 7)`; where 7 is length of the last block of 5 bytes.
//!
//! The alphabet is composed of 58 characters visually not similar to avoid confusion, e.g. both
//! `1` and `l` are not part of the alphabet together, only `1` is present. The full alphabet is
//! composed of: `123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz`
//!
//! ## Examples
//!
//! Encoding and decoding an array of bytes with Monero base58 format:
//!
//! ```rust
//! use base58_monero::{encode, decode, Error};
//!
//! let input = b"Hello World";
//! let encoded_input = encode(input)?;
//!
//! let decoded_input = decode(&encoded_input)?;
//!
//! assert_eq!(&input[..], &decoded_input[..]);
//! # Ok::<(), Error>(())
//! ```
//!
//! With `feature = check` Monero base58 also comes with a `checksum` mode. The checksum is
//! composed with the first 4 bytes of a `Keccak256` result of the string. Encoding and decoding
//! with a checksum:
//!
//! ```rust
//! use base58_monero::{encode_check, decode_check, Error};
//!
//! let input = b"Hello World";
//! let encoded_input = encode_check(input)?;
//!
//! let decoded_input = decode_check(&encoded_input)?;
//!
//! assert_eq!(&input[..], &decoded_input[..]);
//! # Ok::<(), Error>(())
//! ```

#![recursion_limit = "256"]

pub mod base58;

pub use base58::decode;
#[cfg(feature = "check")]
pub use base58::decode_check;
#[cfg(feature = "stream")]
pub use base58::decode_stream;
#[cfg(all(feature = "check", feature = "stream"))]
pub use base58::decode_stream_check;
pub use base58::encode;
#[cfg(feature = "check")]
pub use base58::encode_check;
#[cfg(feature = "stream")]
pub use base58::encode_stream;
#[cfg(all(feature = "check", feature = "stream"))]
pub use base58::encode_stream_check;
pub use base58::Error;
