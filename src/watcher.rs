use std::{path::Path, time::Duration};

use notify::{RecursiveMode, Watcher};
use tokio::sync::{broadcast, mpsc};

const DEBOUNCE_DELAY: Duration = Duration::from_millis(200);

pub struct FileWatcher {
    _watcher: notify::RecommendedWatcher,
    reloads: broadcast::Sender<()>,
}

impl FileWatcher {
    pub fn new(docs: &Path) -> notify::Result<Self> {
        let (reloads, _) = broadcast::channel(16);
        let (tx, mut rx) = mpsc::unbounded_channel();
        let mut watcher = notify::recommended_watcher(move |event| {
            let _ = tx.send(event);
        })?;
        watcher.watch(docs, RecursiveMode::Recursive)?;

        let sender = reloads.clone();
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                match event {
                    Ok(event) if !matches!(event.kind, notify::EventKind::Access(_)) => {
                        let delay = tokio::time::sleep(DEBOUNCE_DELAY);
                        tokio::pin!(delay); // Pin so select! can use it
                        loop {
                            tokio::select! {
                                _ = &mut delay => break,
                                next = rx.recv() => match next {
                                    Some(Err(error)) => tracing::warn!(%error, "docs watcher error"),
                                    Some(Ok(event)) if !matches!(event.kind, notify::EventKind::Access(_)) => {
                                        delay.as_mut().reset(tokio::time::Instant::now() + Duration::from_millis(200));
                                    }
                                    Some(Ok(_)) => {}
                                    None => return,
                                },
                            }
                        }
                        sender.send(()).ok();
                    }
                    Err(error) => tracing::warn!(%error, "docs watcher error"),
                    _ => {}
                }
            }
        });

        Ok(Self {
            // We want the `FileWatcher` to own the watcher so that it doesn't immediately get
            // dropped.
            _watcher: watcher,
            reloads,
        })
    }

    pub fn reloads(&self) -> broadcast::Sender<()> {
        self.reloads.clone()
    }
}
