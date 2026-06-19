mod commands;
mod error;
mod session;
mod sftp;
mod ssh;

use session::SessionManager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(SessionManager::new())
        .invoke_handler(tauri::generate_handler![
            commands::ssh_connect,
            commands::ssh_send_input,
            commands::ssh_resize,
            commands::ssh_disconnect,
            commands::sftp_list,
            commands::sftp_upload,
            commands::sftp_download,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
