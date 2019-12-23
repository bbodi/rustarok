use crate::systems::snapshot_sys::GameSnapshots;
use rustarok_common::common::EngineTime;
use rustarok_common::components::controller::ControllerComponent;
use rustarok_common::packets::to_server::ToServerPacket;
use specs::prelude::*;

// Singleton
pub struct IntentionSenderSystem {
    cid: u32,
}

impl IntentionSenderSystem {
    pub fn new() -> IntentionSenderSystem {
        IntentionSenderSystem { cid: 0 }
    }
}

impl<'a> System<'a> for IntentionSenderSystem {
    type SystemData = (
        ReadStorage<'a, ControllerComponent>,
        ReadExpect<'a, EngineTime>,
        WriteExpect<'a, Vec<ToServerPacket>>,
        WriteExpect<'a, GameSnapshots>,
    );

    fn run(
        &mut self,
        (mut controller_storage, time, mut to_server, mut snapshots): Self::SystemData,
    ) {
        let ok = time.tick % 3 == 0;
        for controller in (&controller_storage).join() {
            let controller: &ControllerComponent = controller;
            if ok {
                if let Some(ref intention) = controller.intention {
                    self.cid += 1;
                    snapshots.set_client_last_command_id(self.cid);
                    to_server.push(ToServerPacket::Intention {
                        cid: self.cid,
                        client_tick: time.tick,
                        intention: intention.clone(),
                    });
                }
            }
            snapshots.add_intention((self.cid, controller.intention.clone()));
        }
    }
}
