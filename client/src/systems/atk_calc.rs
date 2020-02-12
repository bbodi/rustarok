use nalgebra::Isometry2;
use ncollide2d::query::Proximity;
use specs::prelude::*;
use specs::LazyUpdate;

use crate::components::char::CharacterStateComponent;
use crate::components::status::status::{
    ApplyStatusComponent, ApplyStatusInAreaComponent, RemoveStatusComponent,
    RemoveStatusComponentPayload,
};
use crate::components::{FlyingNumberComponent, FlyingNumberType, SoundEffectComponent};
use crate::runtime_assets::audio::Sounds;
use crate::systems::{SystemEvent, SystemFrameDurations, SystemVariables};
use crate::{ElapsedTime, PhysicEngine};
use rustarok_common::attack::{
    ApplyForceComponent, AreaAttackComponent, DamageDisplayType, HpModificationRequest,
    HpModificationResult, HpModificationResultType, HpModificationType, WeaponType,
};
use rustarok_common::common::{percentage, EngineTime, Percentage, Vec2};
use rustarok_common::components::char::{
    AuthorizedCharStateComponent, CharEntityId, JobId, StaticCharDataComponent,
};
use rustarok_common::config::CommonConfigs;

pub struct AttackSystem {
    hp_mod_requests: Vec<HpModificationRequest>,
}

impl AttackSystem {
    pub fn new() -> AttackSystem {
        AttackSystem {
            hp_mod_requests: Vec::with_capacity(128),
        }
    }
}

