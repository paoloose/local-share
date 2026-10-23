# paoloose share

A shared bucket for dropping files, rewritten as a fullstack Rust app with
[Leptos](https://leptos.dev) (SSR + hydration) and Axum. No logins, no auth.
Every bucket is just a folder on disk and everyone is admin. This is a POC.

The UI, behavior and on-disk data layout are the same as the original Bun app
(kept for reference in `legacy/`), plus live file-list updates over SSE.

## Run

```sh
cargo install cargo-leptos --locked   # once
cargo leptos watch                    # dev, hot reload
```

Open http://localhost:7777.

Release build:

```sh
cargo leptos build --release
# server binary in target/server/release, site assets in target/site
```

Tests:

```sh
cargo test --features ssr
```

## How it works

- First visit prompts for your display name (stored in `localStorage`) and a
  bucket name. A bucket is a folder under `buckets/` (override with
  `PAOLOOSE_BUCKETS_DIR`), created on first write: listing a bucket that does
  not exist simply shows an empty list.
- The bucket prompt accepts a plain name or a link like
  `https://drive.paoloose.site/andes`, and resolves it to `andes`. The address
  bar mirrors the current bucket (`/andes` deep links boot straight into it).
- Files are gzipped in the browser before upload and gzipped again on
  download, so transfers stay small.
- The file list shows owner, type, size and date, sortable by date, owner,
  name or type, with search. Everyone can rename, download or delete anything.
- **Hot reload**: every mutation (and every out-of-band change on disk, via a
  `notify` watcher) is published on an event bus and streamed to open clients
  over server-sent events (`/api/events?bucket=...`), so lists update live
  without refresh.

## Architecture

| Module | What lives there |
| --- | --- |
| `src/shared/` | Isomorphic types (`FileMeta`, `FileEvent`, `SortMode`) and bucket naming rules (URL normalization) |
| `src/storage/` | `BucketStore` trait + `FsStore`: `buckets/<name>/meta.json` manifest and `buckets/<name>/files/<id>` blobs, atomic writes |
| `src/server/` | Leptos server functions (metadata API), axum blob endpoints, event bus + SSE + fs watcher, service registry |
| `src/app/` | UI: state, page, components, modals, the SSE subscription hook |
| `src/shell.rs`, `src/main.rs` | SSR document shell and the axum server wiring (port 7777) |
| `style/main.css` | The stylesheet, processed by cargo-leptos into `pkg/` |

Binary transfers are plain axum routes (scriptable, same wire protocol as the
original app); metadata operations are Leptos server functions.

See [ARCHITECTURE.md](ARCHITECTURE.md) for the full module map, data layout,
event flow and project state.

## API

| Method | Path | Notes |
| --- | --- | --- |
| `GET` | `/{bucket}` | The app, preselected to that bucket |
| `GET` | `/api/events?bucket=` | SSE stream of file events (hot reload) |
| `POST` | `/api/upload?bucket=` | Gzip body, `x-file-name` / `x-owner` / `x-content-type` headers |
| `GET` | `/api/files/{id}/download?bucket=` | Download (gzip-encoded when accepted) |
| server fns | `/api/list_files`, `/api/list_buckets`, `/api/rename_file`, `/api/delete_file` | Called from the UI as plain async functions |

There is no auth anywhere: impersonation is fully supported. It's a POC.
