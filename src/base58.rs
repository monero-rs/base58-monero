// Rust Monero Base58 Library
// Written in 2019-2021 by
//   Monero Rust Contributors
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

//! Base58 encoder and decoder functions and constants
//!
//! ## Stream Examples
//!
//! Async streams can be used with the `stream` feature:
//!
//! ```rust
//! use futures_util::pin_mut;
//! use futures_util::stream::StreamExt;
//! use base58_monero::{encode_stream, Error};
//!
//! # tokio_test::block_on(
//! async {
//!     let mut input: &[u8] = b"Hello World";
//!     let mut w: Vec<char> = vec![];
//!
//!     let s = encode_stream(&mut input);
//!     pin_mut!(s);
//!
//!     while let Some(value) = s.next().await {
//!         w.push(value?);
//!     }
//!
//!     let s: String = w.into_iter().collect();
//!     assert_eq!("D7LMXYjUbXc1fS9Z", &s[..]);
//!     # Ok::<(), Error>(())
//! }
//! # )?;
//! # Ok::<(), Error>(())
//! ```
//! Async decoding with `decode_stream` and `decode_stream_check` is available with the features `check` and
//! `stream` enabled:
//!
//! ```rust
//! use futures_util::pin_mut;
//! use futures_util::stream::StreamExt;
//! use base58_monero::{decode_stream_check, Error};
//!
//! # tokio_test::block_on(
//! async {
//!     let mut input: &[u8] = b"D7LMXYjUbXc5LVkq6vWDY";
//!     let mut w: Vec<u8> = vec![];
//!
//!     let s = decode_stream_check(&mut input);
//!     pin_mut!(s);
//!
//!     while let Some(value) = s.next().await {
//!         w.push(value?);
//!     }
//!
//!     assert_eq!(b"Hello World", &w[..]);
//!     # Ok::<(), Error>(())
//! }
//! # )?;
//! # Ok::<(), Error>(())
//! ```

#[cfg(feature = "stream")]
use async_stream::try_stream;
#[cfg(feature = "stream")]
use futures_util::stream::Stream;
#[cfg(all(feature = "check", feature = "stream"))]
use futures_util::{pin_mut, stream::StreamExt};
#[cfg(feature = "check")]
use tiny_keccak::{Hasher, Keccak};
#[cfg(feature = "stream")]
use tokio::io::AsyncReadExt;

use thiserror::Error;

#[cfg(feature = "stream")]
use std::io;
use std::num::Wrapping;

/// Base58 alphabet, does not contains visualy similar characters
pub const BASE58_CHARS: &[u8] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
/// Resulted block size given a `0..=8` bytes block
pub const ENCODED_BLOCK_SIZES: [usize; 9] = [0, 2, 3, 5, 6, 7, 9, 10, 11];
/// Maximum size of block to encode
pub const FULL_BLOCK_SIZE: usize = 8;
/// Size of an encoded 8 bytes block, i.e. maximum encoded block size
pub const FULL_ENCODED_BLOCK_SIZE: usize = ENCODED_BLOCK_SIZES[FULL_BLOCK_SIZE];
/// Size of checksum
pub const CHECKSUM_SIZE: usize = 4;

/// Possible errors when encoding/decoding base58 and base58-check strings
#[derive(Error, Debug)]
pub enum Error {
    /// Invalid block size, must be `1..=8`
    #[error("Invalid block size error")]
    InvalidBlockSize,
    /// Symbol not in base58 alphabet
    #[error("Invalid symbol error")]
    InvalidSymbol,
    /// Invalid 4-bytes checksum
    #[cfg(feature = "check")]
    #[cfg_attr(docsrs, doc(cfg(feature = "check")))]
    #[error("Invalid checksum error")]
    InvalidChecksum,
    /// Decoding overflow
    #[error("Overflow error")]
    Overflow,
    /// IO error on stream
    ///
    /// [PartialEq] implementation return true if the other error is also and IO error but do NOT
    /// test the wrapped errors.
    #[cfg(feature = "stream")]
    #[cfg_attr(docsrs, doc(cfg(feature = "stream")))]
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
}

impl PartialEq for Error {
    fn eq(&self, other: &Self) -> bool {
        match self {
            Error::InvalidBlockSize => matches!(other, Error::InvalidBlockSize),
            Error::InvalidSymbol => matches!(other, Error::InvalidSymbol),
            #[cfg(feature = "check")]
            Error::InvalidChecksum => matches!(other, Error::InvalidChecksum),
            Error::Overflow => matches!(other, Error::Overflow),
            #[cfg(feature = "stream")]
            // Ignore what Io error is wrapped
            Error::Io(_) => matches!(other, Error::Io(_)),
        }
    }
}

/// Utility type for handling results with base58 error type
pub type Result<T> = std::result::Result<T, Error>;

fn u8be_to_u64(data: &[u8]) -> u64 {
    let mut res = 0u64;
    for b in data {
        res = res << 8 | *b as u64;
    }
    res
}

fn encode_block(data: &[u8]) -> Result<[char; FULL_ENCODED_BLOCK_SIZE]> {
    if data.is_empty() || data.len() > FULL_BLOCK_SIZE {
        return Err(Error::InvalidBlockSize);
    }
    let mut res = ['1'; FULL_ENCODED_BLOCK_SIZE];
    let mut num = u8be_to_u64(data);
    let mut i = ENCODED_BLOCK_SIZES[data.len()];
    while i > 0 {
        let remainder: usize = (num % BASE58_CHARS.len() as u64) as usize;
        num /= BASE58_CHARS.len() as u64;
        i -= 1;
        res[i] = BASE58_CHARS[remainder] as char;
    }
    Ok(res)
}

#[derive(Debug, PartialEq, Eq)]
struct DecodedBlock {
    data: [u8; FULL_BLOCK_SIZE],
    size: usize,
}

fn decode_block(data: &[u8]) -> Result<DecodedBlock> {
    if data.len() > FULL_ENCODED_BLOCK_SIZE {
        return Err(Error::InvalidBlockSize);
    }
    let res_size = match ENCODED_BLOCK_SIZES.iter().position(|&x| x == data.len()) {
        Some(size) => size,
        None => return Err(Error::InvalidBlockSize),
    };

    let alpha: Vec<_> = Vec::from(BASE58_CHARS);
    let mut res: u128 = 0;
    let mut order = Wrapping(1);
    data.iter()
        .rev()
        .try_for_each(|&c| match alpha.iter().position(|&x| x == c) {
            Some(digit) => {
                res += order.0 * digit as u128;
                order *= Wrapping(58);
                Ok(())
            }
            None => Err(Error::InvalidSymbol),
        })?;

    let max: u128 = match res_size {
        8 => std::u64::MAX as u128 + 1,
        0..=7 => 1 << (res_size * 8),
        _ => unreachable!(),
    };

    let data = if (res as u128) < max {
        (res as u64).to_be_bytes()
    } else {
        return Err(Error::Overflow);
    };

    Ok(DecodedBlock {
        data,
        size: res_size,
    })
}

/// Encode a byte vector into a base58-encoded string
pub fn encode(data: &[u8]) -> Result<String> {
    let last_block_size = ENCODED_BLOCK_SIZES[data.len() % FULL_BLOCK_SIZE];
    let full_block_count = data.len() / FULL_BLOCK_SIZE;
    let data: Result<Vec<[char; FULL_ENCODED_BLOCK_SIZE]>> =
        data.chunks(FULL_BLOCK_SIZE).map(encode_block).collect();

    let mut i = 0;
    let mut res: Vec<char> = Vec::new();
    data?.into_iter().for_each(|v| {
        if i == full_block_count {
            res.extend_from_slice(&v[..last_block_size]);
        } else {
            res.extend_from_slice(&v);
        }
        i += 1;
    });

    let s: String = res.into_iter().collect();
    Ok(s)
}

