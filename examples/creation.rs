//! This example shows the different ways to create an EventSync.

use event_sync::*;
use std::time::Duration;

/// 500ms between ticks.
const TICKRATE: u32 = 500;

fn main() {
  let starting_ticks = 2;
  let starting_time = Duration::from_millis((starting_ticks * TICKRATE).into());

  // Create an EventSync from 0 with a tickrate of 500ms.
  let event_sync_zero = EventSync::new(TICKRATE);
  // Create an EventSync with 2 ticks having already passed.
  let event_sync_from_ticks = EventSync::from_starting_tick(TICKRATE, starting_ticks, false);
  // Create an EventSync with 1 second having already passed.
  let event_sync_from_time = EventSync::from_starting_time(TICKRATE, starting_time, false);
  // Create a paused EventSync with 1 second having already passed.
  let paused_event_sync = EventSync::from_starting_time(TICKRATE, starting_time, true);

  // Ensure all the starting times are as expected.
  assert_eq!(event_sync_zero.ticks_since_started(), 0);
  assert_eq!(event_sync_from_ticks.ticks_since_started(), 2);
  assert_eq!(event_sync_from_time.ticks_since_started(), 2);
  assert!(paused_event_sync.is_paused());
}
