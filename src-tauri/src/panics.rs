//! Turning a panic into an error the user can read.
//!
//! An unwinding panic inside a command kills that command's future. Tauri
//! reports nothing useful and the window is left waiting on a promise that
//! will never settle — a button that does nothing, forever, with no
//! explanation. Catching it costs one wrapper and turns the worst failure mode
//! into an ordinary error dialog plus a log line with the backtrace.

use std::panic::{catch_unwind, AssertUnwindSafe};

use crate::error::AppError;

/// Run `work`, converting a panic into [`AppError::Internal`].
///
/// The payload is logged rather than shown: a panic message is written for
/// whoever wrote the code, not for whoever is using it.
pub fn guard<T>(what: &str, work: impl FnOnce() -> T) -> Result<T, AppError> {
    match catch_unwind(AssertUnwindSafe(work)) {
        Ok(value) => Ok(value),
        Err(payload) => {
            let detail = describe(&payload);
            tracing::error!(
                target: "app",
                operation = what,
                panic = %detail,
                backtrace = %std::backtrace::Backtrace::force_capture(),
                "a command panicked"
            );
            Err(AppError::Internal(format!("{what} panicked: {detail}")))
        }
    }
}

/// Pull a readable message out of a panic payload.
fn describe(payload: &Box<dyn std::any::Any + Send>) -> String {
    if let Some(text) = payload.downcast_ref::<&'static str>() {
        (*text).to_string()
    } else if let Some(text) = payload.downcast_ref::<String>() {
        text.clone()
    } else {
        "unknown payload".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn work_that_succeeds_passes_its_value_through() {
        assert_eq!(guard("adding", || 2 + 2).unwrap(), 4);
    }

    #[test]
    fn a_panic_becomes_an_internal_error() {
        let outcome = guard("exploding", || -> i32 { panic!("boom") });

        let err = outcome.expect_err("the panic should have been caught");
        assert!(matches!(err, AppError::Internal(_)));
        // The detail is for the log and Diagnostics, not for a dialog body.
        assert!(err.to_string().contains("internal"), "{err}");
    }

    #[test]
    fn the_payload_is_kept_for_the_log() {
        let err = guard("exploding", || -> i32 { panic!("a specific reason") })
            .expect_err("should have panicked");

        let AppError::Internal(detail) = err else {
            panic!("wrong variant");
        };
        assert!(detail.contains("a specific reason"), "{detail}");
        assert!(detail.contains("exploding"), "{detail}");
    }

    #[test]
    fn a_formatted_panic_message_is_kept_too() {
        let value = 42;
        let err = guard("exploding", || -> i32 { panic!("bad value: {value}") })
            .expect_err("should have panicked");

        let AppError::Internal(detail) = err else {
            panic!("wrong variant");
        };
        assert!(detail.contains("bad value: 42"), "{detail}");
    }

    #[test]
    fn a_panic_does_not_poison_later_calls() {
        // The point of catching: the next command still works.
        let _ = guard("first", || -> i32 { panic!("boom") });
        assert_eq!(guard("second", || "fine").unwrap(), "fine");
    }

    #[test]
    fn an_index_out_of_bounds_is_caught_like_any_other_panic() {
        let list: Vec<i32> = vec![];
        let outcome = guard("indexing", || list[3]);
        assert!(outcome.is_err());
    }
}
