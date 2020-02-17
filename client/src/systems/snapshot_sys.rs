use specs::prelude::*;

use rustarok_common::common::{float_cmp, EngineTime, LocalTime, ServerTime, SimulationTick};
use rustarok_common::components::char::{
    CharState, LocalCharEntityId, LocalCharStateComp, ServerEntityId,
};
use rustarok_common::components::controller::PlayerIntention;
use rustarok_common::packets::from_server::ServerEntityStateLocal;

use crate::components::char::HasServerIdComponent;
use std::collections::HashMap;

struct CharSnapshots {
    server_id: ServerEntityId,
    snapshots: [LocalCharStateComp; SnapshotStorage::SNAPSHOT_COUNT],
}

impl CharSnapshots {
    fn get_snapshot(&self, tick: u64) -> &LocalCharStateComp {
        return &self.snapshots[index(tick)];
    }

    fn get_mut_snapshot(&mut self, tick: u64) -> &mut LocalCharStateComp {
        return &mut self.snapshots[index(tick)];
    }

    fn add(&mut self, tail: u64, char_snapshot: LocalCharStateComp) {
        *self.get_mut_snapshot(tail) = char_snapshot;
    }

    pub fn print_snapshots(
        &self,
        last_acknowledged_index: u64,
        intentions: &[(u32, Option<PlayerIntention>); SnapshotStorage::SNAPSHOT_COUNT],
        from: i32,
        to: usize,
    ) {
        let from = from.abs() as u64;
        if from > last_acknowledged_index {
            return;
        }
        let text = std::ops::RangeInclusive::new(
            last_acknowledged_index - from,
            last_acknowledged_index + to as u64,
        )
        .map(|it| {
            let snapshot = self.get_snapshot(it);
            let (cid, _intention) = &intentions[index(it)];
            let mut line = format!(
                "{}, cid: {}, pos: ({}, {}), state: - {}",
                it,
                cid,
                snapshot.pos().x,
                snapshot.pos().y,
                snapshot.state().name(),
            );
            if last_acknowledged_index == it {
                line.insert_str(0, "X - ");
            }
            line
        })
        .collect::<Vec<String>>()
        .join("\n");
        log::trace!("\n{}", text);
    }
}

pub struct SnapshotStorage {
    client_last_command_id: u32,
    // TODO: do we need u64?
    last_acknowledged_index: u64,
    last_acknowledged_index_for_server_entities: u64,
    tail: u64,
    last_rollback_at: SimulationTick,
    // last_predicted_index + 1
    snapshots_for_each_char: Vec<CharSnapshots>,
    intentions: [(u32, Option<PlayerIntention>); SnapshotStorage::SNAPSHOT_COUNT],
}

pub enum ServerAckResult {
    Rollback {
        repredict_this_many_frames: usize,
    },
    RemoteEntityCorrection,
    ServerIsAheadOfClient {
        server_state_updates: Vec<ServerEntityStateLocal>,
    },
    Ok,
}

impl ServerAckResult {
    pub fn is_rollback(&self) -> bool {
        match self {
            ServerAckResult::Rollback { .. } => true,
            _ => false,
        }
    }
}

