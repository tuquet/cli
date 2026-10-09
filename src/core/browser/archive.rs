use std::path::{Path, PathBuf};

/// Get relative base folder where browser profiles are saved
pub async fn get_browsers_base_path(data_dir: &str) -> String {
    let settings_path = Path::new(data_dir).join("settings.json");
    if let Ok(content) = tokio::fs::read_to_string(&settings_path).await
        && let Ok(json) = serde_json::from_str::<serde_json::Value>(&content)
        && let Some(path) = json.get("browsersPath").and_then(|p| p.as_str()) {
            return path.to_string();
        }
    "browsers".to_string()
}

pub fn should_skip_zip_entry(name: &str, skip_folders: &[&str]) -> bool {
    skip_folders.iter().any(|skip| name.starts_with(skip))
}

pub fn append_entry_to_zip<W: std::io::Write + std::io::Seek>(
    zip: &mut zip::ZipWriter<W>,
    path: &Path,
    name: &str,
    options: zip::write::FileOptions,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if path.is_dir() {
        let _ = zip.add_directory(name, options);
    } else if path.is_file() {
        zip.start_file(name, options)?;
        if let Ok(mut f) = std::fs::File::open(path) {
            let _ = std::io::copy(&mut f, zip);
        }
    }
    Ok(())
}

/// Compress an inactive browser profile directory into an archive .zip
pub async fn zip_browser_folder(id: &str, data_dir: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let browsers_base_path = get_browsers_base_path(data_dir).await;
    
    let user_data_dir = PathBuf::from(data_dir)
        .join(&browsers_base_path)
        .join(id);

    let zip_path = PathBuf::from(data_dir)
        .join(&browsers_base_path)
        .join(format!("{}.zip", id));

    if !user_data_dir.exists() {
        return Ok(());
    }

    // Pre-sanitize profile before zipping: remove locks and volatile cache
    crate::core::browser::sanitize_browser_profile(&user_data_dir, false).await;

    let user_data_dir_clone = user_data_dir.clone();
    let zip_path_clone = zip_path.clone();
    
    tokio::task::spawn_blocking(move || {
        let file = std::fs::File::create(&zip_path_clone)?;
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::FileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .unix_permissions(0o755);

        // Note: NEVER skip "Network" as modern Chromium (96+) stores cookies in Default/Network/Cookies!
        let skip_folders = vec![
            "Cache",
            "Code Cache",
            "GPUCache",
            "DawnCache",
            "ShaderCache",
            "Crashpad",
            "Default/Cache",
            "Default/Code Cache",
            "Default/GPUCache",
            "Default/DawnCache",
            "Default/ShaderCache",
        ];

        let mut dirs = vec![user_data_dir_clone.clone()];
        while let Some(dir) = dirs.pop() {
            let Ok(entries) = std::fs::read_dir(&dir) else {
                continue;
            };

            for entry in entries.flatten() {
                let path = entry.path();
                let name = path.strip_prefix(&user_data_dir_clone)
                    .unwrap_or(path.as_path())
                    .to_string_lossy()
                    .replace("\\", "/");
                
                if should_skip_zip_entry(&name, &skip_folders) {
                    continue;
                }
                if entry.file_type().map(|t| t.is_symlink()).unwrap_or(false) {
                    continue;
                }

                if path.is_dir() {
                    dirs.push(path.clone());
                }
                let _ = append_entry_to_zip(&mut zip, &path, &name, options);
            }
        }
        zip.finish()?;
        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    }).await??;

    let _ = tokio::fs::remove_dir_all(user_data_dir).await;
    Ok(())
}

/// Restore a browser profile from a compressed .zip archive
pub async fn unzip_browser_folder(id: &str, data_dir: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let browsers_base_path = get_browsers_base_path(data_dir).await;
    
    let user_data_dir = PathBuf::from(data_dir)
        .join(&browsers_base_path)
        .join(id);

    let zip_path = PathBuf::from(data_dir)
        .join(&browsers_base_path)
        .join(format!("{}.zip", id));

    if !zip_path.exists() {
        return Ok(());
    }

    if user_data_dir.exists() {
        tracing::warn!("Browser folder already exists alongside zip. Skipping unzip to preserve crash state.");
        return Ok(());
    }

    let user_data_dir_clone = user_data_dir.clone();
    let zip_path_clone = zip_path.clone();
    
    tokio::task::spawn_blocking(move || {
        let file = std::fs::File::open(&zip_path_clone)?;
        let mut archive = zip::ZipArchive::new(file)?;
        
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let outpath = match file.enclosed_name() {
                Some(path) => user_data_dir_clone.join(path),
                None => continue,
            };

            if (*file.name()).ends_with('/') {
                std::fs::create_dir_all(&outpath)?;
            } else {
                if let Some(p) = outpath.parent()
                    && !p.exists() {
                        std::fs::create_dir_all(p)?;
                    }
                let mut outfile = std::fs::File::create(&outpath)?;
                std::io::copy(&mut file, &mut outfile)?;
            }
        }
        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    }).await??;

    let _ = tokio::fs::remove_file(zip_path).await;
    Ok(())
}
