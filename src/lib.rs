#![doc = include_str!("../README.md")]

use crate::errors::TimeError;
use serde::{Deserialize, Serialize, Serializer};
use std::time::{Duration, SystemTime};

mod errors;

/// A way to synchronize a dynamic number of threads through sleeping.
///
/// This is achieved through cloning and passing around an instance of EventSync to other threads.
///
/// EventSync can be used if you want events between threads to happen at close to the same time.
/// It can also be used if you want to regulate the time in which events occur.
///
/// # Usage
///
/// In order to use EventSync, you start by creating one with [`EventSync::new()`](EventSync::new).
/// You then pass in the desired tickrate for the EventSync to know how long 1 tick should last.
///
/// The tickrate will be an integer reflected as milliseconds, and cannot go below 1.
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
#[derive(Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct EventSync {
  start_time: SystemTime,
  tickrate: u32,
  #[serde(serialize_with = "pause")]
  paused_time: Option<SystemTime>,
}

fn pause<S>(value: &Option<SystemTime>, serializer: S) -> Result<S::Ok, S::Error>
where
  S: Serializer,
{
  value.unwrap_or(SystemTime::now()).serialize(serializer)
}

impl EventSync {
  /// Creates a new instance of [`EventSync`](EventSync).
  ///
  /// Takes the duration of a tick as milliseconds.
  /// If 0 is passed in, 1 will be the assigned tickrate for this instance of EventSync.
  ///
  /// # Examples
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
  pub fn new(tickrate_in_milliseconds: u32) -> Self {
    Self::new_event_sync(tickrate_in_milliseconds, SystemTime::now(), false)
  }

  /// Creates a new instance of [`EventSync`](EventSync) with the given starting time.
  ///
  /// # Example
  ///
  /// ```
  /// use event_sync::*;
  /// use std::time::Duration;
  ///
  /// let tickrate = 10; // 10ms between every tick.
  /// let starting_time = Duration::from_millis(30); // Start 30ms ahead.
  /// let event_sync = EventSync::from_starting_time(tickrate, starting_time);
  ///
  /// assert_eq!(event_sync.ticks_since_started().unwrap(), 3);
  /// ```
  pub fn from_starting_time(tickrate_in_milliseconds: u32, starting_time: Duration) -> Self {
    let starting_time = SystemTime::now() - starting_time;

    Self::new_event_sync(tickrate_in_milliseconds, starting_time, false)
  }

  /// Creates a new instance of [`EventSync`](EventSync) with the given starting tick.
  ///
  /// # Example
  ///
  /// ```
  /// use event_sync::*;
  /// use std::time::Duration;
  ///
  /// let tickrate = 10; // 10ms between every tick.
  /// let starting_tick = 3; // Start 3 ticks ahead.
  /// let event_sync = EventSync::from_starting_tick(tickrate, starting_tick);
  ///
  /// assert_eq!(event_sync.ticks_since_started().unwrap(), 3);
  /// ```
  pub fn from_starting_tick(tickrate_in_milliseconds: u32, starting_tick: u32) -> Self {
    let starting_time = Duration::from_millis((starting_tick * tickrate_in_milliseconds).into());
    let starting_time = SystemTime::now() - starting_time;

    Self::new_event_sync(tickrate_in_milliseconds, starting_time, false)
  }

  /// Create a new [`EventSync`](EventSync) from the given tickrate and system time.
  fn new_event_sync(tickrate: u32, start_time: SystemTime, is_paused: bool) -> Self {
    Self {
      start_time,
      tickrate: tickrate.max(1),
      paused_time: is_paused.then_some(start_time),
    }
  }

