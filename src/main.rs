use axum::extract::DefaultBodyLimit;
use axum::routing::{get, post};
use axum::Router;
use leptos::logging::log;
use leptos::config::get_configuration;
use leptos_axum::{generate_route_list, LeptosRoutes};
use paoloose_share::server::{download, events, services::AppServices};
use paoloose_share::{shell, App};

#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    let conf = get_configuration(None).unwrap();
    let leptos_options = conf.leptos_options;
    let addr = leptos_options.site_addr;

    let buckets_root = std::path::PathBuf::from(
        std::env::var("PAOLOOSE_BUCKETS_DIR").unwrap_or("buckets".into()),
    );
    let services = AppServices::init(buckets_root.clone());
    let _watcher = events::spawn_fs_watcher(buckets_root.clone(), services.events.clone())
        .expect("failed to watch the buckets directory");
    log!("watching buckets at {}", buckets_root.display());

    let routes = generate_route_list(App);

    let app = Router::new()
        .route("/api/upload", post(download::upload))
        .route("/api/files/{id}/download", get(download::download))
        .route("/api/events", get(events::sse))
        .leptos_routes(&leptos_options, routes, {
            let leptos_options = leptos_options.clone();
            move || shell(leptos_options.clone())
        })
        .fallback(leptos_axum::file_and_error_handler(shell))
        .layer(DefaultBodyLimit::max(256 * 1024 * 1024))
        .with_state(leptos_options);

    log!("paoloose share listening on http://{addr}");
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

#[cfg(not(feature = "ssr"))]
pub fn main() {}
