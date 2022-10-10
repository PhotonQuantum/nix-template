use std::sync::Mutex;
use std::time::Duration;

use indicatif::ProgressBar;

static GLOBAL_PROGRESS_BAR: Mutex<Option<ProgressBar>> = Mutex::new(None);

pub fn new_global_progress_bar(f: impl FnOnce() -> ProgressBar) -> ProgressBar {
    let mut guard = GLOBAL_PROGRESS_BAR.lock().unwrap();
    if let Some(old_pb) = guard.take() {
        old_pb.finish();
    }
    let pb = f();
    pb.enable_steady_tick(Duration::from_millis(60));
    *guard = Some(pb.clone());
    pb
}

pub fn with_global_progress_bar(f: impl FnOnce(ProgressBar)) {
    if let Some(pb) = GLOBAL_PROGRESS_BAR.lock().unwrap().clone() {
        f(pb);
    }
}

pub fn delete_global_progress_bar() {
    if let Some(old_pb) = GLOBAL_PROGRESS_BAR.lock().unwrap().take() {
        old_pb.finish();
    }
}
