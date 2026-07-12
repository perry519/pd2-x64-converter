use serde::{Deserialize, Serialize};
use std::sync::{
  Arc,
  atomic::{AtomicBool, Ordering},
};

#[derive(Clone, Default)]
pub struct CancelToken {
  inner: Arc<AtomicBool>,
}

impl CancelToken {
  pub fn cancel(&self) {
    self.inner.store(true, Ordering::SeqCst);
  }

  pub fn is_cancelled(&self) -> bool {
    self.inner.load(Ordering::SeqCst)
  }

  pub fn is_same_source(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.inner, &other.inner)
  }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProgressPhase {
  Scan,
  Process,
  Stage,
  Commit,
  Cleanup,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProgressEvent {
  pub phase: ProgressPhase,
  pub processed: usize,
  pub total: usize,
  pub current_path: Option<String>,
  pub message: Option<String>,
}

#[derive(Clone, Default)]
pub struct ProgressReporter {
  callback: Option<Arc<dyn Fn(ProgressEvent) + Send + Sync>>,
}

impl ProgressReporter {
  pub fn new(callback: impl Fn(ProgressEvent) + Send + Sync + 'static) -> Self {
    Self {
      callback: Some(Arc::new(callback)),
    }
  }

  pub(crate) fn report(&self, event: ProgressEvent) {
    if let Some(callback) = &self.callback {
      let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| callback(event)));
    }
  }
}
