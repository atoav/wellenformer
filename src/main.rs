use std::{
    f32,
    path::PathBuf,
    fs::create_dir_all,
};
use image::ImageBuffer;
use clap::Parser;
use colored::Colorize;
use inquire::Confirm;
use rayon::prelude::*;

mod audio;
use audio::read_audio;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
   /// Path of the audio file that should be rendered
   #[arg(short, long)]
   input: PathBuf,

   /// Path where the resulting png image should be written
   #[arg(short, long)]
   output: PathBuf,

   /// Amount of oversampling to be applied (more takes longer)
   #[arg(short='s', long, default_value_t = 32)]
   oversample: u32,

   /// Background color in RGBA format
   #[arg(long, default_value = "0,0,0,0")]
   background: String,

   /// Background color in RGBA format
   #[arg(long, default_value = "0,0,0,255")]
   foreground: String,

   /// Width of the resulting image in pixels
   #[arg(long, default_value_t = 1920)]
   width: u32,

   /// Height of the resulting image in pixels
   #[arg(long, default_value_t = 120)]
   height: u32,

   /// Overwrite existing files without prompt?
   #[arg(short='y', long)]
   overwrite: bool,

   /// Normalize the audio waveform to fill the vertical space
   #[arg(short='n', long)]
   normalize: bool,
}



fn parse_into_color(argument: &str) -> image::Rgba<u8> {
    let s = argument.trim().to_lowercase();
    match &s[..] {
        "transparent" => image::Rgba([0u8, 0u8, 0u8, 0u8]),
        "none" => image::Rgba([0u8, 0u8, 0u8, 0u8]),
        "red" => image::Rgba([255u8, 0u8, 0u8, 255u8]),
        "yellow" => image::Rgba([255u8, 255u8, 0u8, 255u8]),
        "green" => image::Rgba([0u8, 255u8, 0u8, 255u8]),
        "blue" => image::Rgba([0u8, 0u8, 255u8, 255u8]),
        "cyan" => image::Rgba([0u8, 255u8, 255u8, 255u8]),
        "magenta" => image::Rgba([255u8, 0u8, 255u8, 255u8]),
        "white" => image::Rgba([255u8, 255u8, 255u8, 255u8]),
        "black" => image::Rgba([0u8, 0u8, 0u8, 255u8]),
        _ => {
            match s.split(",").collect::<Vec<&str>>()[..] {
                [lum] => {
                    let l = parse_to_u8(lum);
                    image::Rgba([l, l, l, 255u8])
                },
                [lum, alpha] => {
                    let l = parse_to_u8(lum);
                    let a = parse_to_u8(alpha);
                    image::Rgba([l, l, l, a])
                },
                [red, green, blue] => {
                    let r = parse_to_u8(red);
                    let g = parse_to_u8(green);
                    let b = parse_to_u8(blue);
                    image::Rgba([r, g, b, 255u8])
                },
                [red, green, blue, alpha] => {
                    let r = parse_to_u8(red);
                    let g = parse_to_u8(green);
                    let b = parse_to_u8(blue);
                    let a = parse_to_u8(alpha);
                    image::Rgba([r, g, b, a])
                },
                _ => panic!("Unknown Color \"{s}\"")
            }
        }
    }


}

fn parse_to_u8(string: &str) -> u8 {
    let string = string.trim();
    if string.contains(".") {
        match string.parse::<f32>() {
            Ok(num) => (num.min(1.0).max(0.0) * 255.0) as u8,
            Err(_e) => {
                let error = "Error: ".bold().red();
                let msg = format!("Failed to parse value \"{string}\" for color.");
                eprintln!("{error}{msg}");
                let hint = "Hint:  ".bold().green();
                let msg = "Provide either a color literal (e.g. \"black\" or \"transparent\") or a comma-seperated list of colors in RGB or RGBA format with values ranging either from 0.0 to 1.0 or from 0 - 255.";
                eprintln!("{hint}{msg}");
                std::process::exit(1);
            }
        }
    } else {
        match string.parse::<u32>() {
            Ok(num) => num.min(255).max(0) as u8,
            Err(_e) => {
                let error = "Error: ".bold().red();
                let msg = format!("Failed to parse value \"{string}\" for color.");
                eprintln!("{error}{msg}");
                let hint = "Hint:  ".bold().green();
                let msg = "Provide either a color literal (e.g. \"black\" or \"transparent\") or a comma-seperated list of colors in RGB or RGBA format with values ranging either from 0.0 to 1.0 or from 0 - 255.";
                eprintln!("{hint}{msg}");
                std::process::exit(1);
            }
        }
    }
}

