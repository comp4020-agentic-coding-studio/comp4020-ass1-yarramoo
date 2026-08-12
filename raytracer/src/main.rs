use std::{env, fs::File, io::{self, stdin}, path::Path, process};

use raytracer::demo_scenes::triangle;



fn main() -> std::io::Result<()> {
    // Get out file
    let args: Vec<_> = env::args().collect();
    let mut file = get_render_file(&args)?;

    // book1_final_scene(&mut file)
    // checkered_spheres(&mut file)
    // earth(&mut file)
    // perlin_spheres(&mut file)
    // quads(&mut file)
    // simple_light(&mut file)
    // cornell_box(&mut file)
    // cornell_smoke(&mut file)
    // book2_final_scene(800, 10000, 40, &mut file)
    // book2_final_scene(800, 10000, 40, &mut file) 11418.92 real     89841.33 user       170.50 sys
    triangle(400, 400).write_ppm(&mut file)
}

fn get_render_file(args: &[String]) -> io::Result<File> {
    if args.len() < 2 {
        eprintln!("No render filename provided");
        process::exit(1);
    }

    let filename = &args[1];
    let filename = "./renders/".to_string() + filename;

    if Path::new(&filename).exists() {
        eprintln!("File {} already exists. Overwrite? y/n", filename);
        let mut buf = String::new();
        let _ = stdin().read_line(&mut buf)?;
        let response = buf.chars().next().unwrap_or('n');
        if !response.eq_ignore_ascii_case(&'y') {
            println!("exiting...");
            process::exit(0);
        }
    }

    File::create(filename)
}
