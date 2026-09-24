//! re: <https://exercism.org/tracks/rust/exercises/xorcism/edit>
//!
//! ## Key ideas
//!
//! * Carry lifetimes `<'a>` of original into all data structures
//! * `IntoIterator<Item: Borrow<u8>>` trait bound to accept fn parameters: `&[u8]`, `Vec<u8>`, `Vec<&u8>`, etc
//! * `AsRef<[u8]>` trait bound to accept fn parameters: `&[u8]`, `&str`
//! * `&Key` instead of `Key` fn parameter type to prevent compile error
//!     ```text
//!     returns a value referencing data owned by the current function
//!     ```
//! * `?Sized` trait bound so `&str` can be fn parameter `key: &Key` as `str` is unsized
//!
//! ```sh
//! $ cargo test xorcism
//! $ cargo test xorcism -F xorcism_buffered_write
//!
//! $ cargo bench 'xorcism::tests::key_shorter_than_data::io'
//! test exercism::xorcism::tests::key_shorter_than_data::io::bench_reader_munges           ... bench:          28.68 ns/iter (+/- 1.54)
//! test exercism::xorcism::tests::key_shorter_than_data::io::bench_reader_roundtrip        ... bench:          34.38 ns/iter (+/- 0.26)
//! test exercism::xorcism::tests::key_shorter_than_data::io::bench_writer_munges           ... bench:          17.90 ns/iter (+/- 2.47)
//! test exercism::xorcism::tests::key_shorter_than_data::io::bench_writer_roundtrip        ... bench:          17.18 ns/iter (+/- 0.31)
//! test exercism::xorcism::tests::key_shorter_than_data::io_slow::bench_slow_reader_munges ... bench:   7,592,045.90 ns/iter (+/- 185,168.69)
//! test exercism::xorcism::tests::key_shorter_than_data::io_slow::bench_slow_writer_munges ... bench:   7,554,483.30 ns/iter (+/- 161,346.39)
//!
//! $ cargo bench 'xorcism::tests::key_shorter_than_data::io' -F xorcism_buffered_write
//! test exercism::xorcism::tests::key_shorter_than_data::io::bench_reader_munges           ... bench:          28.76 ns/iter (+/- 0.74)
//! test exercism::xorcism::tests::key_shorter_than_data::io::bench_reader_roundtrip        ... bench:          34.09 ns/iter (+/- 1.34)
//! test exercism::xorcism::tests::key_shorter_than_data::io::bench_writer_munges           ... bench:          46.19 ns/iter (+/- 3.13)
//! test exercism::xorcism::tests::key_shorter_than_data::io::bench_writer_roundtrip        ... bench:          74.46 ns/iter (+/- 5.26)
//! test exercism::xorcism::tests::key_shorter_than_data::io_slow::bench_slow_reader_munges ... bench:   7,598,920.90 ns/iter (+/- 123,870.46)
//! test exercism::xorcism::tests::key_shorter_than_data::io_slow::bench_slow_writer_munges ... bench:   1,267,758.30 ns/iter (+/- 53,539.91)
//! ```

use std::borrow::Borrow;
// #[cfg(feature = "io")]
use std::io::{Read, Write};
/// A munger which XORs a key with some data
#[derive(Clone)]
pub struct Xorcism<'a> {
    key: &'a [u8],

    // key[offset] is the next XOR value to use
    // As such, offset in range [0, key.len()) (exclusive upper)
    offset: usize,
}

impl<'a> Xorcism<'a> {
    /// Create a new Xorcism munger from a key
    ///
    /// Should accept anything which has a cheap conversion to a byte slice.
    pub fn new<Key>(key: &'a Key) -> Xorcism<'a>
    // NB: param is &Key instead of Key to avoid compile error: returns a value referencing data owned by the current function
    where
        Key: AsRef<[u8]> + ?Sized, // key can be: &[u8], &str
    {
        Self {
            key: key.as_ref(),
            offset: 0,
        }
    }

    /// XOR each byte of the input buffer with a byte from the key.
    ///
    /// Note that this is stateful: repeated calls are likely to produce different results,
    /// even with identical inputs.
    pub fn munge_in_place(&mut self, data: &mut [u8]) {
        // "cycle" idea from: https://exercism.org/tracks/rust/exercises/xorcism/solutions/PaulDance
        for (key_item /* : &u8 */, datum_mut /* : &mut u8 */) in self
            .key
            .iter()
            .cycle()
            .skip(self.offset)
            .zip(data.iter_mut())
        {
            *datum_mut ^= *key_item
        }
        // Prepare next xor to use different part of key
        self.offset = (self.offset + data.len()) % self.key.len();
    }

    /// XOR each byte of the data with a byte from the key.
    ///
    /// Note that this is stateful: repeated calls are likely to produce different results,
    /// even with identical inputs.
    ///
    /// Should accept anything which has a cheap conversion to a byte iterator.
    /// Shouldn't matter whether the byte iterator's values are owned or borrowed.
    ///
    /// The signature is the elision of:
    /// ```no-compile
    /// pub fn munge<'s, 'data, 'iter, Data>(
    ///     &'s mut self,
    ///     src: Data,
    /// ) -> impl Iterator<Item = u8> + 'iter
    /// where
    ///     Data: IntoIterator<Item: Borrow<u8>> + 'data,
    ///     // Iterator is valid only as long as its dependencies
    ///     'data: 'iter, // iter uses src
    ///     's: 'iter,    // iter uses self.key + self.offset
    /// ```
    pub fn munge<Data>(&mut self, src: Data) -> impl Iterator<Item = u8>
    where
        Data: IntoIterator<Item: Borrow<u8>>, // e.g., &[u8], Vec<u8>, Vec<&u8>
    {
        self.key.iter().cycle().skip(self.offset).zip(src).map(
            |(key_item /* : &u8 */, src_item /* : impl Borrow<u8> */)| -> u8 {
                // Prepare next xor to use different part of key
                self.offset = (self.offset + 1) % self.key.len();
                *key_item ^ *src_item.borrow()
            },
        )
    }

    // #[cfg(feature = "io")]
    pub fn reader(self, src: impl Read) -> impl Read {
        XorcismReader { engine: self, src }
    }

    // #[cfg(feature = "io")]
    pub fn writer(self, sink: impl Write) -> impl Write {
        XorcismWriter { engine: self, sink }
    }
}

/// [Read] adapter that modifies data on read
struct XorcismReader<'a, R>
where
    R: Read,
{
    engine: Xorcism<'a>,
    /// Original source (used as xor input)
    src: R,
}

impl<'a, R> Read for XorcismReader<'a, R>
where
    R: Read, // for self.src.read()
{
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let len = self.src.read(buf)?;
        self.engine.munge_in_place(&mut buf[..len]);
        Ok(len)
    }
}

/// [Write] adapter that modifies data on write
struct XorcismWriter<'a, W> {
    engine: Xorcism<'a>,
    /// Destination sink (used as xor output)
    sink: W,
}

impl<'a, W> Write for XorcismWriter<'a, W>
where
    W: Write, // for self.sink.write()
{
    #[cfg(not(feature = "xorcism_buffered_write"))]
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut len = 0;
        for datum in self.engine.munge(buf) {
            len += self.sink.write(&[datum])?;
        }
        Ok(len)
    }

    #[cfg(feature = "xorcism_buffered_write")]
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut tmp_buf = Vec::with_capacity(buf.len());
        tmp_buf.extend_from_slice(buf);
        self.engine.munge_in_place(&mut tmp_buf);
        self.sink.write(&tmp_buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.sink.flush()
    }
}

#[cfg(test)]
mod tests {
    extern crate test;
    use super::*;
    use test::Bencher;

