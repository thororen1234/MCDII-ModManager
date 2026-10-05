mod game;
mod intro;
mod mod_manager;
mod utils;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            mod_manager::load_config,
            mod_manager::save_config,
            mod_manager::detect_game,
            mod_manager::set_game_path,
            mod_manager::get_mods,
            mod_manager::set_mods_enabled,
            mod_manager::install_mod,
            mod_manager::delete_mod,
            mod_manager::rename_mod,
            mod_manager::open_mods_folder,
            mod_manager::open_game_folder,
            mod_manager::open_config_dir,
            mod_manager::open_themes_dir,
            mod_manager::list_themes,
            mod_manager::read_theme,
            mod_manager::launch_game,
            mod_manager::update_loader,
            mod_manager::check_loader_update,
            intro::intro_status,
            intro::apply_no_intro,
            intro::restore_intro,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
