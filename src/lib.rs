#![doc = include_str!("../README.md")]

use crate::errors::TimeError;
use inner::*;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use std::{
  sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard},
  time::Duration,
};

mod errors;
mod inner;

/// A way to synchronize a dynamic number of threads through sleeping.
/// Achieved through cloning and passing around an instance of EventSync to other threads.
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
/// // Create an EventSync with a 10ms tickrate.
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
/// assert_eq!(finish, event_sync.time_since_started().as_millis());
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
/// // All cloned EventSyncs will share their data.
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
///
/// # Permissions
///
/// Of course, you don't want just anyone to be able to change whatever they want on an EventSync.
/// To prevent that, event_sync provides two states for an EventSync to be in.
/// Those being [`Mutable`](Mutable), and [`Immutable`](Immutable).
///
/// By calling [`event_sync.clone_immutable()`](EventSync::clone_immutable), you create a copy of the EventSync
/// that cannot call any methods requiring &mut self.
///
/// # Example
/// ```
/// use event_sync::*;
///
/// let tickrate = 10;
/// let event_sync = EventSync::new(tickrate);
///
/// let immutable_event_sync = event_sync.clone_immutable(); // Create an immutable EventSync.
///
/// assert_eq!(immutable_event_sync.get_tickrate(), tickrate);
/// ```
///
/// The type for this Immutable EventSync would look like this:
/// ```
/// use event_sync::{EventSync, Immutable};
///
/// struct TimeKeeper {
///   event_sync: EventSync<Immutable>,
/// }
/// ```
#[derive(Clone, Serialize, Deserialize)]
pub struct EventSync<Access = Mutable> {
  inner: Arc<RwLock<InnerEventSync>>,
  change_access: PhantomData<Access>,
}

/// A state for an EventSync to prevent methods with &mut from being called.
///
/// Any copy of [`EventSync`](EventSync) with this label will be unable to manipulate the underlying data.
///
/// # Example
///
/// ```compile_fail
/// use event_sync::*;
///
/// let tickrate = 10; // 10ms between every tick.
/// let master_event_sync: EventSync<Mutable> = EventSync::new(tickrate);
///
/// let mut immutable_event_sync: EventSync<Immutable> = master_event_sync.clone_immutable();
///
/// // Does not compile.
/// immutable_event_sync.change_tickrate(20);
/// ```
#[derive(Clone, Serialize, Deserialize)]
pub struct Immutable;
/// A state for an EventSync to give access to all methods.
///
/// Any copy of [`EventSync`](EventSync) with this label will be able to modify any underlying data.
/// The data changed will affect any EventSync connected to this one.
///
/// To create an [`EventSync<Immutable>`](Immutable) , use the [`.clone_immutable()`](EventSync::clone_immutable) method on any EventSync with the Mutable label, or clone off an existing Immutable one.
///
/// # Example
///
/// ```
/// use event_sync::*;
///
/// let tickrate = 10;
/// let mut master_event_sync = EventSync::new(tickrate);
///
/// let mut mutable_event_sync = master_event_sync.clone();
///
/// mutable_event_sync.change_tickrate(20);
///
/// assert_eq!(master_event_sync.get_tickrate(), 20);
/// ```
#[derive(Clone, Serialize, Deserialize)]
pub struct Mutable;

