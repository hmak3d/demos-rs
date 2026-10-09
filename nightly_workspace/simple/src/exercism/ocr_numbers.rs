//! re: <https://exercism.org/tracks/rust/exercises/ocr-numbers/edit>
//!
//! ```sh
//! $ cargo t ocr_numbers
//!
//! $ cargo t ocr -F ocr_numbers_functional
//!
//! $ cargo bench ocr_numbers::tests::bench_
//! test exercism::ocr_numbers::tests::bench_recognizes_string_of_decimal_numbers           ... bench:         446.97 ns/iter (+/- 8.85)
//!
//! $ cargo bench ocr_numbers::tests::bench_ -F ocr_numbers_functional
//! test exercism::ocr_numbers::tests::bench_recognizes_string_of_decimal_numbers           ... bench:         652.10 ns/iter (+/- 56.27)
//! ```

// The code below is a stub. Just enough to satisfy the compiler.
// In order to pass the tests you can add-to or change any of this code.

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    InvalidRowCount(usize),
    InvalidColumnCount(usize),
}

/// Blocks of rows that each digit is rendered as.
/// `DIGIT_ART[i]` represents the digit `i`.
const DIGIT_ART: [[&str; 4]; 10] = [
    [
        " _ ", //
        "| |", //
        "|_|", //
        "   ", //
    ],
    [
        "   ", //
        "  |", //
        "  |", //
        "   ", //
    ],
    [
        " _ ", //
        " _|", //
        "|_ ", //
        "   ", //
    ],
    [
        " _ ", //
        " _|", //
        " _|", //
        "   ", //
    ],
    [
        "   ", //
        "|_|", //
        "  |", //
        "   ", //
    ],
    [
        " _ ", //
        "|_ ", //
        " _|", //
        "   ", //
    ],
    [
        " _ ", //
        "|_ ", //
        "|_|", //
        "   ", //
    ],
    [
        " _ ", //
        "  |", //
        "  |", //
        "   ", //
    ],
    [
        " _ ", //
        "|_|", //
        "|_|", //
        "   ", //
    ],
    [
        " _ ", //
        "|_|", //
        " _|", //
        "   ", //
    ],
];

#[cfg(not(feature = "ocr_numbers_functional"))]
pub fn convert(input: &str) -> Result<String, Error> {
    // read chunks of 4 lines
    // for each 4 line chunk
    //      within each stream of the 4 lines
    //          chunk stream in 3 chars
    //      zip the 4 streams together
    // for each zip entry
    //      output 1 digit
    let mut ascii: Vec<String> = Vec::new();
    let mut row_count = 0;
    let mut lines = input.lines();
    while let Some(row1) = lines.next() {
        row_count += 1;
        let row2 = lines.next().ok_or(Error::InvalidRowCount(row_count))?;
        row_count += 1;
        let row3 = lines.next().ok_or(Error::InvalidRowCount(row_count))?;
        row_count += 1;
        let row4 = lines.next().ok_or(Error::InvalidRowCount(row_count))?;
        row_count += 1;

        let chunks1 = StringChunker::<3>::new(row1);
        let chunks2 = StringChunker::<3>::new(row2);
        let chunks3 = StringChunker::<3>::new(row3);
        let chunks4 = StringChunker::<3>::new(row4);

        let stacked_rows_iter = chunks1.zip(chunks2).zip(chunks3).zip(chunks4);

        let mut cur_ascii = String::new();

        for (((row1_chunk, row2_chunk), row3_chunk), row4_chunk) in stacked_rows_iter {
            let needle_art = [row1_chunk?, row2_chunk?, row3_chunk?, row4_chunk?];
            let ascii_digit = match DIGIT_ART
                .iter()
                .enumerate() // Get index into DIGIT_ART that matches
                .find_map(|(as_num, art)| (needle_art == *art).then_some(as_num))
            {
                Some(as_num) => as_num.to_string().chars().next().unwrap(),
                None => '?',
            };
            cur_ascii.push(ascii_digit);
        }

        ascii.push(cur_ascii);
    }
    Ok(ascii.join(","))
}