    #[test]
    fn munge_in_place_identity() {
        let mut xs = Xorcism::new(&[0]);
        let input = "This is super-secret, cutting edge encryption, folks.".as_bytes();
        let mut output = input.to_owned();
        xs.munge_in_place(&mut output);
        assert_eq!(&input, &output);
    }
    #[test]
    fn munge_in_place_roundtrip() {
        let mut xs1 = Xorcism::new(&[1, 2, 3, 4, 5]);
        let mut xs2 = Xorcism::new(&[1, 2, 3, 4, 5]);
        let input = "This is super-secret, cutting edge encryption, folks.".as_bytes();
        let mut cipher = input.to_owned();
        xs1.munge_in_place(&mut cipher);
        assert_ne!(&input, &cipher);
        let mut output = cipher;
        xs2.munge_in_place(&mut output);
        assert_eq!(&input, &output);
    }
    #[test]
    fn munge_in_place_stateful() {
        let mut xs = Xorcism::new(&[1, 2, 3, 4, 5]);
        let input = "This is super-secret, cutting edge encryption, folks.".as_bytes();
        let mut cipher1 = input.to_owned();
        let mut cipher2 = input.to_owned();
        xs.munge_in_place(&mut cipher1);
        xs.munge_in_place(&mut cipher2);
        assert_ne!(&input, &cipher1);
        assert_ne!(&input, &cipher2);
        assert_ne!(&cipher1, &cipher2);
    }
    #[test]
    fn munge_identity() {
        let mut xs = Xorcism::new(&[0]);
        let data = "This is super-secret, cutting edge encryption, folks.";
        assert_eq!(
            xs.munge(data.as_bytes()).collect::<Vec<_>>(),
            data.as_bytes()
        );
    }
    #[test]
    fn statefulness() {
        // we expect Xorcism to be stateful: at the end of a munging run, the key has rotated.
        // this means that until the key has completely rotated around, equal inputs will produce
        // unequal outputs.
        let key = &[0, 1, 2, 3, 4, 5, 6, 7];
        let input = &[0b1010_1010, 0b0101_0101];
        let mut xs = Xorcism::new(&key);
        let out1: Vec<_> = xs.munge(input).collect();
        let out2: Vec<_> = xs.munge(input).collect();
        let out3: Vec<_> = xs.munge(input).collect();
        let out4: Vec<_> = xs.munge(input).collect();
        let out5: Vec<_> = xs.munge(input).collect();
        assert_ne!(out1, out2);
        assert_ne!(out2, out3);
        assert_ne!(out3, out4);
        assert_ne!(out4, out5);
        assert_eq!(out1, out5);
    }
    mod key_shorter_than_data {
        use super::*;
        const KEY: &str = "abcde";
        const INPUT: &str = "123455";
        const EXPECT: &[u8] = &[80, 80, 80, 80, 80, 84];
        /// tests where the key is expressed as `&str`, and input is expressed as `&[u8]`
        mod str_slice {
            use super::*;
            #[test]
            fn munge_in_place() {
                // we transform the input into a `Vec<u8>` despite its presence in this
                // module because of the more restricted syntax that this function accepts
                let mut input = INPUT.as_bytes().to_vec();
                let original = input.clone();
                // in-place munging is stateful on Xorcism, so clone it
                // to ensure the keys positions stay synchronized
                let mut xorcism1 = Xorcism::new(KEY);
                let mut xorcism2 = xorcism1.clone();
                xorcism1.munge_in_place(&mut input);
                assert_eq!(input.len(), original.len());
                assert_ne!(input, original);
                assert_eq!(input, EXPECT);
                xorcism2.munge_in_place(&mut input);
                assert_eq!(input, original);
            }
            #[test]
            fn munges() {
                let mut xorcism = Xorcism::new(KEY);
                let result: Vec<u8> = xorcism.munge(INPUT.as_bytes()).collect();
                assert_eq!(INPUT.len(), result.len());
                assert_ne!(INPUT.as_bytes(), result);
                assert_eq!(result, EXPECT);
            }
            #[test]
            fn round_trip() {
                let mut xorcism1 = Xorcism::new(KEY);
                let mut xorcism2 = xorcism1.clone();
                let munge_iter = xorcism1.munge(INPUT.as_bytes());
                let result: Vec<u8> = xorcism2.munge(munge_iter).collect();
                assert_eq!(INPUT.as_bytes(), result);
            }
        }
        /// tests where the key and input are both expressed as `&[u8]`
        mod slice_slice {
            use super::*;
            #[test]
            fn munge_in_place() {
                let key = KEY.as_bytes();
                // we transform the input into a `Vec<u8>` despite its presence in this
                // module because of the more restricted syntax that this function accepts
                let mut input = INPUT.as_bytes().to_vec();
                let original = input.clone();
                // in-place munging is stateful on Xorcism, so clone it
                // to ensure the keys positions stay synchronized
                let mut xorcism1 = Xorcism::new(key);
                let mut xorcism2 = xorcism1.clone();
                xorcism1.munge_in_place(&mut input);
                assert_eq!(input.len(), original.len());
                assert_ne!(input, original);
                assert_eq!(input, EXPECT);
                xorcism2.munge_in_place(&mut input);
                assert_eq!(input, original);
            }
            #[test]
            fn munges() {
                let key = KEY.as_bytes();
                let input = INPUT.as_bytes();
                let mut xorcism = Xorcism::new(key);
                let result: Vec<u8> = xorcism.munge(input).collect();
                assert_eq!(input.len(), result.len());
                assert_ne!(input, result);
                assert_eq!(result, EXPECT);
            }
            #[test]
            fn round_trip() {
                let key = KEY.as_bytes();
                let input = INPUT.as_bytes();
                let mut xorcism1 = Xorcism::new(key);
                let mut xorcism2 = xorcism1.clone();
                let munge_iter = xorcism1.munge(input);
                let result: Vec<u8> = xorcism2.munge(munge_iter).collect();
                assert_eq!(input, result);
            }
        }
        /// tests where the key is expressed as `&str` and input is expressed as `Vec<u8>`
        mod vec_vec {
            use super::*;
            #[test]
            fn munge_in_place() {
                let mut input = INPUT.as_bytes().to_vec();
                let original = input.clone();
                // in-place munging is stateful on Xorcism, so clone it
                // to ensure the keys positions stay synchronized
                let mut xorcism1 = Xorcism::new(KEY);
                let mut xorcism2 = xorcism1.clone();
                xorcism1.munge_in_place(&mut input);
                assert_eq!(input.len(), original.len());
                assert_ne!(input, original);
                assert_eq!(input, EXPECT);
                xorcism2.munge_in_place(&mut input);
                assert_eq!(input, original);
            }
            #[test]
            fn munges() {
                let owned_input = INPUT.as_bytes().to_vec();
                let mut xorcism = Xorcism::new(KEY);
                let result: Vec<u8> = xorcism.munge(owned_input).collect();
                assert_eq!(INPUT.len(), result.len());
                assert_ne!(INPUT.as_bytes(), result);
                assert_eq!(result, EXPECT);
            }
            #[test]
            fn round_trip() {
                let owned_input = INPUT.as_bytes().to_vec();
                let mut xorcism1 = Xorcism::new(KEY);
                let mut xorcism2 = xorcism1.clone();
                let munge_iter = xorcism1.munge(owned_input);
                let result: Vec<u8> = xorcism2.munge(munge_iter).collect();
                assert_eq!(INPUT.as_bytes(), result);
            }
        }
        // #[cfg(feature = "io")]
        mod io {
            use super::*;

            #[test]
            fn reader_munges() {
                let mut reader = Xorcism::new(KEY).reader(INPUT.as_bytes());
                let mut buf = Vec::with_capacity(INPUT.len());
                let bytes_read = reader.read_to_end(&mut buf).unwrap();
                assert_eq!(bytes_read, INPUT.len());
                assert_eq!(buf, EXPECT);
            }
            #[bench]
            fn bench_reader_munges(bench: &mut Bencher) {
                bench.iter(reader_munges);
            }
            #[test]
            fn reader_roundtrip() {
                let xs = Xorcism::new(KEY);
                let reader1 = xs.clone().reader(INPUT.as_bytes());
                let mut reader2 = xs.clone().reader(reader1);
                let mut buf = Vec::with_capacity(INPUT.len());
                let bytes_read = reader2.read_to_end(&mut buf).unwrap();
                assert_eq!(bytes_read, INPUT.len());
                assert_eq!(buf, INPUT.as_bytes());
            }
            #[bench]
            fn bench_reader_roundtrip(bench: &mut Bencher) {
                bench.iter(reader_roundtrip);
            }
            #[test]
            fn writer_munges() {
                let mut writer_dest = Vec::new();
                {
                    let mut writer = Xorcism::new(KEY).writer(&mut writer_dest);
                    assert!(writer.write_all(INPUT.as_bytes()).is_ok());
                }
                assert_eq!(writer_dest, EXPECT);
            }
            #[bench]
            fn bench_writer_munges(bench: &mut Bencher) {
                bench.iter(writer_munges);
            }
            #[test]
            fn writer_roundtrip() {
                let mut writer_dest = Vec::new();
                let xs = Xorcism::new(KEY);
                {
                    let writer1 = xs.clone().writer(&mut writer_dest);
                    let mut writer2 = xs.writer(writer1);
                    assert!(writer2.write_all(INPUT.as_bytes()).is_ok());
                }
                assert_eq!(writer_dest, INPUT.as_bytes());
            }
            #[bench]
            fn bench_writer_roundtrip(bench: &mut Bencher) {
                bench.iter(writer_roundtrip);
            }
        }
        /// Deal with slow and sluggish I/O
        mod io_slow {
            use super::*;
            use std::thread;
            use std::time::Duration;

