//! Server-side plumbing: service registry, blob transfer endpoints and the
//! hot-reload event bus. Only `api` (the Leptos server functions) compiles for
//! the browser target; everything else is server-only.

pub mod api;

#[cfg(feature = "ssr")]
pub mod download;

#[cfg(feature = "ssr")]
pub mod events;

#[cfg(feature = "ssr")]
pub mod services;
