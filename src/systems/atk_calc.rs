use crate::components::char::CharacterStateComponent;
use crate::components::char::Percentage;
use crate::components::controller::{CharEntityId, WorldCoords};
use crate::components::status::status::{
    ApplyStatusComponent, ApplyStatusComponentPayload, ApplyStatusInAreaComponent, MainStatuses,
    RemoveStatusComponent, RemoveStatusComponentPayload,
};
use crate::components::{
    AttackComponent, AttackType, DamageDisplayType, FlyingNumberComponent, FlyingNumberType,
    SoundEffectComponent,
};
use crate::configs::DevConfig;
use crate::consts::JobId;
use crate::runtime_assets::audio::Sounds;
use crate::systems::{AssetResources, SystemFrameDurations, SystemVariables};
use crate::{ElapsedTime, PhysicEngine};
use nalgebra::{Isometry2, Vector2};
use ncollide2d::query::Proximity;
use rand::Rng;
use specs::prelude::*;
use specs::LazyUpdate;

#[derive(Debug)]
pub enum AttackOutcome {
    Damage(u32),
    Poison(u32),
    Crit(u32),
    Heal(u32),
    Block,
    Absorb,
    Combo {
        single_attack_damage: u32,
        attack_count: u8,
        sum_damage: u32,
    },
}
impl AttackOutcome {
    pub fn create_combo() -> ComboAttackOutcomeBuilder {
        ComboAttackOutcomeBuilder {
            base_atk: 0,
            attack_count: 0,
        }
    }
}

pub struct ComboAttackOutcomeBuilder {
    base_atk: u32,
    attack_count: u8,
}

impl ComboAttackOutcomeBuilder {
    pub fn base_atk(mut self, base_atk: u32) -> ComboAttackOutcomeBuilder {
        self.base_atk = base_atk;
        self
    }

    pub fn attack_count(mut self, attack_count: u8) -> ComboAttackOutcomeBuilder {
        self.attack_count = attack_count;
        self
    }

    pub fn build(self) -> AttackOutcome {
        AttackOutcome::Combo {
            single_attack_damage: self.base_atk,
            attack_count: self.attack_count,
            sum_damage: self.base_atk * self.attack_count as u32,
        }
    }
}

pub struct AttackSystem;

impl<'a> specs::System<'a> for AttackSystem {
    type SystemData = (
        specs::Entities<'a>,
        specs::WriteStorage<'a, CharacterStateComponent>,
        specs::WriteExpect<'a, SystemVariables>,
        specs::ReadExpect<'a, DevConfig>,
        specs::WriteExpect<'a, PhysicEngine>,
        specs::WriteExpect<'a, SystemFrameDurations>,
        specs::Write<'a, LazyUpdate>,
    );

    fn run(
        &mut self,
        (
            entities,
            mut char_state_storage,
            mut system_vars,
            dev_configs,
            mut physics_world,
            mut system_benchmark,
            mut updater,
        ): Self::SystemData,
    ) {
        let _stopwatch = system_benchmark.start_measurement("AttackSystem");

        let mut new_attacks = system_vars
            .area_attacks
            .iter()
            .map(|area_attack| {
                AttackCalculation::damage_chars(
                    &entities,
                    &char_state_storage,
                    &area_attack.area_shape,
                    &area_attack.area_isom,
                    area_attack.source_entity_id,
                    area_attack.typ,
                    area_attack.except,
                )
            })
            .flatten()
            .collect();
        system_vars.attacks.append(&mut new_attacks);
        system_vars.area_attacks.clear();

        // apply area statuses
        let mut new_status_applies = system_vars
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
        system_vars.apply_statuses.append(&mut new_status_applies);
        system_vars.apply_area_statuses.clear();

        for apply_force in &system_vars.pushes {
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
                        .run_at_least_until_seconds(system_vars.time, apply_force.duration);
                } else {
                    log::trace!("Push was denied");
                }
            }
        }
        system_vars.pushes.clear();

        for attack in &system_vars.attacks {
            // TODO: char_state.cannot_control_until should be defined by this code
            // TODO: enemies can cause damages over a period of time, while they can die and be removed,
            // so src data (or an attack specific data structure) must be copied
            log::trace!("Process attack {:?}", attack);
            let outcomes = char_state_storage
                .get(attack.src_entity.0)
                .and_then(|src_char_state| {
                    char_state_storage
                        .get(attack.dst_entity.0)
                        .filter(|it| {
                            it.state().is_alive()
                                && match attack.typ {
                                    AttackType::Heal(_) => src_char_state.team.can_support(it.team),
                                    _ => src_char_state.team.can_attack(it.team),
                                }
                        })
                        .and_then(|dst_char_state| {
                            Some(AttackCalculation::attack(
                                src_char_state,
                                dst_char_state,
                                attack.typ,
                            ))
                        })
                });
            log::trace!("Attack outcomes: {:?}", outcomes);

            if let Some((src_outcomes, dst_outcomes)) = outcomes {
                for outcome in src_outcomes.into_iter() {
                    let attacker_entity = attack.dst_entity;
                    let attacked_entity = attack.src_entity;
                    AttackCalculation::process_outcome(
                        attacker_entity,
                        attacked_entity,
                        outcome,
                        attack,
                        &mut char_state_storage,
                        system_vars.time,
                        &entities,
                        &mut updater,
                        &system_vars.assets,
                    );
                }
                for outcome in dst_outcomes.into_iter() {
                    let attacker_entity = attack.src_entity;
                    let attacked_entity = attack.dst_entity;
                    AttackCalculation::process_outcome(
                        attacker_entity,
                        attacked_entity,
                        outcome,
                        attack,
                        &mut char_state_storage,
                        system_vars.time,
                        &entities,
                        &mut updater,
                        &system_vars.assets,
                    );
                }
            }
        }
        system_vars.attacks.clear();

        // TODO: use a preallocated backbuffer
        let status_changes =
            std::mem::replace(&mut system_vars.apply_statuses, Vec::with_capacity(128));
        AttackSystem::add_new_statuses(
            status_changes,
            &mut char_state_storage,
            &system_vars,
            &dev_configs,
            &entities,
            &mut updater,
            &mut physics_world,
        );

        let status_changes =
            std::mem::replace(&mut system_vars.remove_statuses, Vec::with_capacity(128));
        AttackSystem::remove_statuses(status_changes, &mut char_state_storage);
        system_vars.remove_statuses.clear();
    }
}

