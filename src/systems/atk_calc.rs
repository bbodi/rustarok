use crate::components::char::{CharacterStateComponent, PhysicsComponent, CharState};
use specs::{Entity, LazyUpdate};
use crate::systems::{SystemVariables, SystemFrameDurations};
use crate::{PhysicsWorld, ElapsedTime};
use crate::components::{AttackComponent, FlyingNumberType, FlyingNumberComponent, AttackType};
use specs::prelude::*;
use nalgebra::{Vector2};
use crate::components::skill::{Skills, v2_to_p2};

pub enum AttackOutcome {
    Damage(u32),
    Poison(u32),
    Crit(u32),
    Heal(u32),
    Block,
}

pub struct AttackSystem;

impl<'a> specs::System<'a> for AttackSystem {
    type SystemData = (
        specs::Entities<'a>,
        specs::WriteStorage<'a, AttackComponent>,
        specs::ReadStorage<'a, PhysicsComponent>,
        specs::WriteStorage<'a, CharacterStateComponent>,
        specs::ReadExpect<'a, SystemVariables>,
        specs::ReadExpect<'a, PhysicsWorld>,
        specs::WriteExpect<'a, SystemFrameDurations>,
        specs::Write<'a, LazyUpdate>,
    );

    fn run(&mut self, (
        entities,
        mut attack_storage,
        mut physics_storage,
        mut char_state_storage,
        mut system_vars,
        mut physics_world,
        mut system_benchmark,
        mut updater,
    ): Self::SystemData) {
        let stopwatch = system_benchmark.start_measurement("AttackSystem");

        for (attack_entity_id, attack) in (&entities, &mut attack_storage).join() {
            // TODO: char_state.cannot_control_until should be defined by this code
            // TODO: enemies can cause damages over a period of time, while they can die and be removed,
            // so src data (or an attack specific data structure) must be copied
            let outcome = char_state_storage.get(attack.src_entity).and_then(|src_char_state| {
                char_state_storage.get(attack.dst_entity)
                    .filter(|it| it.state().is_live())
                    .and_then(|dst_char_state| {
                        Some(match &attack.typ {
                            AttackType::Basic => AttackCalculation::attack(src_char_state, dst_char_state),
                            AttackType::Skill(skill) => {
                                AttackCalculation::skill_attack(src_char_state, dst_char_state, &skill)
                            }
                        })
                    })
            });
            if let Some((src_outcomes, dst_outcomes)) = outcome {
                for outcome in src_outcomes.into_iter() {
                    let attacked_entity = attack.src_entity;
                    let src_char_state = char_state_storage.get_mut(attacked_entity).unwrap();
                    AttackCalculation::apply_damage(src_char_state, &outcome, system_vars.time);

                    let char_pos = src_char_state.pos();
                    AttackCalculation::add_flying_damage_entity(
                        &outcome,
                        &entities,
                        &mut updater,
                        attacked_entity,
                        &char_pos.coords,
                        system_vars.time,
                    );
                }
                for outcome in dst_outcomes.into_iter() {
                    let attacked_entity = attack.dst_entity;
                    let dst_char_state = char_state_storage.get_mut(attacked_entity).unwrap();
                    AttackCalculation::apply_damage(dst_char_state, &outcome, system_vars.time);

                    let char_pos = dst_char_state.pos();
                    AttackCalculation::add_flying_damage_entity(
                        &outcome,
                        &entities,
                        &mut updater,
                        attacked_entity,
                        &char_pos.coords,
                        system_vars.time,
                    );
                }
            }
            updater.remove::<AttackComponent>(attack_entity_id);
        }
    }
}

pub struct AttackCalculation;

impl AttackCalculation {
    pub fn attack(src: &CharacterStateComponent, dst: &CharacterStateComponent) -> (Vec<AttackOutcome>, Vec<AttackOutcome>) {
        let mut src_outcomes = vec![];
        let mut dst_outcomes = vec![];
        let atk = src.attack_damage_bonus.add_me_to_as_percentage(src.attack_damage as f32);
        let atk = dst.armor.subtract_me_from_as_percentage(atk) as u32;
        let outcome = if atk == 0 {
            AttackOutcome::Block
        } else {
            AttackOutcome::Damage(atk)
        };
        dst_outcomes.push(outcome);
        return (src_outcomes, dst_outcomes);
    }

    pub fn skill_attack(src: &CharacterStateComponent, dst: &CharacterStateComponent, skill: &Skills) -> (Vec<AttackOutcome>, Vec<AttackOutcome>) {
        let mut src_outcomes = vec![];
        let mut dst_outcomes = vec![];
        let atk = match skill {
            Skills::TestSkill => 600.0,
            Skills::BrutalTestSkill => 600.0,
            Skills::Lightning => 120.0,
            Skills::Heal => 0.0,
        };
        match skill {
            // attacking skills
            Skills::Lightning |
            Skills::TestSkill |
            Skills::BrutalTestSkill=> {
                let atk = dst.armor.subtract_me_from_as_percentage(atk) as u32;
                let outcome = if atk == 0 {
                    AttackOutcome::Block
                } else {
                    AttackOutcome::Damage(atk)
                };
                dst_outcomes.push(outcome);
            },
            // healing skills
            Skills::Heal => {
                dst_outcomes.push(AttackOutcome::Heal(200));
            }
        };
        return (src_outcomes, dst_outcomes);
    }


    pub fn apply_attack_state(src: &mut CharacterStateComponent, dst: &mut CharacterStateComponent) {}

    pub fn apply_damage(
        char_comp: &mut CharacterStateComponent,
        outcome: &AttackOutcome,
        now: ElapsedTime,
    ) {
        match outcome {
            AttackOutcome::Heal(val) => {
                char_comp.hp = char_comp.max_hp.min(char_comp.hp + *val as i32);
            }
            AttackOutcome::Block => {}
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
            AttackOutcome::Poison(_) => (FlyingNumberType::Damage, 0),
            AttackOutcome::Crit(_) => (FlyingNumberType::Damage, 0),
            AttackOutcome::Heal(value) => (FlyingNumberType::Heal, *value),
            AttackOutcome::Block => (FlyingNumberType::Normal, 0)
        };
        updater.insert(damage_entity, FlyingNumberComponent::new(
            typ,
            value,
            target_entity_id,
            3.0,
            v2_to_p2(&char_pos),
            sys_time));
    }
}