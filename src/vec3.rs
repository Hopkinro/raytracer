//! 3D vectors, used for points, directions and RGB colours alike.

use std::ops::{Add, AddAssign, Div, Index, Mul, MulAssign, Neg, Sub};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

pub type Point3 = Vec3;
pub type Color = Vec3;

pub const fn vec3(x: f64, y: f64, z: f64) -> Vec3 {
    Vec3 { x, y, z }
}

impl Vec3 {
    pub const ZERO: Vec3 = vec3(0.0, 0.0, 0.0);
    pub const ONE: Vec3 = vec3(1.0, 1.0, 1.0);

    pub fn dot(self, o: Vec3) -> f64 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }

    pub fn cross(self, o: Vec3) -> Vec3 {
        vec3(
            self.y * o.z - self.z * o.y,
            self.z * o.x - self.x * o.z,
            self.x * o.y - self.y * o.x,
        )
    }

    pub fn length_squared(self) -> f64 {
        self.dot(self)
    }

    pub fn length(self) -> f64 {
        self.length_squared().sqrt()
    }

    pub fn unit(self) -> Vec3 {
        self / self.length()
    }

    /// True when every component is tiny; scattering in such a direction would
    /// produce NaNs, so callers fall back to the surface normal.
    pub fn near_zero(self) -> bool {
        const S: f64 = 1e-8;
        self.x.abs() < S && self.y.abs() < S && self.z.abs() < S
    }

    /// Mirror reflection of `self` about the unit normal `n`.
    pub fn reflect(self, n: Vec3) -> Vec3 {
        self - n * (2.0 * self.dot(n))
    }

    /// Snell's law: bend unit vector `self` through a surface with unit normal
    /// `n`, where `eta_ratio` is n1/n2.
    pub fn refract(self, n: Vec3, eta_ratio: f64) -> Vec3 {
        let cos_theta = (-self).dot(n).min(1.0);
        let perpendicular = (self + n * cos_theta) * eta_ratio;
        let parallel = n * -(1.0 - perpendicular.length_squared()).abs().sqrt();
        perpendicular + parallel
    }

    pub fn min(self, o: Vec3) -> Vec3 {
        vec3(self.x.min(o.x), self.y.min(o.y), self.z.min(o.z))
    }

    pub fn max(self, o: Vec3) -> Vec3 {
        vec3(self.x.max(o.x), self.y.max(o.y), self.z.max(o.z))
    }
}

impl Index<usize> for Vec3 {
    type Output = f64;
    fn index(&self, axis: usize) -> &f64 {
        match axis {
            0 => &self.x,
            1 => &self.y,
            _ => &self.z,
        }
    }
}

impl Add for Vec3 {
    type Output = Vec3;
    fn add(self, o: Vec3) -> Vec3 {
        vec3(self.x + o.x, self.y + o.y, self.z + o.z)
    }
}

impl AddAssign for Vec3 {
    fn add_assign(&mut self, o: Vec3) {
        *self = *self + o;
    }
}

impl Sub for Vec3 {
    type Output = Vec3;
    fn sub(self, o: Vec3) -> Vec3 {
        vec3(self.x - o.x, self.y - o.y, self.z - o.z)
    }
}

impl Neg for Vec3 {
    type Output = Vec3;
    fn neg(self) -> Vec3 {
        vec3(-self.x, -self.y, -self.z)
    }
}

impl Mul<f64> for Vec3 {
    type Output = Vec3;
    fn mul(self, s: f64) -> Vec3 {
        vec3(self.x * s, self.y * s, self.z * s)
    }
}

/// Component-wise product: how a surface's colour filters incoming light.
impl Mul for Vec3 {
    type Output = Vec3;
    fn mul(self, o: Vec3) -> Vec3 {
        vec3(self.x * o.x, self.y * o.y, self.z * o.z)
    }
}

impl MulAssign for Vec3 {
    fn mul_assign(&mut self, o: Vec3) {
        *self = *self * o;
    }
}

impl Div<f64> for Vec3 {
    type Output = Vec3;
    fn div(self, s: f64) -> Vec3 {
        self * (1.0 / s)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Ray {
    pub origin: Point3,
    pub dir: Vec3,
}

impl Ray {
    pub fn new(origin: Point3, dir: Vec3) -> Ray {
        Ray { origin, dir }
    }

    pub fn at(&self, t: f64) -> Point3 {
        self.origin + self.dir * t
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_algebra() {
        let a = vec3(1.0, 2.0, 3.0);
        let b = vec3(4.0, 5.0, 6.0);
        assert_eq!(a + b, vec3(5.0, 7.0, 9.0));
        assert_eq!(a.dot(b), 32.0);
        assert_eq!(a.cross(b), vec3(-3.0, 6.0, -3.0));
        assert!((vec3(3.0, 4.0, 0.0).unit().length() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn reflection_flips_the_normal_component() {
        let incoming = vec3(1.0, -1.0, 0.0);
        assert_eq!(incoming.reflect(vec3(0.0, 1.0, 0.0)), vec3(1.0, 1.0, 0.0));
    }

    #[test]
    fn refraction_obeys_snells_law() {
        // 30 degrees from the normal, entering glass (n = 1.5) from air.
        let theta_in = 30f64.to_radians();
        let incoming = vec3(theta_in.sin(), -theta_in.cos(), 0.0);
        let out = incoming.refract(vec3(0.0, 1.0, 0.0), 1.0 / 1.5);
        let theta_out = out.x.atan2(-out.y);
        assert!((theta_in.sin() - 1.5 * theta_out.sin()).abs() < 1e-9);
        assert!((out.length() - 1.0).abs() < 1e-9);
    }
}
