use std::{collections::HashSet, sync::Weak, time::Duration};

use futures::StreamExt;

use crate::client::Inner;

/// Listens to the server's invalidation stream for as long as the client
/// lives, reconnecting with exponential backoff (0.5s × 1.1ⁿ, capped at
/// n = 100, like the TypeScript client).
///
/// Every (re)connect drops the whole cache, since invalidations may have been
/// missed while disconnected.
pub(crate) fn spawn_listener(
    inner: Weak<Inner>,
    http: reqwest::Client,
    url: String,
) -> tokio::task::AbortHandle {
    tokio::spawn(async move {
        let mut retry: i32 = 0;
        loop {
            match http.get(&url).send().await {
                Ok(res) if res.status().is_success() => {
                    retry = 0;
                    match inner.upgrade() {
                        Some(inner) => {
                            tokio::spawn(async move { inner.invalidate_all().await });
                        }
                        None => return,
                    }
                    let mut body = res.bytes_stream();
                    let mut buffer = Vec::new();
                    while let Some(Ok(chunk)) = body.next().await {
                        let Some(inner) = inner.upgrade() else { return };
                        buffer.extend_from_slice(&chunk);
                        while let Some(end) = buffer.iter().position(|b| *b == b'\n') {
                            let line: Vec<u8> = buffer.drain(..=end).collect();
                            let tags: HashSet<String> = String::from_utf8_lossy(&line)
                                .split_whitespace()
                                .map(String::from)
                                .collect();
                            if !tags.is_empty() {
                                let inner = inner.clone();
                                tokio::spawn(async move { inner.invalidate_tags(&tags).await });
                            }
                        }
                    }
                }
                _ => {}
            }
            if inner.strong_count() == 0 {
                return;
            }
            let delay = 0.5 * 1.1f64.powi(retry.min(100));
            tokio::time::sleep(Duration::from_secs_f64(delay)).await;
            retry += 1;
        }
    })
    .abort_handle()
}
