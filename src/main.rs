use serde_json::{self, Value};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;
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

        // Process themes
        let output_dir = Path::new("output");
        let mut successful_themes: HashMap<String, Vec<serde_json::Value>> = HashMap::new();

        for (extension, theme_families) in &themes_dirs {
            let mut extension_themes = Vec::new();
            for theme_family in theme_families {
                let theme_path = Path::new(extensions_path)
                    .join(extension)
                    .join("themes")
                    .join(theme_family);
                match process_theme(&theme_path, output_dir, extension) {
                    Ok(themes) => extension_themes.extend(themes),
                    Err(e) => eprintln!("Error processing theme {:?}: {}", theme_path, e),
                }
            }
            if !extension_themes.is_empty() {
                successful_themes.insert(extension.to_string(), extension_themes);
            }
        }

        // Generate the main themes.json file
        if let Err(e) = generate_main_themes_json(output_dir, &successful_themes) {
            eprintln!("Error generating main themes.json: {}", e);
        }
    } else {
        println!("The specified path does not exist or is not a directory.");
    }
}

fn process_theme(
    theme_path: &Path,
    output_dir: &Path,
    extension_name: &str,
) -> Result<Vec<serde_json::Value>, io::Error> {
    println!("Processing theme: {:?}", theme_path);

    // Read and parse the theme JSON file
    let theme_content = fs::read_to_string(theme_path)?;
    let theme_json: Value = serde_json::from_str(&theme_content)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    // Create extension directory
    let extension_dir = output_dir.join(extension_name);
    fs::create_dir_all(&extension_dir)?;

    let mut processed_themes = Vec::new();

    // Process each theme in the family
    if let Some(themes) = theme_json["themes"].as_array() {
        for theme in themes {
            let theme_name = theme["name"].as_str().unwrap_or("Unnamed Theme");
            let appearance = theme["appearance"].as_str().unwrap_or("unknown");
            let css_content = generate_css_from_theme(theme);

            // Write CSS file
            let css_file_name = format!("{}.css", theme_name.replace(" ", "_"));
            let css_file_path = extension_dir.join(&css_file_name);
            let mut css_file = File::create(&css_file_path)?;
            css_file.write_all(css_content.as_bytes())?;

            processed_themes.push(serde_json::json!({
                "name": theme_name,
                "appearance": appearance,
                "family": theme_path.file_name().unwrap().to_str().unwrap().replace(".json", ""),
                "css": format!("{}/{}", extension_name, css_file_name)
            }));
        }
    }

    // Generate index.css for the extension
    generate_extension_index_css(&extension_dir, &theme_json)?;

    Ok(processed_themes)
}

fn generate_css_from_theme(theme: &Value) -> String {
    let mut css = String::new();
    css.push_str(":root {\n");

    if let Some(style) = theme["style"].as_object() {
        for (key, value) in style {
            if key != "players" && key != "syntax" {
                css.push_str(&format!("  --{}: {};\n", key.replace(".", "-"), value));
            }
        }
    }

    // Process players
    if let Some(players) = theme["style"]["players"].as_array() {
        for (index, player) in players.iter().enumerate() {
            if let Some(player_obj) = player.as_object() {
                for (key, value) in player_obj {
                    css.push_str(&format!("  --player-{}-{}: {};\n", index + 1, key, value));
                }
            }
        }
    }

    // Process syntax
    if let Some(syntax) = theme["style"]["syntax"].as_object() {
        for (key, value) in syntax {
            if let Some(value_obj) = value.as_object() {
                for (sub_key, sub_value) in value_obj {
                    css.push_str(&format!(
                        "  --syntax-{}-{}: {};\n",
                        key.replace(".", "-"),
                        sub_key,
                        sub_value
                    ));
                }
            }
        }
    }

    css.push_str("}\n");
    css
}

fn generate_extension_index_css(extension_dir: &Path, theme_json: &Value) -> io::Result<()> {
    let mut index_content = String::new();

    if let Some(themes) = theme_json["themes"].as_array() {
        for theme in themes {
            let theme_name = theme["name"].as_str().unwrap_or("Unnamed Theme");
            index_content.push_str(&format!(
                "@import url(\"{}.css\");\n",
                theme_name.replace(" ", "_")
            ));
        }
    }

    let index_path = extension_dir.join("index.css");
    let mut index_file = File::create(index_path)?;
    index_file.write_all(index_content.as_bytes())?;

    Ok(())
}

fn generate_main_themes_json(
    output_dir: &Path,
    themes_dirs: &HashMap<String, Vec<serde_json::Value>>,
) -> io::Result<()> {
    let json_content = serde_json::to_string_pretty(&themes_dirs)?;
    let json_path = output_dir.join("themes.json");
    let mut json_file = File::create(json_path)?;
    json_file.write_all(json_content.as_bytes())?;

    Ok(())
}
