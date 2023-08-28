#![doc = include_str!("../README.md")]

use crate::errors::TimeError;
use std::time::{Duration, SystemTime};

#[cfg(feature = "async_waiting")]
pub use crate::async_waiting::AsyncWaiting;
#[cfg(not(feature = "async_waiting"))]
pub use crate::std_waiting::StdWaiting;

mod async_waiting;
mod errors;
mod std_waiting;

/// A way to synchronize a dynamic number of threads through sleeping.
///
/// This is achieved through cloning and passing around an instance of EventSync to other threads.
///
/// EventSync can be used if you want events between threads to happen at close to the same time.
/// It can also be used if you want to regulate the time in which events occurr.
///
/// # Usage
///
/// In order to use eventsync you start by creating one with [`EventSync::new()`](EventSync::new).
/// You then pass in the desired tickrate for the EventSync to use for how long a tick should last.
///
/// The tickrate will be an integer in milliseconds, and cannot go below 1.
/// If you pass in 0, 1 millisecond will be set as the tickrate.
///
/// ```
/// use event_sync::*;
///
/// let tickrate = 10; // 10ms between every tick
///
/// // Create an event synchronizer with a 10ms tickrate.
/// let event_sync = EventSync::new(tickrate);
/// ```
///
/// You can then use this EventSync for both time tracking and synchronizing threads.
///
/// # Time Tracking
/// ```
/// use event_sync::*;
/// use std::time::Instant;
///
/// let tickrate = 10; // 10ms between every tick
/// let event_sync = EventSync::new(tickrate as u32);
///
/// let start = Instant::now();
///
/// // Wait for 5 ticks (5 * 10)ms.
/// event_sync.wait_for_x_ticks(5);
///
/// let finish = start.elapsed().as_millis();
///
/// // Check that the time it took for the operation was (waited_ticks * tickrate)ms
/// assert_eq!(finish, tickrate * 5);
/// ```
///
/// # Thread Synchronization
/// ```
/// use event_sync::*;
/// use std::thread;
///
/// let tickrate = 10; // 10ms between every tick
/// let event_sync = EventSync::new(tickrate);
///
/// let passed_event_sync = event_sync.clone();
///
/// let handle = thread::spawn(move || {
///   // waiting until 5 ticks have occurred since the creation of event_sync.
///   passed_event_sync.wait_until(5);
///
///   // do something
/// });
///
/// // waiting until 5 ticks have occurred since the creation of event_sync.
/// event_sync.wait_until(5);
///
/// // do something
///
/// handle.join().unwrap();
/// ```
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct EventSync {
  start_time: SystemTime,
  tickrate: u32,
}

impl EventSync {
  /// Creates a new instance of EventSync.
  ///
  /// Takes the duration of a tick as milliseconds.
  /// If 0 is passed in, 1 will be the assigned tickrate for this instance of EventSync.
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

  /// Returns the amount of time that has occurred since the creation of this instance of EventSync.
  ///
  /// # Usage
  /// ```
  /// use event_sync::*;
  /// use std::time::Duration;
  ///
  /// let tickrate = 10; // 10ms between every tick
  /// let event_sync = EventSync::new(tickrate);
  ///
  /// // Wait until 5 ticks have occurred since EventSync creation.
  /// event_sync.wait_until(5);
  ///
  /// let milliseconds_since_started = event_sync.time_since_started().unwrap().as_millis();
  ///
  /// assert_eq!(milliseconds_since_started, 50);
  /// ```
  ///
  /// # Errors
  ///
  /// - An error is returned when the system time has been reversed to before this EventSync was created.
  pub fn time_since_started(&self) -> Result<std::time::Duration, TimeError> {
    self
      .start_time
      .elapsed()
      .ok()
      .ok_or(TimeError::TimeHasReversed)
  }

  /// Returns the amount of ticks that have occurred since the creation of this instance of EventSync.
  ///
  /// # Usage
  /// ```
  /// use event_sync::*;
  ///
  /// let tickrate = 10; // 10ms between every tick
  /// let event_sync = EventSync::new(tickrate);
  ///
  /// event_sync.wait_until(5);
  ///
  /// assert_eq!(event_sync.ticks_since_started(), Ok(5));
  /// ```
  ///
  /// # Errors
  ///
  /// - An error is returned when the system time has been reversed to before this EventSync was created.
  pub fn ticks_since_started(&self) -> Result<u64, TimeError> {
    match self.start_time.elapsed() {
      Ok(time_since_started) => Ok(time_since_started.as_millis() as u64 / self.tickrate as u64),
      Err(_) => Err(TimeError::TimeHasReversed),
    }
  }

  /// Returns the amount of time that has passed since the last tick
  ///
  /// # Errors
  ///
  /// - An error is returned when the system time has been reversed to before this EventSync was created.
  pub fn time_since_last_tick(&self) -> Result<std::time::Duration, TimeError> {
    Ok(Duration::from_millis(
      (self.time_since_started()?.as_millis() % self.tickrate as u128) as u64,
    ))
  }

  /// Returns the amount of time until the next tick will occurr.
  ///
  /// # Errors
  ///
  /// - An error is returned when the system time has been reversed to before this EventSync was created.
  pub fn time_until_next_tick(&self) -> Result<std::time::Duration, TimeError> {
    Ok(Duration::from_millis(self.tickrate as u64).saturating_sub(self.time_since_last_tick()?))
  }
}
