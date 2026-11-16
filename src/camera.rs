//! A thin-lens camera: field of view, orientation, and depth of field.

use crate::rng::Rng;
use crate::vec3::{Point3, Ray, Vec3};

#[derive(Clone, Copy, Debug)]
pub struct CameraSettings {
    pub look_from: Point3,
    pub look_at: Point3,
    pub up: Vec3,
    /// Vertical field of view, in degrees.
    pub vfov: f64,
    /// Cone angle (degrees) of rays through each pixel; 0 = everything sharp.
    pub defocus_angle: f64,
    /// Distance to the plane that's perfectly in focus.
    pub focus_dist: f64,
}

pub struct Camera {
    origin: Point3,
    pixel00: Point3,
    du: Vec3,
    dv: Vec3,
    lens_u: Vec3,
    lens_v: Vec3,
    lens: bool,
}

impl Camera {
    pub fn new(s: CameraSettings, width: usize, height: usize) -> Camera {
        let h = (s.vfov.to_radians() / 2.0).tan();
        let viewport_h = 2.0 * h * s.focus_dist;
        let viewport_w = viewport_h * width as f64 / height as f64;

        // Orthonormal basis: w points backwards, u right, v up.
        let w = (s.look_from - s.look_at).unit();
        let u = s.up.cross(w).unit();
        let v = w.cross(u);

        let du = u * (viewport_w / width as f64);
        let dv = -v * (viewport_h / height as f64);
        let top_left =
            s.look_from - w * s.focus_dist - u * (viewport_w / 2.0) + v * (viewport_h / 2.0);
        let lens_radius = s.focus_dist * (s.defocus_angle.to_radians() / 2.0).tan();

        Camera {
            origin: s.look_from,
            pixel00: top_left + (du + dv) * 0.5,
            du,
            dv,
            lens_u: u * lens_radius,
            lens_v: v * lens_radius,
            lens: s.defocus_angle > 0.0,
        }
    }

    /// A ray through a random point inside pixel (x, y), starting from a random
    /// point on the lens. Averaging many gives anti-aliasing and depth of field.
    pub fn ray(&self, x: usize, y: usize, rng: &mut Rng) -> Ray {
        let target = self.pixel00
            + self.du * (x as f64 + rng.f64() - 0.5)
            + self.dv * (y as f64 + rng.f64() - 0.5);
        let origin = if self.lens {
            let d = rng.in_unit_disk();
            self.origin + self.lens_u * d.x + self.lens_v * d.y
        } else {
            self.origin
        };
        Ray::new(origin, target - origin)
    }
}
