use crate::errors::TimeError;
use crate::EventSync;
use async_trait::async_trait;
use std::time::Duration;

#[async_trait]
pub trait AsyncWaiting {
  async fn wait_until(&self, tick_to_wait_for: u64) -> Result<(), TimeError>;
  async fn wait_for_tick(&self);
  async fn wait_for_x_ticks(&self, ticks_to_wait: u32);
}

#[async_trait]
impl AsyncWaiting for EventSync {
  async fn wait_until(&self, tick_to_wait_for: u64) -> Result<(), TimeError> {
    if self.ticks_since_started()? < tick_to_wait_for {
      let total_time_to_wait = Duration::from_millis(tick_to_wait_for * self.tickrate as u64)
        - self.time_since_started()?;

      tokio::time::sleep(total_time_to_wait).await;
    } else {
      return Err(TimeError::ThatTimeHasAlreadyHappened);
    }

    Ok(())
  }

  async fn wait_for_tick(&self) {
    self.wait_for_x_ticks(1).await;
  }

  async fn wait_for_x_ticks(&self, ticks_to_wait: u32) {
    let ticks_since_started = self.ticks_since_started().unwrap();

    let _ = self
      .wait_until(ticks_since_started + ticks_to_wait as u64)
      .await;
  }
}
