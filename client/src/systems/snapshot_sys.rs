use specs::prelude::*;

use rustarok_common::common::float_cmp;
use rustarok_common::components::char::{AuthorizedCharStateComponent, ServerEntityId};
use rustarok_common::components::controller::PlayerIntention;
use rustarok_common::components::snapshot::CharSnapshot;
use rustarok_common::packets::from_server::ServerEntityState;

use crate::components::char::{DebugServerAckComponent, HasServerIdComponent};

struct CharSnapshots {
    server_id: ServerEntityId,
    snapshots: [CharSnapshot; GameSnapshots::SNAPSHOT_COUNT],
}

impl CharSnapshots {
    fn get_snapshot(&self, tick: u64) -> &CharSnapshot {
        return &self.snapshots[index(tick)];
    }

    fn get_mut_snapshot(&mut self, tick: u64) -> &mut CharSnapshot {
        return &mut self.snapshots[index(tick)];
    }

    fn add(&mut self, tail: u64, char_snapshot: CharSnapshot) {
        *self.get_mut_snapshot(tail) = char_snapshot;
    }

    pub fn print_snapshots(
        &self,
        last_acknowledged_index: u64,
        intentions: &[(u32, Option<PlayerIntention>); GameSnapshots::SNAPSHOT_COUNT],
        from: i32,
        to: u64,
    ) {
        let from = from.abs() as u64;
        if from > last_acknowledged_index {
            return;
        }
        let text = std::ops::RangeInclusive::new(
            last_acknowledged_index - from,
            last_acknowledged_index + to,
        )
        .map(|it| {
            let snapshot = self.get_snapshot(it);
            let x = snapshot.state.pos().x;
            let (cid, intention) = &intentions[index(it)];
            if last_acknowledged_index == it {
                format!("X ({}, {}) - {}", cid, x, intention.is_some())
            } else {
                format!("({}, {}) - {}", cid, x, intention.is_some())
            }
        })
        .collect::<Vec<String>>()
        .join("\n");
        log::debug!("{}", text);
    }
}

pub struct GameSnapshots {
    client_last_command_id: u32,
    // TODO: do we need u64?
    last_acknowledged_index: u64,
    tail: u64,
    had_rollback: bool,
    last_acknowledged_tick: u64,
    // last_predicted_index + 1
    snapshots_for_each_char: Vec<CharSnapshots>,
    intentions: [(u32, Option<PlayerIntention>); GameSnapshots::SNAPSHOT_COUNT],
}

pub enum ServerAckResult {
    Rollback { repredict_this_many_frames: u64 },
    Ok,
}

impl GameSnapshots {
    const SNAPSHOT_COUNT: usize = 64;
    pub fn new() -> GameSnapshots {
        GameSnapshots {
            had_rollback: false,
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
            snapshots_for_each_char: Vec::with_capacity(64),
        }
    }

    pub fn add_predicting_entity(
        &mut self,
        server_id: ServerEntityId,
        initial_state: AuthorizedCharStateComponent,
    ) {
        let arr = unsafe {
            let mut arr: [CharSnapshot; GameSnapshots::SNAPSHOT_COUNT] =
                std::mem::MaybeUninit::zeroed().assume_init();
            for item in &mut arr[..] {
                std::ptr::write(item, CharSnapshot::from(&initial_state));
            }
            arr
        };
        self.snapshots_for_each_char.push(CharSnapshots {
            server_id,
            snapshots: arr,
        });
    }

    pub fn set_client_last_command_id(&mut self, cid: u32) {
        self.client_last_command_id = dbg!(cid);
    }

    pub fn tick(&mut self) {
        self.tail += 1;
    }

    pub fn print_snapshots_for(&self, index: usize, from: i32, to: u64) {
        self.snapshots_for_each_char[index].print_snapshots(
            self.last_acknowledged_index,
            &self.intentions,
            from,
            to,
        );
    }

    pub fn set_predicted_state(&mut self, index: usize, state: &AuthorizedCharStateComponent) {
        let char_snapshots = &mut self.snapshots_for_each_char[index];
        char_snapshots.add(self.tail, CharSnapshot::from(state));
    }

    pub fn get_acked_state_for(&self, index: usize) -> &AuthorizedCharStateComponent {
        let char_snapshots = &self.snapshots_for_each_char[index];
        return &char_snapshots
            .get_snapshot(self.last_acknowledged_index)
            .state;
    }

    pub fn had_been_rollback_in_this_tick(&self) -> bool {
        self.had_rollback
    }

    fn set_state(&mut self, index: usize, tick: u64, state: &AuthorizedCharStateComponent) {
        let char_snapshots = &mut self.snapshots_for_each_char[index];
        char_snapshots.add(tick, CharSnapshot::from(state));
    }

    pub fn add_intention(&mut self, intention: (u32, Option<PlayerIntention>)) {
        self.intentions[index(self.tail)] = intention;
    }

    pub fn pop_intention(&mut self) -> (u32, Option<PlayerIntention>) {
        // it is called before GameSnapshots::add would be called, so tail
        // points to tick where the intention was originally made
        self.intentions[index(self.tail)].clone()
    }

