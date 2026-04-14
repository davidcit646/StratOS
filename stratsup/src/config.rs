use std::fs;
use std::path::{Path, PathBuf};

pub fn resolve_config_path(app_name: &str, config_file: &str) -> Option<PathBuf> {
    let user_path = PathBuf::from(format!("/config/apps/{}/{}", app_name, config_file));
    if Path::new(&user_path).exists() {
        return Some(user_path);
    }

    let system_path = PathBuf::from(format!("/system/etc/{}/{}", app_name, config_file));
    if Path::new(&system_path).exists() {
        return Some(system_path);
    }

    None
}

pub fn resolve_config_or_default(
    app_name: &str,
    config_file: &str,
    built_in_default: &str,
) -> String {
    if let Some(path) = resolve_config_path(app_name, config_file) {
        match fs::read_to_string(path) {
            Ok(contents) => return contents,
            Err(_) => return built_in_default.to_string(),
        }
    }

    built_in_default.to_string()
}

pub fn config_dir(app_name: &str) -> PathBuf {
    PathBuf::from(format!("/config/apps/{}", app_name))
}

pub fn ensure_config_dir(app_name: &str) -> Result<PathBuf, String> {
    let path = config_dir(app_name);
    fs::create_dir_all(&path)
        .map_err(|err| format!("config: failed to create config dir for {}: {}", app_name, err))?;
    Ok(path)
}
