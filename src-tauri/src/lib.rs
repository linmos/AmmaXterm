mod commands;
mod error;
mod importer;
mod keygen;
mod secrets;
mod session;
mod settings;
mod sftp;
mod ssh;
mod store;
mod transfer;
mod tunnel;
mod vault;

use session::SessionManager;
use settings::SettingsStore;
use store::SiteStore;
use tauri::Manager;
use transfer::TransferManager;
use tunnel::TunnelManager;
use vault::VaultState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .manage(SessionManager::new())
        .manage(TunnelManager::new())
        .manage(TransferManager::new())
        .manage(VaultState::default())
        .manage(ssh::HostKeyPrompts::default())
        .setup(|app| {
            // Sites + settings live under the app config dir.
            let config_dir = app.path().app_config_dir()?;
            app.manage(SiteStore::load(config_dir.join("sites.json")));
            app.manage(SettingsStore::load(config_dir.join("settings.json")));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::ssh_connect,
            commands::site_connect,
            commands::ssh_send_input,
            commands::ssh_resize,
            commands::ssh_disconnect,
            commands::session_start_log,
            commands::session_stop_log,
            commands::host_key_decision,
            commands::sftp_list,
            commands::sftp_upload,
            commands::sftp_download,
            commands::sftp_mkdir,
            commands::sftp_rename,
            commands::sftp_delete,
            commands::sftp_chmod,
            commands::site_list,
            commands::site_add,
            commands::site_update,
            commands::site_delete,
            commands::site_copy_secrets,
            commands::site_set_password,
            commands::site_set_passphrase,
            commands::import_ssh_config,
            commands::import_sites_backup,
            commands::import_putty_registry,
            commands::import_putty_reg,
            commands::export_sites,
            commands::settings_get,
            commands::settings_set,
            commands::tunnel_open,
            commands::tunnel_close,
            commands::tunnel_list,
            commands::transfer_upload,
            commands::transfer_download,
            commands::transfer_list,
            commands::transfer_cancel,
            commands::transfer_pause,
            commands::transfer_resume,
            commands::transfer_retry,
            commands::transfer_clear,
            commands::local_list,
            commands::keygen_generate,
            commands::keygen_save,
            commands::vault_status,
            commands::vault_unlock,
            commands::vault_lock,
            commands::vault_set_secret,
            commands::vault_get_secret,
            commands::vault_delete_secret,
            commands::vault_keys,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
