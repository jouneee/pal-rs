use std::env;
use std::path::PathBuf;
use std::process::exit;

#[derive(Debug)]
pub enum Method {
    AreaAverage,
    KMeans,
    ANSI,
}

#[derive(Debug, Clone, Copy)]
pub enum OutputFormat {
    RGB,
    HEX,
}

pub struct Args{
    pub saturation: f32,
    pub method: Method,
    pub format: OutputFormat,
    pub verbose: bool,
    pub preview: bool,
}

impl Default for Args{
    fn default() -> Self {
        Self {
            saturation: 1.0,
            method: Method::AreaAverage,
            format: OutputFormat::HEX,
            verbose: false,
            preview: false,
        }
    }
}

impl Args{
    pub fn from_cli() -> (Args, PathBuf) {
        let args: Vec<String> = env::args().collect();
        let program = &args[0];

        if args.len() < 2 {
            Self::usage(program);
            eprintln!("Error: missing image path");
            exit(1);
        }

        let mut config = Args::default();
        let mut image_path = None;
        let mut i = 1;

        while i < args.len() {
            let arg = &args[i];

            if arg.starts_with('-') {
                i = Self::parse_flag(&arg, &args, i, &mut config, &program);
                continue;
            }

            if image_path.is_none() {
                image_path = Some(arg.clone());
                i += 1;
                continue;
            }
            Self::usage(program);
            eprintln!("Error: unexpected argument '{}'", arg);
            exit(1);
        }

        let image_path_buf = PathBuf::from(image_path.expect("Image path is set above"));
        (config, image_path_buf)
    }

    fn parse_flag(arg: &str, args: &[String], i: usize, config: &mut Args, program: &str) -> usize {
        let next_arg = || {
            if i + 1 < args.len() {
                Some(&args[i + 1])
            } else {
                Self::usage(program);
                eprintln!("Error: '{}' requires a value", arg);
                exit(1);
            }
        };

        match arg {
            "-s" | "--saturation" => {
                config.saturation = next_arg()
                    .unwrap()
                    .parse::<f32>()
                    .unwrap_or_else(|_| {
                        Self::usage(program);
                        eprintln!("Error: invalid saturation value '{}'", next_arg().unwrap());
                        exit(1);
                    });
                i + 2
            }
            "-m" | "--method" => {
                config.method = match next_arg().unwrap().as_str() {
                    "area_average" | "aa" => Method::AreaAverage,
                    "kmeans" | "km"       => Method::KMeans,
                    "ansi" | "an"         => Method::ANSI,
                    _ => {
                        Self::usage(program);
                        eprintln!("Error: unknown method '{}'", next_arg().unwrap());
                        exit(1);
                    }
                };
                i + 2
            }
            "-f" | "--format" => {
                config.format = match next_arg().unwrap().as_str() {
                    "rgb" => OutputFormat::RGB,
                    "hex" => OutputFormat::HEX,
                    _ => {
                        Self::usage(program);
                        eprintln!("Error: unknown format '{}'", next_arg().unwrap());
                        exit(1);
                    }
                };
                i + 2
            }
            "-v" | "--verbose" => {
                config.verbose = true;
                i + 1
            }
            "-p" | "--preview" => {
                config.preview = true;
                i + 1
            }
            _ => {
                Self::usage(program);
                eprintln!("Error: unknown flag '{}'", arg);
                exit(1);
            }
        }
    }

    fn usage(program: &str) {
        eprintln!("Usage {program} [-s][-m][-f][-v] <path_to_image>");
        eprintln!("Arguments:");
        eprintln!("     -s | --saturation   <float>");
        eprintln!("     -m | --method       [area_average(aa) / kmeans(km) / ansi(an)]");
        eprintln!("     -f | --format       [rgb/hex]");
        eprintln!("     -v | --verbose      print colors to stdout");
        eprintln!("     -p | --preview      if passed, won't generate templates");
    }
}
