use std::ops::RangeInclusive;

use specs::prelude::*;

use rustarok_common::common::{float_cmp, v2, EngineTime, Vec2};
use rustarok_common::components::char::{
    AuthorizedCharStateComponent, CharDir, CharState, ControllerEntityId,
};
use rustarok_common::components::controller::{ControllerComponent, PlayerIntention};
use rustarok_common::components::snapshot::{CharSnapshot, WorldSnapshot};
use rustarok_common::packets::from_server::{AckEntry, FromServerPacket};
use rustarok_common::packets::to_server::ToServerPacket;
use rustarok_common::packets::{PacketHandlerThread, SocketId};

use crate::components::controller::LocalPlayerControllerComponent;

pub struct GameSnapshots {
    client_last_command_id: u32,
    // TODO: do we need u64?
    last_acknowledged_index: u64,
    tail: u64,
    last_acknowledged_tick: u64,
    // last_predicted_index + 1
    snapshots: [CWorldSnapshot; GameSnapshots::SNAPSHOT_COUNT],
    intentions: [(u32, Option<PlayerIntention>); GameSnapshots::SNAPSHOT_COUNT],
}

#[derive(Default)]
struct CWorldSnapshot {
    cid: u32,
    snapshot: WorldSnapshot,
}

pub enum ServerAckResult {
    Rollback { repredict_this_many_frames: u64 },
    ServerIsAheadOfClient { acked_tick: u64 },
    Ok,
}

impl GameSnapshots {
    const SNAPSHOT_COUNT: usize = 64;
    pub fn new() -> GameSnapshots {
        GameSnapshots {
            client_last_command_id: 0,
            last_acknowledged_index: 0,
            last_acknowledged_tick: 0,
            tail: 1,
            intentions: unsafe {
                let mut arr: [(u32, Option<PlayerIntention>); GameSnapshots::SNAPSHOT_COUNT] =
                    std::mem::MaybeUninit::zeroed().assume_init();
                for item in &mut arr[..] {
                    std::ptr::write(item, (0, None));
                }
                arr
            },
            snapshots: unsafe {
                let mut arr: [CWorldSnapshot; GameSnapshots::SNAPSHOT_COUNT] =
                    std::mem::MaybeUninit::zeroed().assume_init();
                for item in &mut arr[..] {
                    std::ptr::write(item, CWorldSnapshot::default());
                }
                arr
            },
        }
    }

    pub fn set_client_last_command_id(&mut self, cid: u32) {
        self.client_last_command_id = dbg!(cid);
    }

    pub fn tick(&mut self) {
        self.tail += 1;
    }

    pub fn add(&mut self, snapshot: WorldSnapshot) {
        if GameSnapshots::index(self.tail + 1) == GameSnapshots::index(self.last_acknowledged_index)
        {
            //            TODO
            // we are out of space :(
            //            panic!();
        }
        *self.get_mut_snapshot(self.tail) = CWorldSnapshot {
            cid: self.client_last_command_id,
            snapshot,
        };
    }

    pub fn add_intention(&mut self, intention: (u32, Option<PlayerIntention>)) {
        self.intentions[GameSnapshots::index(self.tail)] = intention;
    }

    pub fn pop_intention(&mut self) -> (u32, Option<PlayerIntention>) {
        // it is called before GameSnapshots::add would be called, so tail
        // points to tick where the intention was originally made
        self.intentions[GameSnapshots::index(self.tail)].clone()
    }

    pub fn reset_tail_index(&mut self) {
        self.tail = self.last_acknowledged_index + 1;
    }

    fn get_snapshot(&self, tick: u64) -> &CWorldSnapshot {
        return &self.snapshots[GameSnapshots::index(tick)];
    }

    fn get_mut_snapshot(&mut self, tick: u64) -> &mut CWorldSnapshot {
        return &mut self.snapshots[GameSnapshots::index(tick)];
    }

    pub fn init(&mut self, entries: &[AckEntry]) {
        let acked = match &entries[0] {
            AckEntry::EntityState { id, char_snapshot } => char_snapshot,
        };
        self.last_acknowledged_index = 0;
        self.tail = 1;
        // so initial Ack packets can compare with something...
        self.set_state(0, &acked.state);
        self.set_state(1, &acked.state);
        self.set_state(2, &acked.state);
        self.set_state(3, &acked.state);
    }

    fn set_state(&mut self, tick: u64, state: &AuthorizedCharStateComponent) {
        self.get_mut_snapshot(tick).snapshot.desktop_snapshot.state = state.clone();
    }