impl<T> EventSync<T> {
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
    self.read_inner().is_paused()
  }

  /// Returns the internal tickrate.
  ///
  /// # Examples
  ///
  /// ```
  /// use event_sync::*;
  ///
  /// let tickrate = 10; // 10ms tickrate.
  /// let event_sync = EventSync::new(tickrate);
  /// let other_event_sync = event_sync.clone();
  ///
  /// assert_eq!(event_sync.get_tickrate(), tickrate);
  /// assert_eq!(other_event_sync.get_tickrate(), tickrate);
  /// ```
  pub fn get_tickrate(&self) -> u32 {
    self.read_inner().get_tickrate()
  }

  /// Waits until an absolute tick has occurred since EventSync creation.
  ///
  /// That means, if you created an instance of EventSync with a tickrate of 10ms,
  /// and you want to wait until 1 second has passed since creation.
  /// You would wait until the 100th tick, as 100 ticks would be 1 second since EventSync Creation.
  ///
  /// # Errors
  ///
  /// - An error is returned when the given time to wait for has already occurred.
  /// - An error is returned if the EventSync is paused.
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
  pub fn wait_until(&self, tick_to_wait_for: u64) -> Result<(), TimeError> {
    let wait_time = self.read_inner().time_until_tick_occurs(tick_to_wait_for)?;

    std::thread::sleep(wait_time);

    Ok(())
  }

  /// Waits until the next tick relative to where now is between ticks.
  ///
  /// Let's say the tickrate is 10ms, and the last tick was 5ms ago.
  /// This method would sleep for 5ms to get to the next tick.
  ///
  /// # Errors
  ///
  /// - An error is returned if the EventSync is paused.
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
  pub fn wait_for_tick(&self) -> Result<(), TimeError> {
    let wait_time = self.read_inner().time_for_tick()?;

    std::thread::sleep(wait_time);

    Ok(())
  }

  /// Waits for the passed in amount of ticks relative to where now is between ticks.
  ///
  /// Let's say the tickrate is 10ms, and the last tick was 5ms ago.
  /// If you wanted to wait for 3 ticks, this method would sleep for 25ms, as that would be 3 ticks from now.
  ///
  /// # Errors
  ///
  /// - An error is returned if the EventSync is paused.
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
  pub fn wait_for_x_ticks(&self, ticks_to_wait: u32) -> Result<(), TimeError> {
    let wait_time = self.read_inner().time_for_x_ticks(ticks_to_wait)?;

    std::thread::sleep(wait_time);

    Ok(())
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
  /// let milliseconds_since_started = event_sync.time_since_started().as_millis();
  ///
  /// assert_eq!(milliseconds_since_started, 50);
  /// ```
  pub fn time_since_started(&self) -> std::time::Duration {
    self.read_inner().time_since_started()
  }

  /// Returns the amount of ticks that have occurred since the creation of this instance of EventSync.
  ///
  /// # Usage
  ///
  /// ```
  /// use event_sync::*;
  ///
  /// let tickrate = 10; // 10ms between every tick
  /// let event_sync = EventSync::new(tickrate);
  ///
  /// event_sync.wait_until(5);
  ///
  /// assert_eq!(event_sync.ticks_since_started(), 5);
  /// ```
  pub fn ticks_since_started(&self) -> u64 {
    self.read_inner().ticks_since_started()
  }

  /// Returns the amount of time that has passed since the last tick
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
  /// assert!(event_sync.time_since_last_tick().as_micros() < 500); // Practically no time should have passed since the last tick.
  /// ```
  pub fn time_since_last_tick(&self) -> std::time::Duration {
    self.read_inner().time_since_last_tick()
  }

  /// Returns the amount of time until the next tick will occur.
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
  /// assert!(event_sync.time_until_next_tick().as_micros() > 500); // Practically no time should have passed since the last tick.
  /// ```
  pub fn time_until_next_tick(&self) -> std::time::Duration {
    self.read_inner().time_until_next_tick()
  }

  /// Obtains a ReadGuard of the [`internal EventSync data`](InnerEventSync).
  fn read_inner(&self) -> RwLockReadGuard<InnerEventSync> {
    self.inner.read().unwrap()
  }
}

impl EventSync<Mutable> {
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
  /// // Create an EventSync with a 10ms tickrate.
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
  /// assert_eq!(finish, event_sync.time_since_started().as_millis());
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
  /// // All cloned EventSyncs will share their data.
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
  pub fn new(tickrate_in_milliseconds: u32) -> Self {
    Self::new_event_sync(tickrate_in_milliseconds, Duration::default(), false)
  }

  /// Creates a new instance of EventSync that starts out paused.
  ///
  /// # Examples
  ///
  /// ```
  /// use event_sync::*;
  ///
  /// let tickrate = 10; // 10ms between every tick.
  /// let event_sync = EventSync::new_paused(tickrate); // Create an event_sync that starts out paused.
  ///
  /// assert!(event_sync.is_paused());
  /// assert!(event_sync.wait_for_tick().is_err());
  /// ```
  pub fn new_paused(tickrate_in_milliseconds: u32) -> Self {
    Self::new_event_sync(tickrate_in_milliseconds, Duration::default(), true)
  }

