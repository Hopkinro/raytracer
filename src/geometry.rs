//! Spheres, bounding boxes, and a bounding volume hierarchy (BVH) that makes
//! "which sphere does this ray hit first?" logarithmic instead of linear.

use crate::material::Material;
use crate::vec3::{Point3, Ray, Vec3};

pub struct Hit {
    pub p: Point3,
    /// Always points against the incoming ray.
    pub normal: Vec3,
    pub t: f64,
    pub front_face: bool,
    pub material: Material,
}

#[derive(Clone, Copy, Debug)]
pub struct Sphere {
    pub center: Point3,
    pub radius: f64,
    pub material: Material,
}

impl Sphere {
    pub fn new(center: Point3, radius: f64, material: Material) -> Sphere {
        Sphere {
            center,
            radius,
            material,
        }
    }

    /// Solve |origin + t·dir - center|² = r² for the nearest t in (t_min, t_max).
    pub fn hit(&self, ray: &Ray, t_min: f64, t_max: f64) -> Option<Hit> {
        let oc = self.center - ray.origin;
        let a = ray.dir.length_squared();
        let h = ray.dir.dot(oc);
        let c = oc.length_squared() - self.radius * self.radius;
        let discriminant = h * h - a * c;
        if discriminant < 0.0 {
            return None;
        }
        let sqrt_d = discriminant.sqrt();
        let mut t = (h - sqrt_d) / a;
        if t <= t_min || t >= t_max {
            t = (h + sqrt_d) / a;
            if t <= t_min || t >= t_max {
                return None;
            }
        }
        let p = ray.at(t);
        let outward = (p - self.center) / self.radius;
        let front_face = ray.dir.dot(outward) < 0.0;
        Some(Hit {
            p,
            normal: if front_face { outward } else { -outward },
            t,
            front_face,
            material: self.material,
        })
    }

    pub fn bounds(&self) -> Aabb {
        let r = Vec3 {
            x: self.radius.abs(),
            y: self.radius.abs(),
            z: self.radius.abs(),
        };
        Aabb {
            min: self.center - r,
            max: self.center + r,
        }
    }
}

/// Axis-aligned bounding box.
#[derive(Clone, Copy, Debug)]
pub struct Aabb {
    pub min: Point3,
    pub max: Point3,
}

impl Aabb {
    pub fn surface_area(&self) -> f64 {
        let d = self.max - self.min;
        2.0 * (d.x * d.y + d.y * d.z + d.z * d.x)
    }

    pub fn union(self, o: Aabb) -> Aabb {
        Aabb {
            min: self.min.min(o.min),
            max: self.max.max(o.max),
        }
    }

    /// The slab test: the ray is inside the box exactly where its intervals
    /// inside the x, y and z slabs all overlap. Returns the entry distance.
    pub fn hit(&self, ray: &Ray, mut t_min: f64, mut t_max: f64) -> Option<f64> {
        for axis in 0..3 {
            let inv = 1.0 / ray.dir[axis];
            let mut t0 = (self.min[axis] - ray.origin[axis]) * inv;
            let mut t1 = (self.max[axis] - ray.origin[axis]) * inv;
            if inv < 0.0 {
                std::mem::swap(&mut t0, &mut t1);
            }
            t_min = t_min.max(t0);
            t_max = t_max.min(t1);
            if t_max <= t_min {
                return None;
            }
        }
        Some(t_min)
    }
}

enum Node {
    Leaf {
        bounds: Aabb,
        start: usize,
        end: usize,
    },
    Inner {
        bounds: Aabb,
        left: usize,
        right: usize,
    },
}

impl Node {
    fn bounds(&self) -> Aabb {
        match *self {
            Node::Leaf { bounds, .. } | Node::Inner { bounds, .. } => bounds,
        }
    }
}

/// A tree of boxes over the spheres. A ray that misses a box skips everything
/// inside it, so a scene with 500 spheres costs a few dozen tests per ray, not 500.
/// Nodes live in one flat Vec (cache-friendly, no pointer chasing).
pub struct Bvh {
    spheres: Vec<Sphere>,
    nodes: Vec<Node>,
}

const LEAF_SIZE: usize = 4;

impl Bvh {
    pub fn new(mut spheres: Vec<Sphere>) -> Bvh {
        let mut nodes = Vec::with_capacity(spheres.len() * 2);
        if !spheres.is_empty() {
            let len = spheres.len();
            build(&mut spheres, 0, len, &mut nodes);
        }
        Bvh { spheres, nodes }
    }

    pub fn len(&self) -> usize {
        self.spheres.len()
    }

    pub fn is_empty(&self) -> bool {
        self.spheres.is_empty()
    }

    /// Front-to-back traversal: at each inner node, visit the child whose box
    /// the ray enters first. Once something is hit, `t_max` shrinks to that
    /// distance, and any box that starts further away is skipped unopened.
    pub fn hit(&self, ray: &Ray, t_min: f64, mut t_max: f64) -> Option<Hit> {
        let mut closest = None;
        let mut stack: Vec<(usize, f64)> = Vec::with_capacity(64);
        if let Some(root) = self.nodes.first() {
            if let Some(t) = root.bounds().hit(ray, t_min, t_max) {
                stack.push((0, t));
            }
        }
        while let Some((i, entry)) = stack.pop() {
            if entry >= t_max {
                continue; // something nearer was found after this was queued
            }
            match self.nodes[i] {
                Node::Leaf { start, end, .. } => {
                    for sphere in &self.spheres[start..end] {
                        if let Some(hit) = sphere.hit(ray, t_min, t_max) {
                            t_max = hit.t; // only nearer hits matter from now on
                            closest = Some(hit);
                        }
                    }
                }
                Node::Inner { left, right, .. } => {
                    let l = self.nodes[left]
                        .bounds()
                        .hit(ray, t_min, t_max)
                        .map(|t| (left, t));
                    let r = self.nodes[right]
                        .bounds()
                        .hit(ray, t_min, t_max)
                        .map(|t| (right, t));
                    match (l, r) {
                        (Some(a), Some(b)) => {
                            let (near, far) = if a.1 <= b.1 { (a, b) } else { (b, a) };
                            stack.push(far);
                            stack.push(near);
                        }
                        (Some(only), None) | (None, Some(only)) => stack.push(only),
                        (None, None) => {}
                    }
                }
            }
        }
        closest
    }

