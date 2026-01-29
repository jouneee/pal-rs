use std::fs;
use std::path::PathBuf;

use crate::colorscheme::{Color, Colorscheme};
use crate::cli::OutputFormat;

pub fn process_template_files(config_path: PathBuf, cache_path: PathBuf, colorscheme: &Colorscheme, format: OutputFormat) -> Result<(), std::io::Error> {
    for entry in fs::read_dir(config_path)? {
        let entry = entry?;
        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        let content = parse_template(path.clone(), colorscheme, format)?;

        let out_path = cache_path.join(entry.file_name());
        fs::write(out_path, content)?;
    }
    Ok(())
}

fn parse_template(template: PathBuf, colorscheme: &Colorscheme, format: OutputFormat) -> Result<String, std::io::Error> {
    let content = fs::read_to_string(template)?;
    let mut result = String::new();
    let mut placeholder = String::new();
    let mut is_inside = 0;

    for c in content.chars() {
        match (is_inside, c) {
            (0, '`') => {
                is_inside = 1;
                placeholder.clear();
            },
            (0, _) => {
                result.push(c);
            },
            (1, '`') => {
                if let Some(repl) = resolve(&placeholder, colorscheme, format) {
                    result.push_str(&repl);
                } else {
                    result.push('`');
                    result.push_str(&placeholder);
                    result.push('`');
                }
                is_inside = 0
            }
            (1, _) => placeholder.push(c),
            _ => {}
        }
    }

    if is_inside == 1 {
        result.push('`');
        result.push_str(&placeholder);
    }

    Ok(result)
}

fn resolve(placeholder: &str, colorscheme: &Colorscheme, format: OutputFormat) -> Option<String> {
    if placeholder.starts_with("@background") {
        return Some(format_color(&colorscheme.background, format))
    } 
    else if placeholder.starts_with("@foreground") {
        return Some(format_color(&colorscheme.foreground, format))
    }
    else if placeholder.starts_with("@color") {
        return placeholder[6..] 
                .parse::<usize>()
                .ok()
                .and_then(|i| colorscheme.palette.get(i))
                .map(|c| format_color(c, format))
    } else {
        return None
    }
}

fn format_color(c: &Color, format: OutputFormat) -> String {
    match format {
        OutputFormat::HEX => format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b),
        OutputFormat::RGB => format!("rgb({},{},{})", c.r, c.g, c.b),
    }
}