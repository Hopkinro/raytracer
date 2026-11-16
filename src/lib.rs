//! A path tracer with no dependencies: spheres, matte/metal/glass/emissive
//! materials, depth of field, a BVH, multithreaded rendering, and a PNG encoder.

pub mod camera;
pub mod geometry;
pub mod material;
pub mod png;
pub mod render;
pub mod rng;
pub mod scene;
pub mod vec3;
