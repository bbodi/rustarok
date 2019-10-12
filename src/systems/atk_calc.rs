use nalgebra::{Isometry2, Vector2};
use ncollide2d::query::Proximity;
use specs::prelude::*;
use specs::LazyUpdate;

use crate::components::char::CharacterStateComponent;
use crate::components::char::Percentage;
use crate::components::controller::{CharEntityId, WorldCoord};
use crate::components::skills::basic_attack::WeaponType;
use crate::components::status::status::{
    ApplyStatusComponent, ApplyStatusComponentPayload, ApplyStatusInAreaComponent, MainStatuses,
    RemoveStatusComponent, RemoveStatusComponentPayload,
};
use crate::components::{
    AreaAttackComponent, DamageDisplayType, FlyingNumberComponent, FlyingNumberType,
    HpModificationRequest, HpModificationRequestResult, HpModificationRequestType,
    SoundEffectComponent,
};
use crate::configs::DevConfig;
use crate::consts::JobId;
use crate::runtime_assets::audio::Sounds;
use crate::systems::{SystemEvent, SystemFrameDurations, SystemVariables};
use crate::{ElapsedTime, PhysicEngine};

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

impl<'a> specs::System<'a> for AttackSystem {
    type SystemData = (
        specs::Entities<'a>,
        specs::WriteStorage<'a, CharacterStateComponent>,
        specs::WriteExpect<'a, SystemVariables>,
        specs::ReadExpect<'a, DevConfig>,
        specs::WriteExpect<'a, PhysicEngine>,
        specs::WriteExpect<'a, SystemFrameDurations>,
        specs::Write<'a, LazyUpdate>,
        Option<specs::Write<'a, Vec<SystemEvent>>>,
    );

    fn run(
        &mut self,
        (
            entities,
            mut char_state_storage,
            mut sys_vars,
            dev_configs,
            mut physics_world,
            mut system_benchmark,
            mut updater,
            mut events,
        ): Self::SystemData,
    ) {
        let _stopwatch = system_benchmark.start_measurement("AttackSystem");

        self.hp_mod_requests.clear();
        std::mem::swap(&mut self.hp_mod_requests, &mut sys_vars.hp_mod_requests);

        {
            let hp_mod_requests = &mut self.hp_mod_requests;
            let mut new_hp_mod_reqs = sys_vars
                .area_hp_mod_requests
                .iter()
                .map(|area_hp_mod| {
                    AttackCalculation::apply_hp_mod_on_area(
                        &entities,
                        &char_state_storage,
                        &area_hp_mod,
                    )
                })
                .flatten()
                .collect();
            hp_mod_requests.append(&mut new_hp_mod_reqs);
            sys_vars.area_hp_mod_requests.clear();
        }

        let hp_mod_requests = &self.hp_mod_requests;

        // apply area statuses
        let mut new_status_applies = sys_vars
            .apply_area_statuses
            .iter()
            .map(|area_status_change| {
                AttackCalculation::apply_statuses_on_area(
                    &entities,
                    &char_state_storage,
                    &area_status_change,
                )
            })
            .flatten()
            .collect();
        sys_vars.apply_statuses.append(&mut new_status_applies);
        sys_vars.apply_area_statuses.clear();

        for apply_force in &sys_vars.pushes {
            if let Some(char_body) = physics_world.bodies.rigid_body_mut(apply_force.body_handle) {
                let char_state = char_state_storage
                    .get_mut(apply_force.dst_entity.0)
                    .unwrap();
                log::trace!("Try to apply push {:?}", apply_force);
                if char_state.statuses.allow_push(apply_force) {
                    log::trace!("Push was allowed");
                    char_body.set_linear_velocity(apply_force.force);
                    let char_state = char_state_storage
                        .get_mut(apply_force.dst_entity.0)
                        .unwrap();
                    char_state
                        .cannot_control_until
                        .run_at_least_until_seconds(sys_vars.time, apply_force.duration);
                } else {
                    log::trace!("Push was denied");
                }
            }
        }
        sys_vars.pushes.clear();

        for hp_mod_req in hp_mod_requests {
            // TODO: char_state.cannot_control_until should be defined by this code
            // TODO: enemies can cause damages over a period of time, while they can die and be removed,
            // so src data (or an attack specific data structure) must be copied
            log::trace!("Process hp_mod_req {:?}", hp_mod_req);
            let hp_mod_req_results =
                char_state_storage
                    .get(hp_mod_req.src_entity.0)
                    .and_then(|src_char_state| {
                        char_state_storage
                            .get(hp_mod_req.dst_entity.0)
                            .filter(|it| {
                                let is_valid = it.state().is_alive()
                                    && match hp_mod_req.typ {
                                        HpModificationRequestType::Heal(_) => {
                                            src_char_state.team.can_support(it.team)
                                        }
                                        _ => src_char_state.team.can_attack(it.team),
                                    };
                                if !is_valid {
                                    log::warn!("Invalid hp_mod_req: {:?}", hp_mod_req);
                                }
                                is_valid
                            })
                            .and_then(|dst_char_state| {
                                Some(AttackCalculation::apply_armor_calc(
                                    src_char_state,
                                    dst_char_state,
                                    hp_mod_req.typ,
                                ))
                            })
                    });
            log::trace!("Attack outcomes: {:?}", hp_mod_req_results);

            for hp_mod_req_result in hp_mod_req_results.into_iter() {
                let attacker_entity = hp_mod_req.src_entity;
                let attacked_entity = hp_mod_req.dst_entity;
                let attacked_entity_state = char_state_storage.get_mut(attacked_entity.0).unwrap();
                let hp_mod_req_result = AttackCalculation::alter_requests_by_statuses(
                    hp_mod_req_result,
                    attacked_entity_state,
                    &mut sys_vars.hp_mod_requests,
                );

                AttackCalculation::apply_damage(
                    attacked_entity_state,
                    &hp_mod_req_result,
                    sys_vars.time,
                );

                // TODO: rather than this, create a common component which
                // contains all the necessary info from which an other system will be able to
                // generate the render and audio commands
                let char_pos = attacked_entity_state.pos();
                AttackCalculation::make_sound(
                    &entities,
                    char_pos,
                    attacked_entity,
                    &hp_mod_req_result,
                    sys_vars.time,
                    &mut updater,
                    &sys_vars.assets.sounds,
                );
                AttackCalculation::add_flying_damage_entity(
                    &hp_mod_req_result,
                    &entities,
                    &mut updater,
                    attacker_entity,
                    attacked_entity,
                    &char_pos,
                    sys_vars.time,
                );

                if let Some(events) = &mut events {
                    events.push(SystemEvent::HpModification {
                        timestamp: sys_vars.tick,
                        src: hp_mod_req.src_entity,
                        dst: hp_mod_req.dst_entity,
                        result: hp_mod_req_result,
                    });
                }
            }
        }

        // TODO: use a preallocated backbuffer
        let status_changes =
            std::mem::replace(&mut sys_vars.apply_statuses, Vec::with_capacity(128));
        AttackSystem::add_new_statuses(
            status_changes,
            &mut char_state_storage,
            &sys_vars,
            &dev_configs,
            &entities,
            &mut updater,
            &mut physics_world,
        );

        let status_changes =
            std::mem::replace(&mut sys_vars.remove_statuses, Vec::with_capacity(128));
        AttackSystem::remove_statuses(status_changes, &mut char_state_storage);
        sys_vars.remove_statuses.clear();
    }
}

