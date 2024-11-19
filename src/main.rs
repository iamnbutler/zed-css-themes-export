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
        let mut themes_dirs: HashMap<String, Vec<String>> = HashMap::new();

        for entry in WalkDir::new(extensions_path)
            .min_depth(1)
            .max_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_dir() {
                let dir_path = entry.path().join("themes");
                if dir_path.exists() && dir_path.is_dir() {
                    if let Some(extension_name) = entry.file_name().to_str() {
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

                        themes_dirs.insert(extension_name.to_string(), theme_families);
                    }
                }
            }
        }

        let output_dir = Path::new("output");
        let mut successful_themes: HashMap<String, Vec<serde_json::Value>> = HashMap::new();
        let mut master_index_content = String::new();

        for (extension, theme_families) in &themes_dirs {
            let mut extension_themes = Vec::new();
            for theme_family in theme_families {
                let theme_path = Path::new(extensions_path)
                    .join(extension)
                    .join("themes")
                    .join(theme_family);
                match process_theme(&theme_path, output_dir, extension) {
                    Ok(themes) => {
                        for theme in &themes {
                            if let Some(css_path) = theme["css"].as_str() {
                                master_index_content
                                    .push_str(&format!("@import url(\"{}\");\n", css_path));
                            }
                        }
                        extension_themes.extend(themes);
                    }
                    Err(e) => eprintln!("Error processing theme {:?}: {}", theme_path, e),
                }
            }
            if !extension_themes.is_empty() {
                successful_themes.insert(extension.clone(), extension_themes);
            }
        }

        let master_index_path = output_dir.join("index.css");
        if let Err(e) = fs::write(master_index_path, master_index_content) {
            eprintln!("Error writing master index.css: {}", e);
        }

        if let Err(e) = generate_main_themes_json(output_dir, &successful_themes) {
            eprintln!("Error generating main themes.json: {}", e);
        }
    } else {
        eprintln!("The specified path does not exist or is not a directory.");
    }
}

fn process_theme(
    theme_path: &Path,
    output_dir: &Path,
    extension_name: &str,
) -> Result<Vec<serde_json::Value>, io::Error> {
    let theme_content = fs::read_to_string(theme_path)?;
    let theme_json: Value = match serde_json::from_str(&theme_content) {
        Ok(json) => json,
        Err(e) => {
            eprintln!("Skipping malformed theme {:?}: {}", theme_path, e);
            return Ok(Vec::new());
        }
    };

    let default_content = fs::read_to_string("src/default.json")?;
    let default_json: Value = serde_json::from_str(&default_content)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    let extension_dir = output_dir.join(to_snake_case(extension_name));
    fs::create_dir_all(&extension_dir)?;

    let mut processed_themes = Vec::new();

    if let Some(themes) = theme_json["themes"].as_array() {
        for theme in themes {
            let theme_name = theme["name"].as_str().unwrap_or("Unnamed Theme");
            let appearance = theme["appearance"].as_str().unwrap_or("unknown");
            let css_content = generate_merged_css(&default_json, theme);

            let css_file_name = format!("{}.css", to_snake_case(theme_name));
            let css_file_path = extension_dir.join(&css_file_name);
            let mut css_file = File::create(&css_file_path)?;
            css_file.write_all(css_content.as_bytes())?;

            let relative_path = format!("{}/{}", to_snake_case(extension_name), css_file_name);

            processed_themes.push(serde_json::json!({
                "name": theme_name,
                "appearance": to_snake_case(appearance),
                "family": theme_path.file_name().unwrap().to_str().unwrap().replace(".json", ""),
                "css": relative_path
            }));
        }
    }

    if !processed_themes.is_empty() {
        generate_extension_index_css(&extension_dir, &theme_json)?;
    }

    Ok(processed_themes)
}

fn generate_merged_css(default: &Value, theme: &Value) -> String {
    let theme_name = theme["name"].as_str().unwrap_or("unnamed");
    let mut css = format!("body.theme.{} {{\n", to_snake_case(theme_name));

    let default_style = default["themes"]
        .as_array()
        .and_then(|themes| themes.first())
        .and_then(|theme| theme.get("style"))
        .and_then(Value::as_object);
    let theme_style = theme.get("style").and_then(Value::as_object);

    if let (Some(default_style), Some(theme_style)) = (default_style, theme_style) {
        for (key, default_value) in default_style {
            if key != "players" && key != "syntax" {
                let final_value = theme_style.get(key).unwrap_or(default_value);
                let value_str = match final_value {
                    Value::String(s) => format!("\"{}\"", s),
                    _ => final_value.to_string(),
                };
                if !value_str.is_empty() && value_str != "null" {
                    css.push_str(&format!("  --{}: {};\n", to_snake_case(key), value_str));
                }
            }
        }

        let empty_vec = vec![];
        let empty_map = serde_json::Map::new();
        let default_players = default_style
            .get("players")
            .and_then(|p| p.as_array())
            .unwrap_or(&empty_vec);
        let theme_players = theme_style
            .get("players")
            .and_then(|p| p.as_array())
            .unwrap_or(&empty_vec);
        let max_players = default_players.len().max(theme_players.len());

        for index in 0..max_players {
            let default_player = default_players
                .get(index)
                .and_then(|p| p.as_object())
                .unwrap_or(&empty_map);
            let theme_player = theme_players
                .get(index)
                .and_then(|p| p.as_object())
                .unwrap_or(&empty_map);

            for (key, default_value) in default_player {
                let final_value = theme_player.get(key).unwrap_or(default_value);
                let value_str = match final_value {
                    Value::String(s) => format!("\"{}\"", s),
                    _ => final_value.to_string(),
                };
                if !value_str.is_empty() && value_str != "null" {
                    css.push_str(&format!(
                        "  --player_{}_{}: {};\n",
                        index + 1,
                        to_snake_case(key),
                        value_str
                    ));
                }
            }
        }

        let empty_syntax = serde_json::Map::new();
        let default_syntax = default_style
            .get("syntax")
            .and_then(|s| s.as_object())
            .unwrap_or(&empty_syntax);
        let theme_syntax = theme_style
            .get("syntax")
            .and_then(|s| s.as_object())
            .unwrap_or(&empty_syntax);

        for (key, default_value) in default_syntax {
            let theme_value = theme_syntax.get(key).unwrap_or(default_value);
            if let Some(value_obj) = theme_value.as_object() {
                for (sub_key, sub_value) in value_obj {
                    let value_str = match sub_value {
                        Value::String(s) => format!("\"{}\"", s),
                        _ => sub_value.to_string(),
                    };
                    if !value_str.is_empty() && value_str != "null" {
                        css.push_str(&format!(
                            "  --syntax_{}_{}: {};\n",
                            to_snake_case(key),
                            to_snake_case(sub_key),
                            value_str
                        ));
                    }
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
                to_snake_case(theme_name)
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

fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    let mut prev_char_is_lowercase = false;

    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 && prev_char_is_lowercase {
                result.push('_');
            }
            result.extend(c.to_lowercase());
        } else if c.is_alphanumeric() {
            result.push(c.to_ascii_lowercase());
        } else {
            result.push('_');
        }
        prev_char_is_lowercase = c.is_lowercase();
    }

    result.trim_end_matches('_').to_string()
}
