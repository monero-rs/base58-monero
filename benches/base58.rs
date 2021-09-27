#![feature(test)]

extern crate test;

#[cfg(test)]
mod tests {
    use base58_monero::{decode, decode_check, encode, encode_check};
    use test::{black_box, Bencher};

    #[bench]
    fn encode_address_without_computing_checksum(b: &mut Bencher) {
        // Check sum is already computed
        let bytes = hex::decode("128b814e46658ab9226127c6f2072b4c9cdee068a6ddb49fac72fb9af128451fbe1bfa4572d7f3f9292d249c4acae8c170c3fff19c3bc10cb6cec32a8ff5983a895160d7a8").unwrap();
        b.iter(|| black_box(encode(bytes.as_ref()).unwrap()))
    }

    #[bench]
    fn encode_address_with_checksum(b: &mut Bencher) {
        // Check sum has to be computed
        let bytes = hex::decode("128b814e46658ab9226127c6f2072b4c9cdee068a6ddb49fac72fb9af128451fbe1bfa4572d7f3f9292d249c4acae8c170c3fff19c3bc10cb6cec32a8ff5983a89").unwrap();
        b.iter(|| black_box(encode_check(bytes.as_ref()).unwrap()))
    }

    #[bench]
    fn decode_address_without_checksum_test(b: &mut Bencher) {
        let s = "46ujSA3XmHz6kXQtiyzWgTTEqobayNDqgVqyRU12qtRtYoJJFHRKe327tToRf8zbyrKry8iNapQxKXaTsi4Fox6mGVZUF1y";
        b.iter(|| black_box(decode(s).unwrap()))
    }

    #[bench]
    fn decode_address_with_checksum_test(b: &mut Bencher) {
        let s = "46ujSA3XmHz6kXQtiyzWgTTEqobayNDqgVqyRU12qtRtYoJJFHRKe327tToRf8zbyrKry8iNapQxKXaTsi4Fox6mGVZUF1y";
        b.iter(|| black_box(decode_check(s).unwrap()))
    }
}