  /// Creates a new instance of [`EventSync`](EventSync) with the given starting time.
  ///
  /// Takes an extra arguement to determine if the EventSync should be paused upon creation or not.
  ///
  /// # Example
  ///
  /// ```
  /// use event_sync::*;
  /// use std::time::Duration;
  ///
  /// let tickrate = 10; // 10ms between every tick.
  /// let starting_time = Duration::from_millis(30); // Start 30ms ahead.
  /// let event_sync = EventSync::from_starting_time(tickrate, starting_time, false);
  ///
  /// assert_eq!(event_sync.ticks_since_started(), 3);
  /// ```
  ///
  /// # Starting Paused
  ///
  /// ```
  /// use event_sync::*;
  /// use std::time::Duration;
  ///
  /// let tickrate = 10; // 10ms between every tick.
  /// let starting_time = Duration::from_millis(30); // Start 30ms ahead.
  /// let mut event_sync = EventSync::from_starting_time(tickrate, starting_time, true);
  ///
  /// assert!(event_sync.is_paused());
  /// event_sync.unpause().unwrap();
  ///
  /// assert_eq!(event_sync.ticks_since_started(), 3);
  /// ```
  pub fn from_starting_time(
    tickrate_in_milliseconds: u32,
    elapsed_time: Duration,
    start_paused: bool,
  ) -> Self {
    Self::new_event_sync(tickrate_in_milliseconds, elapsed_time, start_paused)
  }

  /// Creates a new instance of [`EventSync`](EventSync) with the given starting tick.
  ///
  /// Takes an extra arguement to determine if the EventSync should be paused upon creation or not.
  ///
  /// # Example
  ///
  /// ```
  /// use event_sync::*;
  /// use std::time::Duration;
  ///
  /// let tickrate = 10; // 10ms between every tick.
  /// let starting_tick = 3; // Start 3 ticks ahead.
  /// let event_sync = EventSync::from_starting_tick(tickrate, starting_tick, false);
  ///
  /// assert_eq!(event_sync.ticks_since_started(), 3);
  /// ```
  ///
  /// # Starting Paused
  ///
  /// ```
  /// use event_sync::*;
  /// use std::time::Duration;
  ///
  /// let tickrate = 10; // 10ms between every tick.
  /// let starting_tick = 3; // Start 3 ticks ahead.
  /// let mut event_sync = EventSync::from_starting_tick(tickrate, starting_tick, true);
  ///
  /// assert!(event_sync.is_paused());
  /// event_sync.unpause().unwrap();
  ///
  /// assert_eq!(event_sync.ticks_since_started(), 3);
  /// ```
  pub fn from_starting_tick(
    tickrate_in_milliseconds: u32,
    starting_tick: u32,
    start_paused: bool,
  ) -> Self {
    let elapsed_time = Duration::from_millis((starting_tick * tickrate_in_milliseconds).into());

    Self::new_event_sync(tickrate_in_milliseconds, elapsed_time, start_paused)
  }

  /// Create a new [`EventSync`](EventSync) from the given tickrate and whether or not the EventSync is started paused.
  /// If paused, the stored passed time will be the passed in elapsed_time.
  fn new_event_sync(tickrate: u32, elapsed_time: Duration, is_paused: bool) -> Self {
    let inner = InnerEventSync::new(tickrate, elapsed_time, is_paused);

    Self {
      inner: Arc::new(RwLock::new(inner)),
      change_access: PhantomData,
    }
  }

  /// This creates an Immutable instance of [`EventSync`](EventSync).
  ///
  /// This version of EventSync cannot change any of the underlying data, only being able to use/read the data.
  /// Methods such a waiting for a certain tick are fine, however pausing, unpausing, changing tickrate, etc. are not possible through an Immutable EventSync.
  ///
  /// Additionally, Immutable [`EventSync`](EventSync) can only create other Immutable instances of itself.
  pub fn clone_immutable(&self) -> EventSync<Immutable> {
    EventSync {
      inner: self.inner.clone(),
      change_access: PhantomData,
    }
  }

  /// Obtains a WriteGuard of the [`internal EventSync data`](InnerEventSync).
  fn write_inner(&mut self) -> RwLockWriteGuard<InnerEventSync> {
    self.inner.write().unwrap()
  }

  /// Restarts the starting time.
  /// This will also restart the time for every EventSync cloned off of this one.
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
  /// assert_eq!(event_sync.ticks_since_started(), 0); // 0 ticks is returned because the EventSync was restarted.
  /// ```
  pub fn restart(&mut self) {
    self.write_inner().restart();
  }

