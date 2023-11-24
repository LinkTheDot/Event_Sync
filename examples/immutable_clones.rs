//! This example shows how you can clone an EventSync without giving it permission to change internal values.

use event_sync::{EventSync, Immutable};

struct MyTimeKeeper {
  mutable: EventSync,
  immutable: EventSync<Immutable>,
}

fn main() {
  let event_sync = EventSync::new(10);
  let mut time_keeper = MyTimeKeeper {
    immutable: event_sync.clone_immutable(),
    mutable: event_sync,
  };

  // Method is not available to call from the Immutable version.
  // time_keeper.immutable.pause();
  time_keeper.mutable.pause();

  // Method is not available to call from the Immutable version.
  // time_keeper.immutable.unpause();

  // Show that both EventSyncs are connected.
  assert!(time_keeper.immutable.is_paused());
  assert!(time_keeper.immutable.wait_for_tick().is_err());

  // Unpause with the mutable EventSync.
  time_keeper.mutable.unpause().unwrap();

  println!("Now waiting.");
  // The Immutable EventSync has the same methods as Mutable EventSyncs that take &self, but not &mut self.
  time_keeper.immutable.wait_for_x_ticks(100).unwrap();
  println!("Finished_waiting.");
}
