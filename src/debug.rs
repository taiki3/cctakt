//! Debug logging utilities
//!
//! Provides debug logging that only activates in debug builds.
//! In release builds, all debug_log! calls are no-ops.

use std::fs::OpenOptions;
use std::io::Write;
use std::sync::Mutex;
use std::path::Path;

static DEBUG_FILE: Mutex<Option<std::fs::File>> = Mutex::new(None);

/// Initialize debug logging (only in debug builds)
#[cfg(debug_assertions)]
pub fn init() {
    let mut file_guard = DEBUG_FILE.lock().unwrap();
    if file_guard.is_none() {
        if let Ok(file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open("debug.log")
        {
            *file_guard = Some(file);
            drop(file_guard);
            log("=== Debug session started ===");
        }
    }
}

#[cfg(not(debug_assertions))]
pub fn init() {}

/// Log a message to debug.log (only in debug builds)
#[cfg(debug_assertions)]
pub fn log(message: &str) {
    let timestamp = chrono::Local::now().format("%H:%M:%S%.3f");
    let line = format!("[{}] {}\n", timestamp, message);

    if let Ok(mut file_guard) = DEBUG_FILE.lock() {
        if let Some(ref mut file) = *file_guard {
            let _ = file.write_all(line.as_bytes());
            let _ = file.flush();
        }
    }
}

#[cfg(not(debug_assertions))]
pub fn log(_message: &str) {}

/// Log a message with a category prefix
#[cfg(debug_assertions)]
pub fn log_category(category: &str, message: &str) {
    log(&format!("[{}] {}", category, message));
}

#[cfg(not(debug_assertions))]
pub fn log_category(_category: &str, _message: &str) {}

/// Log worker stream-json output
#[cfg(debug_assertions)]
pub fn log_worker(worker_id: &str, event_type: &str, content: &str) {
    let truncated = if content.len() > 200 {
        format!("{}...", content.chars().take(200).collect::<String>())
    } else {
        content.to_string()
    };
    log(&format!("[WORKER:{}] {} | {}", worker_id, event_type, truncated));
}

#[cfg(not(debug_assertions))]
pub fn log_worker(_worker_id: &str, _event_type: &str, _content: &str) {}

/// Log task state changes
#[cfg(debug_assertions)]
pub fn log_task(task_id: &str, old_status: &str, new_status: &str) {
    log(&format!("[TASK:{}] {} -> {}", task_id, old_status, new_status));
}

#[cfg(not(debug_assertions))]
pub fn log_task(_task_id: &str, _old_status: &str, _new_status: &str) {}

/// Log worktree operations
#[cfg(debug_assertions)]
pub fn log_worktree(operation: &str, path: &Path) {
    log(&format!("[WORKTREE] {} {}", operation, path.display()));
}

#[cfg(not(debug_assertions))]
pub fn log_worktree(_operation: &str, _path: &Path) {}

/// Macro for convenient debug logging
#[macro_export]
macro_rules! debug_log {
    ($($arg:tt)*) => {
        $crate::debug::log(&format!($($arg)*))
    };
}
