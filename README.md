# Zed Themes CSS Variable Exporter

Converts all of the current Zed themes to CSS variables. Do with this what you will.

## What it does

- Processes themes from [Zed extensions](https://github.com/zed-industries/extensions)
- Merges with default styles
- Generates individual CSS files per theme
- Creates a master index.css
- Produces a themes.json with metadata

## Usage

Run: `cargo run --release`

Expects extensions in `../extensions/extensions`. (clone this in the same directory as extensions)

Output in `./output/`:
- CSS files for each theme
- Extension-specific index.css files
- Master index.css
- themes.json

Skips problematic themes, continuing with others.
