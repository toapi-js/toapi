//! The platform layer: spawning tasks, timers, wall-clock time and `Send`
//! bounds, for native targets (tokio) and the browser
//! (`wasm32-unknown-unknown`, through wasm-bindgen).
//!
//! WASI targets have a working `std` clock and use the native code paths
//! except for the runtime, which they'd have to provide through tokio.

use std::{future::Future, pin::Pin, time::Duration};

/// `Send` on native targets. In the browser there is only one thread and
/// futures holding JavaScript values aren't `Send`, so it's implemented for
/// every type there.
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
pub trait MaybeSend: Send {}
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
impl<T: Send> MaybeSend for T {}

/// `Send` on native targets. In the browser there is only one thread and
/// futures holding JavaScript values aren't `Send`, so it's implemented for
/// every type there.
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
pub trait MaybeSend {}
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
impl<T> MaybeSend for T {}

/// A boxed future that is `Send` on native targets.
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;
/// A boxed future that is `Send` on native targets.
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

pub(crate) fn boxed<'a, F>(future: F) -> BoxFuture<'a, F::Output>
where
    F: Future + MaybeSend + 'a,
{
    Box::pin(future)
}

/// Runs a future in the background.
///
/// # Panics
///
/// On native targets, when called outside of a tokio runtime.
pub(crate) fn spawn(future: impl Future<Output = ()> + MaybeSend + 'static) {
    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    tokio::spawn(future);
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    wasm_bindgen_futures::spawn_local(future);
}

pub(crate) async fn sleep(duration: Duration) {
    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    tokio::time::sleep(duration).await;
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    gloo_timers::future::sleep(duration).await;
}

/// Milliseconds since the Unix epoch, the unit of `X-TAPI-Expires-At`.
pub(crate) fn now_ms() -> u64 {
    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    return std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64);
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    return js_sys::Date::now() as u64;
}

/// A pseudo-random delay up to `max`. It only spreads out expiries, so a
/// counter mixed with SplitMix64 is random enough and works on every target.
pub(crate) fn jitter(max: Duration) -> Duration {
    use std::sync::atomic::{AtomicU64, Ordering};

    static STATE: AtomicU64 = AtomicU64::new(0);
    let mut z = STATE.fetch_add(0x9E37_79B9_7F4A_7C15, Ordering::Relaxed);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    max.mul_f64((z >> 11) as f64 / (1u64 << 53) as f64)
}

/// The HTTP client used when none is configured. Native clients keep
/// cookies themselves; in the browser, the browser does.
pub(crate) fn default_http_client() -> reqwest::Client {
    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    return reqwest::Client::builder()
        .cookie_store(true)
        .build()
        .expect("failed to build the HTTP client");
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    return reqwest::Client::new();
}

/// Sends cookies with the request. Needed in the browser for the session
/// cookie to reach the server (cross-origin APIs must allow credentials in
/// their CORS headers); native clients send their cookie store anyway.
pub(crate) fn with_credentials(request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    return request.fetch_credentials_include();
    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    return request;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jitter_stays_below_max_and_varies() {
        let max = Duration::from_millis(1000);
        let values: Vec<Duration> = (0..100).map(|_| jitter(max)).collect();
        assert!(values.iter().all(|v| *v < max));
        assert!(values.windows(2).any(|w| w[0] != w[1]));
    }

    #[test]
    fn now_is_after_2020() {
        assert!(now_ms() > 1_577_836_800_000);
    }
}
