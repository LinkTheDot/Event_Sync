//! EventSync is a crate that can be used to synchronize events to only occurr between fixed gaps of time.
//!
//! Say you wanted an event to occurr every 10ms, but it event takes a few milliseconds to setup that event.
//! You'd end up having to sleep 10ms + the time it took to setup the event.
//!
//! That's where EventSync comes in.
//! You can create an EventSync with a tickrate of 10ms, setup your event, then wait until the next tick.
//! Aslong as the time it took to setup the event was <10ms, waiting for the next tick would ensure exactly 10ms had ocurred since the last event.
//!
//! # Getting Started
//!
//! In order to use event_sync, you start by creating an instance of [`EventSync`](EventSync) with [`EventSync::new()`](EventSync::new).
//! You then pass in the desired tickrate for the EventSync to know how long a tick should last.
//!
//! The tickrate will be an integer represented as milliseconds, and cannot go below 1.
//! If you pass in 0, 1 millisecond will be set as the tickrate.
//!
//! ```
//! use event_sync::EventSync;
//!
//! let tickrate = 10; // 10ms between every tick
//!
//! // Create an event synchronizer with a 10ms tickrate.
//! let event_sync = EventSync::new(tickrate);
//! ```
//!
//! With this, you can call methods such as [`wait_for_x_ticks()`](EventSync::wait_for_x_ticks).
//! Which will wait for the amount of ticks passed in.
//!
//! # What even is a ``Tick``?
//!
//! A ``Tick`` can be thought of as imaginary markers in time, starting at creation of the EventSync, and
//! separated by the duration of the ``Tickrate``.
//!
//! When you wait for 1 tick, EventSync will sleep it's current thread up to the next tick.
//! If you were to wait for multiple ticks, EventSync sleeps up to the next tick, plus the duration of the remaining ticks to wait for.

use crate::errors::TimeError;
use std::time::{Duration, SystemTime};

mod errors;

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
/// use event_sync::EventSync;
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
/// use event_sync::EventSync;
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
/// use event_sync::EventSync;
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
#[derive(Clone, Eq, PartialEq)]
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

  /// Waits until an absolute tick has occurred since EventSync creation.
  ///
  /// That means, if you created an instance of EventSync with a tickrate of 10ms,
  /// and you want to wait until 1 second has passed since creation.
  /// You would wait until the 100th tick, as 100 ticks would be 1 second since EventSync Creation.
  ///
  /// # Usage
  ///
  /// ```
  /// use event_sync::EventSync;
  ///
  /// let tickrate = 10; // 10ms between every tick
  /// let event_sync = EventSync::new(tickrate);
  ///
  /// // Wait 1 second from the creation of event_sync.
  /// event_sync.wait_until(100).unwrap();
  /// ```
  ///
  /// # Errors
  ///
  /// - An error is returned when the system time has been reversed before this EventSync was created.
  /// - An error is returned when the given time to wait for has already occurred.
  pub fn wait_until(&self, tick_to_wait_for: u64) -> Result<(), TimeError> {
    if self.ticks_since_started()? < tick_to_wait_for {
      let total_time_to_wait = Duration::from_millis(tick_to_wait_for * self.tickrate as u64)
        - self.time_since_started()?;

      std::thread::sleep(total_time_to_wait);
    } else {
      return Err(TimeError::ThatTimeHasAlreadyHappened);
    }

    Ok(())
  }

  /// Waits until the next tick relative to where now is between ticks.
  ///
  /// Let's say the tickrate is 10ms, and the last tick was 5ms ago.
  /// This method would sleep for 5ms to get to the next tick.
  ///
  /// # Usage
  /// ```
  /// use event_sync::EventSync;
  ///
  /// let tickrate = 10; // 10ms between every tick
  /// let event_sync = EventSync::new(tickrate);
  ///
  /// // wait until the next tick
  /// event_sync.wait_for_tick();
  /// ```
  pub fn wait_for_tick(&self) {
    self.wait_for_x_ticks(1);
  }

  /// Waits for the passed in amount of ticks relative to where now is between ticks.
  ///
  /// Let's say the tickrate is 10ms, and the last tick was 5ms ago.
  /// If you wanted to wait for 3 ticks, this method would sleep for 25ms, as that would be 3 ticks from now.
  ///
  /// # Usage
  /// ```
  /// use event_sync::EventSync;
  ///
  /// let tickrate = 10; // 10ms between every tick
  /// let event_sync = EventSync::new(tickrate);
  ///
  /// // wait for 3 ticks
  /// event_sync.wait_for_x_ticks(3);
  /// ```
  pub fn wait_for_x_ticks(&self, ticks_to_wait: u32) {
    let ticks_since_started = self.ticks_since_started().unwrap();

    let _ = self.wait_until(ticks_since_started + ticks_to_wait as u64);
  }

  /// Returns the amount of time that has occurred since the creation of this instance of EventSync.
  ///
  /// # Usage
  /// ```
  /// use event_sync::EventSync;
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
  /// use event_sync::EventSync;
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
      let event_sync = EventSync::new(TEST_TICKRATE);

      event_sync.wait_until(5).unwrap();

      let ticks_since_started = event_sync.ticks_since_started();

      assert_eq!(ticks_since_started, Ok(5));
    }

    #[test]
    fn wait_until_passed_time() {
      let event_sync = EventSync::new(TEST_TICKRATE);

      let expected_result = Err(TimeError::ThatTimeHasAlreadyHappened);

      event_sync.wait_for_x_ticks(3);

      let result = event_sync.wait_until(1);

      assert_eq!(result, expected_result);
    }
  }

  #[test]
  fn time_since_started_logic() {
    let event_sync = EventSync::new(TEST_TICKRATE);

    event_sync.wait_until(2).unwrap();

    let time_since_started = event_sync.time_since_started().unwrap();

    assert_eq!(time_since_started.as_millis(), 20);
  }

  #[test]
  fn ticks_since_started_logic() {
    let event_sync = EventSync::new(TEST_TICKRATE);

    event_sync.wait_until(2).unwrap();

    let ticks_since_started = event_sync.ticks_since_started();

    assert_eq!(ticks_since_started, Ok(2));
  }

  #[test]
  fn wait_for_tick_logic() {
    let event_sync = EventSync::new(TEST_TICKRATE);

    event_sync.wait_for_tick();

    let ticks_since_started = event_sync.ticks_since_started();

    assert_eq!(ticks_since_started, Ok(1));
  }
}
