use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

fn main() {
    let extensions_path = "../extensions/extensions";
    let path = Path::new(extensions_path);

    if path.exists() && path.is_dir() {
        // Initialize a HashMap to store theme families for each extension
        let mut themes_dirs: HashMap<String, Vec<String>> = HashMap::new();

        // Walk through the directory structure
        for entry in WalkDir::new(extensions_path)
            .min_depth(1)
            .max_depth(1) // Limit the search to immediate children of the "extensions" directory
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_dir() {
                let dir_path = entry.path().join("themes");
                if dir_path.exists() && dir_path.is_dir() {
                    if let Some(extension_name) = entry.file_name().to_str() {
                        // Collect theme family names
                        let theme_families: Vec<String> = fs::read_dir(&dir_path)
                            .unwrap()
                            .filter_map(|entry| entry.ok())
                            .filter(|entry| {
                                entry.path().is_file()
                                    && entry.path().extension()
                                        == Some(std::ffi::OsStr::new("json"))
                            })
                            .filter_map(|entry| entry.file_name().into_string().ok())
                            .collect();

                        // Insert into the HashMap
                        themes_dirs.insert(extension_name.to_string(), theme_families);
                    }
                }
            }
        }

        // Print the results
        for (dir, theme_families) in &themes_dirs {
            println!("Extension: {}, Theme Families: {:?}", dir, theme_families);
        }
    } else {
        println!("The specified path does not exist or is not a directory.");
    }
}
