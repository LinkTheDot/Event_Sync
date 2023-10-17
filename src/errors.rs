use std::time::SystemTimeError;
use thiserror::Error;

/// All errors that can be returned when using this crate.
#[derive(Error, Debug)]
pub enum TimeError {
  #[error("An error has occurred when dealing with system time: {}", .0)]
  SystemTimeError(#[from] SystemTimeError),

  /// This error is returned when the [`wait_until()`](crate::EventSync::wait_until) method has been
  /// called with a time that's already occurred.
  #[error("A method with a time input has been told to wait for a time that already happened.")]
  ThatTimeHasAlreadyHappened,
}

impl PartialEq for TimeError {
  fn eq(&self, other: &Self) -> bool {
    std::mem::discriminant(self) == std::mem::discriminant(other)
  }
}

impl Eq for TimeError {}