pub struct AttackCalculation;

impl AttackCalculation {
    pub fn alter_requests_by_statuses(
        outcome: HpModificationRequestResult,
        attacked_entity_state: &mut CharacterStateComponent,
        hp_mod_reqs: &mut Vec<HpModificationRequest>,
    ) -> HpModificationRequestResult {
        // Allow statuses to affect incoming damages/heals
        log::trace!("Attack outcome: {:?}", outcome);
        let outcome = attacked_entity_state
            .statuses
            .affect_incoming_damage(outcome);
        log::trace!("Attack outcome affected by statuses: {:?}", outcome);

        // Last chance to alter the final, calculated damage/heal for statuses
        let outcome = attacked_entity_state
            .statuses
            .right_before_apply_damage(outcome, hp_mod_reqs);
        log::trace!("Attack outcome affected(2nd) by statuses: {:?}", outcome);

        return outcome;
    }

    pub fn apply_hp_mod_on_area(
        entities: &Entities,
        char_storage: &specs::WriteStorage<CharacterStateComponent>,
        area_hpmod_req: &AreaAttackComponent,
    ) -> Vec<HpModificationRequest> {
        let mut result_attacks = vec![];
        for (target_entity_id, char_state) in (entities, char_storage).join() {
            let target_entity_id = CharEntityId(target_entity_id);
            if area_hpmod_req
                .except
                .map(|it| it == target_entity_id)
                .unwrap_or(false)
            {
                continue;
            }

            // for optimized, shape-specific queries
            // ncollide2d::query::distance_internal::
            let coll_result = ncollide2d::query::proximity(
                &area_hpmod_req.area_isom,
                &*area_hpmod_req.area_shape,
                &Isometry2::new(char_state.pos(), 0.0),
                &ncollide2d::shape::Ball::new(1.0),
                0.0,
            );
            if coll_result == Proximity::Intersecting {
                result_attacks.push(HpModificationRequest {
                    src_entity: area_hpmod_req.source_entity_id,
                    dst_entity: target_entity_id,
                    typ: area_hpmod_req.typ,
                });
            }
        }
        return result_attacks;
    }

