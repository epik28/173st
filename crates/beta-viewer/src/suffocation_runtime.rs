//! Native player's solid-head suffocation query, read-only and bounded.
use beta_core::{chunk::BlockData, motion::MotionState};

/// Pinned Beta `fd.h`: material blocks movement and block is an opaque cube.
fn suffocating_id(id: u8) -> bool {
    matches!(id, 1 | 2 | 3 | 4 | 5 | 7 | 12 | 13 | 14 | 15 | 16 | 17 | 19 | 21 | 22 | 23 | 24 | 25 | 35 | 41 | 42 | 43 | 45 | 47 | 48 | 49 | 52 | 54 | 56 | 57 | 58 | 61 | 62 | 73 | 74 | 80 | 82 | 84 | 86 | 87 | 88 | 89 | 91 | 95)
}

/// None if any required cell is unavailable and none is definitely suffocating.
pub fn head_in_block(state: &MotionState, mut block: impl FnMut(i32,i32,i32)->Option<BlockData>) -> Option<bool> {
    let floor = |x:f64| -> Option<i32> { let x=x.floor(); if x.is_finite() && x >= i32::MIN as f64 && x <= i32::MAX as f64 {Some(x as i32)} else {None} };
    let eye = state.bbox.min[1] + f64::from(1.62_f32) - f64::from(state.y_size) + f64::from(0.12_f32);
    let mut missing=false;
    for i in 0..8 {
        let dx = (((i & 1) as f32 - 0.5_f32) * 0.6_f32) * 0.9_f32;
        let dy = (((i >> 1 & 1) as f32) - 0.5_f32) * 0.1_f32;
        let dz = (((i >> 2 & 1) as f32 - 0.5_f32) * 0.6_f32) * 0.9_f32;
        let xyz=(floor(state.position[0]+f64::from(dx))?,floor(eye+f64::from(dy))?,floor(state.position[2]+f64::from(dz))?);
        match block(xyz.0,xyz.1,xyz.2) { Some(b) if suffocating_id(b.id) => return Some(true), None => missing=true, _ => {} }
    }
    if missing {None} else {Some(false)}
}

#[cfg(test)]
mod tests {
 use super::*;
 #[test] fn native_material_registry() {
   for row in include_str!("../../../tests/fixtures/suffocation/material.tsv").lines().skip(1) { let (id, expected)=row.split_once('\t').unwrap(); assert_eq!(suffocating_id(id.parse().unwrap()),expected.parse().unwrap(),"{row}"); }
 }
 #[test] fn native_head_geometry() {
   for row in include_str!("../../../tests/fixtures/suffocation/native.tsv").lines().skip(1) {
     let f:Vec<_>=row.split('\t').collect(); let mut s=MotionState::new([f[0].parse().unwrap(),f[1].parse().unwrap(),f[2].parse().unwrap()]); s.y_size=f[3].parse().unwrap();
     let cell=(f[4].parse().unwrap(),f[5].parse().unwrap(),f[6].parse().unwrap()); let solid:bool=f[7].parse().unwrap();
     let result=head_in_block(&s,|x,y,z|Some(BlockData{id:if solid && (x,y,z)==cell {1} else {0},metadata:0,block_light:0,sky_light:0}));
     assert_eq!(result,Some(f[8].parse().unwrap()),"{row}");
   }
   assert_eq!(head_in_block(&MotionState::new([0.5,0.0,0.5]),|_,_,_|None),None);
 }
}
