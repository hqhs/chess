/// Infinite debug grid,
/// Algorithm descriptionhttps://asliceofrendering.com/scene%20helper/2020/01/05/InfiniteGrid/
use std::{borrow::Cow, f32::consts, mem};

use bytemuck::{Pod, Zeroable};
use discipline::{
    glam::{self, Mat4},
    wgpu::{self, util::DeviceExt},
};

pub struct Grid {}

impl Grid {
    pub fn new(
        format: wgpu::TextureFormat,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        camera_view: &Mat4,
    ) -> Self {
        Self {}
    }
}
