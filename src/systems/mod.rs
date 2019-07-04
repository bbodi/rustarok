use std::time::Instant;
use crate::{Shaders, SpriteResource, Tick, RenderMatrices, MapRenderData, DeltaTime};
use std::collections::HashMap;

pub mod ai;
pub mod input;
pub mod phys;
pub mod render;

pub struct SystemVariables {
    pub shaders: Shaders,
    pub sprite_resources: Vec<SpriteResource>,
    pub head_sprites: Vec<SpriteResource>,
    pub tick: Tick,
    pub dt: DeltaTime,
    pub matrices: RenderMatrices,
    pub map_render_data: MapRenderData,
}

pub struct SystemFrameDurations(pub HashMap<&'static str, u32>);

impl SystemFrameDurations {
    pub fn system_finished(&mut self, started: Instant, name: &'static str) {
        let duration = Instant::now().duration_since(started).as_millis() as u32;
        self.0.insert(name, duration);
    }

    pub fn start_measurement(&mut self, name: &'static str) -> SystemStopwatch {
        SystemStopwatch::new(name, self)
    }
}

pub struct SystemStopwatch<'a> {
    // let now_ms = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis();
    started: Instant,
    name: &'static str,
    times: &'a mut SystemFrameDurations,
}

impl<'a> SystemStopwatch<'a> {
    pub fn new(name: &'static str, times: &'a mut SystemFrameDurations) -> SystemStopwatch<'a> {
        SystemStopwatch {
            started: Instant::now(),
            name,
            times,
        }
    }
}

impl<'a> Drop for SystemStopwatch<'a> {
    fn drop(&mut self) {
        self.times.system_finished(self.started, self.name);
    }
}