    pub fn reset_tail_index(&mut self) {
        self.tail = self.last_acknowledged_index + 1;
    }

    pub fn init(&mut self, acked: &ServerEntityState) {
        self.last_acknowledged_index = 0;
        self.tail = 1;
        // HACK: so initial Ack packets can compare with something...
        self.add_predicting_entity(acked.id, acked.char_snapshot.state.clone());
        self.set_state(0, 1, &acked.char_snapshot.state);
        self.set_state(0, 2, &acked.char_snapshot.state);
        self.set_state(0, 3, &acked.char_snapshot.state);
    }

    pub fn ack_arrived(
        &mut self,
        client_tick: u64,
        acked_cid: u32,
        acked_tick: u64,
        server_state_updates: &[ServerEntityState],
    ) -> ServerAckResult {
        // it assumes that
        // - the server sends the deltas in increasing order
        // - entities that are created are at the end of the vec
        // - the server does not send msg for an entity which has not been registered first

        // LOCAL CLIENT
        let char_snapshots = &self.snapshots_for_each_char[0];
        let snapshot_from_server = &server_state_updates[0].char_snapshot;
        let (mut no_mismatch, increase_indexed_ack_by) = self.ack_arrived_for_local_player(
            client_tick,
            acked_cid,
            acked_tick,
            char_snapshots,
            snapshot_from_server,
        );
        self.last_acknowledged_index += increase_indexed_ack_by;
        let dst_index = if !no_mismatch {
            // if the local char already mispredicts, we can increase the index
            // becase this is where other predictions will be rolled back from
            self.set_state(0, self.last_acknowledged_index, &snapshot_from_server.state);
            self.last_acknowledged_index
        } else {
            0
        };

        for entry_index in 1..server_state_updates.len() {
            if self.snapshots_for_each_char[entry_index].server_id
                != server_state_updates[entry_index].id
            {
                // the current char snapshot did not get an update
                log::warn!(
                    "{:?} != {:?} at index {}",
                    self.snapshots_for_each_char[entry_index].server_id,
                    server_state_updates[entry_index].id,
                    entry_index
                );
                panic!();
            }
            let char_snapshots = &self.snapshots_for_each_char[entry_index];
            let snapshot_from_server = &server_state_updates[entry_index].char_snapshot;
            let other_char_no_mismatch = self.ack_arrived_for_other_entity(
                entry_index,
                char_snapshots,
                snapshot_from_server,
            );

            if !other_char_no_mismatch {
                self.set_state(
                    entry_index,
                    self.last_acknowledged_index + 1,
                    &snapshot_from_server.state,
                );
            }
            no_mismatch &= other_char_no_mismatch;
        }

        self.last_acknowledged_tick = acked_tick;
        self.had_rollback = !no_mismatch;
        return if no_mismatch {
            ServerAckResult::Ok
        } else {
            let repredict_this_many_frames =
                (dbg!(self.tail) - dbg!(self.last_acknowledged_index)) - 1;
            if client_tick > acked_tick {
                self.print_snapshots_for(0, -2, repredict_this_many_frames);
            }
            ServerAckResult::Rollback {
                repredict_this_many_frames,
            }
        };
    }

    fn ack_arrived_for_other_entity(
        &self,
        char_snapshot_index: usize,
        predicted_snapshots: &CharSnapshots,
        snapshot_from_server: &CharSnapshot,
    ) -> bool {
        let predicted_snapshot = predicted_snapshots.get_snapshot(self.last_acknowledged_index + 1);
        log::debug!(
            "predicted_snapshot: v2({}, {}), acked: v2({}, {})",
            predicted_snapshot.state.pos().x,
            predicted_snapshot.state.pos().y,
            snapshot_from_server.state.pos().x,
            snapshot_from_server.state.pos().y
        );
        let misprediction =
            !GameSnapshots::compare_snapshots(&snapshot_from_server, &predicted_snapshot);

        misprediction == false
    }

