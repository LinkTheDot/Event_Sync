use crate::errors::TimeError;
use serde::{Deserialize, Serialize, Serializer};
use std::time::{Duration, Instant};

/// The internal data for EventSync for threadsafe sharing of this value.
#[derive(Clone, Eq, PartialEq, Serialize, Deserialize)]
pub(crate) struct InnerEventSync {
  #[serde(serialize_with = "serialize_paused")]
  state: EventSyncState,
  tickrate: u32,
}

/// The states an EventSync could be in.
///
/// When running, an [`Instant`](std::time::Instant) will be stored, tracking passed time whilst running.
/// When paused, the time that passed whilst running is stored as a [`Duration`](std::time::Duration).
#[derive(Clone, Eq, PartialEq, Serialize, Deserialize)]
enum EventSyncState {
  #[serde(skip_serializing)]
  #[serde(skip_deserializing)]
  Running(Instant),

  Paused(Duration),
}

impl EventSyncState {
  /// Returns true if the state is EventSyncState::Paused().
  fn is_paused(&self) -> bool {
    std::mem::discriminant(self)
      == std::mem::discriminant(&EventSyncState::Paused(Duration::default()))
  }

  /// Changes the state to Paused, and stored the elapsed time while running.
  fn pause(&mut self) {
    if let EventSyncState::Running(time) = self {
      *self = EventSyncState::Paused(time.elapsed())
    }
  }

  /// Changes the state to Running and applies the time that occurred before pausing.
  ///
  /// # Errors
  ///
  /// - If [`Instant::checked_sub`](https://doc.rust-lang.org/stable/std/time/struct.Instant.html#method.checked_sub) fails.
  fn unpause(&mut self) -> Result<(), TimeError> {
    match self {
      EventSyncState::Paused(paused_duration) => {
        if let Some(running_time) = Instant::now().checked_sub(*paused_duration) {
          *self = EventSyncState::Running(running_time);
        } else {
          return Err(TimeError::FailedToStartEventSync);
        };
      }

      _ => return Ok(()),
    }

    Ok(())
  }
}

/// Serializes the EventSync's state field to EventSyncState::Paused whether paused or not.
///
/// Stores the paused Duration with the elapsed time if the EventSync was running.
/// Otherwise serializes with the already existing paused time.
fn serialize_paused<S>(value: &EventSyncState, serializer: S) -> Result<S::Ok, S::Error>
where
  S: Serializer,
{
  match value {
    EventSyncState::Running(time) => EventSyncState::Paused(time.elapsed()).serialize(serializer),
    EventSyncState::Paused(_) => value.serialize(serializer),
  }
}

impl InnerEventSync {
  /// Creates an instance of InnerEventSync with the given tickrate, starting time, and whether or not it starts paused.
  ///
  /// Starting paused will store the passed in subtracted_time.
  pub(crate) fn new(tickrate: u32, subtracted_time: Duration, is_paused: bool) -> Self {
    let state = if is_paused {
      EventSyncState::Paused(subtracted_time)
    } else {
      EventSyncState::Running(Instant::now().checked_sub(subtracted_time).unwrap())
    };

    Self {
      state,
      tickrate: tickrate.max(1),
    }
  }

  // Not used at the moment, but the code will be kept here for if it's ever needed in the future.
  // pub(crate) fn from_starting_time(tickrate_in_milliseconds: u32, starting_time: Duration) -> Self { }
  // pub(crate) fn from_starting_tick(tickrate_in_milliseconds: u32, starting_tick: u32) -> Self { }

  /// Pauses the internal state of the EventSync.
  ///
  /// Does nothing if already paused.
  pub(crate) fn pause(&mut self) {
    self.state.pause();
  }

  /// Changes the internal state to Running and applies the time that occurred before pausing.
  ///
  /// # Errors
  ///
  /// - If [`Instant::checked_sub`](https://doc.rust-lang.org/stable/std/time/struct.Instant.html#method.checked_sub) fails.
  pub(crate) fn unpause(&mut self) -> Result<(), TimeError> {
    self.state.unpause()
  }

