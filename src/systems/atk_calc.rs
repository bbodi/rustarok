use crate::components::char::{CharacterStateComponent, PhysicsComponent, CharState};
use specs::{Entity, LazyUpdate};
use crate::systems::{SystemVariables, SystemFrameDurations};
use crate::{PhysicsWorld, ElapsedTime};
use crate::components::{FlyingNumberType, FlyingNumberComponent, AttackType, AttackComponent};
use specs::prelude::*;
use nalgebra::{Vector2, Isometry2};
use crate::components::status::{ApplyStatusComponentPayload, MainStatuses, ApplyStatusComponent, RemoveStatusComponent, RemoveStatusComponentPayload, ApplyStatusInAreaComponent};
use ncollide2d::query::Proximity;

pub enum AttackOutcome {
    Damage(u32),
    Poison(u32),
    Crit(u32),
    Heal(u32),
    Block,
    Absorb,
}

pub struct AttackSystem;

impl<'a> specs::System<'a> for AttackSystem {
    type SystemData = (
        specs::Entities<'a>,
        specs::ReadStorage<'a, PhysicsComponent>,
        specs::WriteStorage<'a, CharacterStateComponent>,
        specs::WriteExpect<'a, SystemVariables>,
        specs::WriteExpect<'a, PhysicsWorld>,
        specs::WriteExpect<'a, SystemFrameDurations>,
        specs::Write<'a, LazyUpdate>,
    );

    fn run(&mut self, (
        entities,
        mut physics_storage,
        mut char_state_storage,
        mut system_vars,
        mut physics_world,
        mut system_benchmark,
        mut updater,
    ): Self::SystemData) {
        let stopwatch = system_benchmark.start_measurement("AttackSystem");

        let mut new_attacks = system_vars.area_attacks.iter().map(|area_attack| {
            // TODO: I don't want to pollute the code with mutable storages just because
            // I can't degrade a writestorage to a readstorage temporarily, or can I?...
            let read_only_char_storage: &specs::ReadStorage<'a, CharacterStateComponent> = unsafe {
                std::mem::transmute(&char_state_storage)
            };
            AttackCalculation::damage_chars(
                &entities,
                read_only_char_storage,
                &area_attack.area_shape,
                &area_attack.area_isom,
                area_attack.source_entity_id,
                area_attack.typ,
            )
        }).flatten().collect();
        system_vars.attacks.append(&mut new_attacks);
        system_vars.area_attacks.clear();

        // apply area statuses
        let mut new_status_applies = system_vars.apply_area_statuses.iter().map(|area_status_change| {
            // TODO: I don't want to pollute the code with mutable storages just because
            // I can't degrade a writestorage to a readstorage temporarily, or can I?...
            let read_only_char_storage: &specs::ReadStorage<'a, CharacterStateComponent> = unsafe {
                std::mem::transmute(&char_state_storage)
            };
            AttackCalculation::apply_statuses_on_area(
                &entities,
                read_only_char_storage,
                &area_status_change,
            )
        }).flatten().collect();
        system_vars.apply_statuses.append(&mut new_status_applies);
        system_vars.apply_area_statuses.clear();

        for apply_force in &system_vars.pushes {
            if let Some(char_body) = physics_world.rigid_body_mut(apply_force.body_handle) {
                char_body.set_linear_velocity(apply_force.force);
                let char_state = char_state_storage.get_mut(apply_force.dst_entity).unwrap();
                char_state.cannot_control_until.run_at_least_until_seconds(system_vars.time, apply_force.duration);
            }
        }
        system_vars.pushes.clear();

        for attack in &system_vars.attacks {
            // TODO: char_state.cannot_control_until should be defined by this code
            // TODO: enemies can cause damages over a period of time, while they can die and be removed,
            // so src data (or an attack specific data structure) must be copied
            let outcome = char_state_storage.get(attack.src_entity).and_then(|src_char_state| {
                char_state_storage.get(attack.dst_entity)
                    .filter(|it| it.state().is_live())
                    .and_then(|dst_char_state| {
                        Some(
                            AttackCalculation::attack(
                                src_char_state,
                                dst_char_state,
                                attack.typ,
                            ),
                        )
                    })
            });

            if let Some((src_outcomes, dst_outcomes)) = outcome {
                for outcome in src_outcomes.into_iter() {
                    let attacked_entity = attack.src_entity;
                    let attacked_entity_state = char_state_storage.get_mut(attacked_entity).unwrap();

                    // Allow statuses to affect incoming damages/heals
                    let outcome = attacked_entity_state.statuses.affect_incoming_damage(outcome);

                    AttackCalculation::apply_damage(attacked_entity_state, &outcome, system_vars.time);

                    let char_pos = attacked_entity_state.pos();
                    AttackCalculation::add_flying_damage_entity(
                        &outcome,
                        &entities,
                        &mut updater,
                        attacked_entity,
                        &char_pos,
                        system_vars.time,
                    );
                }
                for outcome in dst_outcomes.into_iter() {
                    let attacked_entity = attack.dst_entity;
                    let attacked_entity_state = char_state_storage.get_mut(attacked_entity).unwrap();

                    // Allow statuses to affect incoming damages/heals
                    let outcome = attacked_entity_state.statuses.affect_incoming_damage(outcome);

                    AttackCalculation::apply_damage(attacked_entity_state, &outcome, system_vars.time);

                    let char_pos = attacked_entity_state.pos();
                    AttackCalculation::add_flying_damage_entity(
                        &outcome,
                        &entities,
                        &mut updater,
                        attacked_entity,
                        &char_pos,
                        system_vars.time,
                    );
                }
            }
        }
        system_vars.attacks.clear();

        let status_changes = std::mem::replace(&mut system_vars.apply_statuses, Vec::with_capacity(128));
        AttackSystem::add_new_statuses(
            status_changes,
            &mut char_state_storage,
            system_vars.time,
        );

        let status_changes = std::mem::replace(&mut system_vars.remove_statuses, Vec::with_capacity(128));
        AttackSystem::remove_statuses(
            status_changes,
            &mut char_state_storage,
            system_vars.time,
        );
        system_vars.remove_statuses.clear();
    }
}


