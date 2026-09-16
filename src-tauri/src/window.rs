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

/// How much of the window has to be reachable for the rect to be usable.
///
/// The title bar is the only way to drag a window back with a mouse, so a
/// window whose top edge sits above the screen cannot be recovered without the
/// keyboard. Windows itself will not let you drag a window up there; a saved
/// rect should not be able to put it there either.
const TITLE_BAR_HEIGHT: i32 = 32;
/// Enough of the title bar to actually grab.
const GRABBABLE_WIDTH: i32 = 120;

/// True when the window would be both visible and reachable.
///
/// Guards two cases: the window was last closed on a second monitor that has
/// since been unplugged, and the window's title bar would land off the top of
/// the screen where it cannot be dragged.
fn is_on_screen(window: &WebviewWindow, rect: &WindowRect) -> bool {
    let Ok(monitors) = window.available_monitors() else {
        return false;
    };

    let bounds: Vec<(i32, i32, i32, i32)> = monitors
        .iter()
        .map(|monitor| {
            let pos = monitor.position();
            let size = monitor.size();
            (
                pos.x,
                pos.y,
                pos.x + size.width as i32,
                pos.y + size.height as i32,
            )
        })
        .collect();

    is_reachable(rect, &bounds)
}

/// The geometry half of [`is_on_screen`], split out so it can be tested
/// without a window or a monitor.
fn is_reachable(rect: &WindowRect, monitors: &[(i32, i32, i32, i32)]) -> bool {
    // The strip a person can actually grab: the title bar.
    let bar_left = rect.x;
    let bar_right = rect.x + rect.width as i32;
    let bar_top = rect.y;
    let bar_bottom = rect.y + TITLE_BAR_HEIGHT;

    monitors.iter().any(|&(mx1, my1, mx2, my2)| {
        // The title bar must not be above the monitor's top edge…
        if bar_top < my1 {
            return false;
        }
        // …and enough of it must be within the monitor to click.
        let overlap = bar_right.min(mx2) - bar_left.max(mx1);
        overlap >= GRABBABLE_WIDTH && bar_top < my2 && bar_bottom > my1
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

    /// One 1920x1080 monitor at the origin.
    const PRIMARY: [(i32, i32, i32, i32); 1] = [(0, 0, 1920, 1080)];

    #[test]
    fn an_ordinary_position_is_reachable() {
        assert!(is_reachable(
            &WindowRect {
                x: 100,
                y: 100,
                width: 1100,
                height: 720
            },
            &PRIMARY
        ));
        // Flush to the top-left corner is fine.
        assert!(is_reachable(
            &WindowRect {
                x: 0,
                y: 0,
                width: 1100,
                height: 720
            },
            &PRIMARY
        ));
    }

    #[test]
    fn a_title_bar_above_the_screen_is_refused() {
        // Observed for real: a rect with y = -44 was persisted. The window
        // overlaps the monitor, so an overlap-only check restores it — and the
        // title bar is then unreachable with a mouse.
        assert!(!is_reachable(
            &WindowRect {
                x: 124,
                y: -44,
                width: 1650,
                height: 1050
            },
            &PRIMARY
        ));
        assert!(!is_reachable(
            &WindowRect {
                x: 100,
                y: -1,
                width: 1100,
                height: 720
            },
            &PRIMARY
        ));
    }

    #[test]
    fn a_window_dragged_mostly_off_the_side_is_refused() {
        // Only a sliver of title bar left to grab.
        assert!(!is_reachable(
            &WindowRect {
                x: 1880,
                y: 100,
                width: 1100,
                height: 720
            },
            &PRIMARY
        ));
        assert!(!is_reachable(
            &WindowRect {
                x: -1040,
                y: 100,
                width: 1100,
                height: 720
            },
            &PRIMARY
        ));
    }

    #[test]
    fn a_window_below_the_screen_is_refused() {
        assert!(!is_reachable(
            &WindowRect {
                x: 100,
                y: 1080,
                width: 1100,
                height: 720
            },
            &PRIMARY
        ));
    }

    #[test]
    fn a_second_monitor_above_the_primary_is_still_valid() {
        // A monitor arranged above the primary has negative coordinates, and a
        // window there is perfectly reachable. The rule is about the monitor's
        // own top edge, not about zero.
        let monitors = [(0, 0, 1920, 1080), (0, -1080, 1920, 0)];
        assert!(is_reachable(
            &WindowRect {
                x: 100,
                y: -1000,
                width: 1100,
                height: 720
            },
            &monitors
        ));
    }

    #[test]
    fn an_unplugged_second_monitor_is_refused() {
        assert!(!is_reachable(
            &WindowRect {
                x: 2200,
                y: 100,
                width: 1100,
                height: 720
            },
            &PRIMARY
        ));
    }

    #[test]
    fn rejects_the_sizes_a_half_created_window_reports() {
        // Observed in testing: a window mid-creation reported i16::MAX.
        assert!(!is_plausible(&rect(900, 32_767)));
        assert!(!is_plausible(&rect(0, 0)));
        assert!(!is_plausible(&rect(1100, 10)));
    }
}
