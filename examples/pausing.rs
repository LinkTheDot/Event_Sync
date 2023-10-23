//! This example shows using the .pause() and .unpause() methods.

use event_sync::EventSync;

/// 500ms between ticks.
const TICKRATE: u32 = 500;

fn main() -> anyhow::Result<()> {
  // Create the EventSync with 500ms tickrate.
  let mut event_sync = EventSync::new(TICKRATE);
  // Create another instance of EventSync to wait while the other one is paused.
  let other_event_sync = event_sync.clone();

  // Add some time to the main EventSync, then pause it.
  event_sync.wait_for_x_ticks(3)?;
  event_sync.pause();

  // Use the other EventSync to desync the main one.
  other_event_sync.wait_for_x_ticks(3)?;

  // Unpause the main EventSync, maintaining the fact that 3 ticks and some time have passed.
  event_sync.unpause()?;

  // Ensure that 3 ticks have passed in the main EventSync, even if 6 have technically passed in total.
  assert_eq!(event_sync.ticks_since_started(), Ok(3));

  Ok(())
}
