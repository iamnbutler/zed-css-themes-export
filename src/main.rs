use std::collections::HashMap;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

fn main() {
    let extensions_path = "../extensions/extensions";
    let path = Path::new(extensions_path);

    if path.exists() && path.is_dir() {
        // Initialize a HashMap to store directories containing a "themes" folder
        let mut themes_dirs = HashMap::new();

        // Walk through the directory structure
        for entry in WalkDir::new(extensions_path)
            .min_depth(1)
            .max_depth(2) // Limit the search to immediate children
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_dir() {
                // Get the directory path
                let dir_path = entry.path();
                if dir_path.ends_with("themes") {
                    // Get the parent directory
                    if let Some(parent_dir) = dir_path.parent().and_then(|p| p.file_name()) {
                        // Insert into HashMap filtered by folders that have a `themes` directory
                        themes_dirs.insert(
                            parent_dir.to_string_lossy().to_string(),
                            dir_path.to_path_buf(),
                        );
                    }
                }
            }
        }

        // Print the results
        for (dir, themes_path) in &themes_dirs {
            println!("Directory: {}, Themes path: {:?}", dir, themes_path);
        }
    } else {
        println!("The specified path does not exist or is not a directory.");
    }
}
