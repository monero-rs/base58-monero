//! # Monoero Base58
//!
//! Monero base58 is not like Bitcoin base58, bytes are converted in 8-byte blocks. The last block
//! can have less than 8 bytes, but at least 1 byte. Eight bytes converts to 11 or less Base58
//! characters; if a particular block converts to <11 characters, the conversion pads it with "1"s
//! (1 is 0 in Base58). Likewise, the final block can convert to 11 or less Base58 digits.
//!
//! Due to the conditional padding, the 69-byte string will always convert to 95 Base58 characters
//! (8 * 11 + 7); where 7 is length of the last block of 5 bytes.

#[cfg(feature = "check")] extern crate keccak_hash;

/// Base58 encoder and decoder
pub mod base58;

pub use base58::encode;
pub use base58::decode;
#[cfg(feature = "check")] pub use base58::encode_check;
#[cfg(feature = "check")] pub use base58::decode_check;