#[cfg(feature = "ocr_numbers_functional")]
pub fn convert(input: &str) -> Result<String, Error> {
    // read chunks of 4 lines
    // for each 4 line chunk
    //      within each stream of the 4 lines
    //          chunk stream in 3 chars
    //      zip the 4 streams together
    // for each zip entry
    //      output 1 digit
    let lines = input.lines();

    // Tag line number, and then group 4 lines together at a time
    let mut stacked_rows_iter = lines.enumerate().array_chunks::<4>();

    let ascii: Vec<String> = (&mut stacked_rows_iter)
        // Iterator -> Iterator<Result<String>>
        .map(|rows: [(usize, &str); 4]| {
            // Within each of the 4 lines, break into chunks of 3 chars
            let mut chunks = rows.map(|(_, row)| StringChunker::new(row)).into_iter();

            // NB: rows is len 4, so unwrap() cannot fail
            let chunks1: StringChunker<'_, 3> = chunks.next().unwrap();
            let chunks2: StringChunker<'_, 3> = chunks.next().unwrap();
            let chunks3: StringChunker<'_, 3> = chunks.next().unwrap();
            let chunks4: StringChunker<'_, 3> = chunks.next().unwrap();

            // Zip the chunks streams together, and for each pairing, convert into char (e.g., '0', '9', etc)
            chunks1
                .zip(chunks2)
                .zip(chunks3)
                .zip(chunks4)
                // For each pairing, unwrap error, find offset in DIGIT_ART that matches, then return offset as ascii (a char)
                // Iterator<Item = tuples of Result<&str, _>> -> Iterator<Item = Result<char, _>>
                .map(|(((res1, res2), res3), res4): (_, Result<&str, Error>)| {
                    let needle_art: [&str; 4] = [res1?, res2?, res3?, res4?];
                    let found_num = DIGIT_ART
                        .iter()
                        .enumerate()
                        .find_map(|(as_num, art)| (needle_art == *art).then_some(as_num));
                    match found_num {
                        Some(found_num) => Ok(found_num.to_string().as_bytes()[0].into()),
                        None => Ok('?'),
                    }
                })
                // Across all pairings, hoist up Result ... convert Iterator<Item = Result<char>> -> Result<String>
                .collect::<Result<String, Error>>()
        })
        // Hoist up result and then unwrap; Iterator<Item=Result<T>> -> Result<Vec<T>> -> Vec<T>
        .collect::<Result<Vec<String>, _>>()?;
    if let Some((last_row_num, _)) = stacked_rows_iter.into_remainder().last() {
        // report in 1-based rows, not 0-based
        return Err(Error::InvalidRowCount(last_row_num + 1));
    }
    Ok(ascii.join(","))
}

struct StringChunker<'a, const N: usize> {
    wrapped: &'a str,
    origin_len: usize,
}

impl<'a, const N: usize> StringChunker<'a, N> {
    fn new(wrapped: &'a str) -> Self {
        Self {
            wrapped,
            origin_len: wrapped.len(),
        }
    }
}