pub struct AttackCalculation;

impl AttackCalculation {
    pub fn process_outcome(
        attacker_entity: CharEntityId,
        attacked_entity: CharEntityId,
        outcome: AttackOutcome,
        attack: &AttackComponent,
        char_state_storage: &mut WriteStorage<CharacterStateComponent>,
        now: ElapsedTime,
        entities: &Entities,
        updater: &mut specs::Write<LazyUpdate>,
        assets: &AssetResources,
    ) {
        let attacked_entity_state = char_state_storage.get_mut(attacked_entity.0).unwrap();

        // Allow statuses to affect incoming damages/heals
        log::trace!("Attack outcome: {:?}", outcome);
        let outcome = attacked_entity_state
            .statuses
            .affect_incoming_damage(outcome);
        log::trace!("Attack outcome affected by statuses: {:?}", outcome);

        AttackCalculation::apply_damage(attacked_entity_state, &outcome, now);

        // TODO: rather than this, create a common component which
        // contains all the necessary info from which an other system will be able to
        // generate the render and audio commands
        let char_pos = attacked_entity_state.pos();
        AttackCalculation::make_sound(
            &entities,
            char_pos,
            attacked_entity,
            &outcome,
            attack.typ,
            now,
            updater,
            &assets.sounds,
        );
        AttackCalculation::add_flying_damage_entity(
            &outcome,
            &entities,
            updater,
            attacker_entity,
            attacked_entity,
            &char_pos,
            now,
        );
    }