pub struct AttackCalculation;

impl AttackCalculation {
    pub fn damage_chars(
        entities: &Entities,
        char_storage: &specs::ReadStorage<CharacterStateComponent>,
        skill_shape: &Box<dyn ncollide2d::shape::Shape<f32>>,
        skill_isom: &Isometry2<f32>,
        caster_entity_id: Entity,
        attack_typ: AttackType,
    ) -> Vec<AttackComponent> {
        let mut result_attacks = vec![];
        for (target_entity_id, char_state) in (entities, char_storage).join() {
            // for optimized, shape-specific queries
            // ncollide2d::query::distance_internal::
            let coll_result = ncollide2d::query::proximity(
                &skill_isom, &**skill_shape,
                &Isometry2::new(char_state.pos(), 0.0), &ncollide2d::shape::Ball::new(1.0),
                0.0,
            );
            if coll_result == Proximity::Intersecting {
                result_attacks.push(
                    AttackComponent {
                        src_entity: caster_entity_id,
                        dst_entity: target_entity_id,
                        typ: attack_typ,
                    }
                );
            }
        }
        return result_attacks;
    }

    pub fn apply_statuses_on_area(
        entities: &Entities,
        char_storage: &specs::ReadStorage<CharacterStateComponent>,
        area_status: &ApplyStatusInAreaComponent,
    ) -> Vec<ApplyStatusComponent> {
        let mut result_statuses = vec![];
        for (target_entity_id, char_state) in (entities, char_storage).join() {
            if area_status.except.map(|it| it == target_entity_id).unwrap_or(false) {
                continue;
            }
            // for optimized, shape-specific queries
            // ncollide2d::query::distance_internal::
            let coll_result = ncollide2d::query::proximity(
                &area_status.area_isom, &*area_status.area_shape,
                &Isometry2::new(char_state.pos(), 0.0), &ncollide2d::shape::Ball::new(1.0),
                0.0,
            );
            if coll_result == Proximity::Intersecting {
                result_statuses.push(
                    ApplyStatusComponent {
                        source_entity_id: area_status.source_entity_id,
                        target_entity_id,
                        status: area_status.status.clone(),
                    }
                );
            }
        }
        return result_statuses;
    }


    pub fn attack(
        src: &CharacterStateComponent,
        dst: &CharacterStateComponent,
        typ: AttackType,
    ) -> (Vec<AttackOutcome>, Vec<AttackOutcome>) {
        let mut src_outcomes = vec![];
        let mut dst_outcomes = vec![];
        match typ {
            AttackType::Basic(base_dmg) | AttackType::SpellDamage(base_dmg) => {
                let atk = base_dmg as f32;
                let atk = dst.calculated_attribs.armor.subtract_me_from_as_percentage(atk) as u32;
                let outcome = if atk == 0 {
                    AttackOutcome::Block
                } else {
                    AttackOutcome::Damage(atk)
                };
                dst_outcomes.push(outcome);
            }
            AttackType::Heal(healed) => {
                dst_outcomes.push(AttackOutcome::Heal(healed));
            }
            AttackType::Poison(dmg) => {
                let atk = dst.calculated_attribs.armor.subtract_me_from_as_percentage(dmg as f32) as u32;
                let outcome = if atk == 0 {
                    AttackOutcome::Block
                } else {
                    AttackOutcome::Poison(atk)
                };
                dst_outcomes.push(outcome);
            }
        }
        return (src_outcomes, dst_outcomes);
    }