  /// Restarts the startimg time, and changes self to paused.
  /// This will also restart and pause the time for every EventSync cloned off of this one.
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
  /// event_sync.restart_paused(); // Restart the EventSync.
  ///
  /// assert!(event_sync.is_paused());
  /// ```
  pub fn restart_paused(&mut self) {
    self.write_inner().restart_paused();
  }

  /// Changes how long a tick lasts internally. Retains the time that passed before method call.
  /// That means if 100ms have passed, 100ms will still have passed. The amount of ticks will be the
  /// only thing that's changed.
  ///
  /// Changes the tickrate for all connected EventSyncs.
  ///
  /// # Examples
  ///
  /// ```
  /// use event_sync::*;
  ///
  /// let tickrate = 10; // 10ms tickrate.
  /// let mut event_sync = EventSync::new(tickrate);
  ///
  /// // Wait for 100ms (10 ticks).
  /// event_sync.wait_for_x_ticks(10).unwrap();
  ///
  /// // Change the tickrate to 100ms, 10x what it was before.
  /// event_sync.change_tickrate(tickrate * 10);
  ///
  /// // Ensure that 1 tick has passed, which is now 100ms.
  /// assert_eq!(event_sync.ticks_since_started(), 1);
  /// // Ensure that the tickrate is now 100ms instead of the prior 10ms.
  /// assert_eq!(event_sync.get_tickrate(), 100);
  /// ```
  ///
  /// # EventSyncs are connected
  ///
  /// ```
  /// use event_sync::*;
  ///
  /// let tickrate = 10; // 10ms tickrate.
  /// let event_sync = EventSync::new(tickrate);
  /// let mut other_event_sync = event_sync.clone();
  ///
  /// // Change the tickrate. This will change it for both EventSyncs.
  /// other_event_sync.change_tickrate(tickrate * 2);
  ///
  /// // Ensure the original EventSync's tickrate is also changed.
  /// assert_eq!(event_sync.get_tickrate(), tickrate * 2);
  /// ```
  pub fn change_tickrate(&mut self, new_tickrate: u32) {
    self.write_inner().change_tickrate(new_tickrate);
  }

  /// Unpauses this instance of EventSync if it's been paused.
  /// Any EventSync that was cloned off this one is also unpaused, as they are all connected.
  ///
  /// If the time passed before pausing was 10.1 seconds, that time will be retained when unpaused.
  ///
  /// Calling unpause when the EventSync is already running does nothing.
  ///
  /// # Examples
  ///
  /// ```
  /// use event_sync::EventSync;
  ///
  /// let tickrate = 10; // 10ms between every tick
  /// let mut event_sync = EventSync::new(tickrate);
  /// let other_event_sync = EventSync::new(tickrate); // Create a second one to desync.
  ///
  /// event_sync.wait_for_tick().unwrap(); // Add some time.
  /// event_sync.pause();
  ///
  /// other_event_sync.wait_for_tick().unwrap(); // Desync from the paused EventSync.
  ///
  /// event_sync.unpause().unwrap();
  /// assert_eq!(event_sync.ticks_since_started(), 1); // Only 1 tick has passed while this EventSync wasn't paused.
  /// ```
  ///
  /// # EventSyncs are connected
  ///
  /// ```
  /// use event_sync::EventSync;
  ///
  /// let tickrate = 10; // 10ms between every tick
  /// let mut event_sync = EventSync::new_paused(tickrate);
  /// let other_event_sync = event_sync.clone();
  ///
  /// assert!(other_event_sync.is_paused());
  ///
  /// event_sync.unpause().unwrap();
  ///
  /// assert!(!other_event_sync.is_paused());
  /// ```
  pub fn unpause(&mut self) -> Result<(), TimeError> {
    self.write_inner().unpause()
  }

