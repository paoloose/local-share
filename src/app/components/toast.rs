use crate::app::state::use_app_state;
use leptos::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

#[component]
pub fn Toast() -> impl IntoView {
    let app = use_app_state();

    // Toasts expire after five seconds, like the original app.
    #[cfg(target_arch = "wasm32")]
    Effect::new(move |_| {
        let message = app.toast.get();
        if message.is_empty() {
            return;
        }
        let clear = wasm_bindgen::closure::Closure::once(move || app.clear_toast());
        let scheduled = web_sys::window()
            .and_then(|window| {
                window
                    .set_timeout_with_callback_and_timeout_and_arguments_0(
                        clear.as_ref().unchecked_ref(),
                        5000,
                    )
                    .ok()
            })
            .is_some();
        if scheduled {
            clear.forget();
        }
    });

    view! {
        <div id="toast" class="toast" role="alert" class:hidden=move || app.toast.get().is_empty()>
            {move || app.toast.get()}
        </div>
    }
}
