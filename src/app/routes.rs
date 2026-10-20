//! The bucket page: renders for `/` and `/{bucket}`, owns the file resource.

use crate::app::components::empty_state::empty_state;
use crate::app::components::file_list::file_list;
use crate::app::components::toolbar::Toolbar;
use crate::app::hooks::use_bucket_events;
use crate::app::state::{use_app_state, use_page_state};
use crate::server::api::list_files;
use crate::shared::naming::BucketName;
use crate::shared::types::{FileMeta, SortMode};
use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

#[component]
pub fn SharePage() -> impl IntoView {
    let app = use_app_state();
    let page = use_page_state();
    let params = use_params_map();

    // The URL wins over the stored default, on the server and on hydration.
    {
        let from_path = params.with_untracked(|params| params.get("bucket").unwrap_or_default());
        if let Ok(bucket) = BucketName::parse(&from_path) {
            app.bucket.set(bucket.to_string());
        }
    }

    #[cfg(target_arch = "wasm32")]
    {
        use leptos_router::hooks::use_navigate;
        let navigate = use_navigate();

        // Client-side navigation between / and /{bucket} re-syncs the same way.
        Effect::new(move |_| {
            let from_path = params.read().get("bucket").unwrap_or_default();
            if from_path.is_empty() {
                return;
            }
            if let Ok(bucket) = BucketName::parse(&from_path) {
                if app.bucket.get_untracked() != bucket.to_string() {
                    app.bucket.set(bucket.to_string());
                }
            }
        });

        // Switching buckets keeps the address bar shareable: /{bucket}.
        Effect::new(move |_| {
            let bucket = app.bucket.get();
            if bucket.is_empty() {
                return;
            }
            if params.read().get("bucket").as_deref() != Some(bucket.as_str()) {
                navigate(&format!("/{bucket}"), Default::default());
            }
        });
    }

    let files: Resource<Vec<FileMeta>> = Resource::new(
        move || app.bucket.get(),
        |bucket| async move {
            if bucket.is_empty() {
                return Vec::new();
            }
            list_files(bucket).await.unwrap_or_default()
        },
    );

    // Hot reload: SSE events for this bucket refetch the list.
    use_bucket_events(move || files.refetch());

    // Plain derived closure (not a Memo): memos evaluate eagerly at setup,
    // outside the Suspense boundary, while this stays lazy until rendered.
    let visible = move || {
        let query = page.query.get().to_lowercase();
        let mut files: Vec<FileMeta> = files
            .get()
            .unwrap_or_default()
            .into_iter()
            .filter(|meta| {
                query.is_empty()
                    || meta.name.to_lowercase().contains(&query)
                    || meta.owner.to_lowercase().contains(&query)
            })
            .collect();

        let by_name = |a: &FileMeta, b: &FileMeta| a.name.cmp(&b.name);
        let by_owner = |a: &FileMeta, b: &FileMeta| {
            a.owner
                .cmp(&b.owner)
                .then_with(|| b.uploaded_at.cmp(&a.uploaded_at))
        };
        match page.sort.get() {
            SortMode::DateDesc => files.sort_by(|a, b| b.uploaded_at.cmp(&a.uploaded_at)),
            SortMode::DateAsc => files.sort_by(|a, b| a.uploaded_at.cmp(&b.uploaded_at)),
            SortMode::Owner => files.sort_by(by_owner),
            SortMode::Name => files.sort_by(by_name),
            SortMode::Type => files.sort_by(|a, b| {
                a.type_label()
                    .cmp(&b.type_label())
                    .then_with(|| by_name(a, b))
            }),
        }
        files
    };

    view! {
        <main class="wrap">
            <Toolbar/>
            <section id="file-list" class="file-list" aria-live="polite">
                <Suspense fallback=|| {
                    view! { <div class="empty">"Loading..."</div> }
                }>
                    {move || {
                        let files = visible();
                        if !files.is_empty() {
                            return file_list(files).into_any();
                        }
                        if app.bucket.get().is_empty() {
                            view! {
                                <div class="empty">
                                    <div class="empty-icon"></div>
                                    <h2>"Pick a bucket"</h2>
                                    <p class="muted">
                                        "Open a bucket from the topbar to start dropping files."
                                    </p>
                                </div>
                            }
                            .into_any()
                        } else {
                            empty_state().into_any()
                        }
                    }}
                </Suspense>
            </section>
        </main>
    }
}