/// Encdoe a byte stream in a base58 stream of characters
#[cfg(feature = "stream")]
#[cfg_attr(docsrs, doc(cfg(feature = "stream")))]
pub fn encode_stream<T>(mut data: T) -> impl Stream<Item = Result<char>>
where
    T: AsyncReadExt + Unpin,
{
    try_stream! {
        let mut clen = 0;
        let mut buf = [0; FULL_BLOCK_SIZE];

        loop {
            let len = data.read(&mut buf[clen..]).await?;
            clen += len;

            if len == 0 {
                // EOF reached, final block is created
                if clen > 0 {
                    let block_size = ENCODED_BLOCK_SIZES[clen];
                    for c in &encode_block(&buf[..clen])?[..block_size] {
                        yield *c;
                    }
                }

                break;
            }

            if clen == FULL_BLOCK_SIZE {
                // Buffer is full, yield a full block
                for c in &encode_block(&buf)?[..] {
                    yield *c;
                }

                clen = 0;
            }
        }
    }
}

/// Encode a byte vector into a base58-check string, adds 4 bytes checksum
#[cfg(feature = "check")]
#[cfg_attr(docsrs, doc(cfg(feature = "check")))]
pub fn encode_check(data: &[u8]) -> Result<String> {
    let mut bytes = Vec::from(data);
    let mut checksum = [0u8; 32];
    let mut hasher = Keccak::v256();
    hasher.update(&bytes[..]);
    hasher.finalize(&mut checksum);
    bytes.extend_from_slice(&checksum[..CHECKSUM_SIZE]);
    encode(&bytes[..])
}

/// Encode a byte stream in a base58 stream of characters with a 4 bytes checksum
#[cfg(all(feature = "check", feature = "stream"))]
#[cfg_attr(docsrs, doc(cfg(all(feature = "check", feature = "stream"))))]
pub fn encode_stream_check<T>(mut data: T) -> impl Stream<Item = Result<char>>
where
    T: AsyncReadExt + Unpin,
{
    try_stream! {
        let mut clen = 0;
        let mut buf = [0; FULL_BLOCK_SIZE];
        let mut checksum = [0u8; 32];
        let mut hasher = Keccak::v256();

        loop {
            let len = data.read(&mut buf[clen..]).await?;
            clen += len;

            if len == 0 {
                // EOF reached, final block is created
                hasher.update(&buf[..clen]);
                hasher.finalize(&mut checksum);

                if clen + CHECKSUM_SIZE > FULL_BLOCK_SIZE {
                    // Extend and encode the first bytes of checksum with the last block
                    let sum_size = FULL_BLOCK_SIZE - clen;
                    buf[clen..].copy_from_slice(&checksum[..sum_size]);

                    for c in &encode_block(&buf)?[..] {
                        yield *c;
                    }

                    // Return last encoded checksum bytes
                    let block_size = ENCODED_BLOCK_SIZES[CHECKSUM_SIZE - sum_size];
                    for c in &encode_block(&checksum[sum_size..CHECKSUM_SIZE])?[..block_size] {
                        yield *c;
                    }
                } else {
                    let start = clen;
                    clen += CHECKSUM_SIZE;
                    buf[start..clen].copy_from_slice(&checksum[..CHECKSUM_SIZE]);

                    let block_size = ENCODED_BLOCK_SIZES[clen];
                    for c in &encode_block(&buf[..clen])?[..block_size] {
                        yield *c;
                    }
                }

                break;
            }

            if clen == FULL_BLOCK_SIZE {
                // Buffer is full, yield a full encoded block
                hasher.update(&buf);

                for c in &encode_block(&buf)?[..] {
                    yield *c;
                }

                clen = 0;
            }
        }
    }
}

/// Decode base58-encoded string into a byte vector
pub fn decode(data: &str) -> Result<Vec<u8>> {
    let data: Result<Vec<DecodedBlock>> = data
        .as_bytes()
        .chunks(FULL_ENCODED_BLOCK_SIZE)
        .map(decode_block)
        .collect();
    let mut res = Vec::new();
    data?.into_iter().for_each(|c| {
        let bytes = &c.data[FULL_BLOCK_SIZE - c.size..];
        res.extend_from_slice(bytes);
    });
    Ok(res)
}

/// Decode base58-encoded stream in a byte stream
#[cfg(feature = "stream")]
#[cfg_attr(docsrs, doc(cfg(feature = "stream")))]
pub fn decode_stream<T>(mut data: T) -> impl Stream<Item = Result<u8>>
where
    T: AsyncReadExt + Unpin,
{
    try_stream! {
        let mut clen = 0;
        let mut buf = [0; FULL_ENCODED_BLOCK_SIZE];

        loop {
            let len = data.read(&mut buf[clen..]).await?;
            clen += len;

            if len == 0 {
                // EOF reached
                let block = decode_block(&buf[..clen])?;
                for c in &block.data[FULL_BLOCK_SIZE - block.size..] {
                    yield *c;
                }
                break;
            }

            if clen == FULL_ENCODED_BLOCK_SIZE {
                let block = decode_block(&buf)?;
                for c in &block.data[FULL_BLOCK_SIZE - block.size..] {
                    yield *c;
                }
                clen = 0;
            }
        }
    }
}

/// Decode base58-encoded with 4 bytes checksum string into a byte vector
#[cfg(feature = "check")]
#[cfg_attr(docsrs, doc(cfg(feature = "check")))]
pub fn decode_check(data: &str) -> Result<Vec<u8>> {
    let bytes = decode(data)?;
    let (bytes, checksum) = {
        let len = bytes.len();
        (
            &bytes[..len - CHECKSUM_SIZE],
            &bytes[len - CHECKSUM_SIZE..len],
        )
    };
    let mut check = [0u8; 32];
    let mut hasher = Keccak::v256();
    hasher.update(bytes);
    hasher.finalize(&mut check);

    if &check[..CHECKSUM_SIZE] == checksum {
        Ok(Vec::from(bytes))
    } else {
        Err(Error::InvalidChecksum)
    }
}