impl SnapshotStorage {
    const SNAPSHOT_COUNT: usize = 64;
    pub fn new() -> SnapshotStorage {
        SnapshotStorage {
            client_last_command_id: 0,
            last_rollback_at: SimulationTick::new(),
            last_acknowledged_index: 0,
            last_acknowledged_index_for_server_entities: 0,
            tail: 1,
            intentions: unsafe {
                let mut arr: [(u32, Option<PlayerIntention>); SnapshotStorage::SNAPSHOT_COUNT] =
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
        initial_state: LocalCharStateComp,
    ) {
        let arr = unsafe {
            let mut arr: [LocalCharStateComp; SnapshotStorage::SNAPSHOT_COUNT] =
                std::mem::MaybeUninit::zeroed().assume_init();
            for item in &mut arr[..] {
                std::ptr::write(item, initial_state.clone());
            }
            arr
        };
        self.snapshots_for_each_char.push(CharSnapshots {
            server_id,
            snapshots: arr,
        });
    }

    pub fn set_client_last_command_id(&mut self, cid: u32) {
        self.client_last_command_id = cid;
    }

    pub fn tick(&mut self) {
        self.tail += 1;
    }

    pub fn print_snapshots_for(&self, index: usize, from: i32, to: usize) {
        self.snapshots_for_each_char[index].print_snapshots(
            self.last_acknowledged_index,
            &self.intentions,
            from,
            to,
        );
    }

    pub fn set_predicted_state(&mut self, index: usize, state: &LocalCharStateComp) {
        let char_snapshots = &mut self.snapshots_for_each_char[index];
        char_snapshots.add(self.tail, state.clone());
    }

    pub fn get_acked_state_for(&self, index: usize) -> &LocalCharStateComp {
        let char_snapshots = &self.snapshots_for_each_char[index];
        return &char_snapshots.get_snapshot(self.last_acknowledged_index);
    }

    pub fn get_last_rollback_at(&self) -> SimulationTick {
        self.last_rollback_at
    }

    fn set_state(&mut self, index: usize, tick: u64, state: &LocalCharStateComp) {
        let char_snapshots = &mut self.snapshots_for_each_char[index];
        char_snapshots.add(tick, state.clone());
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

    pub fn get_tail(&self) -> u64 {
        self.tail
    }

    pub fn get_last_acknowledged_index(&self) -> u64 {
        self.last_acknowledged_index
    }

    pub fn get_last_acknowledged_index_for_server_entities(&self) -> u64 {
        self.last_acknowledged_index_for_server_entities
    }

    pub fn init(&mut self, id: ServerEntityId, char_snapshot: &LocalCharStateComp) {
        self.last_acknowledged_index = 0;
        self.tail = 1;
        // HACK: so initial Ack packets can compare with something...
        self.add_predicting_entity(id, char_snapshot.clone());
        self.set_state(0, 1, &char_snapshot);
        self.set_state(0, 2, &char_snapshot);
        self.set_state(0, 3, &char_snapshot);
    }

    pub fn get_unacked_prediction_count(&self) -> usize {
        // why "-1"? tail: 25, last index: 22, we have 23, 24 as unacked predictions
        return ((self.tail - self.last_acknowledged_index) - 1) as usize;
    }

    pub fn ack_arrived(
        &mut self,
        client_tick: SimulationTick,
        acked_cid: u32,
        snapshots_from_server: Vec<ServerEntityStateLocal>,
    ) -> ServerAckResult {
        // it assumes that
        // - the server sends the deltas in increasing order
        // - entities that are created are at the end of the vec
        // - the server does not send msg for an entity which has not been registered first

        log::trace!(
            "unacked preds: {}, \
             acked_cid: {}, client_tick: {:?}\n, \
             last_acknowledged_index: {}, tail: {}",
            self.get_unacked_prediction_count(),
            // not store any prediction yet for the current frame"
            acked_cid,
            client_tick,
            self.last_acknowledged_index,
            self.tail,
        );

        if self.get_unacked_prediction_count() == 0 {
            log::trace!("server_is_ahead_of_client",);
            return ServerAckResult::ServerIsAheadOfClient {
                server_state_updates: snapshots_from_server,
            };
        } else {
            // LOCAL CLIENT
            let (mut need_rollback, increase_index_ack_by) = {
                let char_snapshots = &self.snapshots_for_each_char[0];
                let snapshot_from_server = &snapshots_from_server[0].char_snapshot;
                self.ack_arrived_for_local_player(
                    client_tick,
                    acked_cid,
                    char_snapshots,
                    snapshot_from_server,
                )
            };

            let rollback_only_local_client = need_rollback;
            {
                if !need_rollback {
                    need_rollback = self.ack_arrived_for_server_entities(
                        &self.snapshots_for_each_char[1..],
                        &snapshots_from_server[1..],
                    );
                    if need_rollback {
                        log::trace!("before for REMOTE");
                        let to = (self.tail - self.last_acknowledged_index) as usize;
                        if log::log_enabled!(log::Level::Trace) {
                            self.print_snapshots_for(1, -2, to);
                        }
                    }
                } else {
                    log::trace!("before for LOCAL");
                    let to = (self.tail - self.last_acknowledged_index) as usize;
                    if log::log_enabled!(log::Level::Trace) {
                        self.print_snapshots_for(0, -2, to);
                    }
                }
            }

            // TODO: ugly
            if increase_index_ack_by > 1 {
                self.last_acknowledged_index += increase_index_ack_by - 1;
            }

            if increase_index_ack_by > 0 {
                self.last_acknowledged_index += 1;
            }

            self.last_acknowledged_index_for_server_entities += 1;

            return if need_rollback {
                // overwrite local player's state
                if rollback_only_local_client {
                    self.set_state(
                        0,
                        self.last_acknowledged_index,
                        &snapshots_from_server[0].char_snapshot,
                    );
                }
                self.last_acknowledged_index_for_server_entities = self.tail - 1;
                SnapshotStorage::overwrite_all(
                    self.last_acknowledged_index_for_server_entities,
                    &snapshots_from_server[1..],
                    &mut self.snapshots_for_each_char[1..],
                );

                self.last_rollback_at = client_tick;
                return if rollback_only_local_client {
                    if self.tail > self.last_acknowledged_index {
                        ServerAckResult::Rollback {
                            repredict_this_many_frames: (self.tail
                                - self.last_acknowledged_index
                                - 1)
                                as usize,
                        }
                    } else {
                        // it can happen when the client cannot process the packets for a while
                        // and then it process all of them at once
                        log::warn!(
                            "tail was fixed {} -> {}",
                            self.tail,
                            self.last_acknowledged_index + 1
                        );
                        self.tail = self.last_acknowledged_index + 1;
                        panic!();
                    }
                } else {
                    ServerAckResult::RemoteEntityCorrection
                };
            } else {
                ServerAckResult::Ok
            };
        }
    }

    fn ack_arrived_for_local_player(
        &self,
        client_tick: SimulationTick,
        acked_cid: u32,
        predicted_snapshots: &CharSnapshots,
        snapshot_from_server: &LocalCharStateComp,
    ) -> (bool, u64) {
        let predicted_snapshot = predicted_snapshots.get_snapshot(self.last_acknowledged_index + 1);
        let cid_at_prediction = self.intentions[index(self.last_acknowledged_index + 1)].0;

        log::trace!("cid_at_prediction: {}", cid_at_prediction,);

        if acked_cid < cid_at_prediction {
            log::trace!("acked_cid < predicted_snapshot");
            // The server did not get my command yet.
            // Check if my prediction was correct
            let mut need_rollback =
                !SnapshotStorage::snapshots_match(&snapshot_from_server, &predicted_snapshot);
            if need_rollback {
                // don't rollback, wait for the ack with the current cid
                return (false, 0);
            } else {
                return (false, 1);
            }
        } else if acked_cid > cid_at_prediction {
            // Client might have been too fast and assigned a smaller cid to a prediction than the server
            let mut need_rollback =
                !SnapshotStorage::snapshots_match(&snapshot_from_server, &predicted_snapshot);
            if need_rollback {
                // Client might have been too fast and generated unnecessary predictions
                for i in 1..=SnapshotStorage::SNAPSHOT_COUNT as u64 {
                    let pred = predicted_snapshots.get_snapshot(self.last_acknowledged_index + i);
                    let cid = self.intentions[index(self.last_acknowledged_index + i)].0;
                    if acked_cid == cid {
                        log::trace!("Found after {}", i);
                        log::trace!(
                            "found_predicted: v2({}, {}), acked: v2({}, {})",
                            pred.pos().x,
                            pred.pos().y,
                            snapshot_from_server.pos().x,
                            snapshot_from_server.pos().y
                        );
                        return if SnapshotStorage::snapshots_match(&snapshot_from_server, &pred) {
                            (false, i)
                        } else {
                            (true, 1)
                        };
                    }
                }
                log::trace!("Not found :(");
            }
            return (need_rollback, 1);
        } else {
            let need_rollback =
                !SnapshotStorage::snapshots_match(&snapshot_from_server, &predicted_snapshot);
            return (need_rollback, 1);
        };
    }

    fn ack_arrived_for_server_entities(
        &self,
        predictions: &[CharSnapshots],
        snapshots_from_server: &[ServerEntityStateLocal],
    ) -> bool {
        for server_state_index in 0..snapshots_from_server.len() {
            let snapshot_from_server = &snapshots_from_server[server_state_index];
            let server_entity_prediction_storage = &predictions[server_state_index];
            if server_entity_prediction_storage.server_id != snapshot_from_server.id {
                // the current char snapshot did not get an update
                // TODO: in the future the server won't send updates if there were no changes for an entity
                log::warn!(
                    "{:?} != {:?} at index {}",
                    predictions[server_state_index].server_id,
                    snapshots_from_server[server_state_index].id,
                    server_state_index
                );
                panic!();
            }
            let predicted_snapshot = server_entity_prediction_storage
                .get_snapshot(self.last_acknowledged_index_for_server_entities + 1);
            let need_rollback = !SnapshotStorage::snapshots_match(
                &snapshot_from_server.char_snapshot,
                predicted_snapshot,
            );
            if need_rollback {
                return true;
            }
        }
        return false;
    }

    pub fn overwrite_states(&mut self, states: &[ServerEntityStateLocal]) {
        SnapshotStorage::overwrite_all(
            self.last_acknowledged_index_for_server_entities,
            &states,
            &mut self.snapshots_for_each_char,
        );
    }

    fn overwrite_all(
        tick: u64,
        server_state_updates: &[ServerEntityStateLocal],
        predictions: &mut [CharSnapshots],
    ) {
        for server_state_index in 0..server_state_updates.len() {
            let server_state = &server_state_updates[server_state_index];
            let mut prediction = &mut predictions[server_state_index];
            if prediction.server_id != server_state.id {
                // the current char snapshot did not get an update
                // TODO: in the future the server won't send updates if there were no changes for an entity
                log::warn!(
                    "{:?} != {:?} at index {}",
                    predictions[server_state_index].server_id,
                    server_state_updates[server_state_index].id,
                    server_state_index
                );
                panic!();
            }
            *prediction.get_mut_snapshot(tick) = server_state.char_snapshot.clone();
        }
    }

    fn snapshots_match(acked: &LocalCharStateComp, predicted: &LocalCharStateComp) -> bool {
        let mut matches = float_cmp(acked.pos().x, predicted.pos().x)
            && float_cmp(acked.pos().y, predicted.pos().y);

        let predicted_state = predicted.state();
        let acked_state = acked.state();

        matches &= match predicted_state {
            CharState::Walking(..) => {
                match acked_state {
                    CharState::Idle => false,
                    CharState::ReceivingDamage => false,
                    CharState::Dead => false,
                    CharState::Attacking { .. } => false,
                    CharState::StandBy => false,
                    CharState::Walking(..) => {
                        // if the pos are the same but the target pos differs, don't repredict
                        // TODO but set the new target pos for remote entities
                        true
                    }
                }
            }
            _ => std::mem::discriminant(predicted_state) == std::mem::discriminant(acked_state),
        };
        if !matches {
            log::trace!(
                "predicted: ({}, {}, {:?}) !!!=== acked: v({}, {}, {:?})",
                predicted.pos().x,
                predicted.pos().y,
                predicted.state(),
                acked.pos().x,
                acked.pos().y,
                acked.state(),
            );
        } else {
            log::trace!(
                "predicted: ({}, {}, {}) == acked: ({}, {}, {})",
                predicted.pos().x,
                predicted.pos().y,
                predicted.state().name(),
                acked.pos().x,
                acked.pos().y,
                acked.state().name(),
            );
        }
        matches
    }

    pub fn load_last_acked_remote_entities_state_into_world(
        &self,
        entities: &specs::Entities,
        auth_storage: &mut WriteStorage<LocalCharStateComp>,
        server_id_storage: &ReadStorage<HasServerIdComponent>,
        tick_index: u64,
        skip_index: Option<usize>,
    ) {
        for (i, (entity_id, server_id, auth_state)) in (entities, server_id_storage, auth_storage)
            .join()
            .enumerate()
        {
            if let Some(skip_index) = skip_index {
                if skip_index == i {
                    // don't override local player's prediction
                    continue;
                }
            }
            let char_snapshots = &self.snapshots_for_each_char[i];
            if server_id.server_id != char_snapshots.server_id {
                panic!(
                    "server_id {:?} != char server id {:?}",
                    server_id, char_snapshots.server_id
                );
            }

            let snapshot = char_snapshots.get_snapshot(tick_index);
            *auth_state = snapshot.clone();
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
        ReadStorage<'a, LocalCharStateComp>,
        ReadStorage<'a, HasServerIdComponent>,
        WriteExpect<'a, SnapshotStorage>,
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

fn index(tick: u64) -> usize {
    (tick % SnapshotStorage::SNAPSHOT_COUNT as u64) as usize
}
