mod basic_attack;
mod test_firewall;
mod test_moving;

use crate::asset::texture::DUMMY_TEXTURE_ID_FOR_TEST;
use crate::common::Vec2;
use crate::components::char::Percentage;
use crate::components::char::{
    CharState, CharacterEntityBuilder, CharacterStateComponent, EntityTarget, Team,
};
use crate::components::controller::{CharEntityId, EntitiesBelowCursor};
use crate::components::skills::skills::Skills;
use crate::components::status::status::ApplyStatusComponent;
use crate::components::{HpModificationResultType, HpModificationType};
use crate::configs::DevConfig;
use crate::consts::{JobId, JobSpriteId};
use crate::runtime_assets::audio::Sounds;
use crate::runtime_assets::ecs::create_ecs_world;
use crate::runtime_assets::graphic::Texts;
use crate::runtime_assets::map::PhysicEngine;
use crate::systems::next_action_applier_sys::NextActionApplierSystem;
use crate::systems::render::render_command::RenderCommandCollector;
use crate::systems::spawn_entity_system::SpawnEntitySystem;
use crate::systems::{
    CollisionsFromPrevFrame, RenderMatrices, Sex, Sprites, SystemEvent, SystemFrameDurations,
    SystemVariables,
};
use crate::{register_systems, run_main_frame};
use assert_approx_eq::assert_approx_eq;
use log::LevelFilter;
use nalgebra::Vector2;
use specs::prelude::*;
use std::collections::HashMap;
use std::time::Duration;

#[allow(dead_code)]
const TIMESTEP_FOR_30_FPS: f32 = 0.033333333;
#[allow(dead_code)]
const TIMESTEP_FOR_60_FPS: f32 = 0.016666668;
#[allow(dead_code)]
const TIMESTEP_FOR_120_FPS: f32 = 0.0083333333;
#[allow(dead_code)]
const TIMESTEP_FOR_240_FPS: f32 = 0.0041666666;

const TIMESTEP_FOR_TESTS: f32 = TIMESTEP_FOR_30_FPS;

impl Texts {
    pub fn new_for_test() -> Texts {
        Texts {
            skill_name_texts: Default::default(),
            skill_key_texts: Default::default(),
            custom_texts: Default::default(),
            attack_absorbed: DUMMY_TEXTURE_ID_FOR_TEST,
            attack_blocked: DUMMY_TEXTURE_ID_FOR_TEST,
            minus: DUMMY_TEXTURE_ID_FOR_TEST,
            plus: DUMMY_TEXTURE_ID_FOR_TEST,
        }
    }
}

fn setup_ecs_world<'a, 'b>() -> TestUtil<'a, 'b> {
    simple_logging::log_to_stderr(LevelFilter::Trace);

    let mut ecs_world = create_ecs_world();

    // TODO: can I remove render_matrices from system vars?
    let fov = 0.638;
    let render_matrices = RenderMatrices::new(fov, 1024, 768);

    let sys_vars = SystemVariables::new(
        Sprites::new_for_test(),
        Texts::new_for_test(),
        render_matrices,
        HashMap::new(),
        HashMap::new(),
        vec![],
        Sounds::new_for_test(),
        TIMESTEP_FOR_TESTS,
        1024,
        768,
    );

    let ecs_dispatcher = register_systems(None, None, None, true);

    ecs_world.add_resource(sys_vars);
    ecs_world.add_resource(DevConfig::new().unwrap());
    ecs_world.add_resource(RenderCommandCollector::new());
    ecs_world.add_resource(CollisionsFromPrevFrame {
        collisions: HashMap::new(),
    });

    ecs_world.add_resource(PhysicEngine::new());
    ecs_world.add_resource(SystemFrameDurations(HashMap::new()));
    ecs_world.add_resource(Vec::<SystemEvent>::with_capacity(1024));
    return TestUtil {
        ecs_world,
        ecs_dispatcher,
        timestep: TIMESTEP_FOR_TESTS,
    };
}
struct CharAsserter<'a> {
    ecs_world: &'a World,
    char_id: CharEntityId,
}

