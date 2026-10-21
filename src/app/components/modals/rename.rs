use crate::app::state::{use_app_state, Modal};
use crate::server::api::rename_file;
use crate::shared::types::FileMeta;
use leptos::html::Input;
use leptos::prelude::*;
use leptos::task::spawn_local;

#[component]
pub fn RenameModal() -> impl IntoView {
    let app = use_app_state();
    let (draft, set_draft) = signal(String::new());
    let error = RwSignal::new(String::new());
    let target: RwSignal<Option<FileMeta>> = RwSignal::new(None);
    let input_ref: NodeRef<Input> = NodeRef::new();

    Effect::new(move |_| {
        let Modal::Rename(meta) = app.modal.get() else {
            return;
        };
        set_draft.set(meta.name.clone());
        target.set(Some(meta));
        error.set(String::new());
        if let Some(input) = input_ref.get() {
            let _ = input.focus();
            input.select();
        }
    });

    let submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        let Some(meta) = target.get() else {
            return;
        };
        let name = draft.get().trim().to_string();
        if name.is_empty() {
            error.set("Type a new name first.".to_string());
            return;
        }
        let bucket = app.bucket.get();
        spawn_local(async move {
            match rename_file(bucket, meta.id, name).await {
                Ok(_) => app.modal.set(Modal::None),
                Err(err) => error.set(err.to_string()),
            }
        });
    };

    view! {
        <div class="backdrop" class:open=move || matches!(app.modal.get(), Modal::Rename(_))>
            <form class="modal" autocomplete="off" on:submit=submit>
                <h2>"Rename file"</h2>
                <input
                    class="input"
                    type="text"
                    maxlength="255"
                    node_ref=input_ref
                    prop:value=move || draft.get()
                    on:input:target=move |ev| set_draft.set(ev.target().value())
                />
                <p class="field-error" class:hidden=move || error.get().is_empty()>
                    {move || error.get()}
                </p>
                <div class="modal-actions">
                    <button
                        class="btn btn-ghost"
                        type="button"
                        on:click=move |_| app.modal.set(Modal::None)
                    >
                        "Cancel"
                    </button>
                    <button class="btn btn-primary" type="submit">"Save"</button>
                </div>
            </form>
        </div>
    }
}
