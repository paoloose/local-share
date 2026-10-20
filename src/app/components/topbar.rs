use crate::app::state::{use_app_state, Modal};
use leptos::prelude::*;

#[component]
pub fn Topbar() -> impl IntoView {
    let app = use_app_state();
    let label = |value: String| {
        if value.is_empty() {
            "none".to_string()
        } else {
            value
        }
    };
    view! {
        <header class="topbar">
            <div class="brand">
                <span class="brand-mark">"p"</span>
                <span class="brand-name">"paoloose share"</span>
            </div>
            <div class="topbar-actions">
                <button
                    class="chip"
                    type="button"
                    title="Switch bucket"
                    on:click=move |_| app.modal.set(Modal::Bucket { cancellable: true })
                >
                    <span class="chip-label">"bucket:"</span>
                    <strong>{move || label(app.bucket.get())}</strong>
                </button>
                <button
                    class="chip"
                    type="button"
                    title="Change your name"
                    on:click=move |_| app.modal.set(Modal::Identity { cancellable: true })
                >
                    <span class="chip-label">"you:"</span>
                    <strong>{move || label(app.owner.get())}</strong>
                </button>
                <button
                    id="upload-btn"
                    class="btn btn-primary"
                    type="button"
                    on:click=move |_| app.modal.set(Modal::Upload)
                >
                    "+ Upload files"
                </button>
            </div>
        </header>
    }
}