            /// Decorate a [Read] such that each read() call is slowed +
            /// intentionally does _not_ fill up destination output buffer
            /// ... thereby rewarding batching of I/O.
            struct SlowReader<T>(T);

            impl<T> SlowReader<T> {
                fn new(wrapped: T) -> Self {
                    Self(wrapped)
                }
            }

            impl<T> Read for SlowReader<T>
            where
                T: Read,
            {
                fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
                    // Slow down reads
                    thread::sleep(Duration::from_millis(1));

                    // Read fewer bytes than requested to force caller to try again
                    // Breaks up read_to_end
                    let limit = buf.len().min(1);
                    self.0.read(&mut buf[..limit])
                }
            }

            #[test]
            fn slow_reader_munges() {
                let mut reader = Xorcism::new(KEY).reader(SlowReader::new(INPUT.as_bytes()));
                let mut buf = [0u8; INPUT.len()];
                let mut total_bytes_read = 0;

                while total_bytes_read < buf.len() {
                    // Just try once to read. The SlowReader will intentionally
                    // provide less than what was asked for [as the Read trait allows].
                    let bytes_read = reader.read(&mut buf[total_bytes_read..]).unwrap();

                    total_bytes_read += bytes_read;

                    assert!(bytes_read > 0, "some bytes should be read");
                    assert_eq!(
                        &buf[..total_bytes_read],
                        &EXPECT[..total_bytes_read],
                        "destinatino buffer should contain xor-ed output"
                    );
                    assert!(
                        &buf[total_bytes_read..].iter().all(|n| *n == 0),
                        "destination buffer excess should not be modified. read() return length is under reporting changes."
                    );
                }
            }

            #[bench]
            fn bench_slow_reader_munges(bench: &mut Bencher) {
                bench.iter(slow_reader_munges)
            }

            /// Decorate a [Write] such that each write() call is slowed,
            /// thereby rewarding batching of I/O
            struct SlowWriter<T>(T);

            impl<T> SlowWriter<T> {
                fn new(wrapped: T) -> Self {
                    Self(wrapped)
                }
            }

            impl<T> Write for SlowWriter<T>
            where
                T: Write,
            {
                fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                    thread::sleep(Duration::from_millis(1));
                    self.0.write(buf)
                }

                fn flush(&mut self) -> std::io::Result<()> {
                    thread::sleep(Duration::from_millis(1));
                    self.0.flush()
                }
            }

            #[test]
            fn slow_writer_munges() {
                let mut writer_dest = SlowWriter::new(Vec::new());
                {
                    let mut writer = Xorcism::new(KEY).writer(&mut writer_dest);
                    assert!(writer.write_all(INPUT.as_bytes()).is_ok());
                }
                assert_eq!(writer_dest.0, EXPECT);
            }

