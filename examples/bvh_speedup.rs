//! How much does the BVH buy? Fire the same camera rays at the "spheres" scene
//! through the BVH and through a plain loop over every sphere.
//!
//!     cargo run --release --example bvh_speedup

use std::time::Instant;

use raytracer::camera::Camera;
use raytracer::rng::Rng;
use raytracer::scene;

fn main() {
    let scene = scene::spheres();
    let (w, h) = (320, 180);
    let camera = Camera::new(scene.camera, w, h);
    let mut rng = Rng::new(1, 1);
    let rays: Vec<_> = (0..200_000)
        .map(|i| camera.ray(i % w, (i / w) % h, &mut rng))
        .collect();

    let time = |f: &dyn Fn(&raytracer::vec3::Ray) -> bool| {
        let start = Instant::now();
        let hits = rays.iter().filter(|r| f(r)).count();
        (start.elapsed().as_secs_f64(), hits)
    };
    let (bvh_s, bvh_hits) = time(&|r| scene.world.hit(r, 0.001, f64::INFINITY).is_some());
    let (brute_s, brute_hits) = time(&|r| {
        scene
            .world
            .hit_brute_force(r, 0.001, f64::INFINITY)
            .is_some()
    });
    assert_eq!(bvh_hits, brute_hits);

    println!("{} spheres, {} rays", scene.world.len(), rays.len());
    println!(
        "  brute force: {:>7.1} ms  ({:.2} M rays/s)",
        brute_s * 1e3,
        rays.len() as f64 / brute_s / 1e6
    );
    println!(
        "  BVH:         {:>7.1} ms  ({:.2} M rays/s)",
        bvh_s * 1e3,
        rays.len() as f64 / bvh_s / 1e6
    );
    println!("  speedup:     {:.1}x", brute_s / bvh_s);
}
