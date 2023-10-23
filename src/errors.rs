use std::time::SystemTimeError;
use thiserror::Error;

/// All errors that can be returned when using this crate.
#[derive(Error, Debug, Clone)]
pub enum TimeError {
  #[allow(clippy::enum_variant_names)] // It's a wrapper, so I don't think this warning is valid here.
  #[error("An error has occurred when dealing with system time: {}", .0)]
  SystemTimeError(#[from] SystemTimeError),

  /// This error is returned when the [`wait_until()`](crate::EventSync::wait_until) method has been
  /// called with a time that's already occurred.
  #[error("A method with a time input has been told to wait for a time that already happened.")]
  ThatTimeHasAlreadyHappened,

  /// Attempted to call a method on an EventSync that was paused.
  #[error("Attempted to call a time based method on a paused EventSync.")]
  EventSyncPaused,
}

impl PartialEq for TimeError {
  fn eq(&self, other: &Self) -> bool {
    std::mem::discriminant(self) == std::mem::discriminant(other)
  }
}

impl Eq for TimeError {}