    fn ack_arrived_for_local_player(
        &self,
        client_tick: u64,
        acked_cid: u32,
        acked_tick: u64,
        predicted_snapshots: &CharSnapshots,
        snapshot_from_server: &CharSnapshot,
    ) -> (bool, u64) {
        let mut increase_indexed_ack_by = 0;
        let predicted_snapshot = predicted_snapshots.get_snapshot(self.last_acknowledged_index + 1);
        let cid_at_prediction = self.intentions[index(self.last_acknowledged_index + 1)].0;
        log::debug!(
            "server diff: {}, ack diff: {}, last_acked_tick: {}, \
             acked_tick: {}, acked_cid: {}, client_tick: {}\n, cid: {}",
            client_tick as i64 - acked_tick as i64,
            client_tick as i64 - self.last_acknowledged_tick as i64,
            self.last_acknowledged_tick,
            acked_tick,
            acked_cid,
            client_tick,
            cid_at_prediction,
        );
        let misprediction = if acked_cid < cid_at_prediction {
            log::debug!(
                "acked_cid < predicted_snapshot.cid: v2({}, {}), acked: v2({}, {})",
                predicted_snapshot.state.pos().x,
                predicted_snapshot.state.pos().y,
                snapshot_from_server.state.pos().x,
                snapshot_from_server.state.pos().y
            );
            // The server did not get my command yet.
            // Check if my prediction was correct
            let mut misprediction =
                !GameSnapshots::compare_snapshots(&snapshot_from_server, &predicted_snapshot);
            if !misprediction {
                increase_indexed_ack_by = 1;
            }

            false
        } else if acked_cid > cid_at_prediction {
            // Client might have been too fast and assigned a smaller cid to a prediction than the server
            log::debug!(
                "cur_predicted2: v2({}, {}), acked: v2({}, {})",
                predicted_snapshot.state.pos().x,
                predicted_snapshot.state.pos().y,
                snapshot_from_server.state.pos().x,
                snapshot_from_server.state.pos().y
            );
            let mut misprediction =
                !GameSnapshots::compare_snapshots(&snapshot_from_server, &predicted_snapshot);
            if misprediction {
                // Client might have been too fast and generated unnecessary predictions
                for i in 1..=GameSnapshots::SNAPSHOT_COUNT as u64 {
                    let pred = predicted_snapshots.get_snapshot(self.last_acknowledged_index + i);
                    let cid = self.intentions[index(self.last_acknowledged_index + i)].0;
                    if acked_cid == cid {
                        log::debug!("Found after {}", i);
                        log::debug!(
                            "found_predicted: v2({}, {}), acked: v2({}, {})",
                            pred.state.pos().x,
                            pred.state.pos().y,
                            snapshot_from_server.state.pos().x,
                            snapshot_from_server.state.pos().y
                        );
                        misprediction =
                            !GameSnapshots::compare_snapshots(&snapshot_from_server, &pred);
                        if misprediction {
                            increase_indexed_ack_by = 1;
                        } else {
                            increase_indexed_ack_by = i;
                        }
                        break;
                    }
                }
            } else {
                increase_indexed_ack_by = 1;
                log::debug!("match");
            }
            misprediction
        } else {
            log::debug!(
                "predicted_snapshot: v2({}, {}), acked: v2({}, {})",
                predicted_snapshot.state.pos().x,
                predicted_snapshot.state.pos().y,
                snapshot_from_server.state.pos().x,
                snapshot_from_server.state.pos().y
            );
            let misprediction =
                !GameSnapshots::compare_snapshots(&snapshot_from_server, &predicted_snapshot);
            increase_indexed_ack_by = 1;
            misprediction
        };

        (!misprediction, increase_indexed_ack_by)
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

    pub fn load_last_acked_state_into_world(
        &self,
        entities: &specs::Entities,
        auth_storage: &mut WriteStorage<AuthorizedCharStateComponent>,
        server_id_storage: &ReadStorage<HasServerIdComponent>,
        debug_ack_storage: &mut WriteStorage<DebugServerAckComponent>,
    ) {
        for (i, (entity_id, _server_id, auth_state)) in (entities, server_id_storage, auth_storage)
            .join()
            .enumerate()
        {
            let snapshot =
                &self.snapshots_for_each_char[i].get_snapshot(self.last_acknowledged_index);

            auth_state.set_pos(snapshot.state.pos());
            auth_state.set_state(snapshot.state.state().clone(), snapshot.state.dir());
            auth_state.target = snapshot.state.target.clone();
        }
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
        ReadStorage<'a, HasServerIdComponent>,
        WriteExpect<'a, GameSnapshots>,
    );

    fn run(
        &mut self,
        (auth_char_state_storage, server_id_storage, mut snapshots): Self::SystemData,
    ) {
        let mut i = 0;
        // TODO check prediction overflow
        //        if index(self.tail + 1) == index(self.last_acknowledged_index) {
        //            //            TODO
        //            // we are out of space :(
        //            //            panic!();
        //        }
        for (_server_id, auth_char_state) in (&server_id_storage, &auth_char_state_storage).join() {
            snapshots.set_predicted_state(i, auth_char_state);
            i += 1;
        }
    }
}

pub struct DebugServerAckComponentFillerSystem;

impl<'a> System<'a> for DebugServerAckComponentFillerSystem {
    type SystemData = (
        ReadStorage<'a, AuthorizedCharStateComponent>,
        ReadStorage<'a, HasServerIdComponent>,
        WriteStorage<'a, DebugServerAckComponent>,
        ReadExpect<'a, GameSnapshots>,
    );

    fn run(
        &mut self,
        (auth_char_state_storage, server_id_storage, mut debug_ack_storage, snapshots): Self::SystemData,
    ) {
        let mut i = 0;

        for (_server_id, auth_char_state, debug_ack) in (
            &server_id_storage,
            &auth_char_state_storage,
            &mut debug_ack_storage,
        )
            .join()
        {
            debug_ack.acked_snapshot = CharSnapshot::from(snapshots.get_acked_state_for(i));
            debug_ack.had_rollback = snapshots.had_been_rollback_in_this_tick();
            i += 1;
        }
    }
}

fn index(tick: u64) -> usize {
    (tick % GameSnapshots::SNAPSHOT_COUNT as u64) as usize
}