macro_rules! get_char {
    ($self:ident) => {
        $self
            .ecs_world
            .read_storage::<CharacterStateComponent>()
            .get($self.char_id.0)
            .unwrap()
    };
}
impl<'a> CharAsserter<'a> {
    pub fn state(self, expected_state: CharState) -> CharAsserter<'a> {
        assert_eq!(expected_state, *get_char!(self).state());
        self
    }

    pub fn has_status<T: 'static>(self) -> CharAsserter<'a> {
        assert!(get_char!(self).statuses.get_status::<T>().is_some());
        self
    }

    pub fn is_casting(self) -> CharAsserter<'a> {
        assert!(match get_char!(self).state() {
            CharState::CastingSkill(_) => true,
            _ => false,
        });
        self
    }

    pub fn has_no_active_status(self) -> CharAsserter<'a> {
        assert_eq!(0, get_char!(self).get_status_count());
        self
    }

    pub fn has_less_than_max_hp(self) -> CharAsserter<'a> {
        assert!(get_char!(self).calculated_attribs().max_hp > get_char!(self).hp);
        self
    }

    pub fn has_max_hp(self) -> CharAsserter<'a> {
        assert_eq!(
            get_char!(self).calculated_attribs().max_hp,
            get_char!(self).hp
        );
        self
    }

    pub fn movement_speed(self, expected: Percentage) -> CharAsserter<'a> {
        assert_eq!(
            expected,
            get_char!(self).calculated_attribs().movement_speed,
        );
        self
    }

    pub fn pos(self, expected_x: f32, expected_y: f32) -> CharAsserter<'a> {
        let pos = get_char!(self).pos();
        assert_approx_eq!(expected_x, pos.x, 0.2);
        assert_approx_eq!(expected_y, pos.y, 0.2);
        self
    }

    pub fn pos_y_greater_than(self, expected_y: f32) -> CharAsserter<'a> {
        let pos = get_char!(self).pos();
        assert!(pos.y > expected_y, "{} > {} is false", pos.y, expected_y);
        self
    }

    pub fn pos_y_lesser_than(self, expected_y: f32) -> CharAsserter<'a> {
        let pos = get_char!(self).pos();
        assert!(pos.y < expected_y, "{} < {} is false", pos.y, expected_y);
        self
    }
}

struct OrderedEventAsserter<'a> {
    ecs_world: &'a World,
    index: usize,
}

