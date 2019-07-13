//! This module provides context management for `RequestInfo` that is
//! similar to the one provided by `scoped_tls`, but it allows both
//! scoped and guarded modifications of the thread-local context.

use std::cell::RefCell;

use scoped_tls::scoped_thread_local;

use crate::payload::RequestInfo;

scoped_thread_local!(
    static SCOPED_CONTEXT: RequestInfo
);
thread_local! {
    static DEFAULT_CONTEXT: RefCell<Option<RequestInfo>> = RefCell::new(None);
}

pub fn get() -> Option<RequestInfo> {
    if SCOPED_CONTEXT.is_set() {
        SCOPED_CONTEXT.with(|r| Some(r.clone()))
    } else {
        DEFAULT_CONTEXT.with(|r| r.borrow().clone())
    }
}

pub fn with<R, F>(r: &RequestInfo, f: F) -> R
where
    F: FnOnce() -> R,
{
    SCOPED_CONTEXT.set(r, f)
}

pub fn set(r: RequestInfo) {
    DEFAULT_CONTEXT.with(|ctx| {
        let mut ctx = ctx.borrow_mut();
        *ctx = Some(r);
    });
}

pub fn unset() {
    DEFAULT_CONTEXT.with(|ctx| {
        let mut ctx = ctx.borrow_mut();
        *ctx = None;
    });
}
