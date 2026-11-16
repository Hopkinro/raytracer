//! A small, fast, seedable random number generator (PCG32), plus the random
//! sampling helpers a path tracer needs. Seeding per image row makes renders
//! reproducible no matter how rows are spread across threads.

use crate::vec3::{vec3, Vec3};

pub struct Rng {
    state: u64,
    inc: u64,
}

impl Rng {
    pub fn new(seed: u64, stream: u64) -> Rng {
        let mut rng = Rng {
            state: 0,
            inc: (stream << 1) | 1,
        };
        rng.next_u32();
        rng.state = rng.state.wrapping_add(seed);
        rng.next_u32();
        rng
    }

    pub fn next_u32(&mut self) -> u32 {
        let old = self.state;
        self.state = old.wrapping_mul(6364136223846793005).wrapping_add(self.inc);
        let xorshifted = (((old >> 18) ^ old) >> 27) as u32;
        let rot = (old >> 59) as u32;
        xorshifted.rotate_right(rot)
    }

    /// Uniform in [0, 1).
    pub fn f64(&mut self) -> f64 {
        let bits = ((self.next_u32() as u64) << 21) ^ (self.next_u32() as u64);
        (bits & ((1 << 53) - 1)) as f64 / (1u64 << 53) as f64
    }

    pub fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.f64()
    }

    /// Uniformly distributed direction (a point on the unit sphere).
    pub fn unit_vector(&mut self) -> Vec3 {
        loop {
            let p = vec3(
                self.range(-1.0, 1.0),
                self.range(-1.0, 1.0),
                self.range(-1.0, 1.0),
            );
            let len2 = p.length_squared();
            if 1e-160 < len2 && len2 <= 1.0 {
                return p / len2.sqrt();
            }
        }
    }

    /// Uniform point in the unit disk (z = 0), for sampling the lens aperture.
    pub fn in_unit_disk(&mut self) -> Vec3 {
        loop {
            let p = vec3(self.range(-1.0, 1.0), self.range(-1.0, 1.0), 0.0);
            if p.length_squared() < 1.0 {
                return p;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_sequence() {
        let (mut a, mut b) = (Rng::new(7, 1), Rng::new(7, 1));
        for _ in 0..100 {
            assert_eq!(a.next_u32(), b.next_u32());
        }
        assert_ne!(Rng::new(7, 1).next_u32(), Rng::new(7, 2).next_u32());
    }

    #[test]
    fn floats_are_in_range_and_roughly_uniform() {
        let mut rng = Rng::new(42, 0);
        let n = 100_000;
        let mut sum = 0.0;
        for _ in 0..n {
            let x = rng.f64();
            assert!((0.0..1.0).contains(&x));
            sum += x;
        }
        assert!((sum / n as f64 - 0.5).abs() < 0.01);
    }

    #[test]
    fn unit_vectors_have_length_one_and_no_bias() {
        let mut rng = Rng::new(1, 0);
        let mut mean = Vec3::ZERO;
        for _ in 0..50_000 {
            let v = rng.unit_vector();
            assert!((v.length() - 1.0).abs() < 1e-9);
            mean += v / 50_000.0;
        }
        assert!(mean.length() < 0.02, "directions are biased: mean {mean:?}");
    }
}