    /// Reference implementation for tests: check every sphere.
    pub fn hit_brute_force(&self, ray: &Ray, t_min: f64, mut t_max: f64) -> Option<Hit> {
        let mut closest = None;
        for sphere in &self.spheres {
            if let Some(hit) = sphere.hit(ray, t_min, t_max) {
                t_max = hit.t;
                closest = Some(hit);
            }
        }
        closest
    }
}

/// Split along the box's longest axis at the median centre: simple, and good
/// enough for spheres scattered across a plane.
fn build(spheres: &mut [Sphere], start: usize, end: usize, nodes: &mut Vec<Node>) -> usize {
    let slice = &mut spheres[start..end];
    let bounds = slice
        .iter()
        .map(Sphere::bounds)
        .reduce(Aabb::union)
        .unwrap();
    let index = nodes.len();
    if slice.len() <= LEAF_SIZE {
        nodes.push(Node::Leaf { bounds, start, end });
        return index;
    }

    // A giant object (like the 1000-unit "ground" sphere) would inflate the box
    // of whichever subtree it landed in, so rays could never skip that subtree.
    // Give it a leaf of its own at this level instead. (A surface-area-heuristic
    // builder makes the same choice; this is the one case that matters here.)
    let (biggest, area) = slice
        .iter()
        .enumerate()
        .map(|(i, s)| (i, s.bounds().surface_area()))
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .unwrap();
    if area > 0.5 * bounds.surface_area() {
        let last = slice.len() - 1;
        slice.swap(biggest, last);
        nodes.push(Node::Leaf { bounds, start, end }); // placeholder, replaced below
        let left = build(spheres, start, end - 1, nodes);
        let right = build(spheres, end - 1, end, nodes);
        nodes[index] = Node::Inner {
            bounds,
            left,
            right,
        };
        return index;
    }

    let extent = bounds.max - bounds.min;
    let axis = if extent.x > extent.y && extent.x > extent.z {
        0
    } else if extent.y > extent.z {
        1
    } else {
        2
    };
    let mid = slice.len() / 2;
    slice.select_nth_unstable_by(mid, |a, b| a.center[axis].total_cmp(&b.center[axis]));
    nodes.push(Node::Leaf { bounds, start, end }); // placeholder, replaced below
    let left = build(spheres, start, start + mid, nodes);
    let right = build(spheres, start + mid, end, nodes);
    nodes[index] = Node::Inner {
        bounds,
        left,
        right,
    };
    index
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::Rng;
    use crate::vec3::{vec3, Color};

    const MATTE: Material = Material::Diffuse { albedo: Color::ONE };

    #[test]
    fn ray_hits_sphere_front_and_back() {
        let s = Sphere::new(vec3(0.0, 0.0, -5.0), 1.0, MATTE);
        let hit = s
            .hit(
                &Ray::new(Vec3::ZERO, vec3(0.0, 0.0, -1.0)),
                0.001,
                f64::INFINITY,
            )
            .unwrap();
        assert!((hit.t - 4.0).abs() < 1e-12 && hit.front_face);
        assert_eq!(hit.normal, vec3(0.0, 0.0, 1.0));
        // From inside the sphere we hit the far wall, with the normal flipped toward us.
        let inside = s
            .hit(
                &Ray::new(vec3(0.0, 0.0, -5.0), vec3(0.0, 0.0, -1.0)),
                0.001,
                f64::INFINITY,
            )
            .unwrap();
        assert!((inside.t - 1.0).abs() < 1e-12 && !inside.front_face);
        assert_eq!(inside.normal, vec3(0.0, 0.0, 1.0));
        assert!(s
            .hit(
                &Ray::new(Vec3::ZERO, vec3(0.0, 1.0, 0.0)),
                0.001,
                f64::INFINITY
            )
            .is_none());
    }

    #[test]
    fn bvh_agrees_with_brute_force() {
        let mut rng = Rng::new(3, 3);
        let spheres = (0..400)
            .map(|_| {
                let c = vec3(
                    rng.range(-20.0, 20.0),
                    rng.range(-2.0, 2.0),
                    rng.range(-20.0, 20.0),
                );
                Sphere::new(c, rng.range(0.1, 1.5), MATTE)
            })
            .collect();
        let bvh = Bvh::new(spheres);
        let mut hits = 0;
        for _ in 0..20_000 {
            let origin = vec3(
                rng.range(-25.0, 25.0),
                rng.range(-5.0, 5.0),
                rng.range(-25.0, 25.0),
            );
            let ray = Ray::new(origin, rng.unit_vector());
            let fast = bvh.hit(&ray, 0.001, f64::INFINITY).map(|h| h.t);
            let slow = bvh.hit_brute_force(&ray, 0.001, f64::INFINITY).map(|h| h.t);
            assert_eq!(fast, slow);
            hits += fast.is_some() as usize;
        }
        assert!(
            hits > 1_000,
            "test rays should actually hit things ({hits})"
        );
    }
}
