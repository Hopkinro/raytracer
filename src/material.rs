//! How surfaces scatter (or emit) light.

use crate::geometry::Hit;
use crate::rng::Rng;
use crate::vec3::{Color, Ray};

#[derive(Clone, Copy, Debug)]
pub enum Material {
    /// Matte: scatters in a cosine-weighted random direction.
    Diffuse { albedo: Color },
    /// Mirror-like; `fuzz` (0..1) roughens the reflection.
    Metal { albedo: Color, fuzz: f64 },
    /// Glass, water, diamond...: refracts or reflects depending on the angle.
    Glass { ior: f64 },
    /// A light source. It doesn't scatter, it just shines.
    Light { emit: Color },
}

impl Material {
    /// Returns the scattered ray and how much it's attenuated, or None if the
    /// light is absorbed.
    pub fn scatter(&self, ray: &Ray, hit: &Hit, rng: &mut Rng) -> Option<(Ray, Color)> {
        match *self {
            Material::Diffuse { albedo } => {
                // Normal + random unit vector gives a Lambertian (cos θ) distribution.
                let mut dir = hit.normal + rng.unit_vector();
                if dir.near_zero() {
                    dir = hit.normal;
                }
                Some((Ray::new(hit.p, dir), albedo))
            }
            Material::Metal { albedo, fuzz } => {
                let reflected = ray.dir.unit().reflect(hit.normal) + rng.unit_vector() * fuzz;
                (reflected.dot(hit.normal) > 0.0).then(|| (Ray::new(hit.p, reflected), albedo))
            }
            Material::Glass { ior } => {
                let ratio = if hit.front_face { 1.0 / ior } else { ior };
                let unit = ray.dir.unit();
                let cos_theta = (-unit).dot(hit.normal).min(1.0);
                let sin_theta = (1.0 - cos_theta * cos_theta).sqrt();
                // Past the critical angle there's no refraction, only total
                // internal reflection; otherwise reflect with Fresnel probability.
                let cannot_refract = ratio * sin_theta > 1.0;
                let dir = if cannot_refract || schlick(cos_theta, ratio) > rng.f64() {
                    unit.reflect(hit.normal)
                } else {
                    unit.refract(hit.normal, ratio)
                };
                Some((Ray::new(hit.p, dir), Color::ONE))
            }
            Material::Light { .. } => None,
        }
    }

    pub fn emitted(&self) -> Color {
        match *self {
            Material::Light { emit } => emit,
            _ => Color::ZERO,
        }
    }
}

/// Schlick's approximation of Fresnel reflectance: glass seen edge-on is a mirror.
pub fn schlick(cosine: f64, ratio: f64) -> f64 {
    let r0 = ((1.0 - ratio) / (1.0 + ratio)).powi(2);
    r0 + (1.0 - r0) * (1.0 - cosine).powi(5)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresnel_reflectance_grows_toward_grazing_angles() {
        let head_on = schlick(1.0, 1.0 / 1.5);
        let grazing = schlick(0.05, 1.0 / 1.5);
        assert!(
            (head_on - 0.04).abs() < 1e-9,
            "glass reflects ~4% head-on, got {head_on}"
        );
        assert!(grazing > 0.7);
    }
}
