use crate::app::state::{use_app_state, Modal, BUCKET_KEY};
use crate::server::api::list_buckets;
use crate::shared::naming::{bucket_from_input, normalize_bucket_input};
use leptos::html::Input;
use leptos::prelude::*;
use leptos_router::hooks::use_navigate;

#[component]
pub fn BucketModal() -> impl IntoView {
    let app = use_app_state();
    let navigate = use_navigate();
    let (draft, set_draft) = signal(String::new());
    let error = RwSignal::new(String::new());
    let input_ref: NodeRef<Input> = NodeRef::new();

    // Existing buckets, refreshed each time the prompt opens.
    let buckets: Resource<Vec<String>> = Resource::new(
        move || matches!(app.modal.get(), Modal::Bucket { .. }),
        |open| async move {
            if !open {
                return Vec::new();
            }
            list_buckets().await.unwrap_or_default()
        },
    );

    Effect::new(move |_| {
        let Modal::Bucket { .. } = app.modal.get() else {
            return;
        };
        set_draft.set(app.bucket.get());
        error.set(String::new());
        if let Some(input) = input_ref.get() {
            let _ = input.focus();
            input.select();
        }
    });

    let submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        let normalized = normalize_bucket_input(&draft.get());
        match bucket_from_input(&draft.get()) {
            Ok(bucket) => {
                let name = bucket.to_string();
                set_draft.set(normalized);
                app.bucket.set(name.clone());
                crate::app::state::storage_set(BUCKET_KEY, &name);
                app.modal.set(Modal::None);
                navigate(&format!("/{name}"), Default::default());
            }
            Err(err) => error.set(err.to_string()),
        }
    };

    view! {
        <div class="backdrop" class:open=move || matches!(app.modal.get(), Modal::Bucket { .. })>
            <form class="modal" autocomplete="off" on:submit=submit>
                <h2>"Open a bucket"</h2>
                <p class="muted">
                    "A bucket is just a folder name on disk. No auth. Type a name, a link like "
                    <strong>"https://drive.paoloose.site/andes"</strong>
                    ", or pick an existing one."
                </p>
                <input
                    class="input"
                    type="text"
                    maxlength="255"
                    placeholder="e.g. team-dump"
                    node_ref=input_ref
                    prop:value=move || draft.get()
                    on:input:target=move |ev| set_draft.set(ev.target().value())
                />
                <p class="field-error" class:hidden=move || error.get().is_empty()>
                    {move || error.get()}
                </p>
                <div class="bucket-list">
                    <Suspense fallback=|| ()>
                        {move || {
                            buckets
                                .get()
                                .unwrap_or_default()
                                .into_iter()
                                .map(|name| {
                                    let pill_name = name.clone();
                                    view! {
                                        <button
                                            class="bucket-pill"
                                            type="button"
                                            on:click=move |_| {
                                                set_draft.set(pill_name.clone());
                                            }
                                        >
                                            {name}
                                        </button>
                                    }
                                })
                                .collect_view()
                        }}
                    </Suspense>
                </div>
                <div class="modal-actions">
                    <button
                        class="btn btn-ghost"
                        type="button"
                        class:hidden=move || {
                            !matches!(app.modal.get(), Modal::Bucket { cancellable: true })
                        }
                        on:click=move |_| app.modal.set(Modal::None)
                    >
                        "Cancel"
                    </button>
                    <button id="bucket-save" class="btn btn-primary" type="submit">
                        "Open bucket"
                    </button>
                </div>
            </form>
        </div>
    }
}
