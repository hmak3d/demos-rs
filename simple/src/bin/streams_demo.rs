//! Explore async streams from futures-util
//!
//! Key ideas:
//! - data structures
//!     - iterator of futures
//!     - stream of future
//!     - stream of values
//! - question: how to compose?
//!     - [futures_util::future::join_all]
//!     - consume a [futures_util::Stream] ... there are 3 ways to go from iterator of futures to stream of values
//!         1. pass iterator to [futures_util::stream::iter] and chain [StreamExt::buffer_unordered]
//!         1. collect iterator into [futures_util::stream::FuturesUnordered]
//!         1. collect iterator into [futures_util::stream::FuturesOrdered]
//!
//! ```sh
//! $ cargo r --bin streams_demo
//! # =>
//! test_join took 1.0061985s
//! test_iter_fut_to_stream_buffered_1 took 3.009122958s
//! test_iter_fut_to_stream_buffered_2 took 2.006612667s
//! test_iter_fut_to_stream_buffered_3 took 1.00140225s
//! test_iter_fut_to_FuturesUnordered took 1.00370425s
//! test_iter_fut_to_FuturesOrdered took 1.001533s
//! ```

use futures_util::future::join_all;
use futures_util::stream::{FuturesOrdered, FuturesUnordered};
use futures_util::{Stream, StreamExt, stream};
use std::time::{Duration, Instant};
use tokio::time as async_time;

async fn do_work() {
    async_time::sleep(Duration::from_secs(1)).await
}

macro_rules! time_it {
    // Just show timing. e.g., test_join took 1.001435708s
    ($desc:tt, $body:expr) => {{
        let start = Instant::now();
        $body;
        let elapsed = Instant::now() - start;
        println!("{} took {:?}", $desc, elapsed);
        elapsed
    }};
    // Show timing *and* assert that duration is within range
    ($desc:tt, $expected_duration:expr, $body:expr) => {{
        let elapsed = time_it!($desc, $body);
        const EPSILON: Duration = Duration::from_millis(20);
        assert!(elapsed >= ($expected_duration - EPSILON));
        assert!(elapsed <= ($expected_duration + EPSILON));
    }};
}

/// Get some futures that takes a while to be ready
fn get_slow_futures() -> Vec<impl Future<Output = ()>> {
    (0..3).map(|_| do_work()).collect::<Vec<_>>()
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    time_it!(
        "test_join",
        // 1 second (all 3 in parallel)
        Duration::from_secs(1),
        {
            // join_all() will poll() all futures w/ no throttling
            join_all(get_slow_futures()).await;
        }
    );

    time_it!(
        "test_iter_fut_to_stream_buffered_1",
        // 3 second (all 3 in serial)
        // Sliding window of in-flight futures is 1
        Duration::from_secs(3),
        {
            // stream::iter() converts Iterator<Item: Future<T>> to Stream<Item: Future<T>>.
            // buffer_unordered() throttles the polling to a sliding window of futures.
            // sliding window of 1 here will make futures run serially.
            let mut stream = assert_stream::<(), _>(
                assert_stream(stream::iter(get_slow_futures().into_iter()))
                    // Stream<Item: Future<T>> -> Stream<T>
                    .buffer_unordered(1),
            );
            while let Some(_fut) = stream.next().await {}
        }
    );

    time_it!(
        "test_iter_fut_to_stream_buffered_2",
        // 2 second (2 in parallel, then 1 in serial)
        // Sliding window of in-flight futures is 2
        Duration::from_secs(2),
        {
            // stream::iter() converts Iterator<Item: Future<T>> to Stream<Item: Future<T>>.
            // buffer_unordered() throttles the polling to a sliding window of futures.
            // sliding window of 2 here will make early futures run parallel, followed by last future serially.
            let mut stream = assert_stream::<(), _>(
                assert_stream(stream::iter(get_slow_futures().into_iter()))
                    // Stream<Item: Future<T>> -> Stream<T>
                    .buffer_unordered(2),
            );
            while let Some(_fut) = stream.next().await {}
        }
    );

    time_it!(
        "test_iter_fut_to_stream_buffered_3",
        // 1 second (all 3 in parallel)
        // Sliding window of in-flight futures is 3
        Duration::from_secs(1),
        {
            // stream::iter() converts Iterator<Item: Future<T>> to Stream<Item: Future<T>>.
            // buffer_unordered() throttles the polling to a sliding window of futures.
            // sliding window of 3 here will make all futures run in parallel.
            let mut stream = assert_stream::<(), _>(
                assert_stream(stream::iter(get_slow_futures().into_iter()))
                    // Stream<Item: Future<T>> -> Stream<T>
                    .buffer_unordered(3),
            );
            while let Some(_fut) = stream.next().await {}
        }
    );

    time_it!(
        "test_iter_fut_to_FuturesUnordered",
        // 1 second (all 3 in parallel)
        Duration::from_secs(1),
        {
            // FuturesUnordered converts Iterator<Item: Future<T>> to Stream<Item = T>.
            // Will poll() all futures w/ no throttling.
            let mut stream: FuturesUnordered<_> =
                assert_stream::<(), _>(get_slow_futures().into_iter().collect());
            while let Some(_fut) = stream.next().await {}
        }
    );

    time_it!(
        "test_iter_fut_to_FuturesOrdered",
        // 1 second (all 3 in parallel)
        Duration::from_secs(1),
        {
            // FuturesOrdered converts Iterator<Item: Future<T>> to Stream<Item = T>.
            // Will poll() all futures w/ no throttling.
            // Unlike FuturesUnordered, FuturesOrdered resolve in the original order.
            let mut stream: FuturesOrdered<_> =
                assert_stream::<(), _>(get_slow_futures().into_iter().collect());
            while let Some(_fut) = stream.next().await {}
        }
    );
}

/// Useful method to use IDE rust-analyzer to discover the Item for a [Stream]
#[allow(dead_code)]
fn assert_stream<T, S>(stream: S) -> S
where
    S: Stream<Item = T>,
{
    stream
}
