use anyhow::{bail, Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub struct FilesystemTool;

impl FilesystemTool {
    pub fn search_files<P: AsRef<Path>>(root: P, query: &str, max_results: usize) -> Result<Vec<PathBuf>> {
        let mut results = Vec::new();
        let q_lower = query.to_lowercase();

        for entry in WalkDir::new(root)
            .max_depth(8)
            .into_iter()
            .filter_entry(|e| {
                if e.depth() == 0 {
                    return true;
                }
                let name = e.file_name().to_string_lossy();
                !name.starts_with('.') && name != "target" && name != "node_modules"
            })
            .flatten()
        {
            let path = entry.path();
            if let Some(file_name) = path.file_name() {
                if file_name.to_string_lossy().to_lowercase().contains(&q_lower) {
                    results.push(path.to_path_buf());
                    if results.len() >= max_results {
                        break;
                    }
                }
            }
        }
        Ok(results)
    }

    pub fn read_file<P: AsRef<Path>>(path: P, max_lines: usize) -> Result<String> {
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read file {:?}", path.as_ref()))?;

        let lines: Vec<&str> = content.lines().take(max_lines).collect();
        let truncated = lines.len() < content.lines().count();
        let mut res = lines.join("\n");
        if truncated {
            res.push_str("\n... [Output truncated]");
        }
        Ok(res)
    }

    pub fn write_file<P: AsRef<Path>>(path: P, content: &str) -> Result<()> {
        if let Some(parent) = path.as_ref().parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, content)?;
        Ok(())
    }

    pub fn delete_file<P: AsRef<Path>>(path: P) -> Result<()> {
        if path.as_ref().is_dir() {
            fs::remove_dir_all(path)?;
        } else {
            fs::remove_file(path)?;
        }
        Ok(())
    }

    pub fn find_duplicates<P: AsRef<Path>>(dir: P) -> Result<Vec<(PathBuf, PathBuf)>> {
        let mut size_map: HashMap<u64, Vec<PathBuf>> = HashMap::new();
        for entry in WalkDir::new(dir)
            .max_depth(4)
            .into_iter()
            .flatten()
        {
            if entry.file_type().is_file() {
                if let Ok(meta) = entry.metadata() {
                    let len = meta.len();
                    if len > 0 {
                        size_map.entry(len).or_default().push(entry.path().to_path_buf());
                    }
                }
            }
        }

        let mut duplicates = Vec::new();
        for (_size, paths) in size_map {
            if paths.len() > 1 {
                // For paths with identical size, compare content
                for i in 0..paths.len() {
                    for j in (i + 1)..paths.len() {
                        if let (Ok(c1), Ok(c2)) = (fs::read(&paths[i]), fs::read(&paths[j])) {
                            if c1 == c2 {
                                duplicates.push((paths[i].clone(), paths[j].clone()));
                            }
                        }
                    }
                }
            }
        }
        Ok(duplicates)
    }

    pub fn organize_directory<P: AsRef<Path>>(dir: P) -> Result<String> {
        let dir = dir.as_ref();
        if !dir.is_dir() {
            bail!("{:?} is not a directory", dir);
        }

        let mut count = 0;
        for entry in fs::read_dir(dir)?.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    let sub_folder = match ext.to_lowercase().as_str() {
                        "pdf" | "doc" | "docx" | "txt" | "md" => "Documents",
                        "jpg" | "jpeg" | "png" | "webp" | "gif" | "svg" => "Images",
                        "mp4" | "mkv" | "mov" | "avi" => "Videos",
                        "mp3" | "wav" | "flac" | "ogg" => "Audio",
                        "zip" | "tar" | "gz" | "zst" | "7z" => "Archives",
                        "rs" | "py" | "js" | "ts" | "c" | "cpp" | "json" => "Code",
                        _ => "Misc",
                    };
                    let target_dir = dir.join(sub_folder);
                    fs::create_dir_all(&target_dir)?;
                    let target_file = target_dir.join(path.file_name().unwrap());
                    fs::rename(&path, &target_file)?;
                    count += 1;
                }
            }
        }
        Ok(format!("Organized {} files in {:?}", count, dir))
    }
}
