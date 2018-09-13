fibers_timeout_queue
====================

[![Crates.io: fibers_timeout_queue](https://img.shields.io/crates/v/fibers_timeout_queue.svg)](https://crates.io/crates/fibers_timeout_queue)
[![Documentation](https://docs.rs/fibers_timeout_queue/badge.svg)](https://docs.rs/fibers_timeout_queue)
[![Build Status](https://travis-ci.org/sile/fibers_timeout_queue.svg?branch=master)](https://travis-ci.org/sile/fibers_timeout_queue)
[![Code Coverage](https://codecov.io/gh/sile/fibers_timeout_queue/branch/master/graph/badge.svg)](https://codecov.io/gh/sile/fibers_timeout_queue/branch/master)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

This crate provides a timeout queue based on [`fibers`] crate.

[Documentation](https://docs.rs/fibers_timeout_queue)

[`fibers`]: https://github.com/dwango/fibers-rs


Examples
--------

```rust
use fibers_timeout_queue::TimeoutQueue;
use std::time::Duration;
use std::thread;

let mut queue = TimeoutQueue::new();
assert_eq!(queue.pop(), None); // `queue` is empty

queue.push(1, Duration::from_millis(1000));
queue.push(2, Duration::from_millis(100));
queue.push(3, Duration::from_millis(10));
assert_eq!(queue.pop(), None); // No expired items

thread::sleep(Duration::from_millis(50));
assert_eq!(queue.pop(), Some(3)); // There is an expired item
assert_eq!(queue.pop(), None);
```