  /// Pauses this instance of EventSync. Does not pause any other EventSync connected.
  ///
  /// When paused, the time that passed is retained.
  /// If 10.1 seconds have passed, that time will be retained after paused.
  ///
  /// Calling pause when already paused does nothing.
  ///
  /// # Warning
  ///
  /// If the system time was reversed to before EventSync start time before calling pause, the EventSync cannot be unpaused.
  ///
  /// # Examples
  ///
  /// ```
  /// use event_sync::EventSync;
  ///
  /// let tickrate = 10; // 10ms between every tick
  /// let mut event_sync = EventSync::new(tickrate);
  /// let other_event_sync = event_sync.clone(); // Create a second one to desync.
  ///
  /// event_sync.wait_for_tick().unwrap(); // Add some time.
  /// event_sync.pause();
  ///
  /// other_event_sync.wait_for_tick().unwrap(); // Desync from the paused EventSync.
  ///
  /// event_sync.unpause().unwrap();
  /// assert_eq!(event_sync.ticks_since_started(), Ok(1)); // Only 1 tick has passed while this EventSync wasn't paused.
  /// ```
  pub fn pause(&mut self) {
    if self.paused_time.is_none() {
      self.paused_time = Some(SystemTime::now());
    }
  }

  /// Unpauses this instance of EventSync if it's been paused.
  /// If the time passed before pausing was 10.1 seconds, that time will be retained when unpaused.
  ///
  /// Calling unpause when the EventSync is already running does nothing.
  ///
  /// # Errors
  ///
  /// - An error is returned if the system time was reversed to before EventSync start time before calling [`pause`](EventSync::pause).
  ///
  /// # Examples
  ///
  /// ```
  /// use event_sync::EventSync;
  ///
  /// let tickrate = 10; // 10ms between every tick
  /// let mut event_sync = EventSync::new(tickrate);
  /// let other_event_sync = event_sync.clone(); // Create a second one to desync.
  ///
  /// event_sync.wait_for_tick().unwrap(); // Add some time.
  /// event_sync.pause();
  ///
  /// other_event_sync.wait_for_tick().unwrap(); // Desync from the paused EventSync.
  ///
  /// event_sync.unpause().unwrap();
  /// assert_eq!(event_sync.ticks_since_started(), Ok(1)); // Only 1 tick has passed while this EventSync wasn't paused.
  /// ```
  pub fn unpause(&mut self) -> Result<(), TimeError> {
    let Some(paused_time) = self.paused_time else {
      return Ok(());
    };
    let time_running = paused_time.duration_since(self.start_time)?;

    *self = Self::from_starting_time(self.tickrate, time_running);

    Ok(())
  }

  /// Returns true if this instance of EventSyunc has been paused.
  ///
  /// Call [`event_sync.unpause()`](EventSync::unpause) to unpause the eventsync.
  /// The time that's passed before pausing is retained.
  ///
  /// # Examples
  ///
  /// ```
  /// use event_sync::EventSync;
  ///
  /// let tickrate = 10; // 10ms between every tick
  /// let mut event_sync = EventSync::new(tickrate);
  ///
  /// event_sync.pause();
  ///
  /// assert!(event_sync.is_paused());
  /// ```
  pub fn is_paused(&self) -> bool {
    self.paused_time.is_some()
  }

  /// A convenience method that will return an error if the event sync is paused.
  fn err_if_paused(&self) -> Result<(), TimeError> {
    if self.is_paused() {
      return Err(TimeError::EventSyncPaused);
    }

    Ok(())
  }

  /// Restarts the starting time.
  ///
  /// This will only restart the starting time for this instance of EventSync.
  /// Any other instances tied to this one will not be reset.
  ///
  /// Unpauses if paused, resetting the time.
  ///
  /// # Examples
  /// ```
  /// use event_sync::EventSync;
  ///
  /// let tickrate = 10; // 10ms between every tick
  /// let mut event_sync = EventSync::new(tickrate);
  ///
  /// event_sync.wait_for_tick().unwrap(); // Add some time.
  ///
  /// event_sync.restart(); // Restart the EventSync.
  ///
  /// assert_eq!(event_sync.ticks_since_started(), Ok(0)); // 0 ticks is returned because the EventSync was restarted.
  /// ```
  pub fn restart(&mut self) {
    self.start_time = SystemTime::now();
    self.paused_time = None;
  }

  /// Restarts the starting time.
  ///
  /// This will only change the tickrate for this instance of EventSync.
  /// Any other instances tied to this one will not be changed.
  ///
  /// The amount of ticks returned are now based on the new tickrate.
  /// Let's say the previous tickrate was 10ms, and 10 ticks have passed (100ms).
  /// If you change the tickrate to 100ms, `event_sync.ticks_since_start()` will return 1.
  pub fn change_tickrate(&mut self, new_tickrate: u32) {
    self.tickrate = new_tickrate.max(1);
  }

