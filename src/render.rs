//! The path tracer itself, spread across every CPU core.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::thread;

use crate::camera::Camera;
use crate::rng::Rng;
use crate::scene::Scene;
use crate::vec3::{Color, Ray, Vec3};

#[derive(Clone, Copy, Debug)]
pub struct Settings {
    pub width: usize,
    pub height: usize,
    pub samples: usize,
    pub max_depth: usize,
    pub threads: usize,
    pub seed: u64,
}

/// Follow one light path backwards from the camera. Each bounce multiplies the
/// path's `throughput` by the surface colour; light is added when the path hits
/// an emitter or escapes to the sky. A loop, not recursion.
pub fn trace(scene: &Scene, mut ray: Ray, max_depth: usize, rng: &mut Rng) -> Color {
    let mut throughput = Color::ONE;
    let mut radiance = Color::ZERO;
    for _ in 0..max_depth {
        // t_min of 0.001 avoids "shadow acne": re-hitting the surface we just left.
        let Some(hit) = scene.world.hit(&ray, 0.001, f64::INFINITY) else {
            return radiance + throughput * scene.sky.color(&ray);
        };
        radiance += throughput * hit.material.emitted();
        match hit.material.scatter(&ray, &hit, rng) {
            Some((scattered, attenuation)) => {
                throughput *= attenuation;
                ray = scattered;
            }
            None => return radiance,
        }
    }
    radiance // ran out of bounces: treat the rest as darkness
}

/// Render to 8-bit RGB. Threads pull rows from a shared counter, so fast rows
/// (sky) and slow rows (glass) balance out. Each row has its own RNG stream, so
/// the image is identical whatever the thread count or scheduling.
pub fn render(scene: &Scene, s: Settings, mut on_progress: impl FnMut(usize, usize)) -> Vec<u8> {
    let camera = Camera::new(scene.camera, s.width, s.height);
    let next_row = AtomicUsize::new(0);
    let mut image = vec![0u8; s.width * s.height * 3];
    let (tx, rx) = mpsc::channel::<(usize, Vec<u8>)>();

    thread::scope(|scope| {
        for _ in 0..s.threads.max(1) {
            let tx = tx.clone();
            let (camera, next_row) = (&camera, &next_row);
            scope.spawn(move || loop {
                let y = next_row.fetch_add(1, Ordering::Relaxed);
                if y >= s.height {
                    break;
                }
                let mut rng = Rng::new(s.seed, y as u64);
                let mut row = Vec::with_capacity(s.width * 3);
                for x in 0..s.width {
                    let mut sum = Vec3::ZERO;
                    for _ in 0..s.samples {
                        let sample =
                            trace(scene, camera.ray(x, y, &mut rng), s.max_depth, &mut rng);
                        sum += clamp_firefly(sample);
                    }
                    row.extend_from_slice(&to_rgb8(sum / s.samples as f64));
                }
                if tx.send((y, row)).is_err() {
                    break;
                }
            });
        }
        drop(tx);
        for (done, (y, row)) in rx.iter().enumerate() {
            image[y * s.width * 3..(y + 1) * s.width * 3].copy_from_slice(&row);
            on_progress(done + 1, s.height);
        }
    });
    image
}

/// Very bright, very rare samples (a diffuse bounce that happens to hit a small
/// light) show up as speckles. Capping one sample's contribution trades a little
/// bias for much less noise.
fn clamp_firefly(c: Color) -> Color {
    const MAX: f64 = 12.0;
    let peak = c.x.max(c.y).max(c.z);
    if peak > MAX {
        c * (MAX / peak)
    } else {
        c
    }
}

/// Linear light -> gamma 2.0 -> bytes.
fn to_rgb8(c: Color) -> [u8; 3] {
    let channel = |v: f64| (v.max(0.0).sqrt().min(0.999) * 256.0) as u8;
    [channel(c.x), channel(c.y), channel(c.z)]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene;

    fn settings(threads: usize) -> Settings {
        Settings {
            width: 48,
            height: 27,
            samples: 4,
            max_depth: 8,
            threads,
            seed: 9,
        }
    }

    #[test]
    fn output_is_deterministic_regardless_of_thread_count() {
        let scene = scene::glass();
        let one = render(&scene, settings(1), |_, _| {});
        let many = render(&scene, settings(8), |_, _| {});
        assert_eq!(one.len(), 48 * 27 * 3);
        assert_eq!(one, many);
    }

    #[test]
    fn the_sky_is_blue_above_and_white_at_the_horizon() {
        let scene = scene::glass();
        let mut rng = Rng::new(1, 1);
        let up = trace(
            &scene,
            Ray::new(Vec3::ZERO, crate::vec3::vec3(0.0, 1.0, 0.0)),
            8,
            &mut rng,
        );
        assert!(up.z > up.x, "looking straight up should be blue: {up:?}");
    }

    #[test]
    fn every_scene_renders() {
        for name in scene::NAMES {
            let scene = scene::by_name(name).unwrap();
            let pixels = render(
                &scene,
                Settings {
                    width: 16,
                    height: 9,
                    samples: 2,
                    ..settings(2)
                },
                |_, _| {},
            );
            assert!(pixels.iter().any(|&p| p > 0), "{name} rendered black");
        }
    }
}
