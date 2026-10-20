use crate::app::state::{use_app_state, Modal};
use crate::shared::types::{format_size, format_when, FileMeta};
use leptos::prelude::*;

pub fn file_row(meta: FileMeta) -> impl IntoView {
    let app = use_app_state();

    let name = meta.name.clone();
    let title = meta.name.clone();
    let sub = format!("{} · {}", meta.owner, format_size(meta.size));
    let when = format_when(&meta.uploaded_at);
    let badge = meta.type_label();

    let download_name = meta.name.clone();
    let download_id = meta.id.clone();
    let rename_target = meta.clone();
    let delete_target = meta;

    view! {
        <article class="file-row">
            <div class="type-badge">{badge}</div>
            <div class="file-main">
                <div class="file-name" title=title>{name}</div>
                <div class="file-sub">
                    {sub}
                    <span class="dot">"·"</span>
                    {when}
                </div>
            </div>
            <div class="file-actions">
                <a
                    class="icon-btn"
                    href=move || format!("/api/files/{download_id}/download?bucket={}", app.bucket.get())
                    download=download_name.clone()
                >
                    "Download"
                </a>
                <button
                    class="icon-btn"
                    type="button"
                    on:click=move |_| app.modal.set(Modal::Rename(rename_target.clone()))
                >
                    "Rename"
                </button>
                <button
                    class="icon-btn danger"
                    type="button"
                    on:click=move |_| app.modal.set(Modal::ConfirmDelete(delete_target.clone()))
                >
                    "Delete"
                </button>
            </div>
        </article>
    }
}
