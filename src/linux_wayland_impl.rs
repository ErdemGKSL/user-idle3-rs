use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use wayland_client::globals::{registry_queue_init, GlobalListContents};
use wayland_client::protocol::wl_registry::WlRegistry;
use wayland_client::protocol::wl_seat::WlSeat;
use wayland_client::{Connection, Dispatch, QueueHandle};

use wayland_protocols::ext::idle_notify::v1::client::ext_idle_notification_v1::{
    self, ExtIdleNotificationV1,
};
use wayland_protocols::ext::idle_notify::v1::client::ext_idle_notifier_v1::ExtIdleNotifierV1;

use crate::Error;

/// Timeout passed to the compositor. The protocol reports `Idled` only after
/// this much continuous user inactivity, so the minimum reportable idle time
/// is also this value.
const IDLE_TIMEOUT_SECS: u64 = 3;
const IDLE_TIMEOUT_MS: u32 = (IDLE_TIMEOUT_SECS * 1000) as u32;

/// `Some(t)` when the compositor most recently signalled `Idled` at instant
/// `t`. Cleared on `Resumed`.
static IDLE_SINCE: Mutex<Option<Instant>> = Mutex::new(None);

/// Holds the result of the one-shot listener initialization so repeated calls
/// to `get_idle_time` are cheap and return a consistent error if the
/// compositor does not implement `ext-idle-notify-v1`.
static INIT_RESULT: OnceLock<Result<(), String>> = OnceLock::new();

struct State;

impl Dispatch<WlRegistry, GlobalListContents> for State {
    fn event(
        _: &mut Self,
        _: &WlRegistry,
        _: <WlRegistry as wayland_client::Proxy>::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<WlSeat, ()> for State {
    fn event(
        _: &mut Self,
        _: &WlSeat,
        _: <WlSeat as wayland_client::Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ExtIdleNotifierV1, ()> for State {
    fn event(
        _: &mut Self,
        _: &ExtIdleNotifierV1,
        _: <ExtIdleNotifierV1 as wayland_client::Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ExtIdleNotificationV1, ()> for State {
    fn event(
        _: &mut Self,
        _: &ExtIdleNotificationV1,
        event: <ExtIdleNotificationV1 as wayland_client::Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            ext_idle_notification_v1::Event::Idled => {
                if let Ok(mut guard) = IDLE_SINCE.lock() {
                    *guard = Some(Instant::now());
                }
            }
            ext_idle_notification_v1::Event::Resumed => {
                if let Ok(mut guard) = IDLE_SINCE.lock() {
                    *guard = None;
                }
            }
            _ => {}
        }
    }
}

fn init_and_spawn() -> Result<(), String> {
    let conn = Connection::connect_to_env()
        .map_err(|e| format!("wayland connect failed: {e}"))?;

    let (globals, mut event_queue) = registry_queue_init::<State>(&conn)
        .map_err(|e| format!("wayland registry init failed: {e}"))?;
    let qh = event_queue.handle();

    let notifier: ExtIdleNotifierV1 = globals
        .bind(&qh, 1..=1, ())
        .map_err(|e| format!("ext_idle_notifier_v1 unavailable: {e}"))?;

    let seat: WlSeat = globals
        .bind(&qh, 1..=8, ())
        .map_err(|e| format!("wl_seat unavailable: {e}"))?;

    let notification =
        notifier.get_idle_notification(IDLE_TIMEOUT_MS, &seat, &qh, ());

    // Flush the request to the compositor before returning success.
    let mut state = State;
    event_queue
        .roundtrip(&mut state)
        .map_err(|e| format!("wayland roundtrip failed: {e}"))?;

    thread::spawn(move || {
        // Keep proxies alive for the lifetime of the listener thread.
        let _keep = (notifier, seat, notification);
        let mut state = State;
        loop {
            if event_queue.blocking_dispatch(&mut state).is_err() {
                break;
            }
        }
    });

    Ok(())
}

fn ensure_listener_started() -> Result<(), Error> {
    let res = INIT_RESULT.get_or_init(init_and_spawn);
    match res {
        Ok(()) => Ok(()),
        Err(e) => Err(Error::new(e.clone())),
    }
}

pub fn get_idle_time() -> Result<Duration, Error> {
    ensure_listener_started()?;

    let guard = IDLE_SINCE
        .lock()
        .map_err(|_| Error::new("wayland idle state poisoned"))?;
    match *guard {
        Some(t) => Ok(t.elapsed() + Duration::from_secs(IDLE_TIMEOUT_SECS)),
        None => Ok(Duration::ZERO),
    }
}