  /// Pauses this instance of EventSync.
  /// Any EventSync that was cloned off this one is also paused, as they are all connected.
  ///
  /// When paused, the time that passed is retained.
  /// If 10.1 seconds have passed, that time will be retained after paused.
  ///
  /// Calling pause when already paused does nothing.
  ///
  /// # Examples
  ///
  /// ```
  /// use event_sync::EventSync;
  ///
  /// let tickrate = 10; // 10ms between every tick
  /// let mut event_sync = EventSync::new(tickrate);
  /// let other_event_sync = EventSync::new(tickrate); // Create a second one to desync.
  ///
  /// event_sync.wait_for_tick().unwrap(); // Add some time.
  /// event_sync.pause();
  ///
  /// other_event_sync.wait_for_tick().unwrap(); // Desync from the paused EventSync.
  ///
  /// event_sync.unpause().unwrap();
  /// assert_eq!(event_sync.ticks_since_started(), 1); // Only 1 tick has passed while this EventSync wasn't paused.
  /// ```
  ///
  /// # EventSyncs are connected
  ///
  /// ```
  /// use event_sync::EventSync;
  ///
  /// let tickrate = 10; // 10ms between every tick
  /// let mut event_sync = EventSync::new(tickrate);
  /// let other_event_sync = event_sync.clone();
  ///
  /// event_sync.pause();
  ///
  /// assert!(other_event_sync.is_paused());
  ///
  /// ```
  pub fn pause(&mut self) {
    self.write_inner().pause()
  }
}

impl<T> PartialEq for EventSync<T> {
  fn eq(&self, other: &Self) -> bool {
    *self.read_inner() == *other.read_inner()
  }
}

impl<T> Eq for EventSync<T> {}

impl<T> std::fmt::Debug for EventSync<T> {
  fn fmt(
    &self,
    formatter: &mut std::fmt::Formatter<'_>,
  ) -> std::result::Result<(), std::fmt::Error> {
    write!(formatter, "{:?}", self.time_since_started())
  }
}

impl<T> std::fmt::Display for EventSync<T> {
  fn fmt(
    &self,
    formatter: &mut std::fmt::Formatter<'_>,
  ) -> std::result::Result<(), std::fmt::Error> {
    write!(formatter, "{:?}", self)
  }
}

impl Default for EventSync {
  fn default() -> Self {
    Self::new(10)
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

      assert_eq!(ticks_since_started, 5);
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

    let time_since_started = event_sync.time_since_started();

    assert_eq!(time_since_started.as_millis(), TEST_TICKRATE as u128 * 2);
  }

  #[test]
  fn ticks_since_started_logic() {
    let event_sync = EventSync::new(TEST_TICKRATE);

    event_sync.wait_until(2).unwrap();

    let ticks_since_started = event_sync.ticks_since_started();

    assert_eq!(ticks_since_started, 2);
  }

  #[test]
  fn wait_for_tick_logic() {
    let event_sync = EventSync::new(TEST_TICKRATE);

    event_sync.wait_for_tick().unwrap();

    let ticks_since_started = event_sync.ticks_since_started();

    assert_eq!(ticks_since_started, 1);
  }

  #[test]
  fn time_since_last_tick_logic() {
    let tickrate = 1;
    let event_sync = EventSync::new(tickrate);

    event_sync.wait_for_tick().unwrap();

    let time_since_last_tick = event_sync.time_since_last_tick();

    assert!((tickrate as u128 * 1000000) > time_since_last_tick.as_nanos());
    assert_ne!(time_since_last_tick.as_nanos(), 0);
  }

  #[test]
  fn time_since_last_tick_accuracy() {
    let event_sync = EventSync::new(TEST_TICKRATE);
    let extra_wait_time = 2;

    event_sync.wait_for_tick().unwrap();

    std::thread::sleep(Duration::from_millis(extra_wait_time as u64));

    let time_since_last_tick = event_sync.time_since_last_tick();

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
      (event_sync.time_until_next_tick().as_micros() as f64 / 1000.0).ceil();

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
      let event_sync = EventSync::from_starting_tick(TEST_TICKRATE, STARTING_TICKS, false);

      assert_eq!(event_sync.ticks_since_started(), STARTING_TICKS as u64);
    }

    #[test]
    fn from_ticks_paused() {
      let mut event_sync = EventSync::from_starting_tick(TEST_TICKRATE, STARTING_TICKS, true);

      assert!(event_sync.is_paused());

      event_sync.unpause().unwrap();

      assert_eq!(event_sync.ticks_since_started(), STARTING_TICKS as u64);
    }

    #[test]
    fn from_time() {
      let starting_time = Duration::from_millis((STARTING_TICKS * TEST_TICKRATE).into());
      let event_sync = EventSync::from_starting_time(TEST_TICKRATE, starting_time, false);

      assert_eq!(event_sync.ticks_since_started(), STARTING_TICKS as u64);
    }

