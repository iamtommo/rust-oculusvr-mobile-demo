use crate::anim::layer::SkeletalLayer;
use std::cell::Cell;
use crate::model::SkeletalMesh;
use crate::anim::skeletal::sample_bear;
use math::matrix::float4x4;

pub struct SkeletalComposer {
    pub global_playback_speed: Cell<f32>,
    pub layers: Vec<SkeletalLayer>,

    time: f64
}

impl SkeletalComposer {
    pub fn new(global_playback_speed: f32, layers: Vec<SkeletalLayer>) -> SkeletalComposer {
        SkeletalComposer {
            global_playback_speed: Cell::new(global_playback_speed),
            layers,
            time: 0.0
        }
    }

    pub fn update(&mut self, delta_time: f64) {
        self.time = self.time + delta_time;
    }

    pub fn sample(&self, entity: &SkeletalMesh) -> Vec<float4x4> {
        let layer = &self.layers[0];
        return sample_bear(entity, &layer.anim, (self.time as f32) % layer.anim.max_time);
    }
}