  /// Returns the tickrate for this instance of EventSync.
  pub fn get_tickrate(&self) -> u32 {
    self.tickrate
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
  /// - An error is returned if the EventSync is paused.
  pub fn wait_until(&self, tick_to_wait_for: u64) -> Result<(), TimeError> {
    self.err_if_paused()?;

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
  ///
  /// # Errors
  ///
  /// - An error is returned when the system time has been reversed before this EventSync was created.
  /// - An error is returned when the given time to wait for has already occurred.
  /// - An error is returned if the EventSync is paused.
  pub fn wait_for_tick(&self) -> Result<(), TimeError> {
    self.err_if_paused()?;

    self.wait_for_x_ticks(1)
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
  ///
  /// # Errors
  ///
  /// - An error is returned when the system time has been reversed before this EventSync was created.
  /// - An error is returned when the given time to wait for has already occurred.
  /// - An error is returned if the EventSync is paused.
  pub fn wait_for_x_ticks(&self, ticks_to_wait: u32) -> Result<(), TimeError> {
    self.err_if_paused()?;

    let ticks_since_started = self.ticks_since_started()?;

    self.wait_until(ticks_since_started + ticks_to_wait as u64)
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
  /// - An error is returned if the EventSync is paused.
  /// - An error is returned when the system time has been reversed to before this EventSync was created.
  pub fn time_since_started(&self) -> Result<std::time::Duration, TimeError> {
    self.err_if_paused()?;

    self.start_time.elapsed().map_err(Into::into)
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
  /// - An error is returned if the EventSync is paused.
  /// - An error is returned when the system time has been reversed to before this EventSync was created.
  pub fn ticks_since_started(&self) -> Result<u64, TimeError> {
    self.err_if_paused()?;

    Ok(self.start_time.elapsed()?.as_millis() as u64 / self.tickrate as u64)
  }

  /// Returns the amount of time that has passed since the last tick
  ///
  /// # Errors
  ///
  /// - An error is returned if the EventSync is paused.
  /// - An error is returned when the system time has been reversed to before this EventSync was created.
  ///
  /// # Examples
  ///
  /// ```
  /// use event_sync::EventSync;
  ///
  /// let tickrate = 10; // 10ms between every tick.
  /// let event_sync = EventSync::new(tickrate);
  ///
  /// event_sync.wait_for_tick().unwrap();
  ///
  /// assert!(event_sync.time_since_last_tick().unwrap().as_micros() < 500); // Practically no time should have passed since the last tick.
  /// ```
  pub fn time_since_last_tick(&self) -> Result<std::time::Duration, TimeError> {
    self.err_if_paused()?;

    Ok(Duration::from_nanos(
      (self.time_since_started()?.as_nanos() % (self.tickrate as u128 * 1000000)) as u64,
    ))
  }

  /// Returns the amount of time until the next tick will occur.
  ///
  /// # Errors
  ///
  /// - An error is returned if the EventSync is paused.
  /// - An error is returned when the system time has been reversed to before this EventSync was created.
  ///
  /// # Examples
  ///
  /// ```
  /// use event_sync::EventSync;
  ///
  /// let tickrate = 10; // 10ms between every tick.
  /// let event_sync = EventSync::new(tickrate);
  ///
  /// event_sync.wait_for_tick().unwrap();
  ///
  /// assert!(event_sync.time_until_next_tick().unwrap().as_micros() > 500); // Practically no time should have passed since the last tick.
  /// ```
  pub fn time_until_next_tick(&self) -> Result<std::time::Duration, TimeError> {
    self.err_if_paused()?;

    Ok(Duration::from_millis(self.tickrate as u64).saturating_sub(self.time_since_last_tick()?))
  }
}

impl std::fmt::Debug for EventSync {
  fn fmt(
    &self,
    formatter: &mut std::fmt::Formatter<'_>,
  ) -> std::result::Result<(), std::fmt::Error> {
    write!(formatter, "{:?}", self.time_since_started())
  }
}

impl std::fmt::Display for EventSync {
  fn fmt(
    &self,
    formatter: &mut std::fmt::Formatter<'_>,
  ) -> std::result::Result<(), std::fmt::Error> {
    write!(formatter, "{:?}", self)
  }
}

// Tests have a chance to fail due to their time sensitive nature.
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

      event_sync.wait_for_x_ticks(3).unwrap();

      let result = event_sync.wait_until(1);

      assert_eq!(result, expected_result);
    }
  }

  #[test]
  fn time_since_started_logic() {
    let event_sync = EventSync::new(TEST_TICKRATE);

    event_sync.wait_until(2).unwrap();

    let time_since_started = event_sync.time_since_started().unwrap();

    assert_eq!(time_since_started.as_millis(), TEST_TICKRATE as u128 * 2);
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

    event_sync.wait_for_tick().unwrap();

    let ticks_since_started = event_sync.ticks_since_started();

    assert_eq!(ticks_since_started, Ok(1));
  }

  #[test]
  fn time_since_last_tick_logic() {
    let tickrate = 1;
    let event_sync = EventSync::new(tickrate);

    event_sync.wait_for_tick().unwrap();

    let time_since_last_tick = event_sync.time_since_last_tick().unwrap();

    assert!((tickrate as u128 * 1000000) > time_since_last_tick.as_nanos());
    assert_ne!(time_since_last_tick.as_nanos(), 0);
  }

  #[test]
  fn time_since_last_tick_accuracy() {
    let event_sync = EventSync::new(TEST_TICKRATE);
    let extra_wait_time = 2;

    event_sync.wait_for_tick().unwrap();

    std::thread::sleep(Duration::from_millis(extra_wait_time as u64));

    let time_since_last_tick = event_sync.time_since_last_tick().unwrap();

    assert_eq!(time_since_last_tick.as_millis(), extra_wait_time);
  }

  #[test]
  fn time_until_next_tick_logic() {
    let event_sync = EventSync::new(TEST_TICKRATE);
    let extra_wait_time = 2;

    event_sync.wait_for_tick().unwrap();

    std::thread::sleep(Duration::from_millis(extra_wait_time as u64));

    // Round to account for time passed since this was called.
    // Converting directly to milliseconds will round down, which will always be 7.
    // We want to round up in the case of milliseconds, making it always be 8.
    let time_until_next_tick =
      (event_sync.time_until_next_tick().unwrap().as_micros() as f64 / 1000.0).ceil();

    println!("{:?}", time_until_next_tick);
    assert_eq!(
      time_until_next_tick as u64,
      (TEST_TICKRATE - extra_wait_time) as u64
    );
  }

  mod from_start_logic {
    use super::*;

    const STARTING_TICKS: u32 = 10;

    #[test]
    fn from_ticks() {
      let event_sync = EventSync::from_starting_tick(TEST_TICKRATE, STARTING_TICKS);

      assert_eq!(
        event_sync.ticks_since_started().unwrap(),
        STARTING_TICKS as u64
      );
    }

    #[test]
    fn from_time() {
      let starting_time = Duration::from_millis((STARTING_TICKS * TEST_TICKRATE).into());
      let event_sync = EventSync::from_starting_time(TEST_TICKRATE, starting_time);

      assert_eq!(
        event_sync.ticks_since_started().unwrap(),
        STARTING_TICKS as u64
      );
    }
  }

  #[test]
  fn methods_error_when_paused() {
    let mut event_sync = EventSync::new(TEST_TICKRATE);
    event_sync.wait_for_x_ticks(3).unwrap();
    event_sync.pause();

    assert_eq!(event_sync.wait_until(4), Err(TimeError::EventSyncPaused));
    assert_eq!(event_sync.wait_for_tick(), Err(TimeError::EventSyncPaused));
    assert_eq!(
      event_sync.wait_for_x_ticks(1),
      Err(TimeError::EventSyncPaused)
    );
    assert_eq!(
      event_sync.time_since_started(),
      Err(TimeError::EventSyncPaused)
    );
    assert_eq!(
      event_sync.ticks_since_started(),
      Err(TimeError::EventSyncPaused)
    );
    assert_eq!(
      event_sync.time_since_last_tick(),
      Err(TimeError::EventSyncPaused)
    );
    assert_eq!(
      event_sync.time_until_next_tick(),
      Err(TimeError::EventSyncPaused)
    );
  }

  #[cfg(test)]
  mod pausing_logic {
    use super::*;

    #[test]
    fn time_is_retained_when_pausing_and_unpausing() {
      let mut event_sync = EventSync::new(TEST_TICKRATE);
      let other_event_sync = event_sync.clone();

      event_sync.wait_for_x_ticks(3).unwrap();
      event_sync.pause();

      other_event_sync.wait_for_x_ticks(3).unwrap();

      event_sync.unpause().unwrap();

      assert_eq!(event_sync.ticks_since_started(), Ok(3));
    }

    #[test]
    fn time_is_still_tracked_after_unpausing() {
      let mut event_sync = EventSync::new(TEST_TICKRATE);
      let other_event_sync = event_sync.clone();

      event_sync.wait_for_x_ticks(3).unwrap();
      event_sync.pause();

      other_event_sync.wait_for_x_ticks(3).unwrap();

      event_sync.unpause().unwrap();
      event_sync.wait_for_tick().unwrap();

      assert_eq!(event_sync.ticks_since_started(), Ok(4));
    }

    #[test]
    fn restart_unpauses_eventsync() {
      let mut event_sync = EventSync::new(TEST_TICKRATE);
      event_sync.wait_for_tick().unwrap();
      event_sync.pause();

      event_sync.restart();

      event_sync.wait_for_x_ticks(2).unwrap();

      assert_eq!(event_sync.ticks_since_started(), Ok(2));
      assert!(!event_sync.is_paused());
    }
  }

  #[cfg(test)]
  mod serde_implementation_logic {
    use super::*;

    #[test]
    fn serialize_pauses() {
      let event_sync = EventSync::new(TEST_TICKRATE);
      let other_event_sync = event_sync.clone();

      event_sync.wait_for_tick().unwrap();

      let serialized_event_sync = serde_json::to_string(&event_sync).unwrap();

      other_event_sync.wait_for_tick().unwrap();

      let mut deserialized_event_sync =
        serde_json::from_str::<EventSync>(&serialized_event_sync).unwrap();

      assert!(deserialized_event_sync.is_paused());

      deserialized_event_sync.unpause().unwrap();

      assert_eq!(deserialized_event_sync.ticks_since_started(), Ok(1));
    }

    #[test]
    fn serialize_doesnt_overwrite_existing_pause_value() {
      let mut event_sync = EventSync::new(TEST_TICKRATE);

      event_sync.wait_for_tick().unwrap();
      event_sync.pause();

      let serialized_event_sync = serde_json::to_string(&event_sync).unwrap();

      let mut deserialized_event_sync =
        serde_json::from_str::<EventSync>(&serialized_event_sync).unwrap();

      assert!(deserialized_event_sync.is_paused());

      deserialized_event_sync.unpause().unwrap();

      assert_eq!(deserialized_event_sync.ticks_since_started(), Ok(1));
    }
  }

  #[test]
  fn get_tickrate_logic() {
    let event_sync = EventSync::new(TEST_TICKRATE);

    assert_eq!(event_sync.get_tickrate(), TEST_TICKRATE);
  }

  #[test]
  fn change_tickrate_logic() {
    let mut event_sync = EventSync::new(TEST_TICKRATE);

    event_sync.wait_for_x_ticks(2).unwrap();

    event_sync.change_tickrate(TEST_TICKRATE * 2);

    assert_eq!(event_sync.get_tickrate(), TEST_TICKRATE * 2);
    assert_eq!(event_sync.ticks_since_started(), Ok(1));
  }

  #[test]
  fn anyhow_compatibility() {
    fn return_anyhow_error() -> anyhow::Result<()> {
      Err(TimeError::ThatTimeHasAlreadyHappened)?;

      Ok(())
    }

    let _ = return_anyhow_error();
  }
}