    pub fn apply_statuses_on_area(
        entities: &Entities,
        char_storage: &specs::WriteStorage<CharacterStateComponent>,
        area_status: &ApplyStatusInAreaComponent,
    ) -> Vec<ApplyStatusComponent> {
        let mut result_statuses = vec![];
        for (target_entity_id, target_char) in (entities, char_storage).join() {
            let target_entity_id = CharEntityId(target_entity_id);
            if area_status
                .except
                .map(|it| it == target_entity_id)
                .unwrap_or(false)
                || !target_char
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
                &Isometry2::new(target_char.pos(), 0.0),
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
        _src: &CharacterStateComponent,
        dst: &CharacterStateComponent,
        typ: HpModificationRequestType,
    ) -> HpModificationRequestResult {
        return match typ {
            HpModificationRequestType::SpellDamage(base_dmg, damage_render_type) => {
                let dmg = dst
                    .calculated_attribs()
                    .armor
                    .subtract_me_from(base_dmg as i32);
                if dmg <= 0 {
                    HpModificationRequestResult::Blocked
                } else {
                    HpModificationRequestResult::ok(dmg as u32, typ)
                    //                    match damage_render_type {
                    //                        DamageDisplayType::SingleNumber => AttackOutcome::Damage(dmg as u32),
                    //                        DamageDisplayType::Combo(count) => AttackOutcome::create_combo()
                    //                            .base_atk((dmg / count as i32) as u32)
                    //                            .attack_count(count)
                    //                            .build(),
                    //                    }
                }
            }
            HpModificationRequestType::BasicDamage(base_dmg, _damage_render_type, _weapon_type) => {
                let atk = dbg!(base_dmg);
                let atk = dbg!(dst.calculated_attribs().armor).subtract_me_from(atk as i32);
                dbg!(atk);
                if atk <= 0 {
                    HpModificationRequestResult::Blocked
                } else {
                    HpModificationRequestResult::ok(atk as u32, typ)
                }
            }
            HpModificationRequestType::Heal(healed) => HpModificationRequestResult::ok(healed, typ),
            HpModificationRequestType::Poison(dmg) => {
                let atk = dst.calculated_attribs().armor.subtract_me_from(dmg as i32);
                if atk <= 0 {
                    HpModificationRequestResult::Blocked
                } else {
                    HpModificationRequestResult::ok(atk as u32, typ)
                }
            }
        };
    }

    pub fn make_sound(
        entities: &Entities,
        pos: WorldCoord,
        target_entity_id: CharEntityId,
        outcome: &HpModificationRequestResult,
        now: ElapsedTime,
        updater: &mut LazyUpdate,
        sounds: &Sounds,
    ) {
        match outcome {
            HpModificationRequestResult::Ok(hp_mod_req) => match hp_mod_req {
                HpModificationRequestType::BasicDamage(_, _damage_render_type, weapon_type) => {
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
                HpModificationRequestType::SpellDamage(_, _damage_render_type) => {}
                HpModificationRequestType::Heal(_) => {}
                HpModificationRequestType::Poison(_) => {}
            },
            HpModificationRequestResult::Blocked => {}
            HpModificationRequestResult::Absorbed => {}
        }
    }

    fn apply_damage(
        char_comp: &mut CharacterStateComponent,
        outcome: &HpModificationRequestResult,
        now: ElapsedTime,
    ) {
        match outcome {
            HpModificationRequestResult::Ok(hp_req_mod_type) => match hp_req_mod_type {
                HpModificationRequestType::Heal(val) => {
                    char_comp.hp = char_comp
                        .calculated_attribs()
                        .max_hp
                        .min(char_comp.hp + *val as i32);
                }
                HpModificationRequestType::BasicDamage(val, _display_type, _weapon_type) => {
                    char_comp
                        .cannot_control_until
                        .run_at_least_until_seconds(now, 0.1);
                    char_comp.set_receiving_damage();
                    char_comp.hp -= *val as i32;
                }
                HpModificationRequestType::Poison(val) => {
                    char_comp.hp -= *val as i32;
                }
                HpModificationRequestType::SpellDamage(val, _display_type) => {
                    char_comp
                        .cannot_control_until
                        .run_at_least_until_seconds(now, 0.1);
                    char_comp.set_receiving_damage();
                    char_comp.hp -= *val as i32;
                }
            },
            HpModificationRequestResult::Blocked => {}
            HpModificationRequestResult::Absorbed => {}
        }
    }

    pub fn add_flying_damage_entity(
        outcome: &HpModificationRequestResult,
        entities: &Entities,
        updater: &mut LazyUpdate,
        src_entity_id: CharEntityId,
        target_entity_id: CharEntityId,
        char_pos: &Vector2<f32>,
        sys_time: ElapsedTime,
    ) {
        let damage_entity = entities.create();
        let (flying_numer_type, value) = match outcome {
            HpModificationRequestResult::Ok(hp_req_mod) => match hp_req_mod {
                HpModificationRequestType::BasicDamage(value, display_type, ..)
                | HpModificationRequestType::SpellDamage(value, display_type) => match display_type
                {
                    DamageDisplayType::SingleNumber => (FlyingNumberType::Damage, *value),
                    DamageDisplayType::Combo(attack_count) => {
                        let single_attack_damage = *value / (*attack_count as u32);
                        (
                            FlyingNumberType::Combo {
                                single_attack_damage,
                                attack_count: *attack_count,
                            },
                            *value,
                        )
                    }
                },
                HpModificationRequestType::Poison(value) => (FlyingNumberType::Poison, *value),
                HpModificationRequestType::Heal(value) => (FlyingNumberType::Heal, *value),
            },
            HpModificationRequestResult::Blocked => (FlyingNumberType::Block, 0),
            HpModificationRequestResult::Absorbed => (FlyingNumberType::Absorb, 0),
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
        char_state_storage: &mut WriteStorage<CharacterStateComponent>,
        sys_vars: &SystemVariables,
        dev_configs: &DevConfig,
        entities: &Entities,
        updater: &mut LazyUpdate,
        physics_world: &mut PhysicEngine,
    ) {
        for status_change in status_changes.into_iter() {
            if let Some(target_char) = char_state_storage.get_mut(status_change.target_entity_id.0)
            {
                if target_char.hp <= 0 {
                    continue;
                }
                let target_entity_id = status_change.target_entity_id;
                match status_change.status {
                    ApplyStatusComponentPayload::MainStatus(status_name) => {
                        log::debug!(
                            "Applying state '{:?}' on {:?}",
                            status_name,
                            status_change.target_entity_id
                        );
                        match status_name {
                            MainStatuses::Mounted => {
                                let mounted_speedup =
                                    AttackSystem::calc_mounted_speedup(target_char, &dev_configs);
                                target_char.statuses.switch_mounted(mounted_speedup);
                            }
                        }
                    }
                    ApplyStatusComponentPayload::SecondaryStatus(mut box_status) => {
                        box_status.on_apply(
                            status_change.target_entity_id,
                            target_char,
                            entities,
                            updater,
                            sys_vars,
                            physics_world,
                        );
                        target_char.statuses.add(box_status);
                    }
                }
                target_char.recalc_attribs_based_on_statuses();
                log::trace!(
                    "Status added. Attributes({:?}): bonuses: {:?}, current: {:?}",
                    target_entity_id,
                    target_char.attrib_bonuses(),
                    target_char.calculated_attribs()
                );
            }
        }
    }

    fn calc_mounted_speedup(
        target_char: &CharacterStateComponent,
        configs: &DevConfig,
    ) -> Percentage {
        return match target_char.job_id {
            JobId::CRUSADER => configs.stats.player.crusader.mounted_speedup,
            _ => Percentage(30),
        };
    }

    fn remove_statuses(
        status_changes: Vec<RemoveStatusComponent>,
        char_state_storage: &mut WriteStorage<CharacterStateComponent>,
    ) {
        for status_change in status_changes.into_iter() {
            if let Some(target_char) = char_state_storage.get_mut(status_change.target_entity_id.0)
            {
                match &status_change.status {
                    RemoveStatusComponentPayload::MainStatus(status_name) => {
                        log::debug!(
                            "Removing state '{:?}' from {:?}",
                            status_name,
                            status_change.target_entity_id
                        );
                        target_char.statuses.remove_main_status(*status_name);
                    }
                    RemoveStatusComponentPayload::RemovingStatusType(status_type) => {
                        target_char.statuses.remove_by_nature(*status_type);
                    }
                }
                target_char.recalc_attribs_based_on_statuses();
                log::trace!(
                    "Status removed. Attributes({:?}): bonuses: {:?}, current: {:?}",
                    status_change.target_entity_id,
                    target_char.attrib_bonuses(),
                    target_char.calculated_attribs()
                );
            }
        }
    }
}
