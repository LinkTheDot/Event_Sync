# Event Sync

EventSync is a crate that can be used to synchronize events to only occur
between fixed gaps of time.

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
event_sync = "0.4.4"
```

## Features

- **Time Tracking**: Track elapsed time and ticks with precision
- **Thread Synchronization**: Synchronize multiple threads to run at fixed intervals
- **Pause/Resume**: Pause and resume time tracking while preserving elapsed time
- **Mutable/Immutable Permissions**: Control which clones can modify the EventSync state
- **Serialization Support**: Serialize and deserialize EventSync instances with serde
- **Flexible Creation**: Create EventSync from scratch, or from a starting tick/time

**Note**: This library is **synchronous only** and uses `std::thread::sleep` for timing.
It is not designed for async/await patterns or async runtimes like tokio or async-std.

## Overview

Say you wanted an event to occur every 10ms, but it takes a few milliseconds to
setup that event. You'd end up having to sleep 10ms + the time it took to setup
the event.

That's where EventSync comes in. You can create an EventSync with a tickrate of
10ms, setup your event, then wait until the next tick. As long as the time it
took to setup the event was <10ms, waiting for the next tick would ensure
exactly 10ms had occurred since the last event. That would look something like
this

```rust
use event_sync::*;

let tickrate = 10; // 10ms between every tick.
let event_sync = EventSync::new(tickrate);

// multi-ms long task

event_sync.wait_for_tick();
// repeat the task
```

## Getting Started

In order to use event_sync, you start by creating an instance of `EventSync`
with `EventSync::new()`. You then pass in the desired tickrate for the EventSync
to know how long a tick should last. (For more ways of creating an EventSync, check
the examples)

The tickrate will be an integer represented as milliseconds, and cannot go
below 1. If you pass in 0, 1 millisecond will be set as the tickrate.

```rust
use event_sync::*;

let tickrate = 10; // 10ms between every tick

// Create an event synchronizer with a 10ms tickrate.
let event_sync = EventSync::new(tickrate);
```

With this, you can call methods such as `wait_for_x_ticks()`. Which will wait
for the amount of ticks passed in.

That would look something like this:

```rust
use event_sync::*;

let tickrate = 10;
let event_sync = EventSync::new(tickrate);

// multi-ms long task.

// wait for the next 2 ticks
event_sync.wait_for_x_ticks(2);
// repeat the task
```

This would make it so the task in question would only start running every 20ms.

## What even is a `Tick`?

A `Tick` can be thought of as imaginary markers in time, starting at creation of
the EventSync, and separated by the duration of the `Tickrate`.

When you wait for 1 tick, EventSync will sleep its current thread up to the
next tick. If you were to wait for multiple ticks, EventSync sleeps up to the
next tick, plus the duration of the remaining ticks to wait for.

Another way to describe it. Say we had a tickrate of 10ms, and it's been 5ms
since the last tick. If you then waited 1 tick, EventSync will sleep for 5ms,
which is the duration until the next tick marker.

## Permissions

EventSync can exist in two states: `Mutable` and `Immutable`.
These states indicate which methods can be called on an instance of `EventSync`.

The reason these exist is due to the connectedness of an EventSync. There needs to be some sort
of hierarchy involved so that not just anybody can change the world.

### Mutable EventSync

A `Mutable` EventSync has full access to all methods, including those that modify the state:
- `pause()` / `unpause()`
- `change_tickrate()`
- `restart()` / `restart_paused()`

If you had some sort of master struct that holds an EventSync, this is what would hold a Mutable copy:

```rust
use event_sync::*;

struct MasterTimeKeeper {
  synchronizer: EventSync<Mutable>,
}
```

### Immutable EventSync

An `Immutable` EventSync can only read data and wait for ticks. It cannot modify the underlying state.
This is useful when you want to synchronize threads but prevent them from changing the tickrate or pausing.

To create an Immutable EventSync, use the `clone_immutable()` method:

```rust
use event_sync::*;

struct MasterTimeKeeper {
  synchronizer: EventSync<Mutable>,
}

let tickrate = 10;
let master = MasterTimeKeeper { synchronizer: EventSync::new(tickrate) };

// Create an immutable clone that can't modify the state
let connected: EventSync<Immutable> = master.synchronizer.clone_immutable();

// This immutable clone can still wait for ticks
connected.wait_for_tick();

// But it cannot pause, unpause, or change the tickrate
// connected.pause(); // This won't compile!
```

Both Mutable and Immutable EventSyncs remain connected - changes made through the Mutable copy
will affect all connected EventSyncs.

## Serialization

EventSync supports serialization and deserialization through `serde`. When an EventSync is serialized,
it is automatically paused to preserve the exact elapsed time. When deserialized, it remains paused
until you call `unpause()`.

```rust
use event_sync::EventSync;

let tickrate = 10;
let event_sync = EventSync::new(tickrate);

// Add some time
event_sync.wait_for_x_ticks(5);

// Serialize (this pauses the EventSync)
let serialized = serde_json::to_string(&event_sync).unwrap();

// Deserialize (remains paused)
let mut deserialized: EventSync = serde_json::from_str(&serialized).unwrap();

assert!(deserialized.is_paused());
deserialized.unpause().unwrap();

// The elapsed time is preserved
assert_eq!(deserialized.ticks_since_started(), 5);
```

## Error Handling

Several methods in EventSync return `Result<(), TimeError>`. The possible errors are:

- `TimeError::ThatTimeHasAlreadyHappened`: Returned when `wait_until()` is called with a tick that has already passed
- `TimeError::EventSyncPaused`: Returned when trying to wait while the EventSync is paused
- `TimeError::FailedToStartEventSync`: Returned if there's an internal error when unpausing

Example:

```rust
use event_sync::*;

let tickrate = 10;
let event_sync = EventSync::new(tickrate);

// Wait for 5 ticks
event_sync.wait_for_x_ticks(5).unwrap();

// This will error because tick 3 has already occurred
if let Err(e) = event_sync.wait_until(3) {
    println!("Error waiting: {}", e);
}
```
