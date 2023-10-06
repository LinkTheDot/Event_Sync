//! This example shows an ever increasing amount of threads synchronizing.
//!
//! Each thread will print which number they are around the same time.

use event_sync::*;
use std::thread;
use std::thread::JoinHandle;

/// 10ms between ticks.
const TICKRATE: u32 = 500;

fn main() {
  let thread_count = 3;

  let event_sync = EventSync::new(TICKRATE);
  let mut thread_handles: Vec<JoinHandle<()>> = vec![];

  for x in 0..thread_count {
    let event_sync_moved = event_sync.clone();

    let handle = thread::spawn(move || {
      for _ in 0..(thread_count - x) {
        println!("New thread made, I am {x}");

        event_sync_moved.wait_for_x_ticks(2);
      }
    });

    thread_handles.push(handle);

    event_sync.wait_for_tick();
    println!();
    event_sync.wait_for_tick();
  }

  for thread_handle in thread_handles {
    let _ = thread_handle.join();
  }
}
