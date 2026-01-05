use super::Config;
use notify::{Event, EventKind, RecursiveMode, Result as NotifyResult, Watcher};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::sleep;
use tracing::{error, info, warn};

/// Watches config file for changes and reloads automatically
pub struct ConfigWatcher {
    config: Arc<RwLock<Config>>,
    config_path: PathBuf,
}

impl ConfigWatcher {
    pub fn new(config: Config, config_path: PathBuf) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            config_path,
        }
    }

    /// Get a clone of the current config
    pub fn config(&self) -> Arc<RwLock<Config>> {
        self.config.clone()
    }

    /// Start watching for config file changes
    pub async fn start(self) -> anyhow::Result<()> {
        let config = self.config.clone();
        let path = self.config_path.clone();

        tokio::spawn(async move {
            if let Err(e) = watch_config_file(path.clone(), config).await {
                error!("Config watcher error: {}", e);
            }
        });

        Ok(())
    }
}

async fn watch_config_file(path: PathBuf, config: Arc<RwLock<Config>>) -> anyhow::Result<()> {
    let (tx, mut rx) = tokio::sync::mpsc::channel(10);

    // Create file watcher
    let mut watcher = notify::recommended_watcher(move |res: NotifyResult<Event>| {
        if let Ok(event) = res {
            // Only care about modify events
            if matches!(event.kind, EventKind::Modify(_)) {
                let _ = tx.blocking_send(());
            }
        }
    })?;

    // Watch the config file
    watcher.watch(&path, RecursiveMode::NonRecursive)?;
    info!("Watching config file for changes: {:?}", path);

    // Debounce duration (wait for file writes to finish)
    let debounce_duration = Duration::from_millis(100);
    let mut last_reload = tokio::time::Instant::now();

    while rx.recv().await.is_some() {
        let now = tokio::time::Instant::now();

        // Debounce: ignore rapid successive events
        if now.duration_since(last_reload) < debounce_duration {
            continue;
        }

        // Wait a bit to let file writes complete
        sleep(debounce_duration).await;

        // Try to reload config
        match Config::from_file(&path) {
            Ok(new_config) => {
                let mut config_guard = config.write().await;
                *config_guard = new_config;
                info!("Config reloaded successfully");
                last_reload = now;
            }
            Err(e) => {
                warn!("Failed to reload config, keeping old config: {}", e);
            }
        }
    }

    Ok(())
}
