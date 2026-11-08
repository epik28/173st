//! Data and simulation core for Minecraft Beta 1.7.3.
//!
//! This initial crate establishes the workspace boundary. Game behavior and Java
//! compatibility will be added in small, independently verified tasks.

pub mod chunk;
pub mod geometry;
pub mod java_compat;
pub mod java_random;
pub mod math;
pub mod motion;
pub mod nbt;
pub mod region;
pub mod world;
