use crate::errors::TimeError;
use crate::EventSync;
use std::time::Duration;

pub trait StdWaiting {
  /// Waits until an absolute tick has occurred since EventSync creation.
  ///
  /// That means, if you created an instance of EventSync with a tickrate of 10ms,
  /// and you want to wait until 1 second has passed since creation.
  /// You would wait until the 100th tick, as 100 ticks would be 1 second since EventSync Creation.
  ///
  /// # Usage
  ///
  /// ```
  /// use event_sync::*;
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
  fn wait_until(&self, tick_to_wait_for: u64) -> Result<(), TimeError>;

  /// Waits until the next tick relative to where now is between ticks.
  ///
  /// Let's say the tickrate is 10ms, and the last tick was 5ms ago.
  /// This method would sleep for 5ms to get to the next tick.
  ///
  /// # Usage
  /// ```
  /// use event_sync::*;
  ///
  /// let tickrate = 10; // 10ms between every tick
  /// let event_sync = EventSync::new(tickrate);
  ///
  /// // wait until the next tick
  /// event_sync.wait_for_tick();
  /// ```
  fn wait_for_tick(&self);

  /// Waits for the passed in amount of ticks relative to where now is between ticks.
  ///
  /// Let's say the tickrate is 10ms, and the last tick was 5ms ago.
  /// If you wanted to wait for 3 ticks, this method would sleep for 25ms, as that would be 3 ticks from now.
  ///
  /// # Usage
  /// ```
  /// use event_sync::*;
  ///
  /// let tickrate = 10; // 10ms between every tick
  /// let event_sync = EventSync::new(tickrate);
  ///
  /// // wait for 3 ticks
  /// event_sync.wait_for_x_ticks(3);
  /// ```
  fn wait_for_x_ticks(&self, ticks_to_wait: u32);
}

impl StdWaiting for EventSync {
  fn wait_until(&self, tick_to_wait_for: u64) -> Result<(), TimeError> {
    if self.ticks_since_started()? < tick_to_wait_for {
      let total_time_to_wait = Duration::from_millis(tick_to_wait_for * self.tickrate as u64)
        - self.time_since_started()?;

      std::thread::sleep(total_time_to_wait);
    } else {
      return Err(TimeError::ThatTimeHasAlreadyHappened);
    }

    Ok(())
  }

  fn wait_for_tick(&self) {
    self.wait_for_x_ticks(1);
  }

  fn wait_for_x_ticks(&self, ticks_to_wait: u32) {
    let ticks_since_started = self.ticks_since_started().unwrap();

    let _ = self.wait_until(ticks_since_started + ticks_to_wait as u64);
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

    event_sync.wait_for_tick();

    let ticks_since_started = event_sync.ticks_since_started();

    assert_eq!(ticks_since_started, Ok(1));
  }

  #[test]
  fn time_since_last_tick_logic() {
    let event_sync = EventSync::new(TEST_TICKRATE);
    let extra_wait_time = 2;

    event_sync.wait_for_tick();

    std::thread::sleep(Duration::from_millis(extra_wait_time as u64));

    let time_since_last_tick = event_sync.time_since_last_tick().unwrap();

    assert_eq!(time_since_last_tick.as_millis(), extra_wait_time);
  }

  #[test]
  fn time_until_next_tick_logic() {
    let event_sync = EventSync::new(TEST_TICKRATE);
    let extra_wait_time = 2;

    event_sync.wait_for_tick();

    std::thread::sleep(Duration::from_millis(extra_wait_time as u64));

    let time_since_last_tick = event_sync.time_until_next_tick().unwrap();

    assert_eq!(
      time_since_last_tick.as_millis(),
      (TEST_TICKRATE as u128 - extra_wait_time)
    );
  }
}
