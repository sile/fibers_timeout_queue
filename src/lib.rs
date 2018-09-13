//! This crate provides a timeout queue based on [`fibers`] crate.
//!
//! [`fibers`]: https://github.com/dwango/fibers-rs
//!
//! # Examples
//!
//! ```
//! use fibers_timeout_queue::TimeoutQueue;
//! use std::time::Duration;
//! use std::thread;
//!
//! let mut queue = TimeoutQueue::new();
//! assert_eq!(queue.pop(), None); // `queue` is empty
//!
//! queue.push(1, Duration::from_millis(1000));
//! queue.push(2, Duration::from_millis(100));
//! queue.push(3, Duration::from_millis(10));
//! assert_eq!(queue.pop(), None); // No expired items
//!
//! thread::sleep(Duration::from_millis(50));
//! assert_eq!(queue.pop(), Some(3)); // There is an expired item
//! assert_eq!(queue.pop(), None);
//! ```
#![warn(missing_docs)]
extern crate fibers;
extern crate futures;

use fibers::time::timer::{self, Timeout};
use futures::{Async, Future};
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::time::{Duration, SystemTime};

/// Timeout queue.
///
/// This contains items that to be dequeued when the associated timeouts have expired.
#[derive(Debug)]
pub struct TimeoutQueue<T> {
    queue: BinaryHeap<Item<T>>,
    next_timeout: Option<Timeout>,
}
impl<T> TimeoutQueue<T> {
    /// Makes a new `TimeoutQueue` instance.
    pub fn new() -> Self {
        TimeoutQueue {
            queue: BinaryHeap::new(),
            next_timeout: None,
        }
    }

    /// Enqueues the given item to the queue.
    ///
    /// The item will be dequeued by calling `pop` method after the specified timeout has expired.
    ///
    /// # Examples
    ///
    /// ```
    /// use fibers_timeout_queue::TimeoutQueue;
    /// use std::time::Duration;
    /// use std::thread;
    ///
    /// let mut queue = TimeoutQueue::new();
    /// queue.push(3, Duration::from_millis(5));
    ///
    /// assert_eq!(queue.pop(), None);
    /// thread::sleep(Duration::from_millis(10));
    /// assert_eq!(queue.pop(), Some(3));
    /// ```
    pub fn push(&mut self, item: T, timeout: Duration) {
        let expiry_time = SystemTime::now() + timeout;
        let reset_next_timeout = self
            .queue
            .peek()
            .map_or(false, |x| expiry_time < x.expiry_time);

        self.queue.push(Item { expiry_time, item });
        if reset_next_timeout {
            self.next_timeout = None;
        }
        self.poll_timeout();
    }

    /// Tries dequeuing an item of which timeout has expired.
    ///
    /// If there are no such items, this will return `None`.
    ///
    /// # Examples
    ///
    /// ```
    /// use fibers_timeout_queue::TimeoutQueue;
    /// use std::time::Duration;
    /// use std::thread;
    ///
    /// let mut queue = TimeoutQueue::new();
    /// assert_eq!(queue.pop(), None); // `queue` is empty
    ///
    /// queue.push(3, Duration::from_millis(5));
    /// assert_eq!(queue.pop(), None); // No expired items
    ///
    /// thread::sleep(Duration::from_millis(10));
    /// assert_eq!(queue.pop(), Some(3)); // There is an expired item
    /// ```
    pub fn pop(&mut self) -> Option<T> {
        self.filter_pop(|_| true)
    }

    /// A variant of `pop` method that filters items located in the queue's prefix.
    ///
    /// If the invocation of `filter(item)` returns `false`, the item will be discarded.
    ///
    /// This method is very efficient when there are many items that
    /// become unconscious before the timeout expires.
    ///
    /// # Examples
    ///
    /// ```
    /// use fibers_timeout_queue::TimeoutQueue;
    /// use std::time::Duration;
    /// use std::thread;
    ///
    /// let mut queue = TimeoutQueue::new();
    /// for i in 0..10 {
    ///     queue.push(i, Duration::from_millis(i));
    /// }
    /// assert_eq!(queue.filter_pop(|&n| n > 5), None);
    ///
    /// thread::sleep(Duration::from_millis(10));
    /// assert_eq!(queue.pop(), Some(6));
    /// ```
    pub fn filter_pop<F>(&mut self, filter: F) -> Option<T>
    where
        F: Fn(&T) -> bool,
    {
        let now = SystemTime::now();
        while let Some(x) = self.queue.pop() {
            if !filter(&x.item) {
                continue;
            }
            if x.expiry_time > now {
                self.queue.push(x);
                break;
            }
            return Some(x.item);
        }
        self.poll_timeout();
        None
    }

    /// Returns the number of items holded in the queue.
    ///
    /// # Examples
    ///
    /// ```
    /// use fibers_timeout_queue::TimeoutQueue;
    /// use std::time::Duration;
    ///
    /// let mut queue = TimeoutQueue::new();
    /// queue.push(1, Duration::from_millis(100));
    /// queue.push(2, Duration::from_millis(20));
    /// assert_eq!(queue.len(), 2);
    /// ```
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    /// Returns `true` if the queue has no items, otherwise `false`.
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    fn poll_timeout(&mut self) {
        if let Ok(Async::Ready(_)) = self.next_timeout.poll() {
            self.next_timeout = self
                .queue
                .peek()
                .and_then(|x| x.expiry_time.duration_since(SystemTime::now()).ok())
                .map(timer::timeout);
        }
    }
}
impl<T> Default for TimeoutQueue<T> {
    fn default() -> Self {
        TimeoutQueue {
            queue: BinaryHeap::new(),
            next_timeout: None,
        }
    }
}

#[derive(Debug)]
struct Item<T> {
    expiry_time: SystemTime,
    item: T,
}
impl<T> PartialOrd for Item<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        other.expiry_time.partial_cmp(&self.expiry_time)
    }
}
impl<T> Ord for Item<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        other.expiry_time.cmp(&self.expiry_time)
    }
}
impl<T> PartialEq for Item<T> {
    fn eq(&self, other: &Self) -> bool {
        self.expiry_time == other.expiry_time
    }
}
impl<T> Eq for Item<T> {}

#[cfg(test)]
mod tests {
    use std::thread;

    use super::*;

    #[test]
    fn push_and_pop_works() {
        let mut queue = TimeoutQueue::new();
        assert!(queue.is_empty());

        queue.push(1, Duration::from_millis(20));
        queue.push(2, Duration::from_millis(10));
        assert_eq!(queue.pop(), None);
        assert_eq!(queue.len(), 2);

        thread::sleep(Duration::from_millis(12));
        assert_eq!(queue.pop(), Some(2));
        assert_eq!(queue.len(), 1);

        thread::sleep(Duration::from_millis(10));
        assert_eq!(queue.pop(), Some(1));
        assert_eq!(queue.len(), 0);
    }

    #[test]
    fn filter_pop_works() {
        let mut queue = TimeoutQueue::new();
        for i in 0..100 {
            queue.push(i, Duration::from_millis(i));
        }
        thread::sleep(Duration::from_millis(50));

        assert_eq!(queue.len(), 100);
        assert_eq!(queue.filter_pop(|n| *n == 25), Some(25));
        assert_eq!(queue.filter_pop(|n| *n == 80), None);
        assert_eq!(queue.len(), 20);

        thread::sleep(Duration::from_millis(50));
        assert_eq!(queue.pop(), Some(80));
        assert_eq!(queue.len(), 19);
    }
}
