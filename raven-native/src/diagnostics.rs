use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

const MAX_LOG_BYTES: u64 = 2 * 1024 * 1024;

fn log_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

pub fn log_path() -> PathBuf {
    let base = dirs::data_local_dir().unwrap_or_else(std::env::temp_dir);
    base.join("Raven Notch").join("logs").join("compatibility.log")
}

pub fn log(scope: &str, message: &str) {
    let Ok(_guard) = log_lock().lock() else {
        return;
    };

    let path = log_path();
    let Some(parent) = path.parent() else {
        return;
    };
    if fs::create_dir_all(parent).is_err() {
        return;
    }

    if path.metadata().map(|meta| meta.len() > MAX_LOG_BYTES).unwrap_or(false) {
        let previous = path.with_extension("previous.log");
        let _ = fs::remove_file(&previous);
        let _ = fs::rename(&path, previous);
    }

    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(
            file,
            "[{}] [{}] {}",
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"),
            scope,
            message
        );
    }
}

pub fn log_startup() {
    log(
        "STARTUP",
        &format!(
            "Raven {} arch={} os={} diagnostics={}",
            env!("CARGO_PKG_VERSION"),
            std::env::consts::ARCH,
            std::env::consts::OS,
            log_path().display()
        ),
    );
}
