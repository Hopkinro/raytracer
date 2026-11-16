use std::io::Write;
use std::process::ExitCode;
use std::time::Instant;

use raytracer::{png, render, scene};

const USAGE: &str = "usage: raytracer [--scene spheres|night|glass] [--width N] [--height N]
                 [--samples N] [--depth N] [--threads N] [--seed N] [--out FILE.png]";

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}\n{USAGE}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<(), String> {
    let mut scene_name = "spheres".to_string();
    let mut out = "render.png".to_string();
    let threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);
    let mut s = render::Settings {
        width: 1200,
        height: 0,
        samples: 100,
        max_depth: 50,
        threads,
        seed: 1,
    };

    let mut args = std::env::args().skip(1);
    while let Some(flag) = args.next() {
        if flag == "-h" || flag == "--help" {
            println!("{USAGE}");
            return Ok(());
        }
        let value = args.next().ok_or(format!("{flag} needs a value"))?;
        let number = || {
            value
                .parse::<usize>()
                .map_err(|_| format!("{flag} expects a number, got {value:?}"))
        };
        match flag.as_str() {
            "--scene" => scene_name = value.clone(),
            "--out" => out = value.clone(),
            "--width" => s.width = number()?,
            "--height" => s.height = number()?,
            "--samples" => s.samples = number()?,
            "--depth" => s.max_depth = number()?,
            "--threads" => s.threads = number()?,
            "--seed" => s.seed = number()? as u64,
            _ => return Err(format!("unknown option {flag}")),
        }
    }
    if s.height == 0 {
        s.height = s.width * 9 / 16;
    }
    if s.width == 0 || s.height == 0 || s.samples == 0 {
        return Err("width, height and samples must be positive".into());
    }
    let scene = scene::by_name(&scene_name).ok_or_else(|| {
        format!(
            "unknown scene {scene_name:?} (choose from {})",
            scene::NAMES.join(", ")
        )
    })?;

    eprintln!(
        "rendering '{scene_name}': {}x{}, {} samples/pixel, {} spheres, {} threads",
        s.width,
        s.height,
        s.samples,
        scene.world.len(),
        s.threads
    );
    let start = Instant::now();
    let pixels = render::render(&scene, s, |done, total| {
        if done % 8 == 0 || done == total {
            eprint!("\r  {:>3}% ({done}/{total} rows)", done * 100 / total);
            std::io::stderr().flush().ok();
        }
    });
    let elapsed = start.elapsed().as_secs_f64();
    let rays = (s.width * s.height * s.samples) as f64;
    eprintln!(
        "\n  done in {elapsed:.1}s ({:.2} M camera rays/s)",
        rays / elapsed / 1e6
    );

    let bytes = png::encode_rgb(s.width as u32, s.height as u32, &pixels);
    std::fs::write(&out, bytes).map_err(|e| format!("writing {out}: {e}"))?;
    eprintln!("  wrote {out}");
    Ok(())
}
