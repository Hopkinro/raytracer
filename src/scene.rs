//! Built-in scenes.

use crate::camera::CameraSettings;
use crate::geometry::{Bvh, Sphere};
use crate::material::Material::{self, *};
use crate::rng::Rng;
use crate::vec3::{vec3, Color, Ray};

pub enum Sky {
    /// White at the horizon fading to blue overhead.
    Day,
    /// Almost black, so the light comes from emissive spheres.
    Night,
}

impl Sky {
    pub fn color(&self, ray: &Ray) -> Color {
        let t = 0.5 * (ray.dir.unit().y + 1.0);
        match self {
            Sky::Day => Color::ONE * (1.0 - t) + vec3(0.5, 0.7, 1.0) * t,
            Sky::Night => vec3(0.02, 0.02, 0.05) * (1.0 - t) + vec3(0.0, 0.0, 0.01) * t,
        }
    }
}

pub struct Scene {
    pub world: Bvh,
    pub camera: CameraSettings,
    pub sky: Sky,
}

pub const NAMES: [&str; 3] = ["spheres", "night", "glass"];

pub fn by_name(name: &str) -> Option<Scene> {
    match name {
        "spheres" => Some(spheres()),
        "night" => Some(night()),
        "glass" => Some(glass()),
        _ => None,
    }
}

fn random_color(rng: &mut Rng, lo: f64, hi: f64) -> Color {
    vec3(rng.range(lo, hi), rng.range(lo, hi), rng.range(lo, hi))
}

/// A field of small random spheres around three big ones (glass, matte, mirror).
pub fn spheres() -> Scene {
    let mut rng = Rng::new(2026, 0);
    let mut list = vec![Sphere::new(
        vec3(0.0, -1000.0, 0.0),
        1000.0,
        Diffuse {
            albedo: vec3(0.5, 0.5, 0.45),
        },
    )];
    let big = [
        vec3(0.0, 1.0, 0.0),
        vec3(-4.0, 1.0, 0.0),
        vec3(4.0, 1.0, 0.0),
    ];
    for a in -11..11 {
        for b in -11..11 {
            let center = vec3(a as f64 + 0.9 * rng.f64(), 0.2, b as f64 + 0.9 * rng.f64());
            if big.iter().any(|&c| (center - c).length() < 1.2) {
                continue;
            }
            let pick = rng.f64();
            let material = if pick < 0.7 {
                Diffuse {
                    albedo: random_color(&mut rng, 0.0, 1.0) * random_color(&mut rng, 0.0, 1.0),
                }
            } else if pick < 0.9 {
                Metal {
                    albedo: random_color(&mut rng, 0.5, 1.0),
                    fuzz: rng.range(0.0, 0.5),
                }
            } else {
                Glass { ior: 1.5 }
            };
            list.push(Sphere::new(center, 0.2, material));
        }
    }
    list.push(Sphere::new(big[0], 1.0, Glass { ior: 1.5 }));
    list.push(Sphere::new(
        big[1],
        1.0,
        Diffuse {
            albedo: vec3(0.4, 0.2, 0.1),
        },
    ));
    list.push(Sphere::new(
        big[2],
        1.0,
        Metal {
            albedo: vec3(0.7, 0.6, 0.5),
            fuzz: 0.0,
        },
    ));

    Scene {
        world: Bvh::new(list),
        camera: CameraSettings {
            look_from: vec3(13.0, 2.0, 3.0),
            look_at: vec3(0.0, 0.0, 0.0),
            up: vec3(0.0, 1.0, 0.0),
            vfov: 20.0,
            defocus_angle: 0.6,
            focus_dist: 10.0,
        },
        sky: Sky::Day,
    }
}

/// Glowing orbs on a dark floor: all the light comes from the scene itself.
pub fn night() -> Scene {
    let mut rng = Rng::new(7, 0);
    let mut list = vec![Sphere::new(
        vec3(0.0, -1000.0, 0.0),
        1000.0,
        Diffuse {
            albedo: vec3(0.45, 0.45, 0.5),
        },
    )];
    let warm = Light {
        emit: vec3(6.0, 3.6, 1.6),
    };
    let cool = Light {
        emit: vec3(1.4, 3.0, 6.0),
    };
    list.push(Sphere::new(vec3(0.0, 1.0, 0.0), 1.0, Glass { ior: 1.5 }));
    list.push(Sphere::new(vec3(0.0, 1.0, 0.0), -0.9, Glass { ior: 1.5 })); // hollow glass bubble
    list.push(Sphere::new(
        vec3(-2.3, 0.7, -0.6),
        0.7,
        Metal {
            albedo: vec3(0.9, 0.9, 0.9),
            fuzz: 0.02,
        },
    ));
    list.push(Sphere::new(
        vec3(2.2, 0.8, 0.6),
        0.8,
        Diffuse {
            albedo: vec3(0.8, 0.3, 0.3),
        },
    ));
    list.push(Sphere::new(vec3(-0.9, 0.35, 1.8), 0.35, warm));
    list.push(Sphere::new(vec3(1.2, 0.3, -1.7), 0.3, cool));
    // A soft fill light above and behind the camera, out of frame.
    list.push(Sphere::new(
        vec3(0.0, 7.0, 2.0),
        1.5,
        Light {
            emit: vec3(1.6, 1.5, 1.4),
        },
    ));
    for _ in 0..40 {
        let angle = rng.range(0.0, std::f64::consts::TAU);
        let r = rng.range(3.0, 6.5);
        let size = rng.range(0.06, 0.14);
        let center = vec3(r * angle.cos(), size, r * angle.sin());
        let material: Material = if rng.f64() < 0.35 {
            Light {
                emit: random_color(&mut rng, 0.5, 1.0) * 4.0,
            }
        } else {
            Metal {
                albedo: random_color(&mut rng, 0.6, 1.0),
                fuzz: 0.1,
            }
        };
        list.push(Sphere::new(center, size, material));
    }
    Scene {
        world: Bvh::new(list),
        camera: CameraSettings {
            look_from: vec3(0.0, 2.2, 7.5),
            look_at: vec3(0.0, 0.7, 0.0),
            up: vec3(0.0, 1.0, 0.0),
            vfov: 38.0,
            defocus_angle: 0.4,
            focus_dist: 7.4,
        },
        sky: Sky::Night,
    }
}

/// Three spheres, close up: the quickest scene to render.
pub fn glass() -> Scene {
    let list = vec![
        Sphere::new(
            vec3(0.0, -100.5, -1.0),
            100.0,
            Diffuse {
                albedo: vec3(0.8, 0.8, 0.0),
            },
        ),
        Sphere::new(
            vec3(0.0, 0.0, -1.2),
            0.5,
            Diffuse {
                albedo: vec3(0.1, 0.2, 0.5),
            },
        ),
        Sphere::new(vec3(-1.0, 0.0, -1.0), 0.5, Glass { ior: 1.5 }),
        Sphere::new(vec3(-1.0, 0.0, -1.0), -0.4, Glass { ior: 1.5 }),
        Sphere::new(
            vec3(1.0, 0.0, -1.0),
            0.5,
            Metal {
                albedo: vec3(0.8, 0.6, 0.2),
                fuzz: 0.3,
            },
        ),
    ];
    Scene {
        world: Bvh::new(list),
        camera: CameraSettings {
            look_from: vec3(-2.0, 2.0, 1.0),
            look_at: vec3(0.0, 0.0, -1.0),
            up: vec3(0.0, 1.0, 0.0),
            vfov: 30.0,
            defocus_angle: 0.0,
            focus_dist: 3.4,
        },
        sky: Sky::Day,
    }
}
