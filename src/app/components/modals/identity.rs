use crate::app::state::{use_app_state, Modal, OWNER_KEY};
use leptos::html::Input;
use leptos::prelude::*;

#[component]
pub fn IdentityModal() -> impl IntoView {
    let app = use_app_state();
    let (draft, set_draft) = signal(String::new());
    let error = RwSignal::new(String::new());
    let input_ref: NodeRef<Input> = NodeRef::new();

    Effect::new(move |_| {
        let Modal::Identity { .. } = app.modal.get() else {
            return;
        };
        set_draft.set(app.owner.get());
        error.set(String::new());
        if let Some(input) = input_ref.get() {
            let _ = input.focus();
            input.select();
        }
    });

    let submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        let name = draft.get().trim().to_string();
        if name.is_empty() {
            error.set("Type a name first.".to_string());
            return;
        }
        app.owner.set(name.clone());
        crate::app::state::storage_set(OWNER_KEY, &name);
        if app.bucket.get().is_empty() {
            app.modal.set(Modal::Bucket { cancellable: false });
        } else {
            app.modal.set(Modal::None);
        }
    };

    view! {
        <div class="backdrop" class:open=move || matches!(app.modal.get(), Modal::Identity { .. })>
            <form class="modal" autocomplete="off" on:submit=submit>
                <h2>"Who are you?"</h2>
                <p class="muted">
                    "No logins here. Tell the bucket who is dropping files. Impersonation is a feature."
                </p>
                <input
                    class="input"
                    type="text"
                    maxlength="64"
                    placeholder="Your display name"
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
                        class:hidden=move || {
                            !matches!(app.modal.get(), Modal::Identity { cancellable: true })
                        }
                        on:click=move |_| app.modal.set(Modal::None)
                    >
                        "Cancel"
                    </button>
                    <button id="identity-save" class="btn btn-primary" type="submit">
                        "That's me"
                    </button>
                </div>
            </form>
        </div>
    }
}
