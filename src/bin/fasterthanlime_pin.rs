//! Experiment code example in fasterthanlime "Pin and suffering"
//! [article](https://fasterthanli.me/articles/pin-and-suffering) to understand
//! [std::future::Future] and [std::pin]

use anyhow::Result;

/// Read a file using vanilla [tokio::io::AsyncRead]
mod v1 {
    use super::*;
    use tokio::fs::File;
    use tokio::io::AsyncReadExt;

    pub async fn do_it() -> Result<()> {
        let mut f = File::open("/dev/urandom").await?;
        let mut buf = [0u8; 32];
        let read_len = f.read_exact(&mut buf).await?;
        println!("Read {} bytes {:?}", read_len, buf);
        Ok(())
    }
}

/// Pass through to [tokio::io::AsyncRead]
mod v2 {
    use super::*;
    use std::pin::Pin;
    use std::task::{Context, Poll};
    use tokio::fs::File;
    use tokio::io::{AsyncRead, AsyncReadExt, ReadBuf};

    pub struct ReadWrap<R> {
        read: R,
    }

    impl<R> ReadWrap<R> {
        pub fn new(read: R) -> Self {
            Self { read }
        }
    }

    impl<R: AsyncRead + Unpin> AsyncRead for ReadWrap<R> {
        fn poll_read(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &mut ReadBuf<'_>,
        ) -> Poll<std::io::Result<()>> {
            Pin::new(&mut self.read).poll_read(cx, buf)
        }
    }

    pub async fn do_it() -> Result<()> {
        let f = File::open("/dev/urandom").await?;
        let mut f: ReadWrap<File> = ReadWrap::new(f);
        let mut buf = [0u8; 32];
        let read_len = f.read_exact(&mut buf).await?;
        println!("Read {} bytes {:?}", read_len, buf);
        Ok(())
    }
}

/// Pass through to [tokio::io::AsyncRead] with delay and making wrapper [Unpin]
/// \[which forces some of its !Unpin fields to go onto the heap\]
///
/// FIXME Verify above statement is true
mod v3 {
    use super::*;
    use std::pin::Pin;
    use std::task::{Context, Poll};
    use std::time::Duration;
    use tokio::fs::File;
    use tokio::io::{AsyncRead, AsyncReadExt, ReadBuf};
    use tokio::time::{self, Instant, Sleep};

    /// NB: The size of [ReadWrap] \[in the return returned by
    /// [AsyncReadExt::read_exact()]\] will _not directly_ include the size of [Sleep]
    /// but instead just a pointer to Sleep.
    /// 
    /// FIXME Answer: So if [ReadWrap] is on the stack, its [ReadWrap::sleep] is on the heap
    /// \[because Box is always on the heap\]?
    pub struct ReadWrap<R> {
        read: Pin<Box<R>>,
        sleep: Pin<Box<Sleep>>,
    }

    impl<R> ReadWrap<R> {
        pub fn new(read: R) -> Self {
            Self {
                read: Box::pin(read),
                sleep: Box::pin(time::sleep(Duration::from_secs(1))),
            }
        }
    }

    impl<R: AsyncRead + Unpin> AsyncRead for ReadWrap<R> {
        fn poll_read(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &mut ReadBuf<'_>,
        ) -> Poll<std::io::Result<()>> {
            match self.sleep.as_mut().poll(cx) {
                Poll::Ready(_) => {
                    // woke up => read into buffer
                    self.sleep
                        .as_mut()
                        .reset(Instant::now() + Duration::from_secs(1));
                    self.read.as_mut().poll_read(cx, buf)
                }
                // continue sleeping
                Poll::Pending => Poll::Pending,
            }
        }
    }

    pub async fn do_it() -> Result<()> {
        let f = File::open("/dev/urandom").await?;
        let mut f = ReadWrap::new(f);

        // FIXME Answer: Will ReadWrap be on stack as it does _not_ cross await points?
        let mut f: Pin<&mut ReadWrap<File>> = Pin::new(&mut f);

        let mut buf = [0u8; 32];
        let now = Instant::now();
        let read_len = f.read_exact(&mut buf).await?;
        println!(
            "Read {} bytes {:?} after {:?}",
            read_len,
            buf,
            now.elapsed()
        );
        Ok(())
    }
}

/// Pass through to [tokio::io::AsyncRead] with delay but make wrapper *not* [Unpin]
/// \[so that its !Unpin fields can stay on the stack\]
///
/// FIXME Verify above statement is true
mod v4 {
    use super::*;
    use std::pin::{Pin, pin};
    use std::task::{Context, Poll};
    use std::time::Duration;
    use tokio::fs::File;
    use tokio::io::{AsyncRead, AsyncReadExt, ReadBuf};
    use tokio::time::{self, Instant, Sleep};

    /// NB: The size of [ReadWrap] \[in the return returned by
    /// [AsyncReadExt::read_exact()]\] will include the size of [Sleep]
    ///
    /// FIXME Answer: So if [ReadWrap] is on the stack, so will its [ReadWrap::sleep]?
    pub struct ReadWrap<R> {
        read: R,
        sleep: Sleep,
    }

    impl<R> ReadWrap<R> {
        pub fn new(read: R) -> Self {
            Self {
                read,
                sleep: time::sleep(Duration::from_secs(1)),
            }
        }
    }

    impl<R: AsyncRead + Unpin> AsyncRead for ReadWrap<R> {
        fn poll_read(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &mut ReadBuf<'_>,
        ) -> Poll<std::io::Result<()>> {
            let (mut read, mut sleep) = unsafe {
                let this = self.get_unchecked_mut();
                (
                    Pin::new(&mut this.read),
                    Pin::new_unchecked(&mut this.sleep),
                )
            };
            match sleep.as_mut().poll(cx) {
                Poll::Ready(_) => {
                    // woke up => read into buffer
                    sleep.reset(Instant::now() + Duration::from_secs(1));
                    read.as_mut().poll_read(cx, buf)
                }
                // continue sleeping
                Poll::Pending => Poll::Pending,
            }
        }
    }

    pub async fn do_it() -> Result<()> {
        let f = File::open("/dev/urandom").await?;
        let f_before_pin = ReadWrap::new(f);

        // NB: Unlike v3, the usage of ReadWrap is more complicated
        // FIXME Answer: Will ReadWrap be on stack as it does _not_ cross await points?
        let mut f: Pin<&mut ReadWrap<File>> = pin!(f_before_pin);

        // NB: Following
        //      std::hint::black_box(f_before_pin);
        // will *not* compile because pin!() at https://doc.rust-lang.org/beta/src/core/pin.rs.html#2035
        // uses "super let" to move it to an inaccessible var

        let mut buf = [0u8; 32];
        let now = Instant::now();
        let read_len = f.read_exact(&mut buf).await?;
        println!(
            "Read {} bytes {:?} after {:?}",
            read_len,
            buf,
            now.elapsed()
        );
        Ok(())
    }
}

#[tokio::main]
pub async fn main() -> Result<()> {
    // v1::do_it().await?;
    // v2::do_it().await?;
    // v3::do_it().await?;
    v4::do_it().await?;
    Ok(())
}