fn create_output_directories(path: &PathBuf) {
    let mut p = path.clone();
    if p.pop() {
        // There are directories in this path that may or may not need to be created
        if !p.exists() {
            match create_dir_all(&p) {
                Ok(_) => println!("Created output directory: \"{}\"", p.to_string_lossy().green()),
                Err(e) => {
                    let error = "Error: ".bold().red();
                    let msg = format!("Could not create output directory \"{}\": {}", p.display(), e);
                    eprintln!("{error}{msg}");
                    std::process::exit(1);
                }
            }
        }
    }
}


fn prepare_output_path(path: &PathBuf) -> PathBuf {
    let mut p = path.clone();
    if p.extension().is_none() {
        p.set_extension("png");
    } else if p.extension().unwrap().to_str().expect("REASON").to_lowercase() != "png" {
        let new_extension = format!("{}.png", p.extension().unwrap().to_string_lossy());
        p.set_extension(new_extension);
    }
    p
}


fn main() {
    use std::time::Instant;
    let now = Instant::now();

    let args = Args::parse();

    // Ensure that the input file is a file
    if !args.input.is_file() {
        let error = "Error: ".bold().red();
        let msg = format!("The input file \"{}\" does not exist (or is not a file)", args.input.to_string_lossy().yellow());
        eprintln!("{error}{msg}");
        std::process::exit(1);
    }

    let output = prepare_output_path(&args.output);

    // Exit if we don't want to overwrite
    if output.is_file() && !args.overwrite {
        // The file exists and should not be overwritten without prompt
        let msg = format!("{}There is already a file at the specified output path! {}", "Warning: ".red(), "Overwrite?".red());
        let ans = Confirm::new(&msg)
        .with_default(false)
        .prompt();

        match ans {
            Ok(true) => {
                ()
            },
            _ => {
                std::process::exit(1);
            }
        }
    }

    create_output_directories(&output);

    // Parse the colors
    let background_color = parse_into_color(&args.background);
    let foreground_color = parse_into_color(&args.foreground);

    // Caluculate the internal width
    let width = args.width as u32 * args.oversample;
    let height = args.height as u32;

    let (channels, samples) = read_audio(&args.input);
    
    let sample_count = samples.len();

    let samples_per_pixel = sample_count  as f64/ (width as f64);

    let (minimum, maximum) = (-1.0, 1.0);

    let factor = if args.normalize {
        let factor = samples.iter().fold(0.0f32, |a, &b| a.abs().max(b.abs())) as f64;
        // Times two because we render half the waveform here
        factor * 2.0
    } else {
        2.0
    };

    let graph: Vec<u32> = 
    samples.par_iter()
           // .step_by(channels.into())
           .map(|s| {
                let sample = if s < &0.0 {
                    // (4.0 * (s as f64 / minimum as f64)).tanh()
                    factor * *s as f64 / minimum as f64
                } else {
                    // (4.0 * ( s as f64 / maximum as f64)).tanh()
                    factor * *s as f64 / maximum as f64
                };
                let pixel_height = (sample * args.height as f64).round();
                pixel_height as u32
            })
           .collect();

    // TODO: Add parallel creation of image buffer
    let mut img = ImageBuffer::from_fn(width, height, |x, y| {
        let start_sample_index = (x as f64 * samples_per_pixel).round() as usize;
        let end_sample_index = (((x+1) as f64 * samples_per_pixel).round() as usize).min(sample_count);

        let range = end_sample_index - start_sample_index;
        let pixel_height = (graph[start_sample_index..end_sample_index].iter()
                                .sum::<u32>() as f64 / range as f64).round() as usize;
        if (height - (y+1)) < pixel_height  as u32{
            foreground_color
        } else {
            background_color
        }
    });

    println!("Processed {} Audio Samples", sample_count/channels);
    println!("Saving image to \"{}\" )", &output.display());
    img = image::imageops::resize(&img, args.width, height,  image::imageops::FilterType::Lanczos3);
    img.save(output).unwrap();

    let elapsed = now.elapsed();
    let msg = format!("Finished after {:.2?}", elapsed).green();
    println!("{}", msg);

}




#[cfg(test)]
mod tests {
    use crate::parse_into_color;

    #[test]
    fn is_transparent() {
        let color = parse_into_color("0,0,0,0");
        assert_eq!(color, image::Rgba([0,0,0,0]));
        let color = parse_into_color("0, 0, 0, 0");
        assert_eq!(color, image::Rgba([0,0,0,0]));
        let color = parse_into_color("none");
        assert_eq!(color, image::Rgba([0,0,0,0]));
        let color = parse_into_color("transparent");
        assert_eq!(color, image::Rgba([0,0,0,0]));
    }

    #[test]
    fn is_black() {
        let color = parse_into_color("0,0,0,255");
        assert_eq!(color, image::Rgba([0,0,0,255]));
        let color = parse_into_color("0, 0, 0, 1.0");
        assert_eq!(color, image::Rgba([0,0,0,255]));
        let color = parse_into_color("black");
        assert_eq!(color, image::Rgba([0,0,0,255]));
    }
}