  /// Returns true if the current state of the EventSync is EventSyncState::Running().
  pub(crate) fn is_paused(&self) -> bool {
    self.state.is_paused()
  }

  /// A convenience method that will return an error if the event sync is paused.
  ///
  /// # Errors
  ///
  /// - When self is paused..?
  pub(crate) fn err_if_paused(&self) -> Result<(), TimeError> {
    if self.is_paused() {
      return Err(TimeError::EventSyncPaused);
    }

    Ok(())
  }

  /// Sets the EventSync state to Running, overwriting any data in the previous state.
  pub(crate) fn restart(&mut self) {
    self.state = EventSyncState::Running(Instant::now());
  }

  /// Sets the EventSync state to Paused(Duration::default()), overwriting any data in the previous state.
  pub(crate) fn restart_paused(&mut self) {
    self.state = EventSyncState::Paused(Duration::default());
  }

  /// Change the internally stored tickrate
  pub(crate) fn change_tickrate(&mut self, new_tickrate: u32) {
    self.tickrate = new_tickrate.max(1);
  }

  /// Returns the currently stored tickrate.
  pub(crate) fn get_tickrate(&self) -> u32 {
    self.tickrate
  }

  /// Returns the exact amount of time to sleep to reach a specified tick.
  ///
  /// If 1.6 ticks have passed, and 3 is passed in, 1.4 * tickrate is returned.
  pub(crate) fn time_until_tick_occurs(
    &self,
    tick_to_wait_for: u64,
  ) -> Result<Duration, TimeError> {
    self.err_if_paused()?;

    if self.ticks_since_started() < tick_to_wait_for {
      Ok(
        Duration::from_millis(tick_to_wait_for * self.get_tickrate() as u64)
          - self.time_since_started(),
      )
    } else {
      Err(TimeError::ThatTimeHasAlreadyHappened)
    }
  }

  /// Returns the amount of time needed to sleep until the next tick.
  ///
  /// Let's say the tickrate is 10ms, and the last tick was 5ms ago.
  /// This method would return Duration(5ms), which is the time to the next tick.
  ///
  /// # Errors
  ///
  /// - An error is returned if the EventSync is paused.
  pub(crate) fn time_for_tick(&self) -> Result<Duration, TimeError> {
    self.err_if_paused()?;

    self.time_for_x_ticks(1)
  }

  /// Returns the amount of time to wait for the desired amount of ticks.
  ///
  /// Let's say the tickrate is 10ms, and the last tick was 5ms ago.
  /// If you wanted to wait for 3 ticks, this method would return 25ms, as that would be 3 ticks from now.
  ///   
  /// # Errors
  ///
  /// - An error is returned if the EventSync is paused.
  pub(crate) fn time_for_x_ticks(&self, ticks_to_wait: u32) -> Result<Duration, TimeError> {
    self.err_if_paused()?;

    let ticks_since_started = self.ticks_since_started();

    self.time_until_tick_occurs(ticks_since_started + ticks_to_wait as u64)
  }

  /// Returns the amount of time that has occurred since the creation of this instance of EventSync.
  pub(crate) fn time_since_started(&self) -> std::time::Duration {
    match self.state {
      EventSyncState::Running(instant) => instant.elapsed(),
      EventSyncState::Paused(time) => time,
    }
  }

  /// Returns the amount of ticks that have occurred since the creation of this instance of EventSync.
  pub(crate) fn ticks_since_started(&self) -> u64 {
    let time_passed = match self.state {
      EventSyncState::Running(instant) => instant.elapsed().as_millis(),
      EventSyncState::Paused(time) => time.as_millis(),
    };

    (time_passed / self.tickrate as u128) as u64
  }

  /// Returns the amount of time that has passed since the last tick
  pub(crate) fn time_since_last_tick(&self) -> std::time::Duration {
    Duration::from_nanos(
      (self.time_since_started().as_nanos() % (self.get_tickrate() as u128 * 1000000)) as u64,
    )
  }

  /// Returns the amount of time until the next tick will occur.
  pub(crate) fn time_until_next_tick(&self) -> std::time::Duration {
    Duration::from_millis(self.get_tickrate() as u64).saturating_sub(self.time_since_last_tick())
  }
}
