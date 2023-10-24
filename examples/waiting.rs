//! This example shows an ever increasing amount of threads synchronizing.
//!
//! Each thread will print which number they are around the same time.

use event_sync::*;
use std::thread;
use std::thread::JoinHandle;

/// 500ms between ticks.
const TICKRATE: u32 = 500;

fn main() -> anyhow::Result<()> {
  let thread_count = 5;

  // Create an EventSync with a tick lasting 500ms.
  let event_sync = EventSync::new(TICKRATE);
  let mut thread_handles: Vec<JoinHandle<()>> = vec![];

  // Spawn 5 threads that will all print which thread they are around the same time (every 2 ticks).
  for x in 0..thread_count {
    let event_sync_moved = event_sync.clone();

    let handle = thread::spawn(move || {
      for _ in 0..(thread_count - x) {
        println!("New thread made, I am {x}");

        event_sync_moved.wait_for_x_ticks(2).unwrap();
      }
    });

    thread_handles.push(handle);

    event_sync.wait_for_tick()?;
    println!(); // Print some space between when the threads say who they are.
    event_sync.wait_for_tick()?;
  }

  for thread_handle in thread_handles {
    let _ = thread_handle.join();
  }

  Ok(())
}
