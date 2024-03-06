use std::{fs::File, io::BufReader};

use clap::Parser;
use dmi::icon::{self, DmiVersion, Icon, IconState};
use image::{DynamicImage, GenericImageView, ImageBuffer, Rgba, RgbaImage};

#[derive(Parser)]
struct Cli {
    /// The path of the DMI file being passed as input.
    #[arg(long)]
    input: std::path::PathBuf,
    /// A list of states to export separated by commas, e.g. "foo,bar". If not specified, all states are exported.
    #[arg(long, default_value = "")]
    states: String,
    /// The path of the image being used to mask the icon states.
    #[arg(long)]
    base_image: std::path::PathBuf,
    /// The path of the DMI file to output to. If it already exists, the states will be appended to the end of it.
    #[arg(long)]
    output: std::path::PathBuf,
}

fn main() {
    let args = Cli::parse();
    let filename: std::path::PathBuf = args.input;
    let output_filename: std::path::PathBuf = args.output;
    let file = File::open(filename).unwrap();

    let reader = BufReader::new(file);
    let dmi = Icon::load(reader).unwrap();

    let source_image = match image::open(args.base_image) {
        Ok(i) => i,
        Err(err) => panic!("image open error: {}", err),
    };

    let states:Vec<_> = if args.states.is_empty() {
        dmi.states.iter().map(|x| x.name.as_str()).collect()
    } else {
        args.states.split(',').collect()
    };

    let mut new_states: Vec<IconState> = vec![];

    dmi.states.iter().for_each(|state| {
        if !states.contains(&state.name.as_str()) {
            return;
        }
        let mut new_image_data: Vec<DynamicImage> = vec![];
        for frame in 1..state.frames + 1 {
            for i in 0..state.dirs {
                let dir = icon::DIR_ORDERING[i as usize];

                let data = match state.get_image(&dir, frame) {
                    Ok(d) => d,
                    Err(err) => panic!("state read error: {}", err),
                };
                let new_data: ImageBuffer<image::Rgba<u8>, Vec<u8>> =
                    RgbaImage::from_fn(data.width(), data.height(), |x, y| {
                        let cur_pixel: Rgba<u8> = data.get_pixel(x, y);
                        let source_pixel: Rgba<u8> = source_image.get_pixel(x, y);
                        let r = (source_pixel.0[0] as f32 / 255.0
                            * (cur_pixel.0[3] as f32 / 255.0)
                            * 255.0) as u8;
                        let g: u8 = (source_pixel.0[1] as f32 / 255.0
                            * (cur_pixel.0[3] as f32 / 255.0)
                            * 255.0) as u8;
                        let b: u8 = (source_pixel.0[2] as f32 / 255.0
                            * (cur_pixel.0[3] as f32 / 255.0)
                            * 255.0) as u8;
                        let a: u8 = (source_pixel.0[3] as f32 / 255.0
                            * (cur_pixel.0[3] as f32 / 255.0)
                            * 255.0) as u8;
                        image::Rgba([r, g, b, a])
                    });
                new_image_data.push(DynamicImage::ImageRgba8(new_data));
            }
        }
        new_states.push(IconState {
            name: state.name.clone(),
            dirs: state.dirs,
            frames: state.frames,
            images: new_image_data,
            delay: state.delay.clone(),
            loop_flag: state.loop_flag,
            rewind: state.rewind,
            movement: state.movement,
            hotspot: state.hotspot,
            unknown_settings: state.unknown_settings.clone(),
        })
    });

    if output_filename.exists() {
        let original_icon =
            Icon::load(BufReader::new(File::open(output_filename.clone()).unwrap())).unwrap();
        let mut all_states: Vec<IconState> = vec![];
        original_icon.states.iter().for_each(|s| all_states.push(s.clone()));
        new_states.iter().for_each(|s| all_states.push(s.clone()));
        let new_icon = Icon {
            version: DmiVersion::default(),
            width: dmi.width,
            height: dmi.height,
            states: all_states
        };
        let mut output_file = File::create(output_filename).unwrap();
        new_icon.save(&mut output_file);
    } else {
        let output_icon = Icon {
            version: DmiVersion::default(),
            width: dmi.width,
            height: dmi.height,
            states: new_states,
        };
        let mut output_file = File::create(output_filename).unwrap();

        output_icon.save(&mut output_file);
    }
}