impl<'a> OrderedEventAsserter<'a> {
    pub fn no_other_events(self) -> OrderedEventAsserter<'a> {
        let events = &self.ecs_world.read_resource::<Vec<SystemEvent>>();
        assert_eq!(
            events.len(),
            self.index,
            "events: {:?}",
            events.iter().skip(self.index).collect::<Vec<_>>()
        );
        self
    }

    pub fn status_change(
        mut self,
        expected_char_id: CharEntityId,
        expected_prev_status: CharState,
        expected_next_status: CharState,
    ) -> OrderedEventAsserter<'a> {
        self.status_change_ref(expected_char_id, expected_prev_status, expected_next_status);
        self
    }

    pub fn status_change_ref(
        &mut self,
        expected_char_id: CharEntityId,
        expected_prev_status: CharState,
        expected_next_status: CharState,
    ) {
        if !self.search_event(|event| match event {
            SystemEvent::CharStatusChange(_tick, char_id, from_status, to_status) => {
                expected_char_id == *char_id
                    && *from_status == expected_prev_status
                    && *to_status == expected_next_status
            }
            _ => false,
        }) {
            assert!(
                false,
                "No status change event was found from {:?} to {:?}",
                expected_prev_status, expected_next_status,
            );
        }
    }

    #[allow(dead_code)]
    pub fn state_went_into_attacking(
        mut self,
        attacker_id: CharEntityId,
        attacked_id: CharEntityId,
    ) -> OrderedEventAsserter<'a> {
        if !self.search_event(|event| match event {
            SystemEvent::CharStatusChange(_tick, char_id, _from_status, to_status) => {
                attacker_id == *char_id
                    && match to_status {
                        CharState::Attacking { target, .. } => *target == attacked_id,
                        _ => false,
                    }
            }
            _ => false,
        }) {
            assert!(
                false,
                "No status change event was found from any to attacking for char({:?})",
                attacker_id,
            );
        }
        self
    }

    pub fn state_went_into_casting(
        mut self,
        expected_char_id: CharEntityId,
    ) -> OrderedEventAsserter<'a> {
        if !self.search_event(|event| match event {
            SystemEvent::CharStatusChange(_tick, char_id, _from_status, to_status) => {
                expected_char_id == *char_id
                    && match to_status {
                        CharState::CastingSkill(_) => true,
                        _ => false,
                    }
            }
            _ => false,
        }) {
            assert!(
                false,
                "No status change event was found from any to casting for char({:?})",
                expected_char_id,
            );
        }
        self
    }

    #[allow(dead_code)]
    pub fn basic_damage(
        mut self,
        expected_attacker: CharEntityId,
        expected_attacked: CharEntityId,
    ) -> OrderedEventAsserter<'a> {
        if !self.search_event(|event| match event {
            SystemEvent::HpModification {
                timestamp: _,
                src,
                dst,
                result,
            } => {
                expected_attacker == *src
                    && expected_attacked == *dst
                    && match result.typ {
                        HpModificationResultType::Ok(hp_mod_req) => match hp_mod_req {
                            HpModificationType::SpellDamage(_damage, _display_type) => false,
                            HpModificationType::BasicDamage(_damage, _, _) => true,
                            HpModificationType::Heal(_) => false,
                            HpModificationType::Poison(_) => false,
                        },
                        HpModificationResultType::Blocked => false,
                        HpModificationResultType::Absorbed => false,
                    }
            }
            _ => false,
        }) {
            assert!(
                false,
                "No damage event was found: {:?} -> {:?}",
                expected_attacker, expected_attacked,
            );
        }
        self
    }

    pub fn heal_eq(
        mut self,
        expected_attacker: CharEntityId,
        expected_attacked: CharEntityId,
        expected_heal: u32,
    ) -> OrderedEventAsserter<'a> {
        if !self.search_event(|event| match event {
            SystemEvent::HpModification {
                timestamp: _,
                src,
                dst,
                result,
            } => {
                expected_attacker == *src
                    && expected_attacked == *dst
                    && match result.typ {
                        HpModificationResultType::Ok(hp_mod_req) => match hp_mod_req {
                            HpModificationType::SpellDamage(_damage, _display_type) => false,
                            HpModificationType::BasicDamage(_damage, _, _) => false,
                            HpModificationType::Heal(heal) => expected_heal == heal,
                            HpModificationType::Poison(_) => false,
                        },
                        HpModificationResultType::Blocked => false,
                        HpModificationResultType::Absorbed => false,
                    }
            }
            _ => false,
        }) {
            assert!(
                false,
                "No damage event was found: {:?} -> {:?}, {}",
                expected_attacker, expected_attacked, expected_heal,
            );
        }
        self
    }

    pub fn basic_damage_eq(
        mut self,
        expected_attacker: CharEntityId,
        expected_attacked: CharEntityId,
        expected_dmg: u32,
    ) -> OrderedEventAsserter<'a> {
        if !self.search_event(|event| match event {
            SystemEvent::HpModification {
                timestamp: _,
                src,
                dst,
                result,
            } => {
                expected_attacker == *src
                    && expected_attacked == *dst
                    && match result.typ {
                        HpModificationResultType::Ok(hp_mod_req) => match hp_mod_req {
                            HpModificationType::SpellDamage(_damage, _display_type) => false,
                            HpModificationType::BasicDamage(damage, _, _) => damage == expected_dmg,
                            HpModificationType::Heal(_) => false,
                            HpModificationType::Poison(_) => false,
                        },
                        HpModificationResultType::Blocked => false,
                        HpModificationResultType::Absorbed => false,
                    }
            }
            _ => false,
        }) {
            assert!(
                false,
                "No damage event was found: {:?} -> {:?}, {}",
                expected_attacker, expected_attacked, expected_dmg,
            );
        }
        self
    }

    pub fn spell_damage(
        mut self,
        expected_attacker: CharEntityId,
        expected_attacked: CharEntityId,
    ) -> OrderedEventAsserter<'a> {
        if !self.search_event(|event| match event {
            SystemEvent::HpModification {
                timestamp: _,
                src,
                dst,
                result,
            } => {
                expected_attacker == *src
                    && expected_attacked == *dst
                    && match result.typ {
                        HpModificationResultType::Ok(hp_mod_req) => match hp_mod_req {
                            HpModificationType::SpellDamage(_damage, _display_type) => true,
                            HpModificationType::BasicDamage(_, _, _) => false,
                            HpModificationType::Heal(_) => false,
                            HpModificationType::Poison(_) => false,
                        },
                        HpModificationResultType::Blocked => false,
                        HpModificationResultType::Absorbed => false,
                    }
            }
            _ => false,
        }) {
            assert!(
                false,
                "No damage event was found: {:?} -> {:?}",
                expected_attacker, expected_attacked,
            );
        }
        self
    }

    fn search_event<F>(&mut self, predicate: F) -> bool
    where
        F: Fn(&SystemEvent) -> bool,
    {
        let events = &self.ecs_world.read_resource::<Vec<SystemEvent>>();
        let pos = events.iter().skip(self.index).position(predicate);

        self.index += pos.unwrap_or(0) + 1;
        return pos.is_some();
    }
}

struct TestUtil<'a, 'b> {
    pub ecs_world: World,
    pub ecs_dispatcher: Dispatcher<'a, 'b>,
    pub timestep: f32,
}

