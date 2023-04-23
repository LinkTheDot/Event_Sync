//! ThreadSync is a crate that can be used to synchronize actions between threads.
//!
//! ThreadSync can also be used to lock actions in one thread to only happen between fixed gaps of time.
//!
//! # Getting Started
//!
//! In order to use thread_sync, you start by creating an instance of [`ThreadSync`](ThreadSync) with [`ThreadSync::new()`](ThreadSync::new).
//! You then pass in the desired tickrate for the ThreadSync to know how long a tick should last.
//!
//! The tickrate will be an integer represented as milliseconds, and cannot go below 1.
//! If you pass in 0, 1 millisecond will be set as the tickrate.
//!
//! ```
//! use thread_sync::ThreadSync;
//!
//! let tickrate = 10; // 10ms per tick
//!
//! // Create a thread synchronizer with a 10ms tickrate.
//! let thread_sync = ThreadSync::new(tickrate);
//! ```
//!
//! With this, you can call methods such as [`wait_for_x_ticks()`](ThreadSync::wait_for_x_ticks).
//! Which will wait for the amount of ticks passed in.
//!
//! # What even is a ``Tick``?
//!
//! A ``Tick`` can be thought of as imaginary markers in time, starting at creation of the ThreadSync, and
//! separated by the duration of the ``Tickrate``.
//!
//! When you wait for 1 tick, ThreadSync will sleep it's current thread up to the next tick.
//! If you were to wait for multiple ticks, ThreadSync sleeps up to the next tick, plus the duration of the remaining ticks to wait for.

use crate::errors::TimeError;
use std::time::{Duration, SystemTime};

mod errors;

/// A way to synchronize a dynamic number of threads through sleeping.
///
/// This is achieved through cloning and passing around an instance of ThreadSync to other threads.
///
/// ThreadSync can be used if you want actions between threads to happen at close to the same time.
/// It can also be used if you want to regulate the time in which actions occurr.
///
/// # Usage
///
/// In order to use threadsync you start by creating one with [`ThreadSync::new()`](ThreadSync::new).
/// You then pass in the desired tickrate for the ThreadSync to use for how long a tick should last.
///
/// The tickrate will be an integer in milliseconds, and cannot go below 1.
/// If you pass in 0, 1 millisecond will be set as the tickrate.
///
/// ```
/// use thread_sync::ThreadSync;
///
/// let tickrate = 10; // 10ms per tick
///
/// // Create a thread synchronizer with a 10ms tickrate.
/// let thread_sync = ThreadSync::new(tickrate);
/// ```
///
/// You can then use this ThreadSync for both time tracking and synchronizing threads.
///
/// # Time Tracking
/// ```
/// use thread_sync::ThreadSync;
/// use std::time::Instant;
///
/// let tickrate = 10; // 10ms per tick
/// let thread_sync = ThreadSync::new(tickrate as u32);
///
/// let start = Instant::now();
///
/// // Wait for 5 ticks (5 * 10)ms.
/// thread_sync.wait_for_x_ticks(5);
///
/// let finish = start.elapsed().as_millis();
///
/// // Check that the time it took for the operation was (waited_ticks * tickrate)ms
/// assert_eq!(finish, tickrate * 5);
/// ```
///
/// # Thread Synchronization
/// ```
/// use thread_sync::ThreadSync;
/// use std::thread;
///
/// let tickrate = 10; // 10ms per tick
/// let thread_sync = ThreadSync::new(tickrate);
///
/// let passed_thread_sync = thread_sync.clone();
///
/// let handle = thread::spawn(move || {
///   // waiting until 5 ticks have occurred since the creation of thread_sync.
///   passed_thread_sync.wait_until(5);
///
///   // do something
/// });
///
/// // waiting until 5 ticks have occurred since the creation of thread_sync.
/// thread_sync.wait_until(5);
///
/// // do something
///
/// handle.join().unwrap();
/// ```
#[derive(Clone, Eq, PartialEq)]
pub struct ThreadSync {
  start_time: SystemTime,
  tickrate: u32,
}

impl ThreadSync {
  /// Creates a new instance of ThreadSync.
  ///
  /// Takes the duration of a tick as milliseconds.
  /// If 0 is passed in, 1 will be the assigned tickrate for this instance of ThreadSync.
  pub fn new(tickrate_in_milliseconds: u32) -> Self {
    let tickrate = if tickrate_in_milliseconds > 0 {
      tickrate_in_milliseconds
    } else {
      1
    };

    Self {
      start_time: SystemTime::now(),
      tickrate,
    }
  }