impl<'a, const N: usize> Iterator for StringChunker<'a, N> {
    type Item = Result<&'a str, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        let remain_len = self.wrapped.len();
        if remain_len == 0 {
            None
        } else if !remain_len.is_multiple_of(N) {
            Some(Err(Error::InvalidColumnCount(self.origin_len)))
        } else {
            let s = &self.wrapped[..N];
            self.wrapped = &self.wrapped[N..];
            Some(Ok(s))
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate test;

    use test::Bencher;

    use super as ocr;

    #[test]
    fn input_with_lines_not_multiple_of_four_is_error() {
        #[rustfmt::skip]
        let input = " _ \n".to_string() +
                    "| |\n" +
                    "   ";
        assert_eq!(Err(ocr::Error::InvalidRowCount(3)), ocr::convert(&input));
    }

    #[test]
    fn input_with_columns_not_multiple_of_three_is_error() {
        #[rustfmt::skip]
        let input = "    \n".to_string() +
                    "   |\n" +
                    "   |\n" +
                    "    ";
        assert_eq!(Err(ocr::Error::InvalidColumnCount(4)), ocr::convert(&input));
    }

    #[test]
    fn unrecognized_characters_return_question_mark() {
        #[rustfmt::skip]
        let input = "   \n".to_string() +
                    "  _\n" +
                    "  |\n" +
                    "   ";
        assert_eq!(Ok("?".to_string()), ocr::convert(&input));
    }

    #[test]
    fn recognizes_0() {
        #[rustfmt::skip]
        let input = " _ \n".to_string() +
                    "| |\n" +
                    "|_|\n" +
                    "   ";
        assert_eq!(Ok("0".to_string()), ocr::convert(&input));
    }

    #[test]
    fn recognizes_1() {
        #[rustfmt::skip]
        let input = "   \n".to_string() +
                    "  |\n" +
                    "  |\n" +
                    "   ";
        assert_eq!(Ok("1".to_string()), ocr::convert(&input));
    }

    #[test]
    fn recognizes_2() {
        #[rustfmt::skip]
        let input = " _ \n".to_string() +
                    " _|\n" +
                    "|_ \n" +
                    "   ";
        assert_eq!(Ok("2".to_string()), ocr::convert(&input));
    }

    #[test]
    fn recognizes_3() {
        #[rustfmt::skip]
        let input = " _ \n".to_string() +
                    " _|\n" +
                    " _|\n" +
                    "   ";
        assert_eq!(Ok("3".to_string()), ocr::convert(&input));
    }

    #[test]
    fn recognizes_4() {
        #[rustfmt::skip]
        let input = "   \n".to_string() +
                    "|_|\n" +
                    "  |\n" +
                    "   ";
        assert_eq!(Ok("4".to_string()), ocr::convert(&input));
    }

    #[test]
    fn recognizes_5() {
        #[rustfmt::skip]
        let input = " _ \n".to_string() +
                    "|_ \n" +
                    " _|\n" +
                    "   ";
        assert_eq!(Ok("5".to_string()), ocr::convert(&input));
    }

    #[test]
    fn recognizes_6() {
        #[rustfmt::skip]
        let input = " _ \n".to_string() +
                    "|_ \n" +
                    "|_|\n" +
                    "   ";
        assert_eq!(Ok("6".to_string()), ocr::convert(&input));
    }

    #[test]
    fn recognizes_7() {
        #[rustfmt::skip]
        let input = " _ \n".to_string() +
                    "  |\n" +
                    "  |\n" +
                    "   ";
        assert_eq!(Ok("7".to_string()), ocr::convert(&input));
    }

    #[test]
    fn recognizes_8() {
        #[rustfmt::skip]
        let input = " _ \n".to_string() +
                    "|_|\n" +
                    "|_|\n" +
                    "   ";
        assert_eq!(Ok("8".to_string()), ocr::convert(&input));
    }

    #[test]
    fn recognizes_9() {
        #[rustfmt::skip]
        let input = " _ \n".to_string() +
                    "|_|\n" +
                    " _|\n" +
                    "   ";
        assert_eq!(Ok("9".to_string()), ocr::convert(&input));
    }

    #[test]
    fn recognizes_110101100() {
        #[rustfmt::skip]
        let input = "       _     _        _  _ \n".to_string() +
                    "  |  || |  || |  |  || || |\n" +
                    "  |  ||_|  ||_|  |  ||_||_|\n" +
                    "                           ";
        assert_eq!(Ok("110101100".to_string()), ocr::convert(&input));
    }

    #[test]
    fn replaces_only_garbled_numbers_with_question_mark() {
        #[rustfmt::skip]
        let input = "       _     _           _ \n".to_string() +
                    "  |  || |  || |     || || |\n" +
                    "  |  | _|  ||_|  |  ||_||_|\n" +
                    "                           ";
        assert_eq!(Ok("11?10?1?0".to_string()), ocr::convert(&input));
    }

    #[test]
    fn recognizes_string_of_decimal_numbers() {
        #[rustfmt::skip]
        let input = "    _  _     _  _  _  _  _  _ \n".to_string() +
                    "  | _| _||_||_ |_   ||_||_|| |\n" +
                    "  ||_  _|  | _||_|  ||_| _||_|\n" +
                    "                              ";
        assert_eq!(Ok("1234567890".to_string()), ocr::convert(&input));
    }

    #[bench]
    fn bench_recognizes_string_of_decimal_numbers(bench: &mut Bencher) {
        bench.iter(recognizes_string_of_decimal_numbers)
    }

    #[test]
    fn numbers_across_multiple_lines_are_joined_by_commas() {
        #[rustfmt::skip]
        let input = "    _  _ \n".to_string() +
                    "  | _| _|\n" +
                    "  ||_  _|\n" +
                    "         \n" +
                    "    _  _ \n" +
                    "|_||_ |_ \n" +
                    "  | _||_|\n" +
                    "         \n" +
                    " _  _  _ \n" +
                    "  ||_||_|\n" +
                    "  ||_| _|\n" +
                    "         ";
        assert_eq!(Ok("123,456,789".to_string()), ocr::convert(&input));
    }
}
