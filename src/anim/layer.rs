use std::cell::Cell;
use crate::anim::skeletal::SkeletalAnimation;

pub struct SkeletalLayer {
    pub spec: SkeletalLayerSpec,
    pub anim: SkeletalAnimation
}

pub struct SkeletalLayerSpec {
    pub loopanim: bool,
    pub playback_speed: Cell<f32>
}