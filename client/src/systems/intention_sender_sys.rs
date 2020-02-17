use crate::components::char::HasServerIdComponent;
use crate::components::controller::LocalPlayerController;
use crate::systems::snapshot_sys::SnapshotStorage;
use rustarok_common::common::SimulationTick;
use rustarok_common::components::controller::{PlayerIntention, ToServerPlayerIntention};
use rustarok_common::packets::to_server::ToServerPacket;
use specs::prelude::*;
use std::time::{Duration, Instant};

// Singleton
pub struct IntentionSenderSystem {
    cid: u32,
    time_between_inputs: Duration,
    last_input_at: Instant,
}

impl IntentionSenderSystem {
    pub fn new(input_freq: usize) -> IntentionSenderSystem {
        IntentionSenderSystem {
            cid: 0,
            last_input_at: Instant::now(),
            time_between_inputs: Duration::from_millis((1000 / input_freq) as u64),
        }
    }
}

pub struct ClientCommandId(u32);
impl ClientCommandId {
    pub fn new() -> ClientCommandId {
        ClientCommandId(0)
    }

    pub fn inc(&mut self, snapshots: &mut SnapshotStorage) {
        self.0 += 1;
        snapshots.set_client_last_command_id(self.0);
    }

    pub fn as_u32(&self) -> u32 {
        self.0
    }
}

impl<'a> System<'a> for IntentionSenderSystem {
    type SystemData = (
        ReadStorage<'a, HasServerIdComponent>,
        ReadExpect<'a, LocalPlayerController>,
        ReadExpect<'a, SimulationTick>,
        WriteExpect<'a, Vec<ToServerPacket>>,
        WriteExpect<'a, SnapshotStorage>,
        WriteExpect<'a, ClientCommandId>,
    );

    fn run(
        &mut self,
        (server_id_storage, local_player, tick, mut to_server, mut snapshots, mut cid): Self::SystemData,
    ) {
        if let Some(ref intention) = local_player.controller.intention {
            cid.inc(&mut snapshots);
            log::debug!("CID: {}, new command: {:?}", self.cid, &intention);
            to_server.push(ToServerPacket::Intention {
                cid: cid.as_u32(),
                client_tick: *tick,
                intention: match intention {
                    PlayerIntention::MoveTo(v) => ToServerPlayerIntention::MoveTo(*v),
                    PlayerIntention::MoveTowardsMouse(v) => {
                        ToServerPlayerIntention::MoveTowardsMouse(*v)
                    }
                    PlayerIntention::AttackTowards(v) => ToServerPlayerIntention::AttackTowards(*v),
                    PlayerIntention::Attack(local_id) => ToServerPlayerIntention::Attack(
                        server_id_storage.get((*local_id).into()).unwrap().server_id,
                    ),
                },
            });
        }
        snapshots.add_intention((cid.as_u32(), local_player.controller.intention.clone()));
    }
}
