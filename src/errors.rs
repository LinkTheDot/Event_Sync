use thiserror::Error;

/// All errors that can be returned when using this crate.
#[derive(Error, Debug, Clone)]
pub enum TimeError {
  /// This error is returned when the [`wait_until()`](crate::EventSync::wait_until) method has been
  /// called with a time that's already occurred.
  #[error("A method with a time input has been told to wait for a time that already happened.")]
  ThatTimeHasAlreadyHappened,

  /// Attempted to call a method on an EventSync that was paused.
  #[error("Attempted to call a time based method on a paused EventSync.")]
  EventSyncPaused,

  /// Failed to subtract the passed pause time from an Instant when starting up an EventSync.
  #[error("Attempted to start an EventSync, but an unexpected error occurred.")]
  FailedToStartEventSync,
}

impl PartialEq for TimeError {
  fn eq(&self, other: &Self) -> bool {
    std::mem::discriminant(self) == std::mem::discriminant(other)
  }
}

impl Eq for TimeError {}
