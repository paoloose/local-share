#[cfg(target_arch = "wasm32")]
use crate::app::state::use_app_state;
#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;
#[cfg(target_arch = "wasm32")]
use std::cell::RefCell;
#[cfg(target_arch = "wasm32")]
use std::rc::Rc;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

/// Subscribes to the bucket's SSE stream and calls `on_change` for every
/// event burst, so the file list hot reloads without a refresh.
///
/// Events arrive in bursts (one mutation produces a few), so callbacks are
/// debounced into a single refresh: overlapping resource refetches would
/// otherwise leave the list stuck behind its Suspense fallback.
pub fn use_bucket_events(on_change: impl Fn() + Clone + 'static) {
    #[cfg(target_arch = "wasm32")]
    {
        let app = use_app_state();
        Effect::new(move |_| {
            let bucket = app.bucket.get();
            if bucket.is_empty() {
                return;
            }
            let Ok(source) = web_sys::EventSource::new(&format!("/api/events?bucket={bucket}"))
            else {
                return;
            };

            let timer: Rc<RefCell<Option<i32>>> = Rc::new(RefCell::new(None));
            let handler = Rc::new(on_change.clone());
            let on_message = wasm_bindgen::closure::Closure::<dyn Fn(web_sys::MessageEvent)>::new(
                move |_event: web_sys::MessageEvent| {
                    let Some(window) = web_sys::window() else {
                        return;
                    };
                    if let Some(previous) = timer.borrow_mut().take() {
                        let _ = window.clear_timeout_with_handle(previous);
                    }
                    let handler = Rc::clone(&handler);
                    let scheduled =
                        wasm_bindgen::closure::Closure::once_into_js(move || (*handler)());
                    if let Ok(id) = window
                        .set_timeout_with_callback_and_timeout_and_arguments_0(
                            scheduled.unchecked_ref(),
                            250,
                        )
                    {
                        *timer.borrow_mut() = Some(id);
                    }
                },
            );
            // Keep the closure alive for the page lifetime (bucket switches are
            // rare); the source itself is closed on cleanup below.
            let callback: js_sys::Function = on_message
                .as_ref()
                .unchecked_ref::<js_sys::Function>()
                .clone();
            on_message.forget();
            let _ = source.add_event_listener_with_callback("message", &callback);

            on_cleanup(move || {
                let _ = source.remove_event_listener_with_callback("message", &callback);
                source.close();
            });
        });
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = on_change;
}
