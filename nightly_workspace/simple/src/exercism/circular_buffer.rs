//! re: <https://exercism.org/tracks/rust/exercises/circular-buffer/edit>
//!
//! ```sh
//! $ cargo test circular_buffer
//!
//! $ cargo miri test circular_buffer
//! ```

use std::mem::MaybeUninit;

#[derive(PartialEq, Eq, Debug)]
struct Position(usize);

impl Position {
    fn advance(&self, capacity: usize) -> Position {
        self.incr(1, capacity)
    }

    fn incr(&self, n: usize, capacity: usize) -> Position {
        Position((self.0 + n) % capacity)
    }
}

/// Circular buffer impl.
///
/// ## Design
///
/// We use [Box<[`MaybeUninit<T>`]>] instead of [`Vec<Option<T>>`].
/// This is to keep memory footprint small (as Option will bloat each entry).
/// However, it comes at the cost of manually managing memory w/ unsafe code.
pub struct CircularBuffer<T> {
    data: Box<[MaybeUninit<T>]>,

    len: usize,

    // invariant: data[read_pos..write_position()] values are initialized when (len > 0) where
    // - upper bound exclusive [like normal slice notation]
    // - positions are modulo adjusted ... i.e., write_position() can wrap around
    //
    // Therefore:
    // (read_pos == write_position()) => empty or full [depending on len]

    // data[read_pos                                      ] is next value to read from
    // data[write_position() = (read_pos + len % capacity)] is next value to write to
    read_pos: Position,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    EmptyBuffer,
    FullBuffer,
}

impl<T> CircularBuffer<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            data: Box::<[T]>::new_uninit_slice(capacity),
            len: 0,
            read_pos: Position(0),
        }
    }

    fn capacity(&self) -> usize {
        // fixed array length is max capacity
        self.data.len()
    }

    fn write_position(&self) -> Position {
        self.read_pos.incr(self.len, self.capacity())
    }

    pub fn write(&mut self, element: T) -> Result<(), Error> {
        if self.len == self.capacity() {
            return Err(Error::FullBuffer);
        }

        // Make MaybeUninit initialized
        self.data[self.write_position().0].write(element);

        // Advance write pos
        self.len += 1;
        Ok(())
    }

    pub fn read(&mut self) -> Result<T, Error> {
        if self.len == 0 {
            return Err(Error::EmptyBuffer);
        }

        // data[read_pos] has data
        // => take it and reset MaybeUninit slot state to unitiailized
        let element = std::mem::replace(&mut self.data[self.read_pos.0], MaybeUninit::uninit());

        // Advance read pos, keeping write pos the same
        self.read_pos = self.read_pos.advance(self.capacity());
        self.len -= 1;

        // SAFETY: All values between data[orig(read_pos)..write_position()] are valid when (orig(len) > 0)
        Ok(unsafe { element.assume_init() })
    }

    pub fn clear(&mut self) {
        // Drop all elements between data[read_pos..write_position()] (modulo adjusted)
        while self.read().is_ok() {}
    }

    pub fn overwrite(&mut self, element: T) {
        if self.len == self.capacity() {
            // NB: Like in empty case, the full case has (read_pos == write_position())
            assert_eq!(self.read_pos, self.write_position());

            // full => drop old element, replacing it with new one
            let mut old_element =
                std::mem::replace(&mut self.data[self.read_pos.0], MaybeUninit::new(element));

            // SAFETY: All values between data[orig(read_pos)..write_position()] are valid when (orig(len) > 0)
            unsafe { old_element.assume_init_drop() };

            // Advance read pos [so read() won't return the dropped value]
            self.read_pos = self.read_pos.advance(self.capacity());
        } else {
            // len < capacity => just insert
            self.data[self.write_position().0].write(element);

            // Advance write pos
            self.len += 1;
        }
    }
}

