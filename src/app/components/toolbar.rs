use crate::app::state::use_page_state;
use crate::shared::types::SortMode;
use leptos::prelude::*;

#[component]
pub fn Toolbar() -> impl IntoView {
    let page = use_page_state();
    view! {
        <section class="toolbar">
            <input
                id="search"
                class="search"
                type="search"
                placeholder="Search files or owners..."
                prop:value=move || page.query.get()
                on:input:target=move |ev| page.query.set(ev.target().value())
            />
            <select
                id="sort"
                class="select"
                on:change:target=move |ev| {
                    let mode = match ev.target().value().as_str() {
                        "date-asc" => SortMode::DateAsc,
                        "owner" => SortMode::Owner,
                        "name" => SortMode::Name,
                        "type" => SortMode::Type,
                        _ => SortMode::DateDesc,
                    };
                    page.sort.set(mode);
                }
            >
                <option value="date-desc">"Newest first"</option>
                <option value="date-asc">"Oldest first"</option>
                <option value="owner">"Owner (A-Z)"</option>
                <option value="name">"Name (A-Z)"</option>
                <option value="type">"Type"</option>
            </select>
        </section>
    }
}
