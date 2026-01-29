use std::fs;
use std::env;
use std::hash::DefaultHasher;
use std::fmt::Write;
use std::path::{Path, PathBuf};
use std::hash::{Hash, Hasher};
use std::process::exit;
use std::time::UNIX_EPOCH;
use image::Rgba;
use image::{ImageReader, ImageError, DynamicImage};

mod colorscheme;
mod cli;
mod template;
use crate::colorscheme::{Color, Colorscheme, aaverage_generate_colorscheme, kmeans_generate_colorscheme};
use crate::cli::{Args, Method, OutputFormat};
use crate::template::process_template_files;

fn hash_image_path(image_path: &Path, saturation: &f32, method: &Method, colorschemes_cache_path: &Path) -> PathBuf {
    let mut hasher = DefaultHasher::new();
    image_path.as_os_str().hash(&mut hasher);

    if let Ok(meta) = fs::metadata(image_path) {
        if let Ok(mtime) = meta.modified() {
            mtime.duration_since(UNIX_EPOCH).unwrap().as_secs().hash(&mut hasher);
        }
    }

    saturation.to_bits().hash(&mut hasher);
    match method {
        Method::AreaAverage => 0u8.hash(&mut hasher),
        Method::KMeans      => 1u8.hash(&mut hasher),
    }

    let cache_file_name = format!("{:x}.pal", hasher.finish());
    let cache_file_path = colorschemes_cache_path.join(cache_file_name);
    return cache_file_path
}

fn read_scheme_cache(cache_file_path: &Path) -> Colorscheme {
    let content = fs::read_to_string(cache_file_path).unwrap_or_else(|e| {
        eprintln!("Error: could not read cache file: {}", e);
        exit(1);
    });

    let mut lines = content.lines()
                    .filter(|l| !l.trim().is_empty())
                    .map(|l| l.trim_start_matches('#').trim());

    let background: Color = parse_hex_line(lines.next().unwrap_or_else(|| {
        eprintln!("Error: missing background color in cache");
        exit(1);
    }));
    let foreground: Color = parse_hex_line(lines.next().unwrap_or_else(|| {
        eprintln!("Error: missing foreground color in cache");
        exit(1);
    }));
    let palette: Vec<Color> = lines.map(parse_hex_line).collect();

    return Colorscheme { palette:    palette, 
                         background: background, 
                         foreground: foreground }
}

fn parse_hex_line(s: &str) -> Color {
    if s.len() != 6 {
        eprintln!("Error: color line must be 6 hex chars, got '{}'", s);
        exit(1);
    }
    let r = u8::from_str_radix(&s[0..2], 16).map_err(|_| 
        eprintln!("invalid hex red")  ).unwrap_or_else(|_| { eprintln!("Error: invalid red hex in '{}'", s);
        exit(1);
    });
    let g = u8::from_str_radix(&s[2..4], 16).map_err(|_| 
        eprintln!("invalid hex red")  ).unwrap_or_else(|_| { eprintln!("Error: invalid green hex in '{}'", s);
        exit(1);
    });
    let b = u8::from_str_radix(&s[4..6], 16).map_err(|_| 
        eprintln!("invalid hex red")  ).unwrap_or_else(|_| { eprintln!("Error: invalid blue hex in '{}'", s);
        exit(1);
    });
    
    return Color::from_rgba(Rgba([r, g, b, 255]))
}

fn read_image(image_path: &Path) -> Result<DynamicImage, ImageError> {
    let img = ImageReader::open(image_path)?.decode()?;
    return Ok(img)
}

fn write_scheme_cache(cache_file_path: &Path, colorscheme: &Colorscheme) -> Result<(), ()> {
    let mut content = String::new();
    writeln!(content, "#{:02x}{:02x}{:02x}", colorscheme.background.r, colorscheme.background.g, colorscheme.background.b).ok();
    writeln!(content, "#{:02x}{:02x}{:02x}", colorscheme.foreground.r, colorscheme.foreground.g, colorscheme.foreground.b).ok();
    for c in &colorscheme.palette {
        writeln!(content, "#{:02x}{:02x}{:02x}", c.r, c.g, c.b).ok();
    }

    fs::write(cache_file_path, content).map_err(|_| {
        eprintln!("Error: could not cache colorscheme");
        exit(1)
    })
}

fn handle_paths() -> (PathBuf, PathBuf, PathBuf) {
    let home = env::var("HOME").expect("HOME env not set");
    let config_path = Path::new(&home).join(".config/pal");
    let templates_cache_path = Path::new(&home).join(".cache/pal");
    let colorschemes_cache_path = Path::new(&home).join(".cache/pal/other");
    fs::create_dir_all(&config_path).expect("failed to create config dir");
    fs::create_dir_all(&templates_cache_path).expect("failed to create templates cache dir");
    fs::create_dir_all(&colorschemes_cache_path).expect("failed to create colorschemes cache dir");
    return (config_path, templates_cache_path, colorschemes_cache_path)
}

fn main() -> Result<(), ()> {
    let (conf, image_path) = Args::from_cli();
    let (config_path, templates_cache_path, colorschemes_cache_path) = handle_paths();
    let hashed_image_path = hash_image_path(&image_path, &conf.saturation, &conf.method, &colorschemes_cache_path);
    let colorscheme: Colorscheme;

    if hashed_image_path.exists() {
        colorscheme = read_scheme_cache(&hashed_image_path);
    } else {
        let img = read_image(&image_path).map_err(|_| {
            eprintln!("Error: could not open image '{}'", image_path.display());
            exit(1)
        })?;
        
        colorscheme = match conf.method {
            Method::AreaAverage => aaverage_generate_colorscheme(&img).with_saturation(conf.saturation),
            Method::KMeans => kmeans_generate_colorscheme(&img).with_saturation(conf.saturation),
        };

        let _ = write_scheme_cache(&hashed_image_path, &colorscheme).map_err(|_| {
            eprint!("Warning: failed to cache colorscheme");
        });
    }
    
    let _ = process_template_files(config_path, templates_cache_path, &colorscheme, conf.format).map_err(|e| {
        eprintln!("Error: could not process template files; '{}'", e);
        exit(1)
    });

    if conf.verbose {
        match conf.format {
            OutputFormat::HEX => {
                println!("#{:02x}{:02x}{:02x}", &colorscheme.background.r, &colorscheme.background.g, &colorscheme.background.b);
                println!("#{:02x}{:02x}{:02x}", &colorscheme.foreground.r, &colorscheme.foreground.g, &colorscheme.foreground.b);
                for c in &colorscheme.palette {
                    println!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b);
                }
            }
            OutputFormat::RGB => {
                println!("rgb({}, {}, {})", &colorscheme.background.r, &colorscheme.background.g, &colorscheme.background.b);
                println!("rgb({}, {}, {})", &colorscheme.foreground.r, &colorscheme.foreground.g, &colorscheme.foreground.b);
                for c in &colorscheme.palette {
                    println!("rgb({}, {}, {})", c.r, c.g, c.b);
                }
            }
        }
    }
    
    return Ok(())
}
