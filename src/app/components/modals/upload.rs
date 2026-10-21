use crate::app::state::{use_app_state, AppState, Modal};
use crate::shared::types::format_size;
use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::JsCast;

/// One file waiting in the upload queue, kept as-is until sent so the queue
/// can show live status per file.
#[derive(Clone)]
pub struct QueuedFile {
    pub name: String,
    pub size: u64,
    pub content_type: String,
    pub status: RwSignal<String>,
    pub raw: web_sys::File,
}

fn enqueue(files: web_sys::FileList, queue: RwSignal<Vec<QueuedFile>>) {
    let mut added = Vec::new();
    for index in 0..files.length() {
        let Some(file) = files.get(index) else {
            continue;
        };
        let size = file.size() as u64;
        let content_type = if file.type_().is_empty() {
            "application/octet-stream".to_string()
        } else {
            file.type_()
        };
        added.push(QueuedFile {
            name: file.name(),
            size,
            content_type,
            status: RwSignal::new(format_size(size)),
            raw: file,
        });
    }
    queue.update(|queued| queued.extend(added));
}

/// Reads the file, gzips it and POSTs it to the upload endpoint, mirroring the
/// original app's compressed wire format.
async fn upload_one(app: AppState, item: QueuedFile) -> Result<(), String> {
    item.status.set("compressing...".to_string());
    let buffer = wasm_bindgen_futures::JsFuture::from(item.raw.array_buffer())
        .await
        .map_err(|err| format!("could not read file: {err:?}"))?;
    let bytes = js_sys::Uint8Array::new(&buffer).to_vec();
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    std::io::Write::write_all(&mut encoder, &bytes).map_err(|err| err.to_string())?;
    let compressed = encoder.finish().map_err(|err| err.to_string())?;

    item.status.set("uploading...".to_string());
    let body: wasm_bindgen::JsValue = js_sys::Uint8Array::from(&compressed[..]).into();
    let init = web_sys::RequestInit::new();
    init.set_method("POST");
    init.set_body(&body);
    let url = format!("/api/upload?bucket={}", app.bucket.get());
    let request =
        web_sys::Request::new_with_str_and_init(&url, &init).map_err(|err| format!("{err:?}"))?;
    let headers = request.headers();
    let encode = |value: &str| {
        percent_encoding::utf8_percent_encode(value, percent_encoding::NON_ALPHANUMERIC)
            .to_string()
    };
    headers
        .set("x-file-name", &encode(&item.name))
        .map_err(|err| format!("{err:?}"))?;
    headers
        .set("x-owner", &encode(&app.owner.get()))
        .map_err(|err| format!("{err:?}"))?;
    headers
        .set("x-content-type", &item.content_type)
        .map_err(|err| format!("{err:?}"))?;
    headers
        .set("x-encoding", "gzip")
        .map_err(|err| format!("{err:?}"))?;

    let window = web_sys::window().ok_or_else(|| "no window".to_string())?;
    let response = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|err| format!("{err:?}"))?;
    let response: web_sys::Response = response
        .dyn_into()
        .map_err(|_| "unexpected response".to_string())?;
    if response.ok() {
        Ok(())
    } else {
        Err(format!("server returned {}", response.status()))
    }
}

fn start_upload(app: AppState, queue: RwSignal<Vec<QueuedFile>>) {
    spawn_local(async move {
        for item in queue.get() {
            match upload_one(app, item.clone()).await {
                Ok(()) => {
                    queue.update(|queued| queued.retain(|entry| entry.raw != item.raw));
                }
                Err(err) => {
                    item.status.set("failed".to_string());
                    app.show_toast(format!("Upload failed for {}: {}", item.name, err));
                }
            }
        }
        if queue.get().is_empty() {
            app.modal.set(Modal::None);
        }
    });
}

#[component]
pub fn UploadModal() -> impl IntoView {
    let app = use_app_state();
    let queue: RwSignal<Vec<QueuedFile>> = RwSignal::new(Vec::new());
    let dragging = RwSignal::new(false);
    let browse_ref: NodeRef<leptos::html::Input> = NodeRef::new();

    Effect::new(move |_| {
        if matches!(app.modal.get(), Modal::Upload) {
            queue.set(Vec::new());
            dragging.set(false);
        }
    });

    view! {
        <div class="backdrop" class:open=move || matches!(app.modal.get(), Modal::Upload)>
            <div class="modal">
                <h2>"Drop files into " <span class="accent">{move || app.bucket.get()}</span></h2>
                <div
                    class="dropzone"
                    class:drag=move || dragging.get()
                    on:dragover=move |ev: web_sys::DragEvent| {
                        ev.prevent_default();
                        dragging.set(true);
                    }
                    on:dragleave=move |ev: web_sys::DragEvent| {
                        ev.prevent_default();
                        dragging.set(false);
                    }
                    on:drop=move |ev: web_sys::DragEvent| {
                        ev.prevent_default();
                        dragging.set(false);
                        if let Some(files) = ev.data_transfer().and_then(|data| data.files()) {
                            enqueue(files, queue);
                        }
                    }
                >
                    <div class="dropzone-icon"></div>
                    <p><strong>"Drag and drop"</strong> " files here"</p>
                    <p class="muted">"or"</p>
                    <button
                        class="btn btn-ghost"
                        type="button"
                        on:click=move |_| {
                            if let Some(input) = browse_ref.get() {
                                input.click();
                            }
                        }
                    >
                        "Browse files"
                    </button>
                    <input
                        class="hidden"
                        type="file"
                        multiple
                        node_ref=browse_ref
                        on:change:target=move |ev| {
                            if let Some(files) = ev.target().files() {
                                enqueue(files, queue);
                            }
                        }
                    />
                </div>
                <ul class="queue">
                    {move || {
                        queue
                            .get()
                            .into_iter()
                            .map(|item| {
                                let name = item.name.clone();
                                let status = item.status;
                                view! {
                                    <li>
                                        <span class="queue-name">{name}</span>
                                        <span class="queue-status">{move || status.get()}</span>
                                        <button
                                            type="button"
                                            title="Remove"
                                            on:click=move |_| {
                                                queue
                                                    .update(|queued| {
                                                        queued.retain(|entry| entry.raw != item.raw)
                                                    });
                                            }
                                        >
                                            "x"
                                        </button>
                                    </li>
                                }
                            })
                            .collect_view()
                    }}
                </ul>
                <div class="modal-actions">
                    <button
                        class="btn btn-ghost"
                        type="button"
                        on:click=move |_| app.modal.set(Modal::None)
                    >
                        "Close"
                    </button>
                    <button
                        class="btn btn-primary"
                        type="button"
                        disabled=move || queue.get().is_empty()
                        on:click=move |_| start_upload(app, queue)
                    >
                        "Upload"
                    </button>
                </div>
            </div>
        </div>
    }
}