impl<'a, 'b> TestUtil<'a, 'b> {
    fn frames_needed_for(duration: Duration, timestep: f32) -> u64 {
        let fps = (1000.0 / timestep / 1000.0).round();
        (duration.as_secs_f32() * fps).round() as u64
    }

    pub fn run_frames_n_times(&mut self, count: u64) {
        for _ in 0..count {
            run_main_frame(&mut self.ecs_world, &mut self.ecs_dispatcher);
        }
    }

    pub fn run_for(&mut self, duration: Duration) {
        self.run_frames_n_times(TestUtil::frames_needed_for(duration, self.timestep));
    }

    pub fn create_char(&mut self, pos: Vec2, team: Team) -> CharEntityId {
        let char_id = CharEntityId(self.ecs_world.create_entity().build());
        {
            let updater = &self.ecs_world.read_resource::<LazyUpdate>();
            let physics_world = &mut self.ecs_world.write_resource::<PhysicEngine>();
            let dev_configs = &self.ecs_world.read_resource::<DevConfig>();
            CharacterEntityBuilder::new(char_id, "test_char")
                .insert_sprite_render_descr_component(updater)
                .physics(pos, physics_world, |builder| {
                    builder
                        .collision_group(team.get_collision_group())
                        .circle(1.0)
                })
                .char_state(updater, dev_configs, |ch| {
                    ch.outlook_player(Sex::Male, JobSpriteId::from_job_id(JobId::CRUSADER), 0)
                        .job_id(JobId::CRUSADER)
                        .team(team)
                });
        }
        self.ecs_world.maintain();
        return char_id;
    }

    pub fn create_barricade(&mut self, pos: Vec2, team: Team) {
        SpawnEntitySystem::create_barricade(
            &self.ecs_world.entities(),
            &self.ecs_world.read_resource::<LazyUpdate>(),
            &mut self.ecs_world.write_resource(),
            &self.ecs_world.read_resource(),
            team,
            pos,
        );
        self.ecs_world.maintain();
    }

    pub fn apply_status(&mut self, apply_status: ApplyStatusComponent) {
        self.ecs_world
            .write_resource::<SystemVariables>()
            .apply_statuses
            .push(apply_status);
    }

    pub fn cast_skill_on_pos(&mut self, char_id: CharEntityId, skill: Skills, pos: Vec2) {
        let mut char_storage = self.ecs_world.write_storage::<CharacterStateComponent>();
        let char_state = char_storage.get_mut(char_id.0).unwrap();
        dbg!(char_state.pos());
        dbg!(pos);
        NextActionApplierSystem::try_cast_skill(
            skill,
            self.ecs_world.read_resource::<SystemVariables>().time,
            &self.ecs_world.read_resource::<DevConfig>(),
            char_state,
            &pos,
            &EntitiesBelowCursor::new(),
            char_id,
            false, // self target
        );
    }

    pub fn cast_skill_on_self(&mut self, char_id: CharEntityId, skill: Skills) {
        let mut char_storage = self.ecs_world.write_storage::<CharacterStateComponent>();
        let char_state = char_storage.get_mut(char_id.0).unwrap();
        NextActionApplierSystem::try_cast_skill(
            skill,
            self.ecs_world.read_resource::<SystemVariables>().time,
            &self.ecs_world.read_resource::<DevConfig>(),
            char_state,
            &Vector2::zeros(),
            &EntitiesBelowCursor::new(),
            char_id,
            true, // self target
        );
    }

    pub fn set_char_target(&mut self, char_id: CharEntityId, target: EntityTarget) {
        let mut char_storage = self.ecs_world.write_storage::<CharacterStateComponent>();
        let char_state = char_storage.get_mut(char_id.0).unwrap();
        char_state.target = Some(target);
    }

    pub fn assert_on_character(&self, char_id: CharEntityId) -> CharAsserter {
        CharAsserter {
            ecs_world: &self.ecs_world,
            char_id,
        }
    }

    pub fn assert_events_in_order(&self) -> OrderedEventAsserter {
        OrderedEventAsserter {
            ecs_world: &self.ecs_world,
            index: 0,
        }
    }

    //    // test that a is faster than b
    //    TestUtil::assert_events(ecs_world)
    //    .status_change_at(0, a, CharState::Idle, CharState::Walking(v2(100, 100)))
    //    .status_change_at(0, b, CharState::Idle, CharState::Walking(v2(100, 100)))
    //    .status_change_at(50, a, Walk, CharState::Idle)
    //    .status_change_at(100, b, Walk, CharState::Idle);
    //    // no_damage_on(char_id)
    //    //
}
