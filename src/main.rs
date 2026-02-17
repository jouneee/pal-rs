use std::fs;
use std::env;
use std::hash::DefaultHasher;
use std::fmt::Write;
use std::path::{Path, PathBuf};
use std::hash::{Hash, Hasher};
use std::process::exit;
use std::time::UNIX_EPOCH;
use std::io::Cursor;
use image::Rgba;
use image::{ImageReader, ImageError, DynamicImage};

mod colorscheme;
mod cli;
mod template;
use crate::colorscheme::{Color, Colorscheme, aaverage_generate_colorscheme, kmeans_generate_colorscheme, ansi_generate_colorscheme};
use crate::cli::{Args, Method, OutputFormat};
use crate::template::process_template_files;

fn hash_image_uri(image_uri: &str, saturation: &f32, method: &Method, colorschemes_cache_path: &Path) -> PathBuf {
    let mut hasher = DefaultHasher::new();
    image_uri.hash(&mut hasher);

    if let Ok(meta) = fs::metadata(image_uri) {
        if let Ok(mtime) = meta.modified() {
            mtime.duration_since(UNIX_EPOCH).unwrap().as_secs().hash(&mut hasher);
        }
    }

    saturation.to_bits().hash(&mut hasher);
    match method {
        Method::AreaAverage => 0u8.hash(&mut hasher),
        Method::KMeans      => 1u8.hash(&mut hasher),
        Method::ANSI        => 2u8.hash(&mut hasher),
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

fn get_image_from_url(url: &str) -> Result<Vec<u8>, attohttpc::Error> {
    let response = attohttpc::get(url).send().map_err(|_| {
        eprintln!("Error");
        exit(1);
    });
    let data = response.expect("Failed to get image from url").bytes()?;
    Ok(data)
}

fn read_image(image_uri: &str) -> Result<DynamicImage, ImageError> {
    if image_uri.starts_with("http:") || image_uri.starts_with("https:") {
        let bytes = get_image_from_url(image_uri).map_err(|_| {
                eprintln!("Error");
                exit(1);
            }
        );
        let img = ImageReader::new(Cursor::new(bytes.unwrap()))
            .with_guessed_format()?
            .decode()?;
        return Ok(img)
    } else {
        let img = ImageReader::open(image_uri)?.decode()?;
        return Ok(img)
    }
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
    let (conf, image_uri) = Args::from_cli();
    let (config_path, templates_cache_path, colorschemes_cache_path) = handle_paths();
    let hashed_image_uri = hash_image_uri(&image_uri, &conf.saturation, &conf.method, &colorschemes_cache_path);
    let colorscheme: Colorscheme;

    if hashed_image_uri.exists() {
        colorscheme = read_scheme_cache(&hashed_image_uri);
    } else {
        let img = read_image(&image_uri).map_err(|_| {
            eprintln!("Error: could not find image '{}'", image_uri);
            exit(1)
        })?;
        
        colorscheme = match conf.method {
            Method::AreaAverage => aaverage_generate_colorscheme(&img).with_saturation(conf.saturation),
            Method::KMeans      => kmeans_generate_colorscheme(&img).with_saturation(conf.saturation),
            Method::ANSI        => ansi_generate_colorscheme(&img).with_saturation(conf.saturation),
        };

        let _ = write_scheme_cache(&hashed_image_uri, &colorscheme).map_err(|_| {
            eprint!("Warning: failed to cache colorscheme");
        });
    }
    
    if !conf.preview {
        let _ = process_template_files(config_path, templates_cache_path, &colorscheme, conf.format).map_err(|e| {
            eprintln!("Error: could not process template files; '{}'", e);
            exit(1)
        });
    }

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