    #[test]
    fn from_time_paused() {
      let starting_time = Duration::from_millis((STARTING_TICKS * TEST_TICKRATE).into());
      let mut event_sync = EventSync::from_starting_time(TEST_TICKRATE, starting_time, true);

      assert!(event_sync.is_paused());

      event_sync.unpause().unwrap();

      assert_eq!(event_sync.ticks_since_started(), STARTING_TICKS as u64);
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
  }

  #[cfg(test)]
  mod pausing_logic {
    use super::*;

    #[test]
    fn time_is_retained_when_pausing_and_unpausing() {
      let mut event_sync = EventSync::new(TEST_TICKRATE);
      let other_event_sync = EventSync::new(TEST_TICKRATE);

      event_sync.wait_for_x_ticks(3).unwrap();
      event_sync.pause();

      other_event_sync.wait_for_x_ticks(3).unwrap();

      event_sync.unpause().unwrap();

      assert_eq!(event_sync.ticks_since_started(), 3);
    }

    #[test]
    fn time_is_still_tracked_after_unpausing() {
      let mut event_sync = EventSync::new(TEST_TICKRATE);
      let other_event_sync = EventSync::new(TEST_TICKRATE);

      event_sync.wait_for_x_ticks(3).unwrap();
      event_sync.pause();

      other_event_sync.wait_for_x_ticks(3).unwrap();

      event_sync.unpause().unwrap();
      event_sync.wait_for_tick().unwrap();

      assert_eq!(event_sync.ticks_since_started(), 4);
    }

    #[test]
    fn restart_unpauses_eventsync() {
      let mut event_sync = EventSync::new(TEST_TICKRATE);
      event_sync.wait_for_tick().unwrap();
      event_sync.pause();

      event_sync.restart();

      event_sync.wait_for_x_ticks(2).unwrap();

      assert_eq!(event_sync.ticks_since_started(), 2);
      assert!(!event_sync.is_paused());
    }

    #[test]
    fn pausing_pauses_cloned() {
      let event_sync = EventSync::new(TEST_TICKRATE);
      let mut other_event_sync = event_sync.clone();
      other_event_sync.pause();

      let expected_result = Err(TimeError::EventSyncPaused);

      let result = event_sync.wait_for_tick();

      assert_eq!(result, expected_result);
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

      assert_eq!(deserialized_event_sync.ticks_since_started(), 1);
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

      assert_eq!(deserialized_event_sync.ticks_since_started(), 1);
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

    // Does not compile.
    event_sync.change_tickrate(TEST_TICKRATE * 2);

    assert_eq!(event_sync.get_tickrate(), TEST_TICKRATE * 2);
    assert_eq!(event_sync.ticks_since_started(), 1);
  }

  #[test]
  fn anyhow_compatibility() {
    fn return_anyhow_error() -> anyhow::Result<()> {
      Err(TimeError::ThatTimeHasAlreadyHappened)?;

      Ok(())
    }

    let _ = return_anyhow_error();
  }

  #[test]
  fn mutable_partial_eq_logic() {
    let event_sync = EventSync::new(1);
    let copied_event_sync = event_sync.clone();
    let separate_event_sync = EventSync::new(1);

    assert_eq!(event_sync, copied_event_sync);
    assert_ne!(event_sync, separate_event_sync);
    assert_ne!(copied_event_sync, separate_event_sync);
  }

  #[test]
  fn immutable_partial_eq_logic() {
    let event_sync = EventSync::new(1);
    let copied_event_sync_1 = event_sync.clone_immutable();
    let copied_event_sync_2 = event_sync.clone_immutable();
    let separate_event_sync = EventSync::new(1);
    let separate_copied_event_sync = separate_event_sync.clone_immutable();

    assert_ne!(event_sync, separate_event_sync);
    assert_ne!(copied_event_sync_1, separate_copied_event_sync);
    assert_eq!(copied_event_sync_1, copied_event_sync_2);
  }

  #[test]
  fn debug_and_display_logic() {
    let event_sync = EventSync::new(1);
    let copied_event_sync = event_sync.clone_immutable();

    let _mutable_debug = format!("{:?}", event_sync);
    let _mutable_display = format!("{}", event_sync);

    let _immutable_debug = format!("{:?}", copied_event_sync);
    let _immutable_display = format!("{}", copied_event_sync);

    // Can't compare as microseconds of time would pass between each format call.
    // This is mostly to test if both mutable and immutable can event format into Debug and Display
    // in the first place.
  }
}
