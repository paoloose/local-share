use leptos::prelude::*;

pub fn empty_state() -> impl IntoView {
    view! {
        <div class="empty">
            <div class="empty-icon"></div>
            <h2>"No files dropped yet"</h2>
            <p>
                "Hit " <strong>"+ Upload files"</strong> " and drop anything into this bucket."
            </p>
        </div>
    }
}
