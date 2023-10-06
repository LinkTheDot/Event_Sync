# Event Sync

EventSync is a crate that can be used to synchronize events to only occur
between fixed gaps of time.

Say you wanted an event to occur every 10ms, but it takes a few milliseconds to
setup that event. You'd end up having to sleep 10ms + the time it took to setup
the event.

That's where EventSync comes in. You can create an EventSync with a tickrate of
10ms, setup your event, then wait until the next tick. Aslong as the time it
took to setup the event was <10ms, waiting for the next tick would ensure
exactly 10ms had ocurred since the last event. That would look something like
this

```rust
use event_sync::EventSync;

let tickrate = 10; // 10ms between every tick.
let event_sync = EventSync::new(tickrate);

// multi-ms long task

event_sync.wait_for_tick();
// repeat the task
```

## Getting Started

In order to use event_sync, you start by creating an instance of `EventSync`
with `EventSync::new()`. You then pass in the desired tickrate for the EventSync
to know how long a tick should last.

The tickrate will be an integer represented as milliseconds, and cannot go
below 1. If you pass in 0, 1 millisecond will be set as the tickrate.

```rust
use event_sync::EventSync;

let tickrate = 10; // 10ms between every tick

// Create an event synchronizer with a 10ms tickrate.
let event_sync = EventSync::new(tickrate);
```

With this, you can call methods such as `wait_for_x_ticks()`. Which will wait
for the amount of ticks passed in.

That would look something like this:

```rust
use event_sync::EventSync;

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

When you wait for 1 tick, EventSync will sleep it's current thread up to the
next tick. If you were to wait for multiple ticks, EventSync sleeps up to the
next tick, plus the duration of the remaining ticks to wait for.

Another way to describe it. Say we had a tickrate of 10ms, and it's been 5ms
since the last tick. If you then waited 1 tick, EventSync will sleep for 5ms
which is how long until the next tick marker.