impl<T> Drop for CircularBuffer<T> {
    fn drop(&mut self) {
        self.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;

    #[test]
    fn reading_empty_buffer_should_fail() {
        let mut buffer = CircularBuffer::<i32>::new(1);
        assert_eq!(buffer.read(), Err(Error::EmptyBuffer));
    }
    #[test]
    fn can_read_an_item_just_written() {
        let mut buffer = CircularBuffer::new(1);
        assert!(buffer.write(1).is_ok());
        assert_eq!(buffer.read(), Ok(1));
    }
    #[test]
    fn each_item_may_only_be_read_once() {
        let mut buffer = CircularBuffer::new(1);
        assert!(buffer.write(1).is_ok());
        assert_eq!(buffer.read(), Ok(1));
        assert_eq!(buffer.read(), Err(Error::EmptyBuffer));
    }
    #[test]
    fn items_are_read_in_the_order_they_are_written() {
        let mut buffer = CircularBuffer::new(2);
        assert!(buffer.write(1).is_ok());
        assert!(buffer.write(2).is_ok());
        assert_eq!(buffer.read(), Ok(1));
        assert_eq!(buffer.read(), Ok(2));
    }
    #[test]
    fn full_buffer_can_t_be_written_to() {
        let mut buffer = CircularBuffer::new(1);
        assert!(buffer.write(1).is_ok());
        assert_eq!(buffer.write(2), Err(Error::FullBuffer));
    }
    #[test]
    fn a_read_frees_up_capacity_for_another_write() {
        let mut buffer = CircularBuffer::new(1);
        assert!(buffer.write(1).is_ok());
        assert_eq!(buffer.read(), Ok(1));
        assert!(buffer.write(2).is_ok());
        assert_eq!(buffer.read(), Ok(2));
    }
    #[test]
    fn read_position_is_maintained_even_across_multiple_writes() {
        let mut buffer = CircularBuffer::new(3);
        assert!(buffer.write(1).is_ok());
        assert!(buffer.write(2).is_ok());
        assert_eq!(buffer.read(), Ok(1));
        assert!(buffer.write(3).is_ok());
        assert_eq!(buffer.read(), Ok(2));
        assert_eq!(buffer.read(), Ok(3));
    }
    #[test]
    fn items_cleared_out_of_buffer_can_t_be_read() {
        let mut buffer = CircularBuffer::new(1);
        assert!(buffer.write(1).is_ok());
        buffer.clear();
        assert_eq!(buffer.read(), Err(Error::EmptyBuffer));
    }
    #[test]
    fn clear_frees_up_capacity_for_another_write() {
        let mut buffer = CircularBuffer::new(1);
        assert!(buffer.write(1).is_ok());
        buffer.clear();
        assert!(buffer.write(2).is_ok());
        assert_eq!(buffer.read(), Ok(2));
    }
    #[test]
    fn clear_does_nothing_on_empty_buffer() {
        let mut buffer = CircularBuffer::new(1);
        buffer.clear();
        assert!(buffer.write(1).is_ok());
        assert_eq!(buffer.read(), Ok(1));
    }
    #[test]
    fn overwrite_acts_like_write_on_non_full_buffer() {
        let mut buffer = CircularBuffer::new(2);
        assert!(buffer.write(1).is_ok());
        buffer.overwrite(2);
        assert_eq!(buffer.read(), Ok(1));
        assert_eq!(buffer.read(), Ok(2));
    }
    #[test]
    fn overwrite_replaces_the_oldest_item_on_full_buffer() {
        let mut buffer = CircularBuffer::new(2);
        assert!(buffer.write(1).is_ok());
        assert!(buffer.write(2).is_ok());
        buffer.overwrite(3);
        assert_eq!(buffer.read(), Ok(2));
        assert_eq!(buffer.read(), Ok(3));
    }
    #[test]
    fn overwrite_replaces_the_oldest_item_remaining_in_buffer_following_a_read() {
        let mut buffer = CircularBuffer::new(3);
        assert!(buffer.write(1).is_ok());
        assert!(buffer.write(2).is_ok());
        assert!(buffer.write(3).is_ok());
        assert_eq!(buffer.read(), Ok(1));
        assert!(buffer.write(4).is_ok());
        buffer.overwrite(5);
        assert_eq!(buffer.read(), Ok(3));
        assert_eq!(buffer.read(), Ok(4));
        assert_eq!(buffer.read(), Ok(5));
    }
    #[test]
    fn initial_clear_does_not_affect_wrapping_around() {
        let mut buffer = CircularBuffer::new(2);
        buffer.clear();
        assert!(buffer.write(1).is_ok());
        assert!(buffer.write(2).is_ok());
        buffer.overwrite(3);
        buffer.overwrite(4);
        assert_eq!(buffer.read(), Ok(3));
        assert_eq!(buffer.read(), Ok(4));
        assert_eq!(buffer.read(), Err(Error::EmptyBuffer));
    }
    #[test]
    fn char_buffer() {
        let mut buffer = CircularBuffer::new(1);
        assert!(buffer.write('A').is_ok());
    }
    #[test]
    fn string_buffer() {
        let mut buffer = CircularBuffer::new(1);
        assert!(buffer.write("Testing".to_string()).is_ok());
    }
    #[test]
    fn clear_actually_frees_up_its_elements() {
        let mut buffer = CircularBuffer::new(1);
        let element = Rc::new(());
        assert!(buffer.write(Rc::clone(&element)).is_ok());
        assert_eq!(Rc::strong_count(&element), 2);
        buffer.clear();
        assert_eq!(Rc::strong_count(&element), 1);
    }
    #[test]
    fn dropping_the_buffer_drops_its_elements() {
        let element = Rc::new(());
        {
            let mut buffer = CircularBuffer::new(1);
            assert!(buffer.write(Rc::clone(&element)).is_ok());
            assert_eq!(Rc::strong_count(&element), 2);
        }
        assert_eq!(Rc::strong_count(&element), 1);
    }
}
