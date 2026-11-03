//! Gamify on/off and mode toggles. Host-side: they read and write this machine's Vox config
//! file, not the workspace database.

#[derive(Debug, serde::Serialize)]
pub struct GamifySettingsDto {
    pub enabled: bool,
    pub mode: String,
}

#[tauri::command]
pub async fn get_gamify_settings() -> Result<GamifySettingsDto, String> {
    let cfg = vox_gamify::config_gate::load_disk();
    Ok(GamifySettingsDto {
        enabled: cfg.gamify_enabled,
        mode: cfg.gamify_mode.as_config_str().to_string(),
    })
}

#[tauri::command]
pub async fn set_gamify_settings(enabled: bool, mode: String) -> Result<(), String> {
    let mut cfg = vox_gamify::config_gate::load_disk();
    cfg.gamify_enabled = enabled;
    cfg.gamify_mode = match mode.to_lowercase().as_str() {
        "serious" => vox_config::GamifyMode::Serious,
        "learning" => vox_config::GamifyMode::Learning,
        _ => vox_config::GamifyMode::Balanced,
    };
    cfg.save().map_err(|e| format!("save config: {e}"))?;
    Ok(())
}
