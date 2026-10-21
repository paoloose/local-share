use crate::app::state::{use_app_state, Modal};
use crate::server::api::delete_file;
use crate::shared::types::FileMeta;
use leptos::prelude::*;
use leptos::task::spawn_local;

#[component]
pub fn ConfirmModal() -> impl IntoView {
    let app = use_app_state();
    let target: RwSignal<Option<FileMeta>> = RwSignal::new(None);

    Effect::new(move |_| {
        if let Modal::ConfirmDelete(meta) = app.modal.get() {
            target.set(Some(meta));
        }
    });

    let confirm = move |_| {
        let Some(meta) = target.get() else {
            return;
        };
        let bucket = app.bucket.get();
        spawn_local(async move {
            match delete_file(bucket, meta.id).await {
                Ok(_) => app.modal.set(Modal::None),
                Err(err) => app.show_toast(err.to_string()),
            }
        });
    };

    view! {
        <div
            class="backdrop"
            class:open=move || matches!(app.modal.get(), Modal::ConfirmDelete(_))
        >
            <div class="modal">
                <h2>"Delete file?"</h2>
                <p class="muted">
                    {move || {
                        target
                            .get()
                            .map(|meta| {
                                format!(
                                    "\"{}\" will be gone for everyone in this bucket.",
                                    meta.name
                                )
                            })
                            .unwrap_or_default()
                    }}
                </p>
                <div class="modal-actions">
                    <button
                        class="btn btn-ghost"
                        type="button"
                        on:click=move |_| app.modal.set(Modal::None)
                    >
                        "Keep it"
                    </button>
                    <button class="btn btn-danger" type="button" on:click=confirm>
                        "Delete"
                    </button>
                </div>
            </div>
        </div>
    }
}
