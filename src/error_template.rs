use leptos::logging::log;
use leptos::prelude::*;

/// Renders a recoverable error inside an `<ErrorBoundary/>`, matching the
/// pattern from the Leptos axum examples.
pub fn error_view(errors: Errors) -> impl IntoView {
    log!("{errors:?}");
    view! {
        <div class="field-error">
            <h2>"Something went sideways"</h2>
            <ul>
                {errors
                    .into_iter()
                    .map(|(_, error)| {
                        let message = error.to_string();
                        view! { <li>{message}</li> }
                    })
                    .collect_view()}
            </ul>
        </div>
    }
}
