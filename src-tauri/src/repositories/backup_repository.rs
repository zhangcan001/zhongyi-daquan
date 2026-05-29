use crate::errors::{AppError, AppResult};
use std::fs;
use std::path::{Path, PathBuf};

pub fn ensure_backup_dir(data_dir: &Path, backup_id: &str) -> AppResult<PathBuf> {
    let dir = data_dir.join("backups").join(backup_id);
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn copy_dir_if_exists(source: &Path, destination: &Path) -> AppResult<bool> {
    if !source.exists() {
        return Ok(false);
    }
    if destination.exists() {
        fs::remove_dir_all(destination)?;
    }
    copy_dir_recursive(source, destination)?;
    Ok(true)
}

pub fn replace_dir_if_exists(source: &Path, destination: &Path) -> AppResult<bool> {
    if !source.exists() {
        return Ok(false);
    }
    if destination.exists() {
        fs::remove_dir_all(destination)?;
    }
    copy_dir_recursive(source, destination)?;
    Ok(true)
}

pub fn validate_backup_dir(backup_dir: &Path) -> AppResult<()> {
    let manifest = backup_dir.join("backup_manifest.json");
    let database = backup_dir.join("database").join("zhongyi.db");
    if !manifest.exists() {
        return Err(AppError::InvalidInput(
            "备份目录缺少 backup_manifest.json".to_string(),
        ));
    }
    if !database.exists() {
        return Err(AppError::InvalidInput(
            "备份目录缺少 database/zhongyi.db".to_string(),
        ));
    }
    Ok(())
}

fn copy_dir_recursive(source: &Path, destination: &Path) -> AppResult<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            copy_dir_recursive(&source_path, &destination_path)?;
        } else if file_type.is_file() {
            if let Some(parent) = destination_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&source_path, &destination_path)?;
        }
    }
    Ok(())
}
