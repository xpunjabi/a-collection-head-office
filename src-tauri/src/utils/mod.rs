use std::path::PathBuf;
use std::fs;

pub fn get_app_dir() -> PathBuf {
    let mut base = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    base.push("com.airdropia.collectionheadoffice");
    // Ensure app directory exists
    let _ = fs::create_dir_all(&base);
    base
}

pub fn get_db_path() -> PathBuf {
    let mut dir = get_app_dir();
    dir.push("database.db");
    dir
}

pub fn get_images_dir() -> PathBuf {
    let mut dir = get_app_dir();
    dir.push("images");
    let _ = fs::create_dir_all(&dir);
    dir
}
