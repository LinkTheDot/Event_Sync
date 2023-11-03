//! Shows the use of serializing and deserializing an instance of EventSync.
//! Showing also that time does not pass for a stored EventSync.

use event_sync::EventSync;

/// 500ms between ticks.
const TICKRATE: u32 = 500;

fn main() {
  // Create the EventSync with 500ms tickrate.
  let event_sync = EventSync::new(TICKRATE);

  // Add some time from creation.
  event_sync.wait_for_x_ticks(2).unwrap();

  // Store the EventSync.
  // This will pause it ensuring that only 2 ticks will have passed once we deserialize.
  let serialized_event_sync = serde_json::to_string(&event_sync).unwrap();

  // Desynchronize the stored and still live EventSync.
  event_sync.wait_for_tick().unwrap();

  // Deserialize the stored EventSync.
  let mut deserialized_event_sync =
    serde_json::from_str::<EventSync>(&serialized_event_sync).unwrap();
  // Unpause the EventSync.
  deserialized_event_sync.unpause().unwrap();

  // Ensure that 2 ticks have still passed for the stored EventSync, even if 3 have technically passed real time.
  assert_eq!(deserialized_event_sync.ticks_since_started(), Ok(2));
}