    pub fn apply_damage(
        char_comp: &mut CharacterStateComponent,
        outcome: &AttackOutcome,
        now: ElapsedTime,
    ) {
        match outcome {
            AttackOutcome::Heal(val) => {
                char_comp.hp = char_comp
                    .calculated_attribs
                    .max_hp
                    .min(char_comp.hp + *val as i32);
            }
            AttackOutcome::Damage(val) => {
                char_comp.cannot_control_until.run_at_least_until_seconds(now, 0.1);
                char_comp.set_state(CharState::ReceivingDamage, char_comp.dir());
                char_comp.hp -= dbg!(*val) as i32;
            }
            AttackOutcome::Poison(val) => {
                char_comp.hp -= *val as i32;
            }
            AttackOutcome::Crit(val) => {
                char_comp.cannot_control_until.run_at_least_until_seconds(now, 0.1);
                char_comp.set_state(CharState::ReceivingDamage, char_comp.dir());
                char_comp.hp -= *val as i32;
            }
            AttackOutcome::Block => {}
            AttackOutcome::Absorb => {}
        }
    }

    pub fn add_flying_damage_entity(
        outcome: &AttackOutcome,
        entities: &Entities,
        updater: &mut specs::Write<LazyUpdate>,
        target_entity_id: Entity,
        char_pos: &Vector2<f32>,
        sys_time: ElapsedTime,
    ) {
        let damage_entity = entities.create();
        let (typ, value) = match outcome {
            AttackOutcome::Damage(value) => (FlyingNumberType::Damage, *value),
            AttackOutcome::Poison(value) => (FlyingNumberType::Poison, *value),
            AttackOutcome::Crit(value) => (FlyingNumberType::Damage, *value),
            AttackOutcome::Heal(value) => (FlyingNumberType::Heal, *value),
            AttackOutcome::Block => (FlyingNumberType::Block, 0),
            AttackOutcome::Absorb => (FlyingNumberType::Absorb, 0),
        };
        updater.insert(damage_entity, FlyingNumberComponent::new(
            typ,
            value,
            target_entity_id,
            3.0,
            *char_pos,
            sys_time));
    }
}

impl AttackSystem {
    fn add_new_statuses(
        status_changes: Vec<ApplyStatusComponent>,
        char_state_storage: &mut WriteStorage<CharacterStateComponent>,
        now: ElapsedTime) {
        for status_change in status_changes.into_iter() {
            if let Some(target_char) = char_state_storage.get_mut(status_change.target_entity_id) {
                if target_char.hp <= 0 {
                    continue;
                }
                match status_change.status {
                    ApplyStatusComponentPayload::MainStatus(status_name) => {
                        log::debug!("Applying state '{:?}' on {:?}", status_name, status_change.target_entity_id);
                        match status_name {
                            MainStatuses::Mounted => {
                                target_char.statuses.switch_mounted();
                            }
                            MainStatuses::Stun => {}
                            MainStatuses::Poison => {
                                target_char.statuses.add_poison(
                                    status_change.source_entity_id,
                                    now,
                                    now.add_seconds(15.0),
                                );
                            }
                        }
                    }
                    ApplyStatusComponentPayload::SecondaryStatus(box_status) => {
                        target_char.statuses.add(box_status);
                    }
                }
                target_char.calculated_attribs = target_char
                    .statuses
                    .calc_attribs(&target_char.outlook);
            }
        }
    }

    fn remove_statuses(
        status_changes: Vec<RemoveStatusComponent>,
        char_state_storage: &mut WriteStorage<CharacterStateComponent>,
        now: ElapsedTime) {
        for status_change in status_changes.into_iter() {
            if let Some(target_char) = char_state_storage.get_mut(status_change.target_entity_id) {
                match &status_change.status {
                    RemoveStatusComponentPayload::MainStatus(status_name) => {
                        log::debug!("Removing state '{:?}' from {:?}", status_name, status_change.target_entity_id);
                        target_char.statuses.remove_main_status(*status_name);
                    }
                    RemoveStatusComponentPayload::SecondaryStatus(status_type) => {
                        target_char.statuses.remove(*status_type);
                    }
                }
                target_char.calculated_attribs = target_char
                    .statuses
                    .calc_attribs(&target_char.outlook);
            }
        }
    }
}