impl<'a> System<'a> for AttackSystem {
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, CharacterStateComponent>,
        WriteStorage<'a, AuthorizedCharStateComponent>,
        WriteStorage<'a, StaticCharDataComponent>,
        WriteExpect<'a, SystemVariables>,
        WriteExpect<'a, PhysicEngine>,
        WriteExpect<'a, SystemFrameDurations>,
        ReadExpect<'a, EngineTime>,
        WriteExpect<'a, Vec<HpModificationRequest>>,
        WriteExpect<'a, Vec<AreaAttackComponent>>,
        WriteExpect<'a, Vec<ApplyForceComponent>>,
        WriteExpect<'a, CommonConfigs>,
        Write<'a, LazyUpdate>,
        Option<Write<'a, Vec<SystemEvent>>>,
    );

    fn run(
        &mut self,
        (
            entities,
            mut char_state_storage,
            mut auth_char_state_storage,
            mut static_char_data_storage,
            mut sys_vars,
            mut physics_world,
            mut system_benchmark,
            time,
            mut hp_mod_requests,
            mut area_hp_mod_requests,
            mut pushes,
            dev_configs,
            mut updater,
            mut events,
        ): Self::SystemData,
    ) {
        if !time.can_simulation_run() {
            return;
        }
        let _stopwatch = system_benchmark.start_measurement("AttackSystem");

        self.hp_mod_requests.clear();
        std::mem::swap(&mut self.hp_mod_requests, &mut hp_mod_requests);

        {
            let hp_mod_requests = &mut self.hp_mod_requests;
            let mut new_hp_mod_reqs = area_hp_mod_requests
                .iter()
                .map(|area_hp_mod| {
                    AttackCalculation::apply_hp_mod_on_area(
                        &entities,
                        &auth_char_state_storage,
                        &area_hp_mod,
                    )
                })
                .flatten()
                .collect();
            hp_mod_requests.append(&mut new_hp_mod_reqs);
            area_hp_mod_requests.clear();
        }

        // apply area statuses
        let mut new_status_applies = sys_vars
            .apply_area_statuses
            .iter()
            .map(|area_status_change| {
                AttackCalculation::apply_statuses_on_area(
                    &entities,
                    &auth_char_state_storage,
                    &static_char_data_storage,
                    &area_status_change,
                )
            })
            .flatten()
            .collect();
        sys_vars.apply_statuses.append(&mut new_status_applies);
        sys_vars.apply_area_statuses.clear();

        for apply_force in pushes.iter() {
            //            TODO2 push
            //            if let Some(char_body) = physics_world.bodies.rigid_body_mut(apply_force.body_handle) {
            //                let char_state = char_state_storage
            //                    .get_mut(apply_force.dst_entity.into())
            //                    .unwrap();
            //                log::trace!("Try to apply push {:?}", apply_force);
            //                if char_state.statuses.allow_push(apply_force) {
            //                    log::trace!("Push was allowed");
            //                    char_body.set_linear_velocity(apply_force.force);
            //                    let char_state = char_state_storage
            //                        .get_mut(apply_force.dst_entity.into())
            //                        .unwrap();
            //                    char_state
            //                        .cannot_control_until
            //                        .run_at_least_until_seconds(time.now(), apply_force.duration);
            //                } else {
            //                    log::trace!("Push was denied");
            //                }
            //            }
        }
        pushes.clear();

        for hp_mod_req in self.hp_mod_requests.drain(..) {
            // TODO: char_state.cannot_control_until should be defined by this code
            // TODO: enemies can cause damages over a period of time, while they can die and be removed,
            // so src data (or an attack specific data structure) must be copied
            log::trace!("Process hp_mod_req {:?}", hp_mod_req);
            // copy them so hp_mod_req can be moved into the closure
            let attacker_id = hp_mod_req.src_entity;
            let attacked_id = hp_mod_req.dst_entity;

            let hp_mod_req_results = if let Some(src_char_state) =
                auth_char_state_storage.get(hp_mod_req.src_entity.into())
            {
                let src_auth_state = auth_char_state_storage
                    .get(hp_mod_req.src_entity.into())
                    .unwrap();
                if let Some(dst_char_state) =
                    auth_char_state_storage.get(hp_mod_req.dst_entity.into())
                {
                    let src_static_data = static_char_data_storage
                        .get(hp_mod_req.src_entity.into())
                        .unwrap();
                    let dst_static_data = static_char_data_storage
                        .get(hp_mod_req.dst_entity.into())
                        .unwrap();
                    let dst_auth_state = auth_char_state_storage
                        .get(hp_mod_req.dst_entity.into())
                        .unwrap();
                    let is_valid = dst_auth_state.state().is_alive()
                        && match hp_mod_req.typ {
                            HpModificationType::Heal(_) => {
                                src_static_data.team.can_support(dst_static_data.team)
                            }
                            _ => src_static_data.team.can_attack(dst_static_data.team),
                        };
                    if !is_valid {
                        log::warn!("Invalid hp_mod_req: {:?}", hp_mod_req);
                    }
                    if is_valid {
                        Some(AttackCalculation::apply_armor_calc(
                            src_char_state,
                            dst_char_state,
                            hp_mod_req,
                        ))
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            };

            log::trace!("Attack outcomes: {:?}", hp_mod_req_results);

            for hp_mod_req_result in hp_mod_req_results.into_iter() {
                dbg!(&hp_mod_req_result);
                let (hp_mod_req_result, char_pos) = {
                    let attacked_entity_state =
                        char_state_storage.get_mut(attacked_id.into()).unwrap();
                    let hp_mod_req_result = AttackCalculation::alter_requests_by_attacked_statuses(
                        hp_mod_req_result,
                        attacked_entity_state,
                        &mut hp_mod_requests,
                    );

                    let attacked_entity_auth_state =
                        auth_char_state_storage.get_mut(attacked_id.into()).unwrap();
                    AttackCalculation::apply_damage(
                        attacked_entity_auth_state,
                        &hp_mod_req_result,
                        time.now(),
                    );

                    attacked_entity_state
                        .statuses
                        .hp_mod_has_been_applied_on_me(
                            attacked_id,
                            &hp_mod_req_result,
                            &mut hp_mod_requests,
                        );
                    // TODO: rather than this, create a common component which
                    // contains all the necessary info from which an other system will be able to
                    // generate the render and audio commands
                    let attacked_entity_auth_state =
                        auth_char_state_storage.get(attacked_id.into()).unwrap();
                    (hp_mod_req_result, attacked_entity_auth_state.pos())
                };

                {
                    let attacker_entity_state =
                        char_state_storage.get_mut(attacker_id.into()).unwrap();
                    attacker_entity_state
                        .statuses
                        .hp_mod_has_been_applied_on_enemy(
                            attacker_id,
                            &hp_mod_req_result,
                            &mut hp_mod_requests,
                        );
                }

                AttackCalculation::make_sound(
                    &entities,
                    char_pos,
                    attacked_id,
                    &hp_mod_req_result,
                    time.now(),
                    &mut updater,
                    &sys_vars.assets.sounds,
                );
                AttackCalculation::add_flying_damage_entity(
                    &hp_mod_req_result,
                    &entities,
                    &mut updater,
                    attacker_id,
                    attacked_id,
                    &char_pos,
                    time.now(),
                );

                if let Some(events) = &mut events {
                    events.push(SystemEvent::HpModification {
                        timestamp: time.simulation_frame,
                        src: attacker_id,
                        dst: attacked_id,
                        // TODO2
                        //                        result: hp_mod_req_result,
                    });
                }
            }
        }

        // TODO: use a preallocated backbuffer
        let status_changes =
            std::mem::replace(&mut sys_vars.apply_statuses, Vec::with_capacity(128));
        AttackSystem::add_new_statuses(
            status_changes,
            &mut auth_char_state_storage,
            &sys_vars,
            &time,
            &entities,
            &mut updater,
            &mut physics_world,
            &dev_configs,
        );

        let status_changes =
            std::mem::replace(&mut sys_vars.remove_statuses, Vec::with_capacity(128));
        AttackSystem::remove_statuses(status_changes, &mut char_state_storage, &dev_configs);
        sys_vars.remove_statuses.clear();
    }
}

pub struct AttackCalculation;

impl AttackCalculation {
    pub fn alter_requests_by_attacked_statuses(
        outcome: HpModificationResult,
        attacked_entity_state: &mut CharacterStateComponent,
        hp_mod_reqs: &mut Vec<HpModificationRequest>,
    ) -> HpModificationResult {
        // Allow statuses to affect incoming damages/heals
        let outcome = attacked_entity_state
            .statuses
            .hp_mod_is_calculated_but_not_applied_yet(outcome, hp_mod_reqs);
        log::trace!("Attack outcome affected) by statuses: {:?}", outcome);

        return outcome;
    }

    pub fn apply_hp_mod_on_area(
        entities: &Entities,
        auth_char_storage: &WriteStorage<AuthorizedCharStateComponent>,
        area_hpmod_req: &AreaAttackComponent,
    ) -> Vec<HpModificationRequest> {
        let mut result_attacks = vec![];
        for (target_entity_id, char_state) in (entities, auth_char_storage).join() {
            let target_entity_id = CharEntityId::new(target_entity_id);
            if area_hpmod_req
                .except
                .map(|it| it == target_entity_id)
                .unwrap_or(false)
            {
                continue;
            }

            // for optimized, shape-specific queries
            // ncollide2d::query::distance_internal::
            // TODO2
            //            let coll_result = ncollide2d::query::proximity(
            //                &area_hpmod_req.area_isom,
            //                &*area_hpmod_req.area_shape,
            //                &Isometry2::new(char_state.pos(), 0.0),
            //                &ncollide2d::shape::Ball::new(1.0),
            //                0.1,
            //            );
            //            if coll_result == Proximity::Intersecting {
            //                result_attacks.push(HpModificationRequest {
            //                    src_entity: area_hpmod_req.source_entity_id,
            //                    dst_entity: target_entity_id,
            //                    typ: area_hpmod_req.typ,
            //                });
            //            }
        }
        return result_attacks;
    }

    pub fn apply_statuses_on_area(
        entities: &Entities,
        auth_char_storage: &WriteStorage<AuthorizedCharStateComponent>,
        static_char_data_storage: &WriteStorage<StaticCharDataComponent>,
        area_status: &ApplyStatusInAreaComponent,
    ) -> Vec<ApplyStatusComponent> {
        let mut result_statuses = vec![];
        for (target_entity_id, target_auth_char, target_static_data) in
            (entities, auth_char_storage, static_char_data_storage).join()
        {
            let target_entity_id = CharEntityId::new(target_entity_id);
            if area_status
                .except
                .map(|it| it == target_entity_id)
                .unwrap_or(false)
                || !target_static_data
                    .team
                    .is_compatible(area_status.nature, area_status.caster_team)
            {
                continue;
            }
            // for optimized, shape-specific queries
            // ncollide2d::query::distance_internal::
            let coll_result = ncollide2d::query::proximity(
                &area_status.area_isom,
                &*area_status.area_shape,
                &Isometry2::new(target_auth_char.pos(), 0.0),
                &ncollide2d::shape::Ball::new(1.0),
                0.0,
            );
            if coll_result == Proximity::Intersecting {
                result_statuses.push(ApplyStatusComponent {
                    source_entity_id: area_status.source_entity_id,
                    target_entity_id,
                    status: area_status.status.clone(),
                });
            }
        }
        return result_statuses;
    }

    pub fn apply_armor_calc(
        _src: &AuthorizedCharStateComponent,
        dst: &AuthorizedCharStateComponent,
        hp_mod_req: HpModificationRequest,
    ) -> HpModificationResult {
        return match hp_mod_req.typ {
            HpModificationType::SpellDamage(base_dmg, _damage_render_type) => {
                let dmg = dst
                    .calculated_attribs()
                    .armor
                    .subtract_me_from(base_dmg as i32);
                if dmg <= 0 {
                    hp_mod_req.blocked()
                } else {
                    hp_mod_req.allow(dmg as u32)
                }
            }
            HpModificationType::BasicDamage(base_dmg, _damage_render_type, _weapon_type) => {
                let atk = dbg!(base_dmg);
                let atk = dbg!(dst.calculated_attribs().armor).subtract_me_from(atk as i32);
                dbg!(atk);
                if atk <= 0 {
                    hp_mod_req.blocked()
                } else {
                    hp_mod_req.allow(atk as u32)
                }
            }
            HpModificationType::Heal(healed) => hp_mod_req.allow(healed),
            HpModificationType::Poison(dmg) => {
                let atk = dst.calculated_attribs().armor.subtract_me_from(dmg as i32);
                if atk <= 0 {
                    hp_mod_req.blocked()
                } else {
                    hp_mod_req.allow(dmg)
                }
            }
        };
    }

    pub fn make_sound(
        entities: &Entities,
        pos: Vec2,
        target_entity_id: CharEntityId,
        outcome: &HpModificationResult,
        now: ElapsedTime,
        updater: &mut LazyUpdate,
        sounds: &Sounds,
    ) {
        match outcome.typ {
            HpModificationResultType::Ok(hp_mod_req) => match hp_mod_req {
                HpModificationType::BasicDamage(_, _damage_render_type, weapon_type) => {
                    let entity = entities.create();
                    updater.insert(
                        entity,
                        SoundEffectComponent {
                            target_entity_id,
                            sound_id: match weapon_type {
                                WeaponType::Sword => sounds.attack,
                                WeaponType::Arrow => sounds.arrow_hit,
                                WeaponType::SilverBullet => sounds.gun_attack,
                            },
                            pos,
                            start_time: now,
                        },
                    );
                }
                HpModificationType::SpellDamage(_, _damage_render_type) => {}
                HpModificationType::Heal(_) => {}
                HpModificationType::Poison(_) => {}
            },
            HpModificationResultType::Blocked => {}
            HpModificationResultType::Absorbed => {}
        }
    }

    fn apply_damage(
        auth_char_comp: &mut AuthorizedCharStateComponent,
        outcome: &HpModificationResult,
        now: ElapsedTime,
    ) {
        match outcome.typ {
            HpModificationResultType::Ok(hp_req_mod_type) => match hp_req_mod_type {
                HpModificationType::Heal(val) => {
                    auth_char_comp.hp = auth_char_comp
                        .calculated_attribs()
                        .max_hp
                        .min(auth_char_comp.hp + val as i32);
                }
                HpModificationType::BasicDamage(val, _display_type, _weapon_type) => {
                    auth_char_comp
                        .cannot_control_until
                        .run_at_least_until_seconds(now, 0.1);
                    auth_char_comp.set_receiving_damage();
                    auth_char_comp.hp -= val as i32;
                }
                HpModificationType::Poison(val) => {
                    auth_char_comp.hp -= val as i32;
                }
                HpModificationType::SpellDamage(val, _display_type) => {
                    auth_char_comp
                        .cannot_control_until
                        .run_at_least_until_seconds(now, 0.1);
                    auth_char_comp.set_receiving_damage();
                    auth_char_comp.hp -= val as i32;
                }
            },
            HpModificationResultType::Blocked => {}
            HpModificationResultType::Absorbed => {}
        }
    }

    pub fn add_flying_damage_entity(
        outcome: &HpModificationResult,
        entities: &Entities,
        updater: &mut LazyUpdate,
        src_entity_id: CharEntityId,
        target_entity_id: CharEntityId,
        char_pos: &Vec2,
        sys_time: ElapsedTime,
    ) {
        let damage_entity = entities.create();
        let (flying_numer_type, value) = match outcome.typ {
            HpModificationResultType::Ok(hp_req_mod) => match hp_req_mod {
                HpModificationType::BasicDamage(value, display_type, ..)
                | HpModificationType::SpellDamage(value, display_type) => match display_type {
                    DamageDisplayType::SingleNumber => (FlyingNumberType::Damage, value),
                    DamageDisplayType::Combo(attack_count) => {
                        let single_attack_damage = value / (attack_count as u32);
                        (
                            FlyingNumberType::Combo {
                                single_attack_damage,
                                attack_count,
                            },
                            value,
                        )
                    }
                },
                HpModificationType::Poison(value) => (FlyingNumberType::Poison, value),
                HpModificationType::Heal(value) => (FlyingNumberType::Heal, value),
            },
            HpModificationResultType::Blocked => (FlyingNumberType::Block, 0),
            HpModificationResultType::Absorbed => (FlyingNumberType::Absorb, 0),
        };
        updater.insert(
            damage_entity,
            FlyingNumberComponent::new(
                flying_numer_type,
                value,
                src_entity_id,
                target_entity_id,
                3.0,
                *char_pos,
                sys_time,
            ),
        );
    }
}

impl AttackSystem {
    fn add_new_statuses(
        status_changes: Vec<ApplyStatusComponent>,
        char_state_storage: &mut WriteStorage<AuthorizedCharStateComponent>,
        sys_vars: &SystemVariables,
        time: &EngineTime,
        entities: &Entities,
        updater: &mut LazyUpdate,
        physics_world: &mut PhysicEngine,
        dev_configs: &CommonConfigs,
    ) {
        for mut status_change in status_changes.into_iter() {
            if let Some(target_char) =
                char_state_storage.get_mut(status_change.target_entity_id.into())
            {
                if target_char.hp <= 0 {
                    continue;
                }
                let target_entity_id = status_change.target_entity_id;
                log::debug!(
                    "Applying state '{:?}' on {:?}",
                    status_change.status,
                    status_change.target_entity_id
                );

                status_change.status.on_apply(
                    status_change.target_entity_id,
                    target_char,
                    entities,
                    updater,
                    &sys_vars.assets,
                    time,
                    physics_world,
                );
                // TODO2 statuses
                // target_char.statuses.add(status_change.status);
                target_char.recalc_attribs_based_on_statuses(dev_configs);
                log::trace!(
                    "Status added. Attributes({:?}): current: {:?}",
                    target_entity_id,
                    target_char.calculated_attribs()
                );
            }
        }
    }

    pub fn calc_mounted_speedup(
        target_char: &StaticCharDataComponent,
        configs: &CommonConfigs,
    ) -> Percentage {
        return match target_char.job_id {
            JobId::CRUSADER => configs.stats.player.crusader.mounted_speedup,
            _ => percentage(30),
        };
    }

    fn remove_statuses(
        status_changes: Vec<RemoveStatusComponent>,
        char_state_storage: &mut WriteStorage<CharacterStateComponent>,
        dev_configs: &CommonConfigs,
    ) {
        for status_change in status_changes.into_iter() {
            if let Some(target_char) =
                char_state_storage.get_mut(status_change.target_entity_id.into())
            {
                match &status_change.status {
                    RemoveStatusComponentPayload::RemovingStatusType(status_type) => {
                        target_char.statuses.remove_by_nature(*status_type);
                    }
                    RemoveStatusComponentPayload::RemovingStatusDiscr(discr) => {
                        target_char.statuses.remove(*discr)
                    }
                }
                // TODO2 statuses: target_char legyen Auth
                //                target_char.recalc_attribs_based_on_statuses(dev_configs);
                //                log::trace!(
                //                    "Status removed. Attributes({:?}):  current: {:?}",
                //                    status_change.target_entity_id,
                //                    target_char.calculated_attribs()
                //                );
            }
        }
    }
}
