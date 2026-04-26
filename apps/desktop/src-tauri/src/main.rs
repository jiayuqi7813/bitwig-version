mod commands;

use api::ApiState;

#[cfg(feature = "with-tauri")]
fn main() {
    let api_state = ApiState::default();
    let api_state_clone = api_state.clone();
    tauri::async_runtime::spawn(async move {
        let _ = api::run_server(api_state_clone).await;
    });

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::check_dependencies,
            commands::bind_project,
            commands::set_remote,
            commands::get_status,
            commands::import_latest_dawproject,
            commands::save_version,
            commands::get_history,
            commands::compare_snapshots,
            commands::restore_snapshot,
            commands::git_push,
            commands::git_pull,
            commands::open_folder,
        ])
        .manage(api_state)
        .run(tauri::generate_context!())
        .expect("failed to run tauri app");
}

#[cfg(not(feature = "with-tauri"))]
#[tokio::main]
async fn main() {
    // Non-Tauri fallback so CI can compile crate without native toolkit.
    let state = ApiState::default();
    let _ = api::run_server(state).await;
}