/// Decode base58-encoded stream with a 4 bytes checksum in a decoded byte stream
#[cfg(all(feature = "check", feature = "stream"))]
#[cfg_attr(docsrs, doc(cfg(all(feature = "check", feature = "stream"))))]
pub fn decode_stream_check<T>(data: T) -> impl Stream<Item = Result<u8>>
where
    T: AsyncReadExt + Unpin,
{
    try_stream! {
        let len = CHECKSUM_SIZE + 1;
        let mut clen = 0;
        let mut check = [0; CHECKSUM_SIZE];
        let mut buf = [0; CHECKSUM_SIZE + 1];

        let mut checksum = [0u8; 32];
        let mut hasher = Keccak::v256();

        let data = decode_stream(data);
        pin_mut!(data);

        while let Some(value) = data.next().await {
            buf[clen % len] = value?;
            if (clen >= CHECKSUM_SIZE) {
                check[0] = buf[(clen - CHECKSUM_SIZE) % len];
                hasher.update(&check[0..1]);
                yield check[0];
            }
            clen += 1;
        }

        hasher.finalize(&mut checksum);
        for i in 0..CHECKSUM_SIZE {
            check[i] = buf[(clen - CHECKSUM_SIZE + i) % len];
        }

        if check != &checksum[..CHECKSUM_SIZE] {
            Err(Error::InvalidChecksum)?;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        decode, decode_block, encode, encode_block, u8be_to_u64, Error, ENCODED_BLOCK_SIZES,
        FULL_BLOCK_SIZE, FULL_ENCODED_BLOCK_SIZE,
    };

    #[cfg(feature = "check")]
    use super::{decode_check, encode_check};
    #[cfg(feature = "stream")]
    use super::{decode_stream, encode_stream};
    #[cfg(all(feature = "check", feature = "stream"))]
    use super::{decode_stream_check, encode_stream_check};

    #[cfg(feature = "stream")]
    use futures_util::{pin_mut, stream::StreamExt};

    #[test]
    fn encode_wrong_block() {
        assert_eq!(encode_block(&[0u8; 0]), Err(Error::InvalidBlockSize));
        assert_eq!(
            encode_block(&[0u8; FULL_BLOCK_SIZE + 1]),
            Err(Error::InvalidBlockSize)
        );
    }

    #[test]
    fn encode_empty_value() {
        assert_eq!(encode(&[0u8; 0]), Ok(String::from("")));
    }

    #[test]
    fn decode_wrong_block() {
        assert_eq!(decode_block(&[0u8; 1]), Err(Error::InvalidBlockSize));
        assert_eq!(decode_block(&[0u8; 4]), Err(Error::InvalidBlockSize));
        assert_eq!(decode_block(&[0u8; 8]), Err(Error::InvalidBlockSize));
        assert_eq!(
            decode_block(&[0u8; FULL_ENCODED_BLOCK_SIZE + 1]),
            Err(Error::InvalidBlockSize)
        );
        //assert!(false);
    }

    macro_rules! uint_8be_to_64 {
        ($expected:expr, $string:expr) => {
            assert_eq!($expected, u8be_to_u64($string));
        };
    }

    #[test]
    fn test_u8be_to_u64() {
        uint_8be_to_64!(0x0000000000000001, b"\x01");
        uint_8be_to_64!(0x0000000000000102, b"\x01\x02");
        uint_8be_to_64!(0x0000000000010203, b"\x01\x02\x03");
        uint_8be_to_64!(0x0000000001020304, b"\x01\x02\x03\x04");
        uint_8be_to_64!(0x0000000102030405, b"\x01\x02\x03\x04\x05");
        uint_8be_to_64!(0x0000010203040506, b"\x01\x02\x03\x04\x05\x06");
        uint_8be_to_64!(0x0001020304050607, b"\x01\x02\x03\x04\x05\x06\x07");
        uint_8be_to_64!(0x0102030405060708, b"\x01\x02\x03\x04\x05\x06\x07\x08");
    }

    macro_rules! encode_block {
        ($block:expr, $expected:expr) => {
            let chars = $expected.chars().collect::<Vec<_>>();
            let res = &encode_block($block).unwrap()[..ENCODED_BLOCK_SIZES[$block.len()]];
            assert_eq!(chars, res);
        };
    }

    #[test]
    fn test_base58_encode_block() {
        encode_block!(b"\x00", "11");
        encode_block!(b"\x39", "1z");
        encode_block!(b"\xFF", "5Q");

        encode_block!(b"\x00\x00", "111");
        encode_block!(b"\x00\x39", "11z");
        encode_block!(b"\x01\x00", "15R");
        encode_block!(b"\xFF\xFF", "LUv");

        encode_block!(b"\x00\x00\x00", "11111");
        encode_block!(b"\x00\x00\x39", "1111z");
        encode_block!(b"\x01\x00\x00", "11LUw");
        encode_block!(b"\xFF\xFF\xFF", "2UzHL");

        encode_block!(b"\x00\x00\x00\x39", "11111z");
        encode_block!(b"\xFF\xFF\xFF\xFF", "7YXq9G");
        encode_block!(b"\x00\x00\x00\x00\x39", "111111z");
        encode_block!(b"\xFF\xFF\xFF\xFF\xFF", "VtB5VXc");
        encode_block!(b"\x00\x00\x00\x00\x00\x39", "11111111z");
        encode_block!(b"\xFF\xFF\xFF\xFF\xFF\xFF", "3CUsUpv9t");
        encode_block!(b"\x00\x00\x00\x00\x00\x00\x39", "111111111z");
        encode_block!(b"\xFF\xFF\xFF\xFF\xFF\xFF\xFF", "Ahg1opVcGW");
        encode_block!(b"\x00\x00\x00\x00\x00\x00\x00\x39", "1111111111z");
        encode_block!(b"\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF", "jpXCZedGfVQ");

        encode_block!(b"\x00\x00\x00\x00\x00\x00\x00\x00", "11111111111");
        encode_block!(b"\x00\x00\x00\x00\x00\x00\x00\x01", "11111111112");
        encode_block!(b"\x00\x00\x00\x00\x00\x00\x00\x08", "11111111119");
        encode_block!(b"\x00\x00\x00\x00\x00\x00\x00\x09", "1111111111A");
        encode_block!(b"\x00\x00\x00\x00\x00\x00\x00\x3A", "11111111121");
        encode_block!(b"\x00\xFF\xFF\xFF\xFF\xFF\xFF\xFF", "1Ahg1opVcGW");
        encode_block!(b"\x06\x15\x60\x13\x76\x28\x79\xF7", "22222222222");
        encode_block!(b"\x05\xE0\x22\xBA\x37\x4B\x2A\x00", "1z111111111");
    }

    macro_rules! decode_block_pos {
        ($enc:expr, $expected:expr) => {
            let res = decode_block($enc).unwrap();
            assert_eq!(&$expected[..], &res.data[FULL_BLOCK_SIZE - res.size..]);
        };
    }

    macro_rules! decode_block_neg {
        ($enc:expr, $expected:expr) => {
            assert_eq!(Err($expected), decode_block($enc));
        };
    }

    #[test]
    fn test_base58_decode_block() {
        // 1-byte block
        decode_block_neg!(b"1", Error::InvalidBlockSize);
        decode_block_neg!(b"z", Error::InvalidBlockSize);
        // 2-byte block
        decode_block_pos!(b"11", b"\x00");
        decode_block_pos!(b"5Q", b"\xFF");
        decode_block_neg!(b"5R", Error::Overflow);
        decode_block_neg!(b"zz", Error::Overflow);
        // 3-bytes block
        decode_block_pos!(b"111", b"\x00\x00");
        decode_block_pos!(b"LUv", b"\xFF\xFF");
        decode_block_neg!(b"LUw", Error::Overflow);
        decode_block_neg!(b"zzz", Error::Overflow);
        // 4-bytes block
        decode_block_neg!(b"1111", Error::InvalidBlockSize);
        decode_block_neg!(b"zzzz", Error::InvalidBlockSize);
        // 5-bytes block
        decode_block_pos!(b"11111", b"\x00\x00\x00");
        decode_block_pos!(b"2UzHL", b"\xFF\xFF\xFF");
        decode_block_neg!(b"2UzHM", Error::Overflow);
        decode_block_neg!(b"zzzzz", Error::Overflow);
        // 6-bytes block
        decode_block_pos!(b"111111", b"\x00\x00\x00\x00");
        decode_block_pos!(b"7YXq9G", b"\xFF\xFF\xFF\xFF");
        decode_block_neg!(b"7YXq9H", Error::Overflow);
        decode_block_neg!(b"zzzzzz", Error::Overflow);
        // 7-bytes block
        decode_block_pos!(b"1111111", b"\x00\x00\x00\x00\x00");
        decode_block_pos!(b"VtB5VXc", b"\xFF\xFF\xFF\xFF\xFF");
        decode_block_neg!(b"VtB5VXd", Error::Overflow);
        decode_block_neg!(b"zzzzzzz", Error::Overflow);
        // 8-bytes block
        decode_block_neg!(b"11111111", Error::InvalidBlockSize);
        decode_block_neg!(b"zzzzzzzz", Error::InvalidBlockSize);
        // 9-bytes block
        decode_block_pos!(b"111111111", b"\x00\x00\x00\x00\x00\x00");
        decode_block_pos!(b"3CUsUpv9t", b"\xFF\xFF\xFF\xFF\xFF\xFF");
        decode_block_neg!(b"3CUsUpv9u", Error::Overflow);
        decode_block_neg!(b"zzzzzzzzz", Error::Overflow);
        // 10-bytes block
        decode_block_pos!(b"1111111111", b"\x00\x00\x00\x00\x00\x00\x00");
        decode_block_pos!(b"Ahg1opVcGW", b"\xFF\xFF\xFF\xFF\xFF\xFF\xFF");
        decode_block_neg!(b"Ahg1opVcGX", Error::Overflow);
        decode_block_neg!(b"zzzzzzzzzz", Error::Overflow);
        // 11-bytes block
        decode_block_pos!(b"11111111111", b"\x00\x00\x00\x00\x00\x00\x00\x00");
        decode_block_pos!(b"jpXCZedGfVQ", b"\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF");
        decode_block_neg!(b"jpXCZedGfVR", Error::Overflow);
        decode_block_neg!(b"zzzzzzzzzzz", Error::Overflow);
        // Invalid symbolsb"
        decode_block_neg!(b"01111111111", Error::InvalidSymbol);
        decode_block_neg!(b"11111111110", Error::InvalidSymbol);
        decode_block_neg!(b"11111011111", Error::InvalidSymbol);
        decode_block_neg!(b"I1111111111", Error::InvalidSymbol);
        decode_block_neg!(b"O1111111111", Error::InvalidSymbol);
        decode_block_neg!(b"l1111111111", Error::InvalidSymbol);
        decode_block_neg!(b"_1111111111", Error::InvalidSymbol);
    }

    macro_rules! encode {
        ($expected:expr, $data:expr) => {
            assert_eq!(Ok(String::from($expected)), encode($data));
        };
    }

    #[test]
    fn test_base58_encode() {
        encode!("11", b"\x00");
        encode!("111", b"\x00\x00");
        encode!("11111", b"\x00\x00\x00");
        encode!("111111", b"\x00\x00\x00\x00");
        encode!("1111111", b"\x00\x00\x00\x00\x00");
        encode!("111111111", b"\x00\x00\x00\x00\x00\x00");
        encode!("1111111111", b"\x00\x00\x00\x00\x00\x00\x00");
        encode!("11111111111", b"\x00\x00\x00\x00\x00\x00\x00\x00");
        encode!("1111111111111", b"\x00\x00\x00\x00\x00\x00\x00\x00\x00");
        encode!(
            "11111111111111",
            b"\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00"
        );
        encode!(
            "1111111111111111",
            b"\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00"
        );
        encode!(
            "11111111111111111",
            b"\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00"
        );
        encode!(
            "111111111111111111",
            b"\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00"
        );
        encode!(
            "11111111111111111111",
            b"\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00"
        );
        encode!(
            "111111111111111111111",
            b"\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00"
        );
        encode!(
            "1111111111111111111111",
            b"\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00"
        );
        encode!(
            "22222222222VtB5VXc",
            b"\x06\x15\x60\x13\x76\x28\x79\xF7\xFF\xFF\xFF\xFF\xFF"
        );
    }

    macro_rules! decode_pos {
        ($enc:expr, $expected:expr) => {
            assert_eq!(Ok(Vec::from(&$expected[..])), decode($enc));
        };
    }

    macro_rules! decode_neg {
        ($expected:expr, $enc:expr) => {
            assert_eq!(Err($expected), decode($enc));
        };
    }

    #[test]
    fn test_base58_decode() {
        decode_pos!("", b"");
        decode_pos!("5Q", b"\xFF");
        decode_pos!("LUv", b"\xFF\xFF");
        decode_pos!("2UzHL", b"\xFF\xFF\xFF");
        decode_pos!("7YXq9G", b"\xFF\xFF\xFF\xFF");
        decode_pos!("VtB5VXc", b"\xFF\xFF\xFF\xFF\xFF");
        decode_pos!("3CUsUpv9t", b"\xFF\xFF\xFF\xFF\xFF\xFF");
        decode_pos!("Ahg1opVcGW", b"\xFF\xFF\xFF\xFF\xFF\xFF\xFF");
        decode_pos!("jpXCZedGfVQ", b"\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF");
        decode_pos!("jpXCZedGfVQ5Q", b"\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF");
        decode_pos!(
            "jpXCZedGfVQLUv",
            b"\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF"
        );
        decode_pos!(
            "jpXCZedGfVQ2UzHL",
            b"\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF"
        );
        decode_pos!(
            "jpXCZedGfVQ7YXq9G",
            b"\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF"
        );
        decode_pos!(
            "jpXCZedGfVQVtB5VXc",
            b"\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF"
        );
        decode_pos!(
            "jpXCZedGfVQ3CUsUpv9t",
            b"\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF"
        );
        decode_pos!(
            "jpXCZedGfVQAhg1opVcGW",
            b"\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF"
        );
        decode_pos!(
            "jpXCZedGfVQjpXCZedGfVQ",
            b"\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF"
        );
        // Invalid length
        decode_neg!(Error::InvalidBlockSize, "1");
        decode_neg!(Error::InvalidBlockSize, "z");
        decode_neg!(Error::InvalidBlockSize, "1111");
        decode_neg!(Error::InvalidBlockSize, "zzzz");
        decode_neg!(Error::InvalidBlockSize, "11111111");
        decode_neg!(Error::InvalidBlockSize, "zzzzzzzz");
        decode_neg!(Error::InvalidBlockSize, "123456789AB1");
        decode_neg!(Error::InvalidBlockSize, "123456789ABz");
        decode_neg!(Error::InvalidBlockSize, "123456789AB1111");
        decode_neg!(Error::InvalidBlockSize, "123456789ABzzzz");
        decode_neg!(Error::InvalidBlockSize, "123456789AB11111111");
        decode_neg!(Error::InvalidBlockSize, "123456789ABzzzzzzzz");
        // Overflow
        decode_neg!(Error::Overflow, "5R");
        decode_neg!(Error::Overflow, "zz");
        decode_neg!(Error::Overflow, "LUw");
        decode_neg!(Error::Overflow, "zzz");
        decode_neg!(Error::Overflow, "2UzHM");
        decode_neg!(Error::Overflow, "zzzzz");
        decode_neg!(Error::Overflow, "7YXq9H");
        decode_neg!(Error::Overflow, "zzzzzz");
        decode_neg!(Error::Overflow, "VtB5VXd");
        decode_neg!(Error::Overflow, "zzzzzzz");
        decode_neg!(Error::Overflow, "3CUsUpv9u");
        decode_neg!(Error::Overflow, "zzzzzzzzz");
        decode_neg!(Error::Overflow, "Ahg1opVcGX");
        decode_neg!(Error::Overflow, "zzzzzzzzzz");
        decode_neg!(Error::Overflow, "jpXCZedGfVR");
        decode_neg!(Error::Overflow, "zzzzzzzzzzz");
        decode_neg!(Error::Overflow, "123456789AB5R");
        decode_neg!(Error::Overflow, "123456789ABzz");
        decode_neg!(Error::Overflow, "123456789ABLUw");
        decode_neg!(Error::Overflow, "123456789ABzzz");
        decode_neg!(Error::Overflow, "123456789AB2UzHM");
        decode_neg!(Error::Overflow, "123456789ABzzzzz");
        decode_neg!(Error::Overflow, "123456789AB7YXq9H");
        decode_neg!(Error::Overflow, "123456789ABzzzzzz");
        decode_neg!(Error::Overflow, "123456789ABVtB5VXd");
        decode_neg!(Error::Overflow, "123456789ABzzzzzzz");
        decode_neg!(Error::Overflow, "123456789AB3CUsUpv9u");
        decode_neg!(Error::Overflow, "123456789ABzzzzzzzzz");
        decode_neg!(Error::Overflow, "123456789ABAhg1opVcGX");
        decode_neg!(Error::Overflow, "123456789ABzzzzzzzzzz");
        decode_neg!(Error::Overflow, "123456789ABjpXCZedGfVR");
        decode_neg!(Error::Overflow, "123456789ABzzzzzzzzzzz");
        decode_neg!(Error::Overflow, "zzzzzzzzzzz11");
        // Invalid symbols
        decode_neg!(Error::InvalidSymbol, "10");
        decode_neg!(Error::InvalidSymbol, "11I");
        decode_neg!(Error::InvalidSymbol, "11O11");
        decode_neg!(Error::InvalidSymbol, "11l111");
        decode_neg!(Error::InvalidSymbol, "11_11111111");
        decode_neg!(Error::InvalidSymbol, "1101111111111");
        decode_neg!(Error::InvalidSymbol, "11I11111111111111");
        decode_neg!(Error::InvalidSymbol, "11O1111111111111111111");
        decode_neg!(Error::InvalidSymbol, "1111111111110");
        decode_neg!(Error::InvalidSymbol, "111111111111l1111");
        decode_neg!(Error::InvalidSymbol, "111111111111_111111111");
    }

    #[cfg(feature = "stream")]
    macro_rules! encode_stream {
        ($stream:expr, $expected:expr, $func:expr) => {
            let mut input: &[u8] = $stream;
            let s = $func(&mut input);
            pin_mut!(s);

            let mut w: Vec<char> = vec![];

            while let Some(value) = s.next().await {
                w.push(value.unwrap());
            }

            let s: String = w.into_iter().collect();
            assert_eq!(&$expected[..], &s[..]);
        };
    }

    #[tokio::test(flavor = "multi_thread")]
    #[cfg(feature = "stream")]
    async fn test_base58_encode_stream() {
        encode_stream!(b"\x00", "11", encode_stream);
        encode_stream!(b"\x39", "1z", encode_stream);
        encode_stream!(b"\xFF", "5Q", encode_stream);

        encode_stream!(b"\x00\x00", "111", encode_stream);
        encode_stream!(b"\x00\x39", "11z", encode_stream);
        encode_stream!(b"\x01\x00", "15R", encode_stream);
        encode_stream!(b"\xFF\xFF", "LUv", encode_stream);

        encode_stream!(b"\x00\x00\x00", "11111", encode_stream);
        encode_stream!(b"\x00\x00\x39", "1111z", encode_stream);
        encode_stream!(b"\x01\x00\x00", "11LUw", encode_stream);
        encode_stream!(b"\xFF\xFF\xFF", "2UzHL", encode_stream);

        encode_stream!(b"\x00\x00\x00\x39", "11111z", encode_stream);
        encode_stream!(b"\xFF\xFF\xFF\xFF", "7YXq9G", encode_stream);
        encode_stream!(b"\x00\x00\x00\x00\x39", "111111z", encode_stream);
        encode_stream!(b"\xFF\xFF\xFF\xFF\xFF", "VtB5VXc", encode_stream);
        encode_stream!(b"\x00\x00\x00\x00\x00\x39", "11111111z", encode_stream);
        encode_stream!(b"\xFF\xFF\xFF\xFF\xFF\xFF", "3CUsUpv9t", encode_stream);
        encode_stream!(b"\x00\x00\x00\x00\x00\x00\x39", "111111111z", encode_stream);
        encode_stream!(b"\xFF\xFF\xFF\xFF\xFF\xFF\xFF", "Ahg1opVcGW", encode_stream);

        encode_stream!(
            b"\x00\x00\x00\x00\x00\x00\x00\x39",
            "1111111111z",
            encode_stream
        );
        encode_stream!(
            b"\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF",
            "jpXCZedGfVQ",
            encode_stream
        );
        encode_stream!(
            b"\x00\x00\x00\x00\x00\x00\x00\x00",
            "11111111111",
            encode_stream
        );
        encode_stream!(
            b"\x00\x00\x00\x00\x00\x00\x00\x01",
            "11111111112",
            encode_stream
        );
        encode_stream!(
            b"\x00\x00\x00\x00\x00\x00\x00\x08",
            "11111111119",
            encode_stream
        );
        encode_stream!(
            b"\x00\x00\x00\x00\x00\x00\x00\x09",
            "1111111111A",
            encode_stream
        );
        encode_stream!(
            b"\x00\x00\x00\x00\x00\x00\x00\x3A",
            "11111111121",
            encode_stream
        );
        encode_stream!(
            b"\x00\xFF\xFF\xFF\xFF\xFF\xFF\xFF",
            "1Ahg1opVcGW",
            encode_stream
        );
        encode_stream!(
            b"\x06\x15\x60\x13\x76\x28\x79\xF7",
            "22222222222",
            encode_stream
        );
        encode_stream!(
            b"\x05\xE0\x22\xBA\x37\x4B\x2A\x00",
            "1z111111111",
            encode_stream
        );

        encode_stream!(b"\x00", "11", encode_stream);
        encode_stream!(b"\x00\x00", "111", encode_stream);
        encode_stream!(b"\x00\x00\x00", "11111", encode_stream);
        encode_stream!(b"\x00\x00\x00\x00", "111111", encode_stream);
        encode_stream!(b"\x00\x00\x00\x00\x00", "1111111", encode_stream);
        encode_stream!(b"\x00\x00\x00\x00\x00\x00", "111111111", encode_stream);
        encode_stream!(b"\x00\x00\x00\x00\x00\x00\x00", "1111111111", encode_stream);
        encode_stream!(
            b"\x00\x00\x00\x00\x00\x00\x00\x00",
            "11111111111",
            encode_stream
        );
        encode_stream!(
            b"\x00\x00\x00\x00\x00\x00\x00\x00\x00",
            "1111111111111",
            encode_stream
        );

        encode_stream!(
            b"\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00",
            "11111111111111",
            encode_stream
        );
        encode_stream!(
            b"\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00",
            "1111111111111111",
            encode_stream
        );
        encode_stream!(
            b"\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00",
            "11111111111111111",
            encode_stream
        );
        encode_stream!(
            b"\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00",
            "111111111111111111",
            encode_stream
        );
        encode_stream!(
            b"\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00",
            "11111111111111111111",
            encode_stream
        );
        encode_stream!(
            b"\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00",
            "111111111111111111111",
            encode_stream
        );
        encode_stream!(
            b"\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00",
            "1111111111111111111111",
            encode_stream
        );
        encode_stream!(
            b"\x06\x15\x60\x13\x76\x28\x79\xF7\xFF\xFF\xFF\xFF\xFF",
            "22222222222VtB5VXc",
            encode_stream
        );
    }

    #[cfg(feature = "stream")]
    macro_rules! decode_stream_pos {
        ($enc:expr, $expected:expr) => {
            let mut input: &[u8] = $enc;
            let s = decode_stream(&mut input);
            pin_mut!(s);

            let mut w: Vec<u8> = vec![];

            while let Some(value) = s.next().await {
                w.push(value.unwrap());
            }

            assert_eq!(Vec::from(&$expected[..]), w);
        };
    }

    #[cfg(feature = "stream")]
    macro_rules! decode_stream_neg {
        ($expected:expr, $enc:expr) => {
            let mut input: &[u8] = $enc;
            let s = decode_stream(&mut input);
            pin_mut!(s);

            while let Some(value) = s.next().await {
                match value {
                    Ok(_) => (),
                    Err(e) => assert_eq!($expected, e),
                }
            }
        };
    }

    #[tokio::test(flavor = "multi_thread")]
    #[cfg(feature = "stream")]
    async fn test_base58_decode_stream() {
        decode_stream_pos!(b"", b"");
        decode_stream_pos!(b"5Q", b"\xFF");
        decode_stream_pos!(b"LUv", b"\xFF\xFF");
        decode_stream_pos!(b"2UzHL", b"\xFF\xFF\xFF");
        decode_stream_pos!(b"7YXq9G", b"\xFF\xFF\xFF\xFF");
        decode_stream_pos!(b"VtB5VXc", b"\xFF\xFF\xFF\xFF\xFF");
        decode_stream_pos!(b"3CUsUpv9t", b"\xFF\xFF\xFF\xFF\xFF\xFF");
        decode_stream_pos!(b"Ahg1opVcGW", b"\xFF\xFF\xFF\xFF\xFF\xFF\xFF");
        decode_stream_pos!(b"jpXCZedGfVQ", b"\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF");
        decode_stream_pos!(b"jpXCZedGfVQ5Q", b"\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF");
        decode_stream_pos!(
            b"jpXCZedGfVQLUv",
            b"\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF"
        );
        decode_stream_pos!(
            b"jpXCZedGfVQ2UzHL",
            b"\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF"
        );
        decode_stream_pos!(
            b"jpXCZedGfVQ7YXq9G",
            b"\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF"
        );
        decode_stream_pos!(
            b"jpXCZedGfVQVtB5VXc",
            b"\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF"
        );
        decode_stream_pos!(
            b"jpXCZedGfVQ3CUsUpv9t",
            b"\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF"
        );
        decode_stream_pos!(
            b"jpXCZedGfVQAhg1opVcGW",
            b"\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF"
        );
        decode_stream_pos!(
            b"jpXCZedGfVQjpXCZedGfVQ",
            b"\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF"
        );
        // Invalid length
        decode_stream_neg!(Error::InvalidBlockSize, b"1");
        decode_stream_neg!(Error::InvalidBlockSize, b"z");
        decode_stream_neg!(Error::InvalidBlockSize, b"1111");
        decode_stream_neg!(Error::InvalidBlockSize, b"zzzz");
        decode_stream_neg!(Error::InvalidBlockSize, b"11111111");
        decode_stream_neg!(Error::InvalidBlockSize, b"zzzzzzzz");
        decode_stream_neg!(Error::InvalidBlockSize, b"123456789AB1");
        decode_stream_neg!(Error::InvalidBlockSize, b"123456789ABz");
        decode_stream_neg!(Error::InvalidBlockSize, b"123456789AB1111");
        decode_stream_neg!(Error::InvalidBlockSize, b"123456789ABzzzz");
        decode_stream_neg!(Error::InvalidBlockSize, b"123456789AB11111111");
        decode_stream_neg!(Error::InvalidBlockSize, b"123456789ABzzzzzzzz");
        // Overflow
        decode_stream_neg!(Error::Overflow, b"5R");
        decode_stream_neg!(Error::Overflow, b"zz");
        decode_stream_neg!(Error::Overflow, b"LUw");
        decode_stream_neg!(Error::Overflow, b"zzz");
        decode_stream_neg!(Error::Overflow, b"2UzHM");
        decode_stream_neg!(Error::Overflow, b"zzzzz");
        decode_stream_neg!(Error::Overflow, b"7YXq9H");
        decode_stream_neg!(Error::Overflow, b"zzzzzz");
        decode_stream_neg!(Error::Overflow, b"VtB5VXd");
        decode_stream_neg!(Error::Overflow, b"zzzzzzz");
        decode_stream_neg!(Error::Overflow, b"3CUsUpv9u");
        decode_stream_neg!(Error::Overflow, b"zzzzzzzzz");
        decode_stream_neg!(Error::Overflow, b"Ahg1opVcGX");
        decode_stream_neg!(Error::Overflow, b"zzzzzzzzzz");
        decode_stream_neg!(Error::Overflow, b"jpXCZedGfVR");
        decode_stream_neg!(Error::Overflow, b"zzzzzzzzzzz");
        decode_stream_neg!(Error::Overflow, b"123456789AB5R");
        decode_stream_neg!(Error::Overflow, b"123456789ABzz");
        decode_stream_neg!(Error::Overflow, b"123456789ABLUw");
        decode_stream_neg!(Error::Overflow, b"123456789ABzzz");
        decode_stream_neg!(Error::Overflow, b"123456789AB2UzHM");
        decode_stream_neg!(Error::Overflow, b"123456789ABzzzzz");
        decode_stream_neg!(Error::Overflow, b"123456789AB7YXq9H");
        decode_stream_neg!(Error::Overflow, b"123456789ABzzzzzz");
        decode_stream_neg!(Error::Overflow, b"123456789ABVtB5VXd");
        decode_stream_neg!(Error::Overflow, b"123456789ABzzzzzzz");
        decode_stream_neg!(Error::Overflow, b"123456789AB3CUsUpv9u");
        decode_stream_neg!(Error::Overflow, b"123456789ABzzzzzzzzz");
        decode_stream_neg!(Error::Overflow, b"123456789ABAhg1opVcGX");
        decode_stream_neg!(Error::Overflow, b"123456789ABzzzzzzzzzz");
        decode_stream_neg!(Error::Overflow, b"123456789ABjpXCZedGfVR");
        decode_stream_neg!(Error::Overflow, b"123456789ABzzzzzzzzzzz");
        decode_stream_neg!(Error::Overflow, b"zzzzzzzzzzz11");
        // Invalid symbols
        decode_stream_neg!(Error::InvalidSymbol, b"10");
        decode_stream_neg!(Error::InvalidSymbol, b"11I");
        decode_stream_neg!(Error::InvalidSymbol, b"11O11");
        decode_stream_neg!(Error::InvalidSymbol, b"11l111");
        decode_stream_neg!(Error::InvalidSymbol, b"11_11111111");
        decode_stream_neg!(Error::InvalidSymbol, b"1101111111111");
        decode_stream_neg!(Error::InvalidSymbol, b"11I11111111111111");
        decode_stream_neg!(Error::InvalidSymbol, b"11O1111111111111111111");
        decode_stream_neg!(Error::InvalidSymbol, b"1111111111110");
        decode_stream_neg!(Error::InvalidSymbol, b"111111111111l1111");
        decode_stream_neg!(Error::InvalidSymbol, b"111111111111_111111111");
    }

    macro_rules! encode_address {
        ($expected:expr, $hex:expr, $func:expr) => {
            let hex = hex::decode($hex).unwrap();
            assert_eq!(Ok(String::from($expected)), $func(&hex[..]));
        };
    }

    #[test]
    fn test_base58_encode_address() {
        encode_address!(
            "4Au2dGq2uFHWapfkU1RF4X6tFdY1rKtNfJrfsNSUinrRK3d8ZBViLtz5NGQiBM1xM5LeD4ak5Q2869PfC7hUWuDA5RzvSk5",
            "12f4bd0587c43594b0ddb2ef4e616d24232d14eee07f45b46ac19ef3b11e7c7e6be2a59b6284ad5b1a1b43051d07e788756dcfff36008637322a1c975eeb6149274647451e",
            encode
        );
        encode_address!(
            "47Qa9iJeiYxDzKakP4SxpWD9zKB7B1nYgFcF9TdvxVzXLmdR6dX8BNPKiAyyZqVbcPEr2TYdJrRxC1YfM1APP9qg1oBnVip",
            "1298a05f07a0c9f94da6e0bb1ebe819748ab787e95b72f6157555d2fa45644e076319c740890b4f86fdbe5528942af2c52c6810b6c9773d903437c090d99b397070ee1d7d1",
            encode
        );
        encode_address!(
            "46eu6J7WC5jVYLT2NPovGDRs9NMyJpeH1WcKfBRNZ1CLVDjTDtKopLiYwAmhc4Bx9gf17DGe6CubRe8mm3Z1HNqgTNKbyu8",
            "1284c19bf4557a66aaa18f2af53814d694a7ccf0c6a245bcb10546ea40f6e261a8b6a587843c6943beeba8f386547f53e332bcef66bfee04de027879b51ec5fbe9b5f398bf",
            encode
        );
        encode_address!(
            "46pRWGRUvUvJ3Rh7kRujCW1jMASA18S9xELAuPT28dguAoHfhLZVKqshUHF7XwdmUZjCx1jaEkYHWPPz7WVkz26TMbFxFq2",
            "128916f019baad1f65e2eb2deae8af83045d7be1accf57034fb2b23b72a4cf023a9429b5ffcaf9daf1f4d5e3c85906aefc554f15e95956c185e60e5521cb71b8b681a61491",
            encode
        );
    }

    #[test]
    #[cfg(feature = "check")]
    fn test_base58_encode_check() {
        encode_address!(
            "4Au2dGq2uFHWapfkU1RF4X6tFdY1rKtNfJrfsNSUinrRK3d8ZBViLtz5NGQiBM1xM5LeD4ak5Q2869PfC7hUWuDA5RzvSk5",
            "12f4bd0587c43594b0ddb2ef4e616d24232d14eee07f45b46ac19ef3b11e7c7e6be2a59b6284ad5b1a1b43051d07e788756dcfff36008637322a1c975eeb614927",
            encode_check
        );
        encode_address!(
            "47Qa9iJeiYxDzKakP4SxpWD9zKB7B1nYgFcF9TdvxVzXLmdR6dX8BNPKiAyyZqVbcPEr2TYdJrRxC1YfM1APP9qg1oBnVip",
            "1298a05f07a0c9f94da6e0bb1ebe819748ab787e95b72f6157555d2fa45644e076319c740890b4f86fdbe5528942af2c52c6810b6c9773d903437c090d99b39707",
            encode_check
        );
        encode_address!(
            "46eu6J7WC5jVYLT2NPovGDRs9NMyJpeH1WcKfBRNZ1CLVDjTDtKopLiYwAmhc4Bx9gf17DGe6CubRe8mm3Z1HNqgTNKbyu8",
            "1284c19bf4557a66aaa18f2af53814d694a7ccf0c6a245bcb10546ea40f6e261a8b6a587843c6943beeba8f386547f53e332bcef66bfee04de027879b51ec5fbe9",
            encode_check
        );
        encode_address!(
            "46pRWGRUvUvJ3Rh7kRujCW1jMASA18S9xELAuPT28dguAoHfhLZVKqshUHF7XwdmUZjCx1jaEkYHWPPz7WVkz26TMbFxFq2",
            "128916f019baad1f65e2eb2deae8af83045d7be1accf57034fb2b23b72a4cf023a9429b5ffcaf9daf1f4d5e3c85906aefc554f15e95956c185e60e5521cb71b8b6",
            encode_check
        );
    }

    #[cfg(all(feature = "check", feature = "stream"))]
    macro_rules! encode_stream_address {
        ($stream:expr, $expected:expr, $func:expr) => {
            let mut input: &[u8] = &hex::decode($stream).unwrap()[..];
            let s = $func(&mut input);
            pin_mut!(s);

            let mut w: Vec<char> = vec![];

            while let Some(value) = s.next().await {
                w.push(value.unwrap());
            }

            let s: String = w.into_iter().collect();
            assert_eq!(&$expected[..], &s[..]);
        };
    }

    #[tokio::test(flavor = "multi_thread")]
    #[cfg(all(feature = "check", feature = "stream"))]
    async fn test_base58_encode_stream_check() {
        encode_stream_address!(
            "12f4bd0587c43594b0ddb2ef4e616d24232d14eee07f45b46ac19ef3b11e7c7e6be2a59b6284ad5b1a1b43051d07e788756dcfff36008637322a1c975eeb614927",
            "4Au2dGq2uFHWapfkU1RF4X6tFdY1rKtNfJrfsNSUinrRK3d8ZBViLtz5NGQiBM1xM5LeD4ak5Q2869PfC7hUWuDA5RzvSk5",
            encode_stream_check
        );
        encode_stream_address!(
            "1298a05f07a0c9f94da6e0bb1ebe819748ab787e95b72f6157555d2fa45644e076319c740890b4f86fdbe5528942af2c52c6810b6c9773d903437c090d99b39707",
            "47Qa9iJeiYxDzKakP4SxpWD9zKB7B1nYgFcF9TdvxVzXLmdR6dX8BNPKiAyyZqVbcPEr2TYdJrRxC1YfM1APP9qg1oBnVip",
            encode_stream_check
        );
        encode_stream_address!(
            "1284c19bf4557a66aaa18f2af53814d694a7ccf0c6a245bcb10546ea40f6e261a8b6a587843c6943beeba8f386547f53e332bcef66bfee04de027879b51ec5fbe9",
            "46eu6J7WC5jVYLT2NPovGDRs9NMyJpeH1WcKfBRNZ1CLVDjTDtKopLiYwAmhc4Bx9gf17DGe6CubRe8mm3Z1HNqgTNKbyu8",
            encode_stream_check
        );
        encode_stream_address!(
            "128916f019baad1f65e2eb2deae8af83045d7be1accf57034fb2b23b72a4cf023a9429b5ffcaf9daf1f4d5e3c85906aefc554f15e95956c185e60e5521cb71b8b6",
            "46pRWGRUvUvJ3Rh7kRujCW1jMASA18S9xELAuPT28dguAoHfhLZVKqshUHF7XwdmUZjCx1jaEkYHWPPz7WVkz26TMbFxFq2",
            encode_stream_check
        );
    }

    macro_rules! decode_address {
        ($expected:expr, $addr:expr, $func:expr) => {
            let hex = hex::decode($expected).unwrap();
            assert_eq!(Ok(hex), $func($addr));
        };
    }

    #[cfg(feature = "check")]
    macro_rules! decode_address_neg {
        ($expected:expr, $addr:expr, $func:expr) => {
            assert_eq!(Err($expected), $func($addr));
        };
    }

    #[test]
    fn test_base58_decode_address() {
        decode_address!(
            "12f4bd0587c43594b0ddb2ef4e616d24232d14eee07f45b46ac19ef3b11e7c7e6be2a59b6284ad5b1a1b43051d07e788756dcfff36008637322a1c975eeb6149274647451e",
            "4Au2dGq2uFHWapfkU1RF4X6tFdY1rKtNfJrfsNSUinrRK3d8ZBViLtz5NGQiBM1xM5LeD4ak5Q2869PfC7hUWuDA5RzvSk5",
            decode
        );
        decode_address!(
            "1298a05f07a0c9f94da6e0bb1ebe819748ab787e95b72f6157555d2fa45644e076319c740890b4f86fdbe5528942af2c52c6810b6c9773d903437c090d99b397070ee1d7d1",
            "47Qa9iJeiYxDzKakP4SxpWD9zKB7B1nYgFcF9TdvxVzXLmdR6dX8BNPKiAyyZqVbcPEr2TYdJrRxC1YfM1APP9qg1oBnVip",
            decode
        );
        decode_address!(
            "1284c19bf4557a66aaa18f2af53814d694a7ccf0c6a245bcb10546ea40f6e261a8b6a587843c6943beeba8f386547f53e332bcef66bfee04de027879b51ec5fbe9b5f398bf",
            "46eu6J7WC5jVYLT2NPovGDRs9NMyJpeH1WcKfBRNZ1CLVDjTDtKopLiYwAmhc4Bx9gf17DGe6CubRe8mm3Z1HNqgTNKbyu8",
            decode
        );
        decode_address!(
            "128916f019baad1f65e2eb2deae8af83045d7be1accf57034fb2b23b72a4cf023a9429b5ffcaf9daf1f4d5e3c85906aefc554f15e95956c185e60e5521cb71b8b681a61491",
            "46pRWGRUvUvJ3Rh7kRujCW1jMASA18S9xELAuPT28dguAoHfhLZVKqshUHF7XwdmUZjCx1jaEkYHWPPz7WVkz26TMbFxFq2",
            decode
        );
    }

    #[test]
    #[cfg(feature = "check")]
    fn test_base58_decode_check() {
        decode_address!(
            "12f4bd0587c43594b0ddb2ef4e616d24232d14eee07f45b46ac19ef3b11e7c7e6be2a59b6284ad5b1a1b43051d07e788756dcfff36008637322a1c975eeb614927",
            "4Au2dGq2uFHWapfkU1RF4X6tFdY1rKtNfJrfsNSUinrRK3d8ZBViLtz5NGQiBM1xM5LeD4ak5Q2869PfC7hUWuDA5RzvSk5",
            decode_check
        );
        decode_address!(
            "1298a05f07a0c9f94da6e0bb1ebe819748ab787e95b72f6157555d2fa45644e076319c740890b4f86fdbe5528942af2c52c6810b6c9773d903437c090d99b39707",
            "47Qa9iJeiYxDzKakP4SxpWD9zKB7B1nYgFcF9TdvxVzXLmdR6dX8BNPKiAyyZqVbcPEr2TYdJrRxC1YfM1APP9qg1oBnVip",
            decode_check
        );
        decode_address!(
            "1284c19bf4557a66aaa18f2af53814d694a7ccf0c6a245bcb10546ea40f6e261a8b6a587843c6943beeba8f386547f53e332bcef66bfee04de027879b51ec5fbe9",
            "46eu6J7WC5jVYLT2NPovGDRs9NMyJpeH1WcKfBRNZ1CLVDjTDtKopLiYwAmhc4Bx9gf17DGe6CubRe8mm3Z1HNqgTNKbyu8",
            decode_check
        );
        decode_address!(
            "128916f019baad1f65e2eb2deae8af83045d7be1accf57034fb2b23b72a4cf023a9429b5ffcaf9daf1f4d5e3c85906aefc554f15e95956c185e60e5521cb71b8b6",
            "46pRWGRUvUvJ3Rh7kRujCW1jMASA18S9xELAuPT28dguAoHfhLZVKqshUHF7XwdmUZjCx1jaEkYHWPPz7WVkz26TMbFxFq2",
            decode_check
        );

        decode_address_neg!(
            Error::InvalidChecksum,
            "46pRWGRUvUvJ3Rh7kRujCW1jMASA18S9xELAuPT28dguAoHfhLZVKqshUHF7XwdmUZjCx1jaEkYHWPPz7WVkz26TMbFxFq3",
            decode_check
        );
        decode_address_neg!(
            Error::InvalidChecksum,
            "46Qa9iJeiYxDzKakP4SxpWD9zKB7B1nYgFcF9TdvxVzXLmdR6dX8BNPKiAyyZqVbcPEr2TYdJrRxC1YfM1APP9qg1oBnVip",
            decode_check
        );
        decode_address_neg!(
            Error::InvalidChecksum,
            "46eu6J7WC5jVYLT2NPovGDRs9NMyJpeH1WcKfBRNZ1CLV3jTDtKopLiYwAmhc4Bx9gf17DGe6CubRe8mm3Z1HNqgTNKbyu8",
            decode_check
        );
        decode_address_neg!(
            Error::InvalidChecksum,
            "46pRWGRUvUvJ3Rh7kRujCW1jMASA18S9xELAuPT28dguA1HfhLZVKqshUHF7XwdmUZjCx1jaEkYHWPPz7WVkz26TMbFxFq2",
            decode_check
        );
    }

    #[cfg(all(feature = "check", feature = "stream"))]
    macro_rules! decode_stream_address {
        ($stream:expr, $expected:expr, $func:expr) => {
            let mut input: &[u8] = &$stream[..];
            let s = $func(&mut input);
            pin_mut!(s);

            let mut w: Vec<u8> = vec![];

            while let Some(value) = s.next().await {
                w.push(value.unwrap());
            }

            assert_eq!(hex::decode($expected).unwrap(), w);
        };
    }

    #[cfg(all(feature = "check", feature = "stream"))]
    macro_rules! decode_stream_address_neg {
        ($expected:expr, $stream:expr, $func:expr) => {
            let mut input: &[u8] = &$stream[..];
            let s = $func(&mut input);
            pin_mut!(s);

            while let Some(value) = s.next().await {
                match value {
                    Ok(_) => (),
                    Err(e) => assert_eq!($expected, e),
                }
            }
        };
    }

    #[tokio::test(flavor = "multi_thread")]
    #[cfg(all(feature = "check", feature = "stream"))]
    async fn test_base58_decode_stream_check() {
        decode_stream_address!(
            b"4Au2dGq2uFHWapfkU1RF4X6tFdY1rKtNfJrfsNSUinrRK3d8ZBViLtz5NGQiBM1xM5LeD4ak5Q2869PfC7hUWuDA5RzvSk5",
            "12f4bd0587c43594b0ddb2ef4e616d24232d14eee07f45b46ac19ef3b11e7c7e6be2a59b6284ad5b1a1b43051d07e788756dcfff36008637322a1c975eeb614927",
            decode_stream_check
        );
        decode_stream_address!(
            b"47Qa9iJeiYxDzKakP4SxpWD9zKB7B1nYgFcF9TdvxVzXLmdR6dX8BNPKiAyyZqVbcPEr2TYdJrRxC1YfM1APP9qg1oBnVip",
            "1298a05f07a0c9f94da6e0bb1ebe819748ab787e95b72f6157555d2fa45644e076319c740890b4f86fdbe5528942af2c52c6810b6c9773d903437c090d99b39707",
            decode_stream_check
        );
        decode_stream_address!(
            b"46eu6J7WC5jVYLT2NPovGDRs9NMyJpeH1WcKfBRNZ1CLVDjTDtKopLiYwAmhc4Bx9gf17DGe6CubRe8mm3Z1HNqgTNKbyu8",
            "1284c19bf4557a66aaa18f2af53814d694a7ccf0c6a245bcb10546ea40f6e261a8b6a587843c6943beeba8f386547f53e332bcef66bfee04de027879b51ec5fbe9",
            decode_stream_check
        );
        decode_stream_address!(
            b"46pRWGRUvUvJ3Rh7kRujCW1jMASA18S9xELAuPT28dguAoHfhLZVKqshUHF7XwdmUZjCx1jaEkYHWPPz7WVkz26TMbFxFq2",
            "128916f019baad1f65e2eb2deae8af83045d7be1accf57034fb2b23b72a4cf023a9429b5ffcaf9daf1f4d5e3c85906aefc554f15e95956c185e60e5521cb71b8b6",
            decode_stream_check
        );

        decode_stream_address_neg!(
            Error::InvalidChecksum,
            b"46pRWGRUvUvJ3Rh7kRujCW1jMASA18S9xELAuPT28dguAoHfhLZVKqshUHF7XwdmUZjCx1jaEkYHWPPz7WVkz26TMbFxFq3",
            decode_stream_check
        );
        decode_stream_address_neg!(
            Error::InvalidChecksum,
            b"46Qa9iJeiYxDzKakP4SxpWD9zKB7B1nYgFcF9TdvxVzXLmdR6dX8BNPKiAyyZqVbcPEr2TYdJrRxC1YfM1APP9qg1oBnVip",
            decode_stream_check
        );
        decode_stream_address_neg!(
            Error::InvalidChecksum,
            b"46eu6J7WC5jVYLT2NPovGDRs9NMyJpeH1WcKfBRNZ1CLV3jTDtKopLiYwAmhc4Bx9gf17DGe6CubRe8mm3Z1HNqgTNKbyu8",
            decode_stream_check
        );
        decode_stream_address_neg!(
            Error::InvalidChecksum,
            b"46pRWGRUvUvJ3Rh7kRujCW1jMASA18S9xELAuPT28dguA1HfhLZVKqshUHF7XwdmUZjCx1jaEkYHWPPz7WVkz26TMbFxFq2",
            decode_stream_check
        );
    }
}
