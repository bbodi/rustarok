use crate::systems::snapshot_sys::GameSnapshots;
use rustarok_common::common::EngineTime;
use specs::prelude::*;

pub struct FrameClientEndSystem;

impl<'a> System<'a> for FrameClientEndSystem {
    type SystemData = (WriteExpect<'a, EngineTime>, WriteExpect<'a, GameSnapshots>);

    fn run(&mut self, (mut time, mut snapshots): Self::SystemData) {
        snapshots.tick();
    }
}