    pub fn damage_chars(
        entities: &Entities,
        char_storage: &specs::WriteStorage<CharacterStateComponent>,
        skill_shape: &Box<dyn ncollide2d::shape::Shape<f32>>,
        skill_isom: &Isometry2<f32>,
        caster_entity_id: CharEntityId,
        attack_typ: AttackType,
        except: Option<CharEntityId>,
    ) -> Vec<AttackComponent> {
        let mut result_attacks = vec![];
        for (target_entity_id, char_state) in (entities, char_storage).join() {
            let target_entity_id = CharEntityId(target_entity_id);
            if except.map(|it| it == target_entity_id).unwrap_or(false) {
                continue;
            }

            // for optimized, shape-specific queries
            // ncollide2d::query::distance_internal::
            let coll_result = ncollide2d::query::proximity(
                &skill_isom,
                &**skill_shape,
                &Isometry2::new(char_state.pos(), 0.0),
                &ncollide2d::shape::Ball::new(1.0),
                0.0,
            );
            if coll_result == Proximity::Intersecting {
                result_attacks.push(AttackComponent {
                    src_entity: caster_entity_id,
                    dst_entity: target_entity_id,
                    typ: attack_typ,
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

    pub fn attack(
        _src: &CharacterStateComponent,
        dst: &CharacterStateComponent,
        typ: AttackType,
    ) -> (Vec<AttackOutcome>, Vec<AttackOutcome>) {
        let src_outcomes = vec![];
        let mut dst_outcomes = vec![];
        match typ {
            AttackType::SpellDamage(base_dmg, damage_render_type) => {
                let atk = base_dmg;
                let atk = dst.calculated_attribs().armor.subtract_me_from(atk as i32);
                let outcome = if atk <= 0 {
                    AttackOutcome::Block
                } else {
                    match damage_render_type {
                        DamageDisplayType::SingleNumber => AttackOutcome::Damage(atk as u32),
                        DamageDisplayType::Combo(count) => AttackOutcome::create_combo()
                            .base_atk((atk / count as i32) as u32)
                            .attack_count(count)
                            .build(),
                    }
                };
                dst_outcomes.push(outcome);
            }
            AttackType::Basic(base_dmg, _damage_render_type) => {
                let atk = dbg!(base_dmg);
                let atk = dbg!(dst.calculated_attribs().armor).subtract_me_from(atk as i32);
                dbg!(atk);
                let outcome = if atk <= 0 {
                    AttackOutcome::Block
                } else {
                    let mut rng = rand::thread_rng();
                    if rng.gen::<usize>() % 10 == 0 {
                        AttackOutcome::create_combo()
                            .base_atk(atk as u32)
                            .attack_count(2)
                            .build()
                    } else {
                        AttackOutcome::Damage(atk as u32)
                    }
                };
                dst_outcomes.push(outcome);
            }
            AttackType::Heal(healed) => {
                dst_outcomes.push(AttackOutcome::Heal(healed));
            }
            AttackType::Poison(dmg) => {
                let atk = dst.calculated_attribs().armor.subtract_me_from(dmg as i32);
                let outcome = if atk <= 0 {
                    AttackOutcome::Block
                } else {
                    AttackOutcome::Poison(atk as u32)
                };
                dst_outcomes.push(outcome);
            }
        }
        return (src_outcomes, dst_outcomes);
    }

    pub fn make_sound(
        entities: &Entities,
        pos: WorldCoords,
        target_entity_id: CharEntityId,
        outcome: &AttackOutcome,
        attack_type: AttackType,
        now: ElapsedTime,
        updater: &mut specs::Write<LazyUpdate>,
        sounds: &Sounds,
    ) {
        match outcome {
            AttackOutcome::Heal(_val) => {}
            AttackOutcome::Damage(_val) => match attack_type {
                AttackType::Basic(_, _damage_render_type) => {
                    let entity = entities.create();
                    updater.insert(
                        entity,
                        SoundEffectComponent {
                            target_entity_id,
                            sound_id: sounds.attack,
                            pos,
                            start_time: now,
                        },
                    );
                }
                AttackType::SpellDamage(_, _damage_render_type) => {}
                AttackType::Heal(_) => {}
                AttackType::Poison(_) => {}
            },
            AttackOutcome::Combo {
                single_attack_damage: _,
                attack_count: _,
                sum_damage: _,
            } => {}
            AttackOutcome::Poison(_val) => {}
            AttackOutcome::Crit(_val) => {}
            AttackOutcome::Block => {}
            AttackOutcome::Absorb => {}
        }
    }

    pub fn apply_damage(
        char_comp: &mut CharacterStateComponent,
        outcome: &AttackOutcome,
        now: ElapsedTime,
    ) {
        match outcome {
            AttackOutcome::Heal(val) => {
                char_comp.hp = char_comp
                    .calculated_attribs()
                    .max_hp
                    .min(char_comp.hp + *val as i32);
            }
            AttackOutcome::Damage(val) => {
                char_comp
                    .cannot_control_until
                    .run_at_least_until_seconds(now, 0.1);
                char_comp.set_receiving_damage();
                char_comp.hp -= *val as i32;
            }
            AttackOutcome::Combo {
                single_attack_damage: _,
                attack_count: _,
                sum_damage,
            } => {
                char_comp
                    .cannot_control_until
                    .run_at_least_until_seconds(now, 0.1);
                char_comp.set_receiving_damage();
                char_comp.hp -= *sum_damage as i32;
            }
            AttackOutcome::Poison(val) => {
                char_comp.hp -= *val as i32;
            }
            AttackOutcome::Crit(val) => {
                char_comp
                    .cannot_control_until
                    .run_at_least_until_seconds(now, 0.1);
                char_comp.set_receiving_damage();
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
        src_entity_id: CharEntityId,
        target_entity_id: CharEntityId,
        char_pos: &Vector2<f32>,
        sys_time: ElapsedTime,
    ) {
        let damage_entity = entities.create();
        let (typ, value) = match outcome {
            AttackOutcome::Damage(value) => (FlyingNumberType::Damage, *value),
            AttackOutcome::Combo {
                single_attack_damage,
                attack_count,
                sum_damage,
            } => (
                FlyingNumberType::Combo {
                    single_attack_damage: *single_attack_damage,
                    attack_count: *attack_count,
                },
                *sum_damage,
            ),
            AttackOutcome::Poison(value) => (FlyingNumberType::Poison, *value),
            AttackOutcome::Crit(value) => (FlyingNumberType::Damage, *value),
            AttackOutcome::Heal(value) => (FlyingNumberType::Heal, *value),
            AttackOutcome::Block => (FlyingNumberType::Block, 0),
            AttackOutcome::Absorb => (FlyingNumberType::Absorb, 0),
        };
        updater.insert(
            damage_entity,
            FlyingNumberComponent::new(
                typ,
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
        system_vars: &SystemVariables,
        dev_configs: &DevConfig,
        entities: &Entities,
        updater: &mut LazyUpdate,
        physic_world: &mut PhysicEngine,
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
                            system_vars,
                            physic_world,
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
