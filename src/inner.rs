use crate::errors::TimeError;
use serde::{Deserialize, Serialize, Serializer};
use std::time::{Duration, SystemTime};

/// The internal data for EventSync for threadsafe sharing of this value.
#[derive(Clone, Eq, PartialEq, Serialize, Deserialize)]
pub(crate) struct InnerEventSync {
  start_time: SystemTime,
  tickrate: u32,
  #[serde(serialize_with = "pause")]
  paused_time: Option<SystemTime>,
}

/// Sets the paused_time field when InnerEventSync is serialized.
fn pause<S>(value: &Option<SystemTime>, serializer: S) -> Result<S::Ok, S::Error>
where
  S: Serializer,
{
  value.unwrap_or(SystemTime::now()).serialize(serializer)
}

impl InnerEventSync {
  /// Creates an instance of InnerEventSync with the given tickrate, starting time, and whether or not it starts paused.
  ///
  /// The paused time given is SystemTime::now().
  pub(crate) fn new(tickrate: u32, start_time: SystemTime, is_paused: bool) -> Self {
    Self {
      start_time,
      tickrate: tickrate.max(1),
      paused_time: is_paused.then_some(SystemTime::now()),
    }
  }

  /// Creates an instance of InnerEventSync with SystemTime::now() - starting_time.
  /// Essentially starting with an already determined amount of time passed.
  pub(crate) fn from_starting_time(tickrate_in_milliseconds: u32, starting_time: Duration) -> Self {
    let starting_time = SystemTime::now() - starting_time;

    Self::new(tickrate_in_milliseconds, starting_time, false)
  }

  // Not used at the moment, but the code will be kept here for if it's ever needed.
  // pub(crate) fn from_starting_tick(tickrate_in_milliseconds: u32, starting_tick: u32) -> Self {
  //   let starting_time = Duration::from_millis((starting_tick * tickrate_in_milliseconds).into());
  //   let starting_time = SystemTime::now() - starting_time;
  //
  //   Self::new(tickrate_in_milliseconds, starting_time, false)
  // }

  /// Assigns the paused_time to the current time if it's not already that time.
  pub(crate) fn pause(&mut self) {
    if self.paused_time.is_none() {
      self.paused_time = Some(SystemTime::now());
    }
  }

  pub(crate) fn unpause(&mut self) -> Result<(), TimeError> {
    let Some(paused_time) = self.paused_time else {
      return Ok(());
    };
    let time_running = paused_time.duration_since(self.start_time)?;

    *self = Self::from_starting_time(self.tickrate, time_running);

    Ok(())
  }

  pub(crate) fn is_paused(&self) -> bool {
    self.paused_time.is_some()
  }

  /// A convenience method that will return an error if the event sync is paused.
  ///
  /// # Errors
  ///
  /// - When self is paused..?
  fn err_if_paused(&self) -> Result<(), TimeError> {
    if self.is_paused() {
      return Err(TimeError::EventSyncPaused);
    }

    Ok(())
  }

  /// Restarts the EventSync start time and unpauses if paused.
  pub(crate) fn restart(&mut self) {
    self.start_time = SystemTime::now();
    self.paused_time = None;
  }

  /// Restarts the EventSync start time and sets the pause time to now.
  pub fn restart_paused(&mut self) {
    self.start_time = SystemTime::now();
    self.paused_time = Some(SystemTime::now());
  }

  /// Change the internally stored tickrate
  pub fn change_tickrate(&mut self, new_tickrate: u32) {
    self.tickrate = new_tickrate.max(1);
  }

  /// Returns the currently stored tickrate.
  pub fn get_tickrate(&self) -> u32 {
    self.tickrate
  }

  /// Waits until an absolute tick has occurred since EventSync creation.
  ///
  /// That means, if you created an instance of EventSync with a tickrate of 10ms,
  /// and you want to wait until 1 second has passed since creation.
  /// You would wait until the 100th tick, as 100 ticks would be 1 second since EventSync Creation.
  pub fn wait_until(&self, tick_to_wait_for: u64) -> Result<(), TimeError> {
    self.err_if_paused()?;

    if self.ticks_since_started()? < tick_to_wait_for {
      let total_time_to_wait = Duration::from_millis(tick_to_wait_for * self.get_tickrate() as u64)
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
  pub fn time_since_last_tick(&self) -> Result<std::time::Duration, TimeError> {
    self.err_if_paused()?;

    Ok(Duration::from_nanos(
      (self.time_since_started()?.as_nanos() % (self.get_tickrate() as u128 * 1000000)) as u64,
    ))
  }

  /// Returns the amount of time until the next tick will occur.
  ///
  /// # Errors
  ///
  /// - An error is returned if the EventSync is paused.
  /// - An error is returned when the system time has been reversed to before this EventSync was created.
  pub fn time_until_next_tick(&self) -> Result<std::time::Duration, TimeError> {
    self.err_if_paused()?;

    Ok(
      Duration::from_millis(self.get_tickrate() as u64)
        .saturating_sub(self.time_since_last_tick()?),
    )
  }
}