  /// Waits until the passed in amount of ticks have occurred since ThreadSync creation.
  // TODO fit the term absolute somewhere
  ///
  /// That means, if you created an instance of ThreadSync with a tickrate of 10ms,
  /// and you want to wait until 1 second has passed since creation.
  /// You would pass in 100 to this method, as 100 ticks would be 1 second since ThreadSync Creation.
  ///
  /// # Usage
  ///
  /// ```
  /// use thread_sync::ThreadSync;
  ///
  /// let tickrate = 10; // 10ms per tick
  /// let thread_sync = ThreadSync::new(tickrate);
  ///
  /// // Wait 1 second from the creation of thread_sync.
  /// thread_sync.wait_until(100).unwrap();
  /// ```
  ///
  /// # Errors
  ///
  /// - An error is returned when the system time has been reversed before this ThreadSync was created.
  /// - An error is returned when the given time to wait for has already occurred.
  pub fn wait_until(&self, tick_to_wait_for: u64) -> Result<(), TimeError> {
    if self.ticks_since_started()? < tick_to_wait_for {
      let time_to_wait_from_start = Duration::from_millis(tick_to_wait_for * self.tickrate as u64);
      let total_time_to_wait = time_to_wait_from_start - self.time_since_started()?;

      std::thread::sleep(total_time_to_wait);
    } else {
      return Err(TimeError::ThatTimeHasAlreadyHappened);
    }

    Ok(())
  }

  /// TODO
  // TODO fit the term relative somewhere
  pub fn wait_for_tick(&self) {
    self.wait_for_x_ticks(1);
  }

  /// TODO
  // TODO fit the term relative somewhere
  pub fn wait_for_x_ticks(&self, ticks_to_wait: u32) {
    let ticks_since_started = self.ticks_since_started().unwrap();

    let _ = self.wait_until(ticks_since_started + ticks_to_wait as u64);
  }

  /// Returns the amount of time that has occurred since the creation of this instance of ThreadSync.
  ///
  /// # Usage
  /// ```
  /// use thread_sync::ThreadSync;
  /// use std::time::Duration;
  ///
  /// let tickrate = 10; // 10ms per tick
  /// let thread_sync = ThreadSync::new(tickrate);
  ///
  /// // Wait until 5 ticks have occurred since ThreadSync creation.
  /// thread_sync.wait_until(5);
  ///
  /// let milliseconds_since_started = thread_sync.time_since_started().unwrap().as_millis();
  ///
  /// assert_eq!(milliseconds_since_started, 50);
  /// ```
  ///
  /// # Errors
  ///
  /// - An error is returned when the system time has been reversed to before this ThreadSync was created.
  pub fn time_since_started(&self) -> Result<std::time::Duration, TimeError> {
    self
      .start_time
      .elapsed()
      .ok()
      .ok_or(TimeError::TimeHasReversed)
  }

  /// Returns the amount of ticks that have occurred since the creation of this instance of ThreadSync.
  ///
  /// # Usage
  /// ```
  /// use thread_sync::ThreadSync;
  ///
  /// let tickrate = 10; // 10ms per tick
  /// let thread_sync = ThreadSync::new(tickrate);
  ///
  /// thread_sync.wait_until(5);
  ///
  /// assert_eq!(thread_sync.ticks_since_started(), Ok(5));
  /// ```
  ///
  /// # Errors
  ///
  /// - An error is returned when the system time has been reversed to before this ThreadSync was created.
  pub fn ticks_since_started(&self) -> Result<u64, TimeError> {
    match self.start_time.elapsed() {
      Ok(time_since_started) => Ok(time_since_started.as_millis() as u64 / self.tickrate as u64),
      Err(_) => Err(TimeError::TimeHasReversed),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  /// Tickrate in milliseconds.
  const TEST_TICKRATE: u32 = 10;

  #[cfg(test)]
  mod wait_until_logic {
    use super::*;

    #[test]
    fn wait_until_valid_time() {
      let thread_sync = ThreadSync::new(TEST_TICKRATE);

      thread_sync.wait_until(5).unwrap();

      let ticks_since_started = thread_sync.ticks_since_started();

      assert_eq!(ticks_since_started, Ok(5));
    }

    #[test]
    fn wait_until_passed_time() {
      let thread_sync = ThreadSync::new(TEST_TICKRATE);

      let expected_result = Err(TimeError::ThatTimeHasAlreadyHappened);

      thread_sync.wait_for_x_ticks(3);

      let result = thread_sync.wait_until(1);

      assert_eq!(result, expected_result);
    }
  }

  #[test]
  fn time_since_started_logic() {
    let thread_sync = ThreadSync::new(TEST_TICKRATE);

    thread_sync.wait_until(2).unwrap();

    let time_since_started = thread_sync.time_since_started().unwrap();

    assert_eq!(time_since_started.as_millis(), 20);
  }

  #[test]
  fn ticks_since_started_logic() {
    let thread_sync = ThreadSync::new(TEST_TICKRATE);

    thread_sync.wait_until(2).unwrap();

    let ticks_since_started = thread_sync.ticks_since_started();

    assert_eq!(ticks_since_started, Ok(2));
  }

  #[test]
  fn wait_for_tick_logic() {
    let thread_sync = ThreadSync::new(TEST_TICKRATE);

    thread_sync.wait_for_tick();

    let ticks_since_started = thread_sync.ticks_since_started();

    assert_eq!(ticks_since_started, Ok(1));
  }
}
