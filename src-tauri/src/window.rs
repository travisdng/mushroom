//! Window geometry: remember where the window was, and refuse to restore it
//! somewhere the user cannot see (R1.5, R1.6).

use tauri::{Manager, PhysicalPosition, PhysicalSize, WebviewWindow};

use crate::config::{save, WindowRect};
use crate::state::AppState;

/// Bounds a window rect has to satisfy to be worth saving or restoring.
///
/// A window still being created can report nonsense — a height of 32767
/// (`i16::MAX`) was observed in testing. Restoring that would leave the user
/// with an unusable window and no obvious way back, so implausible rects are
/// dropped and the window is centred at its default size instead.
const MIN_DIM: u32 = 200;
const MAX_DIM: u32 = 16_384;

fn is_plausible(rect: &WindowRect) -> bool {
    (MIN_DIM..=MAX_DIM).contains(&rect.width) && (MIN_DIM..=MAX_DIM).contains(&rect.height)
}

/// True when any part of `rect` overlaps a monitor that currently exists.
///
/// Guards against the classic case: the window was last closed on a second
/// monitor that has since been unplugged.
fn is_on_screen(window: &WebviewWindow, rect: &WindowRect) -> bool {
    let Ok(monitors) = window.available_monitors() else {
        return false;
    };

    let (wx1, wy1) = (rect.x, rect.y);
    let wx2 = rect.x + rect.width as i32;
    let wy2 = rect.y + rect.height as i32;

    monitors.iter().any(|monitor| {
        let pos = monitor.position();
        let size = monitor.size();
        let (mx1, my1) = (pos.x, pos.y);
        let mx2 = pos.x + size.width as i32;
        let my2 = pos.y + size.height as i32;

        wx1 < mx2 && wx2 > mx1 && wy1 < my2 && wy2 > my1
    })
}

/// Apply a saved rect, or centre the window if it would land off-screen, then
/// re-maximise if it was closed maximised.
pub fn restore(window: &WebviewWindow, rect: Option<&WindowRect>, maximized: bool) {
    // Set the normal-state geometry first, then maximise. Doing it in this
    // order means un-maximising returns to the saved size rather than to
    // whatever default the window was created with.
    place(window, rect);

    if maximized {
        let _ = window.maximize();
    }
}

fn place(window: &WebviewWindow, rect: Option<&WindowRect>) {
    let Some(rect) = rect else {
        let _ = window.center();
        return;
    };

    if !is_plausible(rect) {
        tracing::warn!(
            target: "app",
            width = rect.width,
            height = rect.height,
            "saved window size is implausible; centring instead"
        );
        let _ = window.center();
        return;
    }

    if !is_on_screen(window, rect) {
        tracing::info!(
            target: "app",
            "saved window position is off-screen; centring instead"
        );
        let _ = window.center();
        return;
    }

    let _ = window.set_size(PhysicalSize::new(rect.width, rect.height));
    let _ = window.set_position(PhysicalPosition::new(rect.x, rect.y));
}

/// Read the window's current geometry and write it to the config file.
///
/// Called on close rather than on every move: a person drags a window far more
/// often than they close one, and a crash losing the last position is a
/// non-event.
pub fn persist(window: &WebviewWindow) {
    // A minimised window reports the geometry of the minimised state, which is
    // useless to restore.
    if window.is_minimized().unwrap_or(false) {
        return;
    }

    let maximized = window.is_maximized().unwrap_or(false);

    if maximized {
        // A maximised window reports the screen-filling rect. Saving that would
        // reopen a window that fills the screen without being maximised, and
        // un-maximising would do nothing visible. Keep whatever normal-state
        // rect we already had and just record that it was maximised.
        set_maximized_flag(window, true);
        return;
    }

    // inner_size, NOT outer_size: `set_size` restores the *client* area, while
    // `outer_size` includes the border and title bar. Saving the outer size and
    // restoring it as the inner size grows the window by the decorations on
    // every launch. Position has no such asymmetry — both are outer.
    let (Ok(position), Ok(size)) = (window.outer_position(), window.inner_size()) else {
        return;
    };

    let rect = WindowRect {
        x: position.x,
        y: position.y,
        width: size.width,
        height: size.height,
    };

    if !is_plausible(&rect) {
        tracing::warn!(
            target: "app",
            width = rect.width,
            height = rect.height,
            "refusing to save an implausible window size"
        );
        return;
    }

    let Some(state) = window.app_handle().try_state::<AppState>() else {
        return;
    };

    let snapshot = {
        let Ok(mut config) = state.config.lock() else {
            return;
        };
        config.ui.window = Some(rect);
        config.ui.maximized = false;
        config.clone()
    };

    if let Err(err) = save(&state.data_dir, &snapshot) {
        tracing::warn!(target: "app", error = %err, "window geometry could not be saved");
    }
}

/// Record only the maximised flag, leaving the stored normal-state rect alone.
fn set_maximized_flag(window: &WebviewWindow, maximized: bool) {
    let Some(state) = window.app_handle().try_state::<AppState>() else {
        return;
    };

    let snapshot = {
        let Ok(mut config) = state.config.lock() else {
            return;
        };
        config.ui.maximized = maximized;
        config.clone()
    };

    if let Err(err) = save(&state.data_dir, &snapshot) {
        tracing::warn!(target: "app", error = %err, "window state could not be saved");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(w: u32, h: u32) -> WindowRect {
        WindowRect {
            x: 100,
            y: 100,
            width: w,
            height: h,
        }
    }

    #[test]
    fn accepts_ordinary_window_sizes() {
        assert!(is_plausible(&rect(1100, 720)));
        assert!(is_plausible(&rect(800, 560)));
    }

    #[test]
    fn rejects_the_sizes_a_half_created_window_reports() {
        // Observed in testing: a window mid-creation reported i16::MAX.
        assert!(!is_plausible(&rect(900, 32_767)));
        assert!(!is_plausible(&rect(0, 0)));
        assert!(!is_plausible(&rect(1100, 10)));
    }
}