    pub fn ack_arrived(
        &mut self,
        client_tick: u64,
        acked_cid: u32,
        acked_tick: u64,
        entries: &[AckEntry],
    ) -> ServerAckResult {
        // first entry is always state of self
        let state_from_server = match &entries[0] {
            AckEntry::EntityState { id, char_snapshot } => char_snapshot,
        };
        //        debug h miért van még rollback
        //        limitáld le a input mintavételt 2-3 frame-re? h elkerüld azt h 1 klikk 3 commandot küldjön
        let last_unacknowledged_index = self.last_acknowledged_index + 1;
        let predicted_snapshot = self.get_snapshot(last_unacknowledged_index);
        log::debug!(
            "server diff: {}, ack diff: {}, last_acked_tick: {}, \
             acked_tick: {}, acked_cid: {}, client_tick: {}\n, cid: {}",
            client_tick as i64 - acked_tick as i64,
            client_tick as i64 - self.last_acknowledged_tick as i64,
            self.last_acknowledged_tick,
            acked_tick,
            acked_cid,
            client_tick,
            predicted_snapshot.cid,
        );
        if acked_tick >= client_tick {
            //            self.set_state(acked_tick, &state_from_server.pos);
            return ServerAckResult::ServerIsAheadOfClient { acked_tick };
        }
        let misprediction = if acked_cid < predicted_snapshot.cid {
            log::debug!(
                "acked_cid < predicted_snapshot.cid: v2({}, {}), acked: v2({}, {})",
                predicted_snapshot.snapshot.desktop_snapshot.state.pos().x,
                predicted_snapshot.snapshot.desktop_snapshot.state.pos().y,
                state_from_server.state.pos().x,
                state_from_server.state.pos().y
            );
            // The server did not get my command yet.
            // Check if my prediction was correct
            let mut misprediction = !GameSnapshots::compare_snapshots(
                &state_from_server,
                &predicted_snapshot.snapshot.desktop_snapshot,
            );
            if !misprediction {
                self.last_acknowledged_index += 1;
            }
            //            if misprediction {
            //                // Check if she still think where I should be if I haven't done anything
            //                let prev_snapshot = self.get_snapshot(self.last_acknowledged_index);
            //                log::debug!(
            //                    "prev_predicted: v2({}, {}), acked: v2({}, {})",
            //                    prev_snapshot.snapshot.desktop_snapshot.state.pos().x,
            //                    prev_snapshot.snapshot.desktop_snapshot.state.pos().y,
            //                    state_from_server.state.pos().x,
            //                    state_from_server.state.pos().y
            //                );
            //                misprediction = !GameSnapshots::compare_snapshots(
            //                    &state_from_server,
            //                    &prev_snapshot.snapshot.desktop_snapshot,
            //                );
            //                if misprediction {
            //                    // inc it only in case of mispred
            //                    self.last_acknowledged_index += 1;
            //                }
            //            } else {
            //                self.last_acknowledged_index += 1;
            //            }
            //            misprediction
            false
        } else if acked_cid > predicted_snapshot.cid {
            // Client might have been too fast and assigned a smaller cid to a prediction than the server
            log::debug!(
                "cur_predicted2: v2({}, {}), acked: v2({}, {})",
                predicted_snapshot.snapshot.desktop_snapshot.state.pos().x,
                predicted_snapshot.snapshot.desktop_snapshot.state.pos().y,
                state_from_server.state.pos().x,
                state_from_server.state.pos().y
            );
            let mut misprediction = !GameSnapshots::compare_snapshots(
                &state_from_server,
                &predicted_snapshot.snapshot.desktop_snapshot,
            );
            if misprediction {
                // Client might have been too fast and generated unnecessary predictions
                for i in 1..=GameSnapshots::SNAPSHOT_COUNT as u64 {
                    let pred = self.get_snapshot(self.last_acknowledged_index + i);
                    if acked_cid == pred.cid {
                        log::debug!("Found after {}", i);
                        log::debug!(
                            "found_predicted: v2({}, {}), acked: v2({}, {})",
                            pred.snapshot.desktop_snapshot.state.pos().x,
                            pred.snapshot.desktop_snapshot.state.pos().y,
                            state_from_server.state.pos().x,
                            state_from_server.state.pos().y
                        );
                        misprediction = !GameSnapshots::compare_snapshots(
                            &state_from_server,
                            &pred.snapshot.desktop_snapshot,
                        );
                        if misprediction {
                            self.last_acknowledged_index += 1;
                        } else {
                            self.last_acknowledged_index += i;
                        }
                        break;
                    }
                }
            } else {
                self.last_acknowledged_index += 1;
                log::debug!("match");
            }
            misprediction
        } else {
            log::debug!(
                "predicted_snapshot: v2({}, {}), acked: v2({}, {})",
                predicted_snapshot.snapshot.desktop_snapshot.state.pos().x,
                predicted_snapshot.snapshot.desktop_snapshot.state.pos().y,
                state_from_server.state.pos().x,
                state_from_server.state.pos().y
            );
            let misprediction = !GameSnapshots::compare_snapshots(
                &state_from_server,
                &predicted_snapshot.snapshot.desktop_snapshot,
            );
            self.last_acknowledged_index += 1;
            misprediction
        };

        let result = if misprediction {
            let repredict_this_many_frames = (self.tail - self.last_acknowledged_index) - 1;
            if client_tick > acked_tick {
                self.print_snapshots(-2, repredict_this_many_frames);
            }
            self.set_state(self.last_acknowledged_index, &state_from_server.state);
            ServerAckResult::Rollback {
                repredict_this_many_frames,
            }
        } else {
            ServerAckResult::Ok
        };
        self.last_acknowledged_tick = acked_tick;
        result
    }

