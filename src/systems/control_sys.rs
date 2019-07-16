use nalgebra::{Point3, Point2, Vector2, Vector3, Perspective3, Vector4, Matrix4, Isometry2};
use crate::systems::render::DIRECTION_TABLE;
use specs::prelude::*;
use rand::Rng;
use crate::systems::{SystemVariables, SystemFrameDurations};
use crate::video::{VIDEO_WIDTH, VIDEO_HEIGHT};
use sdl2::keyboard::Scancode;
use crate::{ActionIndex, RenderMatrices, PhysicsWorld, TICKS_PER_SECOND, Tick, ElapsedTime};
use crate::cam::Camera;
use ncollide2d::shape::{Cuboid, Ball};
use ncollide2d::query::point_internal::point_query::PointQuery;
use specs::world::EntitiesRes;
use specs::join::JoinIter;
use crate::components::{FlyingNumberType, FlyingNumberComponent};
use crate::components::char::{CharState, PhysicsComponent, CharacterStateComponent, PlayerSpriteComponent, EntityTarget};
use crate::components::controller::{ControllerComponent, ControllerAction, SkillKey};
use crate::components::skill::{PushBackWallSkill, SkillManifestationComponent, Skills};
use nphysics2d::object::Body;
use std::sync::{Arc, Mutex};

pub struct CharacterControlSystem;

impl<'a> specs::System<'a> for CharacterControlSystem {
    type SystemData = (
        specs::Entities<'a>,
        specs::WriteStorage<'a, PhysicsComponent>,
        specs::WriteStorage<'a, CharacterStateComponent>,
        specs::WriteStorage<'a, PlayerSpriteComponent>,
        specs::ReadStorage<'a, ControllerComponent>,
        specs::WriteExpect<'a, SystemVariables>,
        specs::WriteExpect<'a, PhysicsWorld>,
        specs::WriteExpect<'a, SystemFrameDurations>,
        specs::Write<'a, LazyUpdate>,
    );

    fn run(&mut self, (
        entities,
        mut physics_storage,
        mut char_state_storage,
        mut sprite_storage,
        controller_storage,
        mut system_vars,
        mut physics_world,
        mut system_benchmark,
        mut updater,
    ): Self::SystemData) {
        let stopwatch = system_benchmark.start_measurement("CharacterControlSystem");
        let mut rng = rand::thread_rng();
        for (controller) in (&controller_storage).join() {
            // shitty IDE
            let controller: &ControllerComponent = controller;

            let mut char_state = char_state_storage.get_mut(controller.char).unwrap();
            match controller.next_action {
                Some(ControllerAction::MoveOrAttackTo(pos)) => {
                    char_state.target = if let Some(target_entity) = controller.entity_below_cursor {
                        if target_entity != controller.char {
                            Some(EntityTarget::OtherEntity(target_entity))
                        } else {
                            None
                        }
                    } else {
                        Some(EntityTarget::Pos(dbg!(controller.mouse_world_pos)))
                    };
                }
                Some(ControllerAction::MoveTowardsMouse(pos)) => {
                    char_state.target = Some(EntityTarget::Pos(dbg!(controller.mouse_world_pos)));
                }
                Some(ControllerAction::AttackTo(_)) => {}
                Some(ControllerAction::CastingSelectTarget(_)) => {}
                Some(ControllerAction::CancelCastingSelectTarget) => {}
                Some(ControllerAction::Casting(skill_key)) => {
                    if skill_key == SkillKey::Q {
                        let char_pos = {
                            let physics_comp = physics_storage.get(controller.char).unwrap();
                            physics_comp.pos(&physics_world)
                        };
                        let dir = CharacterControlSystem::determine_dir(&controller.mouse_world_pos, &char_pos);
                        let casting_time_seconds = 1.0;
                        let new_state = CharState::CastingSkill {
                            cast_started: system_vars.time,
                            cast_ends: system_vars.time.add_seconds(casting_time_seconds),
                            can_move: false,
                            skill: Arc::new(Mutex::new(Box::new(
                                Skills::TestSkill { pos: controller.mouse_world_pos }
                            ))),
                        };
                        char_state.set_state(new_state, dir);
                    }
                }
                Some(ControllerAction::LeftClick) => {}
                None => {}
            }

            //
//            let mut char_state = char_state_storage.get_mut(controller.char).unwrap();
//            if !char_state.can_move(&system_vars.time) {
//                continue;
//            }
//
//            let mut sprite = sprite_storage.get_mut(controller.char).unwrap();
//
//            if controller.is_key_just_pressed(Scancode::Q) {
//
//            } else if controller.is_key_just_pressed(Scancode::R) {
//                let physics_comp = physics_storage.get(controller.char).unwrap();
//                let mut body = physics_world.rigid_body_mut(physics_comp.body_handle).unwrap();
//                let mut new_pos = Isometry2::identity();
//                new_pos.translation.vector = mouse_world_pos.coords;
//                body.set_position(new_pos);
//            }
        }
    }
}


impl CharacterControlSystem {
    pub fn determine_dir(&target_pos: &Point2<f32>, pos: &Vector2<f32>) -> usize {
        let dir_vec = target_pos - pos;
// "- 90.0"
// The calculated yaw for the camera are 90 at [0;1] and 180 at [1;0] etc,
// this calculation gives a different result which is shifted 90 degrees clockwise,
// so it is 90 at [1;0].
        let dd = dir_vec.x.atan2(dir_vec.y).to_degrees() - 90.0;
        let dd = if dd < 0.0 { dd + 360.0 } else if dd > 360.0 { dd - 360.0 } else { dd };
        let dir_index = (dd / 45.0 + 0.5) as usize % 8;
        return DIRECTION_TABLE[dir_index];
    }
}
