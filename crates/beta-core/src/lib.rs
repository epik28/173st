//! Data and simulation core for Minecraft Beta 1.7.3.
//!
//! This initial crate establishes the workspace boundary. Game behavior and Java
//! compatibility will be added in small, independently verified tasks.

pub mod armor_damage;
pub mod armor_equipment;
pub mod biome;
pub mod block;
pub mod chunk;
pub mod food;
pub mod drowning;
pub mod fall_damage;
pub mod geometry;
pub mod items;
pub mod java_compat;
pub mod java_random;
pub mod light;
pub mod math;
pub mod motion;
pub mod nbt;
pub mod peaceful_regen;
pub mod region;
pub mod water;
pub mod world;