    fn index(tick: u64) -> usize {
        (tick % GameSnapshots::SNAPSHOT_COUNT as u64) as usize
    }

    pub fn print_snapshots(&self, from: i32, to: u64) {
        let from = from.abs() as u64;
        if from > self.last_acknowledged_index {
            return;
        }
        let text = std::ops::RangeInclusive::new(
            self.last_acknowledged_index - from,
            self.last_acknowledged_index + to,
        )
        .map(|it| {
            let snapshot = self.get_snapshot(it);
            let x = snapshot.snapshot.desktop_snapshot.state.pos().x;
            let (_cid, intention) = &self.intentions[GameSnapshots::index(it)];
            let cid = snapshot.cid;
            if self.last_acknowledged_index == it {
                format!("X ({}, {}) - {}", cid, x, intention.is_some())
            } else {
                format!("({}, {}) - {}", cid, x, intention.is_some())
            }
        })
        .collect::<Vec<String>>()
        .join("\n");
        log::debug!("{}", text);
    }

    fn compare_snapshots(acked: &CharSnapshot, predicted: &CharSnapshot) -> bool {
        let result = float_cmp(acked.state.pos().x, predicted.state.pos().x);
        &&float_cmp(acked.state.pos().y, predicted.state.pos().y);
        //        if !result {
        //            log::debug!(
        //                "predicted: v2({}, {}), acked: v2({}, {})",
        //                predicted.state.pos().x,
        //                predicted.state.pos().y,
        //                acked.state.pos().x,
        //                acked.state.pos().y
        //            );
        //        }
        result
    }

    pub fn overwrite_cid(&mut self, from: u64, to: u64, cid: u32) {
        for i in from..=to {
            self.get_mut_snapshot(i).cid = cid;
        }
    }

    pub fn load_last_acked_state_into_world(
        &self,
        auth_storage: &mut WriteStorage<AuthorizedCharStateComponent>,
        controller_storage: &ReadStorage<ControllerComponent>, // todo singleton
        desktop_controller_id: ControllerEntityId,
    ) {
        let snapshot = self.get_snapshot(self.last_acknowledged_index);
        let controller = controller_storage
            .get(desktop_controller_id.into())
            .unwrap();
        let auth_state = auth_storage
            .get_mut(controller.controlled_entity.into())
            .unwrap();
        auth_state.set_pos(snapshot.snapshot.desktop_snapshot.state.pos());
        auth_state.set_state(
            snapshot.snapshot.desktop_snapshot.state.state().clone(),
            snapshot.snapshot.desktop_snapshot.state.dir(),
        );
        auth_state.target = snapshot.snapshot.desktop_snapshot.state.target.clone();
    }
}

pub struct SnapshotSystem;

// specs doesn't ot allow to use as a Resource (not sharable)
impl SnapshotSystem {
    pub fn new() -> SnapshotSystem {
        SnapshotSystem
    }
}

impl<'a> System<'a> for SnapshotSystem {
    type SystemData = (
        ReadStorage<'a, AuthorizedCharStateComponent>,
        ReadStorage<'a, ControllerComponent>,
        ReadStorage<'a, LocalPlayerControllerComponent>,
        WriteExpect<'a, GameSnapshots>,
        ReadExpect<'a, EngineTime>,
    );

    fn run(
        &mut self,
        (
            auth_char_state_storage,
            controller_storage,
            desktop_storage,
            mut snapshots,
            time,
        ): Self::SystemData,
    ) {
        for (controller, _desktop) in (&controller_storage, &desktop_storage).join() {
            if let Some(desktop_char) =
                auth_char_state_storage.get(controller.controlled_entity.into())
            {
                let snapshot = WorldSnapshot {
                    desktop_snapshot: CharSnapshot {
                        state: desktop_char.clone(),
                    },
                };
                snapshots.add(snapshot);
            }
        }
    }
}
