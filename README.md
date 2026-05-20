# user-idle3

This is a fork of [user-idle-rs](https://github.com/olback/user-idle-rs) (previously published as `user-idle2`), updated with Wayland support via the standard `ext-idle-notify-v1` protocol as well as evdev-based fallback tracking.

| OS              | Supported |
| --------------- | --------- |
| Linux           | ✔️*       |
| Windows         | ✔️        |
| MacOS           | ✔️        |

\* The Linux implementation tries the following providers in order:
1. **Mutter (DBus)** — works on GNOME with Wayland or X11.
2. **X11** — works on any X11 desktop.
3. **`ext-idle-notify-v1` (Wayland)** — standard Wayland idle protocol; requires the `wayland` feature. Works on Sway, KWin ≥5.27, Hyprland, Mutter 45+, and any compositor that implements `ext-idle-notify-v1`.
4. **Screensaver (DBus)** — supported by many desktop environments; may report 0 when the screensaver is inactive.
5. **evdev** — low-level input event tracking; requires the `evdev` feature. Works on any Linux desktop (X11 or Wayland) but requires the user to be in the `input` group.

## Usage

```toml
[dependencies]
user-idle3 = "0.7"
```

```rust
use user_idle3::UserIdle;

let idle = UserIdle::get_time().unwrap();

println!("Idle for {} seconds", idle.as_seconds());
println!("Idle for {} minutes", idle.as_minutes());
println!("Duration: {:?}", idle.duration());
```

## Features

### `wayland` (optional)

Enable native Wayland idle detection using the `ext-idle-notify-v1` protocol:

```toml
[dependencies]
user-idle3 = { version = "0.7", features = ["wayland"] }
```

The `wayland` implementation connects to the Wayland compositor, binds `ext_idle_notifier_v1` and `wl_seat`, and requests idle notifications with a 3-second timeout. A background thread dispatches compositor events. When the compositor signals `Idled`, the elapsed time is computed as `time_since_idle_event + 3s`. When the compositor signals `Resumed`, the idle time resets to zero.

Supported compositors:
- Sway
- KWin (KDE Plasma) ≥ 5.27
- Hyprland
- Mutter (GNOME) ≥ 45
- Any compositor implementing `ext-idle-notify-v1`

### `evdev` (optional)

Enable evdev-based idle tracking as a broad fallback:

```toml
[dependencies]
user-idle3 = { version = "0.7", features = ["evdev"] }
```

The `evdev` implementation runs background threads monitoring all input devices for keyboard and mouse events. Works on any Linux desktop (X11 or Wayland).

**Note:** The user must be in the `input` group:
```bash
sudo usermod -aG input $USER
# Log out and back in for the change to take effect
```

Both features can be enabled together:

```toml
[dependencies]
user-idle3 = { version = "0.7", features = ["wayland", "evdev"] }
```
