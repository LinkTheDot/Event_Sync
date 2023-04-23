/// All errors that can be returned when using this crate.
#[derive(Debug, Eq, PartialEq)]
pub enum TimeError {
  /// This error is returned when the system clock has been rolled back to before the clock started.
  TimeHasReversed,

  /// This error is returned when the [`wait_until()`](crate::ThreadSync::wait_until) method has been
  /// called with a time that's already occurred.
  ThatTimeHasAlreadyHappened,
}
