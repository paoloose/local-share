use crate::app::components::file_row::file_row;
use crate::shared::types::FileMeta;
use leptos::prelude::*;

pub fn file_list(files: Vec<FileMeta>) -> impl IntoView {
    files.into_iter().map(file_row).collect_view()
}