            #[bench]
            fn bench_slow_writer_munges(bench: &mut Bencher) {
                bench.iter(slow_writer_munges)
            }
        }
    }
    mod key_len_equal_to_data {
        use super::*;
        const KEY: &str = "The quick brown fox jumped over the lazy dog.";
        const INPUT: &str = "Wait, oops, this is not the pangram exercise!";
        const EXPECT: &[u8] = &[
            3, 9, 12, 84, 93, 85, 6, 12, 27, 83, 78, 82, 27, 31, 7, 83, 70, 6, 11, 0, 4, 26, 25,
            80, 17, 12, 69, 79, 6, 4, 28, 71, 6, 9, 8, 0, 9, 25, 31, 11, 67, 13, 28, 2, 15,
        ];
        /// tests where the key is expressed as `&str`, and input is expressed as `&[u8]`
        mod str_slice {
            use super::*;
            #[test]
            fn munge_in_place() {
                // we transform the input into a `Vec<u8>` despite its presence in this
                // module because of the more restricted syntax that this function accepts
                let mut input = INPUT.as_bytes().to_vec();
                let original = input.clone();
                // in-place munging is stateful on Xorcism, so clone it
                // to ensure the keys positions stay synchronized
                let mut xorcism1 = Xorcism::new(KEY);
                let mut xorcism2 = xorcism1.clone();
                xorcism1.munge_in_place(&mut input);
                assert_eq!(input.len(), original.len());
                assert_ne!(input, original);
                assert_eq!(input, EXPECT);
                xorcism2.munge_in_place(&mut input);
                assert_eq!(input, original);
            }
            #[test]
            fn munges() {
                let mut xorcism = Xorcism::new(KEY);
                let result: Vec<u8> = xorcism.munge(INPUT.as_bytes()).collect();
                assert_eq!(INPUT.len(), result.len());
                assert_ne!(INPUT.as_bytes(), result);
                assert_eq!(result, EXPECT);
            }
            #[test]
            fn round_trip() {
                let mut xorcism1 = Xorcism::new(KEY);
                let mut xorcism2 = xorcism1.clone();
                let munge_iter = xorcism1.munge(INPUT.as_bytes());
                let result: Vec<u8> = xorcism2.munge(munge_iter).collect();
                assert_eq!(INPUT.as_bytes(), result);
            }
        }
        /// tests where the key and input are both expressed as `&[u8]`
        mod slice_slice {
            use super::*;
            #[test]
            fn munge_in_place() {
                let key = KEY.as_bytes();
                // we transform the input into a `Vec<u8>` despite its presence in this
                // module because of the more restricted syntax that this function accepts
                let mut input = INPUT.as_bytes().to_vec();
                let original = input.clone();
                // in-place munging is stateful on Xorcism, so clone it
                // to ensure the keys positions stay synchronized
                let mut xorcism1 = Xorcism::new(key);
                let mut xorcism2 = xorcism1.clone();
                xorcism1.munge_in_place(&mut input);
                assert_eq!(input.len(), original.len());
                assert_ne!(input, original);
                assert_eq!(input, EXPECT);
                xorcism2.munge_in_place(&mut input);
                assert_eq!(input, original);
            }
            #[test]
            fn munges() {
                let key = KEY.as_bytes();
                let input = INPUT.as_bytes();
                let mut xorcism = Xorcism::new(key);
                let result: Vec<u8> = xorcism.munge(input).collect();
                assert_eq!(input.len(), result.len());
                assert_ne!(input, result);
                assert_eq!(result, EXPECT);
            }
            #[test]
            fn round_trip() {
                let key = KEY.as_bytes();
                let input = INPUT.as_bytes();
                let mut xorcism1 = Xorcism::new(key);
                let mut xorcism2 = xorcism1.clone();
                let munge_iter = xorcism1.munge(input);
                let result: Vec<u8> = xorcism2.munge(munge_iter).collect();
                assert_eq!(input, result);
            }
        }
        /// tests where the key is expressed as `&str` and input is expressed as `Vec<u8>`
        mod vec_vec {
            use super::*;
            #[test]
            fn munge_in_place() {
                let mut input = INPUT.as_bytes().to_vec();
                let original = input.clone();
                // in-place munging is stateful on Xorcism, so clone it
                // to ensure the keys positions stay synchronized
                let mut xorcism1 = Xorcism::new(KEY);
                let mut xorcism2 = xorcism1.clone();
                xorcism1.munge_in_place(&mut input);
                assert_eq!(input.len(), original.len());
                assert_ne!(input, original);
                assert_eq!(input, EXPECT);
                xorcism2.munge_in_place(&mut input);
                assert_eq!(input, original);
            }
            #[test]
            fn munges() {
                let owned_input = INPUT.as_bytes().to_vec();
                let mut xorcism = Xorcism::new(KEY);
                let result: Vec<u8> = xorcism.munge(owned_input).collect();
                assert_eq!(INPUT.len(), result.len());
                assert_ne!(INPUT.as_bytes(), result);
                assert_eq!(result, EXPECT);
            }
            #[test]
            fn round_trip() {
                let owned_input = INPUT.as_bytes().to_vec();
                let mut xorcism1 = Xorcism::new(KEY);
                let mut xorcism2 = xorcism1.clone();
                let munge_iter = xorcism1.munge(owned_input);
                let result: Vec<u8> = xorcism2.munge(munge_iter).collect();
                assert_eq!(INPUT.as_bytes(), result);
            }
        }
        // #[cfg(feature = "io")]
        mod io {
            use super::*;
            #[test]
            fn reader_munges() {
                let mut reader = Xorcism::new(KEY).reader(INPUT.as_bytes());
                let mut buf = Vec::with_capacity(INPUT.len());
                let bytes_read = reader.read_to_end(&mut buf).unwrap();
                assert_eq!(bytes_read, INPUT.len());
                assert_eq!(buf, EXPECT);
            }
            #[test]
            fn reader_roundtrip() {
                let xs = Xorcism::new(KEY);
                let reader1 = xs.clone().reader(INPUT.as_bytes());
                let mut reader2 = xs.clone().reader(reader1);
                let mut buf = Vec::with_capacity(INPUT.len());
                let bytes_read = reader2.read_to_end(&mut buf).unwrap();
                assert_eq!(bytes_read, INPUT.len());
                assert_eq!(buf, INPUT.as_bytes());
            }
            #[test]
            fn writer_munges() {
                let mut writer_dest = Vec::new();
                {
                    let mut writer = Xorcism::new(KEY).writer(&mut writer_dest);
                    assert!(writer.write_all(INPUT.as_bytes()).is_ok());
                }
                assert_eq!(writer_dest, EXPECT);
            }
            #[test]
            fn writer_roundtrip() {
                let mut writer_dest = Vec::new();
                let xs = Xorcism::new(KEY);
                {
                    let writer1 = xs.clone().writer(&mut writer_dest);
                    let mut writer2 = xs.writer(writer1);
                    assert!(writer2.write_all(INPUT.as_bytes()).is_ok());
                }
                assert_eq!(writer_dest, INPUT.as_bytes());
            }
        }
    }
    mod key_longer_than_data {
        use super::*;
        const KEY: &str = "A properly cryptographically random key longer than the data can actually be fairly secure.";
        const INPUT: &str = "Text is not cryptographically random.";
        const EXPECT: &[u8] = &[
            21, 69, 8, 6, 79, 25, 22, 82, 2, 22, 84, 67, 17, 11, 9, 4, 27, 8, 21, 19, 17, 24, 1,
            10, 2, 13, 0, 21, 89, 82, 19, 15, 10, 11, 2, 77, 69,
        ];
        /// tests where the key is expressed as `&str`, and input is expressed as `&[u8]`
        mod str_slice {
            use super::*;
            #[test]
            fn munge_in_place() {
                // we transform the input into a `Vec<u8>` despite its presence in this
                // module because of the more restricted syntax that this function accepts
                let mut input = INPUT.as_bytes().to_vec();
                let original = input.clone();
                // in-place munging is stateful on Xorcism, so clone it
                // to ensure the keys positions stay synchronized
                let mut xorcism1 = Xorcism::new(KEY);
                let mut xorcism2 = xorcism1.clone();
                xorcism1.munge_in_place(&mut input);
                assert_eq!(input.len(), original.len());
                assert_ne!(input, original);
                assert_eq!(input, EXPECT);
                xorcism2.munge_in_place(&mut input);
                assert_eq!(input, original);
            }
            #[test]
            fn munges() {
                let mut xorcism = Xorcism::new(KEY);
                let result: Vec<u8> = xorcism.munge(INPUT.as_bytes()).collect();
                assert_eq!(INPUT.len(), result.len());
                assert_ne!(INPUT.as_bytes(), result);
                assert_eq!(result, EXPECT);
            }
            #[test]
            fn round_trip() {
                let mut xorcism1 = Xorcism::new(KEY);
                let mut xorcism2 = xorcism1.clone();
                let munge_iter = xorcism1.munge(INPUT.as_bytes());
                let result: Vec<u8> = xorcism2.munge(munge_iter).collect();
                assert_eq!(INPUT.as_bytes(), result);
            }
        }
        /// tests where the key and input are both expressed as `&[u8]`
        mod slice_slice {
            use super::*;
            #[test]
            fn munge_in_place() {
                let key = KEY.as_bytes();
                // we transform the input into a `Vec<u8>` despite its presence in this
                // module because of the more restricted syntax that this function accepts
                let mut input = INPUT.as_bytes().to_vec();
                let original = input.clone();
                // in-place munging is stateful on Xorcism, so clone it
                // to ensure the keys positions stay synchronized
                let mut xorcism1 = Xorcism::new(key);
                let mut xorcism2 = xorcism1.clone();
                xorcism1.munge_in_place(&mut input);
                assert_eq!(input.len(), original.len());
                assert_ne!(input, original);
                assert_eq!(input, EXPECT);
                xorcism2.munge_in_place(&mut input);
                assert_eq!(input, original);
            }
            #[test]
            fn munges() {
                let key = KEY.as_bytes();
                let input = INPUT.as_bytes();
                let mut xorcism = Xorcism::new(key);
                let result: Vec<u8> = xorcism.munge(input).collect();
                assert_eq!(input.len(), result.len());
                assert_ne!(input, result);
                assert_eq!(result, EXPECT);
            }
            #[test]
            fn round_trip() {
                let key = KEY.as_bytes();
                let input = INPUT.as_bytes();
                let mut xorcism1 = Xorcism::new(key);
                let mut xorcism2 = xorcism1.clone();
                let munge_iter = xorcism1.munge(input);
                let result: Vec<u8> = xorcism2.munge(munge_iter).collect();
                assert_eq!(input, result);
            }
        }
        /// tests where the key is expressed as `&str` and input is expressed as `Vec<u8>`
        mod vec_vec {
            use super::*;
            #[test]
            fn munge_in_place() {
                let mut input = INPUT.as_bytes().to_vec();
                let original = input.clone();
                // in-place munging is stateful on Xorcism, so clone it
                // to ensure the keys positions stay synchronized
                let mut xorcism1 = Xorcism::new(KEY);
                let mut xorcism2 = xorcism1.clone();
                xorcism1.munge_in_place(&mut input);
                assert_eq!(input.len(), original.len());
                assert_ne!(input, original);
                assert_eq!(input, EXPECT);
                xorcism2.munge_in_place(&mut input);
                assert_eq!(input, original);
            }
            #[test]
            fn munges() {
                let owned_input = INPUT.as_bytes().to_vec();
                let mut xorcism = Xorcism::new(KEY);
                let result: Vec<u8> = xorcism.munge(owned_input).collect();
                assert_eq!(INPUT.len(), result.len());
                assert_ne!(INPUT.as_bytes(), result);
                assert_eq!(result, EXPECT);
            }
            #[test]
            fn round_trip() {
                let owned_input = INPUT.as_bytes().to_vec();
                let mut xorcism1 = Xorcism::new(KEY);
                let mut xorcism2 = xorcism1.clone();
                let munge_iter = xorcism1.munge(owned_input);
                let result: Vec<u8> = xorcism2.munge(munge_iter).collect();
                assert_eq!(INPUT.as_bytes(), result);
            }
        }
        // #[cfg(feature = "io")]
        mod io {
            use super::*;
            #[test]
            fn reader_munges() {
                let mut reader = Xorcism::new(KEY).reader(INPUT.as_bytes());
                let mut buf = Vec::with_capacity(INPUT.len());
                let bytes_read = reader.read_to_end(&mut buf).unwrap();
                assert_eq!(bytes_read, INPUT.len());
                assert_eq!(buf, EXPECT);
            }
            #[test]
            fn reader_roundtrip() {
                let xs = Xorcism::new(KEY);
                let reader1 = xs.clone().reader(INPUT.as_bytes());
                let mut reader2 = xs.clone().reader(reader1);
                let mut buf = Vec::with_capacity(INPUT.len());
                let bytes_read = reader2.read_to_end(&mut buf).unwrap();
                assert_eq!(bytes_read, INPUT.len());
                assert_eq!(buf, INPUT.as_bytes());
            }
            #[test]
            fn writer_munges() {
                let mut writer_dest = Vec::new();
                {
                    let mut writer = Xorcism::new(KEY).writer(&mut writer_dest);
                    assert!(writer.write_all(INPUT.as_bytes()).is_ok());
                }
                assert_eq!(writer_dest, EXPECT);
            }
            #[test]
            fn writer_roundtrip() {
                let mut writer_dest = Vec::new();
                let xs = Xorcism::new(KEY);
                {
                    let writer1 = xs.clone().writer(&mut writer_dest);
                    let mut writer2 = xs.writer(writer1);
                    assert!(writer2.write_all(INPUT.as_bytes()).is_ok());
                }
                assert_eq!(writer_dest, INPUT.as_bytes());
            }
        }
    }
    mod shakespearean {
        use super::*;
        const KEY: &str = "Forsooth, let us never break our trust!";
        const INPUT: &str =
            "The sacred brothership in which we share shall never from our hearts be lost.";
        const EXPECT: &[u8] = &[
            18, 7, 23, 83, 28, 14, 23, 26, 73, 68, 76, 7, 6, 79, 1, 27, 69, 28, 22, 30, 12, 2, 0,
            11, 28, 69, 22, 3, 73, 12, 29, 82, 87, 17, 82, 6, 27, 21, 83, 35, 79, 1, 27, 14, 3, 24,
            72, 66, 69, 26, 0, 6, 0, 19, 1, 79, 3, 69, 25, 16, 0, 0, 10, 23, 4, 19, 31, 83, 79, 23,
            23, 0, 24, 29, 6, 7, 90,
        ];
        /// tests where the key is expressed as `&str`, and input is expressed as `&[u8]`
        mod str_slice {
            use super::*;
            #[test]
            fn munge_in_place() {
                // we transform the input into a `Vec<u8>` despite its presence in this
                // module because of the more restricted syntax that this function accepts
                let mut input = INPUT.as_bytes().to_vec();
                let original = input.clone();
                // in-place munging is stateful on Xorcism, so clone it
                // to ensure the keys positions stay synchronized
                let mut xorcism1 = Xorcism::new(KEY);
                let mut xorcism2 = xorcism1.clone();
                xorcism1.munge_in_place(&mut input);
                assert_eq!(input.len(), original.len());
                assert_ne!(input, original);
                assert_eq!(input, EXPECT);
                xorcism2.munge_in_place(&mut input);
                assert_eq!(input, original);
            }
            #[test]
            fn munges() {
                let mut xorcism = Xorcism::new(KEY);
                let result: Vec<u8> = xorcism.munge(INPUT.as_bytes()).collect();
                assert_eq!(INPUT.len(), result.len());
                assert_ne!(INPUT.as_bytes(), result);
                assert_eq!(result, EXPECT);
            }
            #[test]
            fn round_trip() {
                let mut xorcism1 = Xorcism::new(KEY);
                let mut xorcism2 = xorcism1.clone();
                let munge_iter = xorcism1.munge(INPUT.as_bytes());
                let result: Vec<u8> = xorcism2.munge(munge_iter).collect();
                assert_eq!(INPUT.as_bytes(), result);
            }
        }
        /// tests where the key and input are both expressed as `&[u8]`
        mod slice_slice {
            use super::*;
            #[test]
            fn munge_in_place() {
                let key = KEY.as_bytes();
                // we transform the input into a `Vec<u8>` despite its presence in this
                // module because of the more restricted syntax that this function accepts
                let mut input = INPUT.as_bytes().to_vec();
                let original = input.clone();
                // in-place munging is stateful on Xorcism, so clone it
                // to ensure the keys positions stay synchronized
                let mut xorcism1 = Xorcism::new(key);
                let mut xorcism2 = xorcism1.clone();
                xorcism1.munge_in_place(&mut input);
                assert_eq!(input.len(), original.len());
                assert_ne!(input, original);
                assert_eq!(input, EXPECT);
                xorcism2.munge_in_place(&mut input);
                assert_eq!(input, original);
            }
            #[test]
            fn munges() {
                let key = KEY.as_bytes();
                let input = INPUT.as_bytes();
                let mut xorcism = Xorcism::new(key);
                let result: Vec<u8> = xorcism.munge(input).collect();
                assert_eq!(input.len(), result.len());
                assert_ne!(input, result);
                assert_eq!(result, EXPECT);
            }
            #[test]
            fn round_trip() {
                let key = KEY.as_bytes();
                let input = INPUT.as_bytes();
                let mut xorcism1 = Xorcism::new(key);
                let mut xorcism2 = xorcism1.clone();
                let munge_iter = xorcism1.munge(input);
                let result: Vec<u8> = xorcism2.munge(munge_iter).collect();
                assert_eq!(input, result);
            }
        }
        /// tests where the key is expressed as `&str` and input is expressed as `Vec<u8>`
        mod vec_vec {
            use super::*;
            #[test]
            fn munge_in_place() {
                let mut input = INPUT.as_bytes().to_vec();
                let original = input.clone();
                // in-place munging is stateful on Xorcism, so clone it
                // to ensure the keys positions stay synchronized
                let mut xorcism1 = Xorcism::new(KEY);
                let mut xorcism2 = xorcism1.clone();
                xorcism1.munge_in_place(&mut input);
                assert_eq!(input.len(), original.len());
                assert_ne!(input, original);
                assert_eq!(input, EXPECT);
                xorcism2.munge_in_place(&mut input);
                assert_eq!(input, original);
            }
            #[test]
            fn munges() {
                let owned_input = INPUT.as_bytes().to_vec();
                let mut xorcism = Xorcism::new(KEY);
                let result: Vec<u8> = xorcism.munge(owned_input).collect();
                assert_eq!(INPUT.len(), result.len());
                assert_ne!(INPUT.as_bytes(), result);
                assert_eq!(result, EXPECT);
            }
            #[test]
            fn round_trip() {
                let owned_input = INPUT.as_bytes().to_vec();
                let mut xorcism1 = Xorcism::new(KEY);
                let mut xorcism2 = xorcism1.clone();
                let munge_iter = xorcism1.munge(owned_input);
                let result: Vec<u8> = xorcism2.munge(munge_iter).collect();
                assert_eq!(INPUT.as_bytes(), result);
            }
        }
        // #[cfg(feature = "io")]
        mod io {
            use super::*;
            #[test]
            fn reader_munges() {
                let mut reader = Xorcism::new(KEY).reader(INPUT.as_bytes());
                let mut buf = Vec::with_capacity(INPUT.len());
                let bytes_read = reader.read_to_end(&mut buf).unwrap();
                assert_eq!(bytes_read, INPUT.len());
                assert_eq!(buf, EXPECT);
            }
            #[test]
            fn reader_roundtrip() {
                let xs = Xorcism::new(KEY);
                let reader1 = xs.clone().reader(INPUT.as_bytes());
                let mut reader2 = xs.clone().reader(reader1);
                let mut buf = Vec::with_capacity(INPUT.len());
                let bytes_read = reader2.read_to_end(&mut buf).unwrap();
                assert_eq!(bytes_read, INPUT.len());
                assert_eq!(buf, INPUT.as_bytes());
            }
            #[test]
            fn writer_munges() {
                let mut writer_dest = Vec::new();
                {
                    let mut writer = Xorcism::new(KEY).writer(&mut writer_dest);
                    assert!(writer.write_all(INPUT.as_bytes()).is_ok());
                }
                assert_eq!(writer_dest, EXPECT);
            }
            #[test]
            fn writer_roundtrip() {
                let mut writer_dest = Vec::new();
                let xs = Xorcism::new(KEY);
                {
                    let writer1 = xs.clone().writer(&mut writer_dest);
                    let mut writer2 = xs.writer(writer1);
                    assert!(writer2.write_all(INPUT.as_bytes()).is_ok());
                }
                assert_eq!(writer_dest, INPUT.as_bytes());
            }
        }
    }
    mod comics {
        use super::*;
        const KEY: &str = "Who knows what evil lurks in the hearts of men?";
        const INPUT: &str =
            "Spiderman! It's spiderman! Not a bird, or a plane, or a fireman! Just spiderman!";
        const EXPECT: &[u8] = &[
            4, 24, 6, 68, 14, 28, 2, 22, 29, 1, 87, 33, 21, 83, 83, 69, 5, 25, 5, 68, 9, 7, 31, 10,
            29, 1, 73, 32, 79, 0, 72, 4, 0, 10, 12, 19, 22, 88, 83, 79, 29, 70, 65, 77, 21, 2, 94,
            57, 13, 67, 0, 4, 28, 79, 22, 83, 70, 30, 26, 4, 25, 65, 11, 87, 73, 38, 85, 31, 1, 82,
            24, 3, 73, 13, 11, 82, 25, 9, 11, 1,
        ];
        /// tests where the key is expressed as `&str`, and input is expressed as `&[u8]`
        mod str_slice {
            use super::*;
            #[test]
            fn munge_in_place() {
                // we transform the input into a `Vec<u8>` despite its presence in this
                // module because of the more restricted syntax that this function accepts
                let mut input = INPUT.as_bytes().to_vec();
                let original = input.clone();
                // in-place munging is stateful on Xorcism, so clone it
                // to ensure the keys positions stay synchronized
                let mut xorcism1 = Xorcism::new(KEY);
                let mut xorcism2 = xorcism1.clone();
                xorcism1.munge_in_place(&mut input);
                assert_eq!(input.len(), original.len());
                assert_ne!(input, original);
                assert_eq!(input, EXPECT);
                xorcism2.munge_in_place(&mut input);
                assert_eq!(input, original);
            }
            #[test]
            fn munges() {
                let mut xorcism = Xorcism::new(KEY);
                let result: Vec<u8> = xorcism.munge(INPUT.as_bytes()).collect();
                assert_eq!(INPUT.len(), result.len());
                assert_ne!(INPUT.as_bytes(), result);
                assert_eq!(result, EXPECT);
            }
            #[test]
            fn round_trip() {
                let mut xorcism1 = Xorcism::new(KEY);
                let mut xorcism2 = xorcism1.clone();
                let munge_iter = xorcism1.munge(INPUT.as_bytes());
                let result: Vec<u8> = xorcism2.munge(munge_iter).collect();
                assert_eq!(INPUT.as_bytes(), result);
            }
        }
        /// tests where the key and input are both expressed as `&[u8]`
        mod slice_slice {
            use super::*;
            #[test]
            fn munge_in_place() {
                let key = KEY.as_bytes();
                // we transform the input into a `Vec<u8>` despite its presence in this
                // module because of the more restricted syntax that this function accepts
                let mut input = INPUT.as_bytes().to_vec();
                let original = input.clone();
                // in-place munging is stateful on Xorcism, so clone it
                // to ensure the keys positions stay synchronized
                let mut xorcism1 = Xorcism::new(key);
                let mut xorcism2 = xorcism1.clone();
                xorcism1.munge_in_place(&mut input);
                assert_eq!(input.len(), original.len());
                assert_ne!(input, original);
                assert_eq!(input, EXPECT);
                xorcism2.munge_in_place(&mut input);
                assert_eq!(input, original);
            }
            #[test]
            fn munges() {
                let key = KEY.as_bytes();
                let input = INPUT.as_bytes();
                let mut xorcism = Xorcism::new(key);
                let result: Vec<u8> = xorcism.munge(input).collect();
                assert_eq!(input.len(), result.len());
                assert_ne!(input, result);
                assert_eq!(result, EXPECT);
            }
            #[test]
            fn round_trip() {
                let key = KEY.as_bytes();
                let input = INPUT.as_bytes();
                let mut xorcism1 = Xorcism::new(key);
                let mut xorcism2 = xorcism1.clone();
                let munge_iter = xorcism1.munge(input);
                let result: Vec<u8> = xorcism2.munge(munge_iter).collect();
                assert_eq!(input, result);
            }
        }
        /// tests where the key is expressed as `&str` and input is expressed as `Vec<u8>`
        mod vec_vec {
            use super::*;
            #[test]
            fn munge_in_place() {
                let mut input = INPUT.as_bytes().to_vec();
                let original = input.clone();
                // in-place munging is stateful on Xorcism, so clone it
                // to ensure the keys positions stay synchronized
                let mut xorcism1 = Xorcism::new(KEY);
                let mut xorcism2 = xorcism1.clone();
                xorcism1.munge_in_place(&mut input);
                assert_eq!(input.len(), original.len());
                assert_ne!(input, original);
                assert_eq!(input, EXPECT);
                xorcism2.munge_in_place(&mut input);
                assert_eq!(input, original);
            }
            #[test]
            fn munges() {
                let owned_input = INPUT.as_bytes().to_vec();
                let mut xorcism = Xorcism::new(KEY);
                let result: Vec<u8> = xorcism.munge(owned_input).collect();
                assert_eq!(INPUT.len(), result.len());
                assert_ne!(INPUT.as_bytes(), result);
                assert_eq!(result, EXPECT);
            }
            #[test]
            fn round_trip() {
                let owned_input = INPUT.as_bytes().to_vec();
                let mut xorcism1 = Xorcism::new(KEY);
                let mut xorcism2 = xorcism1.clone();
                let munge_iter = xorcism1.munge(owned_input);
                let result: Vec<u8> = xorcism2.munge(munge_iter).collect();
                assert_eq!(INPUT.as_bytes(), result);
            }
        }
        // #[cfg(feature = "io")]
        mod io {
            use super::*;
            #[test]
            fn reader_munges() {
                let mut reader = Xorcism::new(KEY).reader(INPUT.as_bytes());
                let mut buf = Vec::with_capacity(INPUT.len());
                let bytes_read = reader.read_to_end(&mut buf).unwrap();
                assert_eq!(bytes_read, INPUT.len());
                assert_eq!(buf, EXPECT);
            }
            #[test]
            fn reader_roundtrip() {
                let xs = Xorcism::new(KEY);
                let reader1 = xs.clone().reader(INPUT.as_bytes());
                let mut reader2 = xs.clone().reader(reader1);
                let mut buf = Vec::with_capacity(INPUT.len());
                let bytes_read = reader2.read_to_end(&mut buf).unwrap();
                assert_eq!(bytes_read, INPUT.len());
                assert_eq!(buf, INPUT.as_bytes());
            }
            #[test]
            fn writer_munges() {
                let mut writer_dest = Vec::new();
                {
                    let mut writer = Xorcism::new(KEY).writer(&mut writer_dest);
                    assert!(writer.write_all(INPUT.as_bytes()).is_ok());
                }
                assert_eq!(writer_dest, EXPECT);
            }
            #[test]
            fn writer_roundtrip() {
                let mut writer_dest = Vec::new();
                let xs = Xorcism::new(KEY);
                {
                    let writer1 = xs.clone().writer(&mut writer_dest);
                    let mut writer2 = xs.writer(writer1);
                    assert!(writer2.write_all(INPUT.as_bytes()).is_ok());
                }
                assert_eq!(writer_dest, INPUT.as_bytes());
            }
        }
    }
    mod mad_science {
        use super::*;
        const KEY: &str = "TRANSMUTATION_NOTES_1";
        const INPUT: &str = "If wishes were horses, beggars would ride.";
        const EXPECT: &[u8] = &[
            29, 52, 97, 57, 58, 62, 61, 49, 50, 116, 62, 42, 60, 58, 110, 39, 59, 55, 32, 58, 66,
            120, 114, 35, 43, 52, 42, 52, 38, 50, 116, 62, 32, 59, 51, 42, 111, 38, 44, 55, 58, 31,
        ];
        /// tests where the key is expressed as `&str`, and input is expressed as `&[u8]`
        mod str_slice {
            use super::*;
            #[test]
            fn munge_in_place() {
                // we transform the input into a `Vec<u8>` despite its presence in this
                // module because of the more restricted syntax that this function accepts
                let mut input = INPUT.as_bytes().to_vec();
                let original = input.clone();
                // in-place munging is stateful on Xorcism, so clone it
                // to ensure the keys positions stay synchronized
                let mut xorcism1 = Xorcism::new(KEY);
                let mut xorcism2 = xorcism1.clone();
                xorcism1.munge_in_place(&mut input);
                assert_eq!(input.len(), original.len());
                assert_ne!(input, original);
                assert_eq!(input, EXPECT);
                xorcism2.munge_in_place(&mut input);
                assert_eq!(input, original);
            }
            #[test]
            fn munges() {
                let mut xorcism = Xorcism::new(KEY);
                let result: Vec<u8> = xorcism.munge(INPUT.as_bytes()).collect();
                assert_eq!(INPUT.len(), result.len());
                assert_ne!(INPUT.as_bytes(), result);
                assert_eq!(result, EXPECT);
            }
            #[test]
            fn round_trip() {
                let mut xorcism1 = Xorcism::new(KEY);
                let mut xorcism2 = xorcism1.clone();
                let munge_iter = xorcism1.munge(INPUT.as_bytes());
                let result: Vec<u8> = xorcism2.munge(munge_iter).collect();
                assert_eq!(INPUT.as_bytes(), result);
            }
        }
        /// tests where the key and input are both expressed as `&[u8]`
        mod slice_slice {
            use super::*;
            #[test]
            fn munge_in_place() {
                let key = KEY.as_bytes();
                // we transform the input into a `Vec<u8>` despite its presence in this
                // module because of the more restricted syntax that this function accepts
                let mut input = INPUT.as_bytes().to_vec();
                let original = input.clone();
                // in-place munging is stateful on Xorcism, so clone it
                // to ensure the keys positions stay synchronized
                let mut xorcism1 = Xorcism::new(key);
                let mut xorcism2 = xorcism1.clone();
                xorcism1.munge_in_place(&mut input);
                assert_eq!(input.len(), original.len());
                assert_ne!(input, original);
                assert_eq!(input, EXPECT);
                xorcism2.munge_in_place(&mut input);
                assert_eq!(input, original);
            }
            #[test]
            fn munges() {
                let key = KEY.as_bytes();
                let input = INPUT.as_bytes();
                let mut xorcism = Xorcism::new(key);
                let result: Vec<u8> = xorcism.munge(input).collect();
                assert_eq!(input.len(), result.len());
                assert_ne!(input, result);
                assert_eq!(result, EXPECT);
            }
            #[test]
            fn round_trip() {
                let key = KEY.as_bytes();
                let input = INPUT.as_bytes();
                let mut xorcism1 = Xorcism::new(key);
                let mut xorcism2 = xorcism1.clone();
                let munge_iter = xorcism1.munge(input);
                let result: Vec<u8> = xorcism2.munge(munge_iter).collect();
                assert_eq!(input, result);
            }
        }
        /// tests where the key is expressed as `&str` and input is expressed as `Vec<u8>`
        mod vec_vec {
            use super::*;
            #[test]
            fn munge_in_place() {
                let mut input = INPUT.as_bytes().to_vec();
                let original = input.clone();
                // in-place munging is stateful on Xorcism, so clone it
                // to ensure the keys positions stay synchronized
                let mut xorcism1 = Xorcism::new(KEY);
                let mut xorcism2 = xorcism1.clone();
                xorcism1.munge_in_place(&mut input);
                assert_eq!(input.len(), original.len());
                assert_ne!(input, original);
                assert_eq!(input, EXPECT);
                xorcism2.munge_in_place(&mut input);
                assert_eq!(input, original);
            }
            #[test]
            fn munges() {
                let owned_input = INPUT.as_bytes().to_vec();
                let mut xorcism = Xorcism::new(KEY);
                let result: Vec<u8> = xorcism.munge(owned_input).collect();
                assert_eq!(INPUT.len(), result.len());
                assert_ne!(INPUT.as_bytes(), result);
                assert_eq!(result, EXPECT);
            }
            #[test]
            fn round_trip() {
                let owned_input = INPUT.as_bytes().to_vec();
                let mut xorcism1 = Xorcism::new(KEY);
                let mut xorcism2 = xorcism1.clone();
                let munge_iter = xorcism1.munge(owned_input);
                let result: Vec<u8> = xorcism2.munge(munge_iter).collect();
                assert_eq!(INPUT.as_bytes(), result);
            }
        }
        // #[cfg(feature = "io")]
        mod io {
            use super::*;
            #[test]
            fn reader_munges() {
                let mut reader = Xorcism::new(KEY).reader(INPUT.as_bytes());
                let mut buf = Vec::with_capacity(INPUT.len());
                let bytes_read = reader.read_to_end(&mut buf).unwrap();
                assert_eq!(bytes_read, INPUT.len());
                assert_eq!(buf, EXPECT);
            }
            #[test]
            fn reader_roundtrip() {
                let xs = Xorcism::new(KEY);
                let reader1 = xs.clone().reader(INPUT.as_bytes());
                let mut reader2 = xs.clone().reader(reader1);
                let mut buf = Vec::with_capacity(INPUT.len());
                let bytes_read = reader2.read_to_end(&mut buf).unwrap();
                assert_eq!(bytes_read, INPUT.len());
                assert_eq!(buf, INPUT.as_bytes());
            }
            #[test]
            fn writer_munges() {
                let mut writer_dest = Vec::new();
                {
                    let mut writer = Xorcism::new(KEY).writer(&mut writer_dest);
                    assert!(writer.write_all(INPUT.as_bytes()).is_ok());
                }
                assert_eq!(writer_dest, EXPECT);
            }
            #[test]
            fn writer_roundtrip() {
                let mut writer_dest = Vec::new();
                let xs = Xorcism::new(KEY);
                {
                    let writer1 = xs.clone().writer(&mut writer_dest);
                    let mut writer2 = xs.writer(writer1);
                    assert!(writer2.write_all(INPUT.as_bytes()).is_ok());
                }
                assert_eq!(writer_dest, INPUT.as_bytes());
            }
        }
    }
    mod metaphor {
        use super::*;
        const KEY: &str = "Contextualism";
        const INPUT: &str = "The globe is text, its people prose; all the world's a page.";
        const EXPECT: &[u8] = &[
            23, 7, 11, 84, 2, 20, 27, 23, 4, 76, 0, 0, 77, 55, 10, 22, 0, 73, 88, 29, 1, 18, 76,
            25, 22, 2, 51, 3, 11, 84, 21, 10, 27, 6, 4, 87, 73, 18, 1, 47, 79, 26, 28, 0, 88, 3,
            26, 19, 0, 13, 84, 30, 99, 14, 78, 4, 4, 31, 17, 91,
        ];
        /// tests where the key is expressed as `&str`, and input is expressed as `&[u8]`
        mod str_slice {
            use super::*;
            #[test]
            fn munge_in_place() {
                // we transform the input into a `Vec<u8>` despite its presence in this
                // module because of the more restricted syntax that this function accepts
                let mut input = INPUT.as_bytes().to_vec();
                let original = input.clone();
                // in-place munging is stateful on Xorcism, so clone it
                // to ensure the keys positions stay synchronized
                let mut xorcism1 = Xorcism::new(KEY);
                let mut xorcism2 = xorcism1.clone();
                xorcism1.munge_in_place(&mut input);
                assert_eq!(input.len(), original.len());
                assert_ne!(input, original);
                assert_eq!(input, EXPECT);
                xorcism2.munge_in_place(&mut input);
                assert_eq!(input, original);
            }
            #[test]
            fn munges() {
                let mut xorcism = Xorcism::new(KEY);
                let result: Vec<u8> = xorcism.munge(INPUT.as_bytes()).collect();
                assert_eq!(INPUT.len(), result.len());
                assert_ne!(INPUT.as_bytes(), result);
                assert_eq!(result, EXPECT);
            }
            #[test]
            fn round_trip() {
                let mut xorcism1 = Xorcism::new(KEY);
                let mut xorcism2 = xorcism1.clone();
                let munge_iter = xorcism1.munge(INPUT.as_bytes());
                let result: Vec<u8> = xorcism2.munge(munge_iter).collect();
                assert_eq!(INPUT.as_bytes(), result);
            }
        }
        /// tests where the key and input are both expressed as `&[u8]`
        mod slice_slice {
            use super::*;
            #[test]
            fn munge_in_place() {
                let key = KEY.as_bytes();
                // we transform the input into a `Vec<u8>` despite its presence in this
                // module because of the more restricted syntax that this function accepts
                let mut input = INPUT.as_bytes().to_vec();
                let original = input.clone();
                // in-place munging is stateful on Xorcism, so clone it
                // to ensure the keys positions stay synchronized
                let mut xorcism1 = Xorcism::new(key);
                let mut xorcism2 = xorcism1.clone();
                xorcism1.munge_in_place(&mut input);
                assert_eq!(input.len(), original.len());
                assert_ne!(input, original);
                assert_eq!(input, EXPECT);
                xorcism2.munge_in_place(&mut input);
                assert_eq!(input, original);
            }
            #[test]
            fn munges() {
                let key = KEY.as_bytes();
                let input = INPUT.as_bytes();
                let mut xorcism = Xorcism::new(key);
                let result: Vec<u8> = xorcism.munge(input).collect();
                assert_eq!(input.len(), result.len());
                assert_ne!(input, result);
                assert_eq!(result, EXPECT);
            }
            #[test]
            fn round_trip() {
                let key = KEY.as_bytes();
                let input = INPUT.as_bytes();
                let mut xorcism1 = Xorcism::new(key);
                let mut xorcism2 = xorcism1.clone();
                let munge_iter = xorcism1.munge(input);
                let result: Vec<u8> = xorcism2.munge(munge_iter).collect();
                assert_eq!(input, result);
            }
        }
        /// tests where the key is expressed as `&str` and input is expressed as `Vec<u8>`
        mod vec_vec {
            use super::*;
            #[test]
            fn munge_in_place() {
                let mut input = INPUT.as_bytes().to_vec();
                let original = input.clone();
                // in-place munging is stateful on Xorcism, so clone it
                // to ensure the keys positions stay synchronized
                let mut xorcism1 = Xorcism::new(KEY);
                let mut xorcism2 = xorcism1.clone();
                xorcism1.munge_in_place(&mut input);
                assert_eq!(input.len(), original.len());
                assert_ne!(input, original);
                assert_eq!(input, EXPECT);
                xorcism2.munge_in_place(&mut input);
                assert_eq!(input, original);
            }
            #[test]
            fn munges() {
                let owned_input = INPUT.as_bytes().to_vec();
                let mut xorcism = Xorcism::new(KEY);
                let result: Vec<u8> = xorcism.munge(owned_input).collect();
                assert_eq!(INPUT.len(), result.len());
                assert_ne!(INPUT.as_bytes(), result);
                assert_eq!(result, EXPECT);
            }
            #[test]
            fn round_trip() {
                let owned_input = INPUT.as_bytes().to_vec();
                let mut xorcism1 = Xorcism::new(KEY);
                let mut xorcism2 = xorcism1.clone();
                let munge_iter = xorcism1.munge(owned_input);
                let result: Vec<u8> = xorcism2.munge(munge_iter).collect();
                assert_eq!(INPUT.as_bytes(), result);
            }
        }
        // #[cfg(feature = "io")]
        mod io {
            use super::*;
            #[test]
            fn reader_munges() {
                let mut reader = Xorcism::new(KEY).reader(INPUT.as_bytes());
                let mut buf = Vec::with_capacity(INPUT.len());
                let bytes_read = reader.read_to_end(&mut buf).unwrap();
                assert_eq!(bytes_read, INPUT.len());
                assert_eq!(buf, EXPECT);
            }
            #[test]
            fn reader_roundtrip() {
                let xs = Xorcism::new(KEY);
                let reader1 = xs.clone().reader(INPUT.as_bytes());
                let mut reader2 = xs.clone().reader(reader1);
                let mut buf = Vec::with_capacity(INPUT.len());
                let bytes_read = reader2.read_to_end(&mut buf).unwrap();
                assert_eq!(bytes_read, INPUT.len());
                assert_eq!(buf, INPUT.as_bytes());
            }
            #[test]
            fn writer_munges() {
                let mut writer_dest = Vec::new();
                {
                    let mut writer = Xorcism::new(KEY).writer(&mut writer_dest);
                    assert!(writer.write_all(INPUT.as_bytes()).is_ok());
                }
                assert_eq!(writer_dest, EXPECT);
            }
            #[test]
            fn writer_roundtrip() {
                let mut writer_dest = Vec::new();
                let xs = Xorcism::new(KEY);
                {
                    let writer1 = xs.clone().writer(&mut writer_dest);
                    let mut writer2 = xs.writer(writer1);
                    assert!(writer2.write_all(INPUT.as_bytes()).is_ok());
                }
                assert_eq!(writer_dest, INPUT.as_bytes());
            }
        }
    }
    mod emoji {
        use super::*;
        const KEY: &str = "🔑🗝️… 🎹?";
        const INPUT: &str = "⌨️! 🔒+💻+🧠=🔓";
        const EXPECT: &[u8] = &[
            18, 19, 60, 126, 72, 16, 182, 189, 31, 39, 27, 112, 171, 86, 191, 98, 36, 165, 73, 160,
            87, 63, 169, 97, 111, 11, 4,
        ];
        /// tests where the key is expressed as `&str`, and input is expressed as `&[u8]`
        mod str_slice {
            use super::*;
            #[test]
            fn munge_in_place() {
                // we transform the input into a `Vec<u8>` despite its presence in this
                // module because of the more restricted syntax that this function accepts
                let mut input = INPUT.as_bytes().to_vec();
                let original = input.clone();
                // in-place munging is stateful on Xorcism, so clone it
                // to ensure the keys positions stay synchronized
                let mut xorcism1 = Xorcism::new(KEY);
                let mut xorcism2 = xorcism1.clone();
                xorcism1.munge_in_place(&mut input);
                assert_eq!(input.len(), original.len());
                assert_ne!(input, original);
                assert_eq!(input, EXPECT);
                xorcism2.munge_in_place(&mut input);
                assert_eq!(input, original);
            }
            #[test]
            fn munges() {
                let mut xorcism = Xorcism::new(KEY);
                let result: Vec<u8> = xorcism.munge(INPUT.as_bytes()).collect();
                assert_eq!(INPUT.len(), result.len());
                assert_ne!(INPUT.as_bytes(), result);
                assert_eq!(result, EXPECT);
            }
            #[test]
            fn round_trip() {
                let mut xorcism1 = Xorcism::new(KEY);
                let mut xorcism2 = xorcism1.clone();
                let munge_iter = xorcism1.munge(INPUT.as_bytes());
                let result: Vec<u8> = xorcism2.munge(munge_iter).collect();
                assert_eq!(INPUT.as_bytes(), result);
            }
        }
        /// tests where the key and input are both expressed as `&[u8]`
        mod slice_slice {
            use super::*;
            #[test]
            fn munge_in_place() {
                let key = KEY.as_bytes();
                // we transform the input into a `Vec<u8>` despite its presence in this
                // module because of the more restricted syntax that this function accepts
                let mut input = INPUT.as_bytes().to_vec();
                let original = input.clone();
                // in-place munging is stateful on Xorcism, so clone it
                // to ensure the keys positions stay synchronized
                let mut xorcism1 = Xorcism::new(key);
                let mut xorcism2 = xorcism1.clone();
                xorcism1.munge_in_place(&mut input);
                assert_eq!(input.len(), original.len());
                assert_ne!(input, original);
                assert_eq!(input, EXPECT);
                xorcism2.munge_in_place(&mut input);
                assert_eq!(input, original);
            }
            #[test]
            fn munges() {
                let key = KEY.as_bytes();
                let input = INPUT.as_bytes();
                let mut xorcism = Xorcism::new(key);
                let result: Vec<u8> = xorcism.munge(input).collect();
                assert_eq!(input.len(), result.len());
                assert_ne!(input, result);
                assert_eq!(result, EXPECT);
            }
            #[test]
            fn round_trip() {
                let key = KEY.as_bytes();
                let input = INPUT.as_bytes();
                let mut xorcism1 = Xorcism::new(key);
                let mut xorcism2 = xorcism1.clone();
                let munge_iter = xorcism1.munge(input);
                let result: Vec<u8> = xorcism2.munge(munge_iter).collect();
                assert_eq!(input, result);
            }
        }
        /// tests where the key is expressed as `&str` and input is expressed as `Vec<u8>`
        mod vec_vec {
            use super::*;
            #[test]
            fn munge_in_place() {
                let mut input = INPUT.as_bytes().to_vec();
                let original = input.clone();
                // in-place munging is stateful on Xorcism, so clone it
                // to ensure the keys positions stay synchronized
                let mut xorcism1 = Xorcism::new(KEY);
                let mut xorcism2 = xorcism1.clone();
                xorcism1.munge_in_place(&mut input);
                assert_eq!(input.len(), original.len());
                assert_ne!(input, original);
                assert_eq!(input, EXPECT);
                xorcism2.munge_in_place(&mut input);
                assert_eq!(input, original);
            }
            #[test]
            fn munges() {
                let owned_input = INPUT.as_bytes().to_vec();
                let mut xorcism = Xorcism::new(KEY);
                let result: Vec<u8> = xorcism.munge(owned_input).collect();
                assert_eq!(INPUT.len(), result.len());
                assert_ne!(INPUT.as_bytes(), result);
                assert_eq!(result, EXPECT);
            }
            #[test]
            fn round_trip() {
                let owned_input = INPUT.as_bytes().to_vec();
                let mut xorcism1 = Xorcism::new(KEY);
                let mut xorcism2 = xorcism1.clone();
                let munge_iter = xorcism1.munge(owned_input);
                let result: Vec<u8> = xorcism2.munge(munge_iter).collect();
                assert_eq!(INPUT.as_bytes(), result);
            }
        }
        // #[cfg(feature = "io")]
        mod io {
            use super::*;
            #[test]
            fn reader_munges() {
                let mut reader = Xorcism::new(KEY).reader(INPUT.as_bytes());
                let mut buf = Vec::with_capacity(INPUT.len());
                let bytes_read = reader.read_to_end(&mut buf).unwrap();
                assert_eq!(bytes_read, INPUT.len());
                assert_eq!(buf, EXPECT);
            }
            #[test]
            fn reader_roundtrip() {
                let xs = Xorcism::new(KEY);
                let reader1 = xs.clone().reader(INPUT.as_bytes());
                let mut reader2 = xs.clone().reader(reader1);
                let mut buf = Vec::with_capacity(INPUT.len());
                let bytes_read = reader2.read_to_end(&mut buf).unwrap();
                assert_eq!(bytes_read, INPUT.len());
                assert_eq!(buf, INPUT.as_bytes());
            }
            #[test]
            fn writer_munges() {
                let mut writer_dest = Vec::new();
                {
                    let mut writer = Xorcism::new(KEY).writer(&mut writer_dest);
                    assert!(writer.write_all(INPUT.as_bytes()).is_ok());
                }
                assert_eq!(writer_dest, EXPECT);
            }
            #[test]
            fn writer_roundtrip() {
                let mut writer_dest = Vec::new();
                let xs = Xorcism::new(KEY);
                {
                    let writer1 = xs.clone().writer(&mut writer_dest);
                    let mut writer2 = xs.writer(writer1);
                    assert!(writer2.write_all(INPUT.as_bytes()).is_ok());
                }
                assert_eq!(writer_dest, INPUT.as_bytes());
            }
        }
    }
}
