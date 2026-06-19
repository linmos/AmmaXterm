mod commands;
mod error;
mod importer;
mod secrets;
mod session;
mod sftp;
mod ssh;
mod store;

use session::SessionManager;
use store::SiteStore;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(SessionManager::new())
        .manage(ssh::HostKeyPrompts::default())
        .setup(|app| {
            // Sites live in sites.json under the app config dir.
            let config_dir = app.path().app_config_dir()?;
            app.manage(SiteStore::load(config_dir.join("sites.json")));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::ssh_connect,
            commands::site_connect,
            commands::ssh_send_input,
            commands::ssh_resize,
            commands::ssh_disconnect,
            commands::host_key_decision,
            commands::sftp_list,
            commands::sftp_upload,
            commands::sftp_download,
            commands::sftp_mkdir,
            commands::sftp_rename,
            commands::sftp_delete,
            commands::site_list,
            commands::site_add,
            commands::site_update,
            commands::site_delete,
            commands::site_set_password,
            commands::site_set_passphrase,
            commands::import_ssh_config,
            commands::import_sites_backup,
            commands::export_sites,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
