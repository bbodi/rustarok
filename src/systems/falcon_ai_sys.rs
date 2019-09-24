use crate::common::{v2_to_v3, v3_to_v2, ElapsedTime};
use crate::components::char::{
    CharActionIndex, CharacterStateComponent, SpriteRenderDescriptorComponent,
};
use crate::components::controller::{
    CharEntityId, ControllerComponent, ControllerEntityId, PlayerIntention, WorldCoords,
};
use crate::components::skills::falcon_carry::FalconCarryStatus;
use crate::components::status::status::ApplyStatusComponent;
use crate::runtime_assets::map::PhysicEngine;
use crate::systems::next_action_applier_sys::NextActionApplierSystem;
use crate::systems::{SystemFrameDurations, SystemVariables};
use nalgebra::{Isometry2, Vector2, Vector3};
use specs::prelude::*;
use vek::QuadraticBezier3;

pub struct FalconAiSystem;

pub enum FalconState {
    Follow,
    CarryOwner {
        owner_controller_id: ControllerEntityId,
        started_at: ElapsedTime,
        ends_at: ElapsedTime,
        target_is_caught: bool,
        start_pos: Vector3<f32>,
    },
    CarryAlly {
        target_id: CharEntityId,
        start_pos: Vector3<f32>,
        started_at: ElapsedTime,
        ends_at: ElapsedTime,
        target_is_caught: bool,
        end_pos: Vector2<f32>,
    },
}

#[derive(Component)]
pub struct FalconComponent {
    pub owner_entity_id: CharEntityId,
    state: FalconState,
    pub pos: Vector3<f32>,
    acceleration: f32,
    bezier: QuadraticBezier3<f32>,
}

impl FalconComponent {
    pub fn new(owner_entity_id: CharEntityId, start_x: f32, start_y: f32) -> FalconComponent {
        FalconComponent {
            owner_entity_id,
            state: FalconState::Follow,
            pos: Vector3::new(start_x, FALCON_FLY_HEIGHT, start_y),
            acceleration: 0.0,
            bezier: QuadraticBezier3 {
                start: vek::Vec3::new(0.0, 0.0, 0.0),
                ctrl: vek::Vec3::new(0.0, 0.0, 0.0),
                end: vek::Vec3::new(0.0, 0.0, 0.0),
            },
        }
    }

    pub fn carry_owner(
        &mut self,
        owner_controller_id: ControllerEntityId,
        target_pos: &WorldCoords,
        now: ElapsedTime,
        duration: f32,
        sprite: &mut SpriteRenderDescriptorComponent,
    ) {
        match self.state {
            FalconState::CarryOwner { .. } | FalconState::CarryAlly { .. } => return,
            _ => {}
        }
        self.state = FalconState::CarryOwner {
            owner_controller_id,
            started_at: now,
            ends_at: now.add_seconds(duration),
            target_is_caught: false,
            start_pos: self.pos,
        };
        sprite.action_index = CharActionIndex::Walking as usize;
        sprite.direction =
            NextActionApplierSystem::determine_dir(&target_pos, &v3_to_v2(&self.pos));
    }

    pub fn carry_ally(
        &mut self,
        target_entity: CharEntityId,
        target_pos: &WorldCoords,
        now: ElapsedTime,
        duration: f32,
        sprite: &mut SpriteRenderDescriptorComponent,
    ) {
        match self.state {
            FalconState::CarryOwner { .. } | FalconState::CarryAlly { .. } => return,
            _ => {}
        }
        self.state = FalconState::CarryAlly {
            target_id: target_entity,
            start_pos: self.pos,
            started_at: now,
            ends_at: now.add_seconds(duration),
            target_is_caught: false,
            end_pos: Vector2::zeros(),
        };
        sprite.action_index = CharActionIndex::Walking as usize;
        sprite.direction =
            NextActionApplierSystem::determine_dir(&target_pos, &v3_to_v2(&self.pos));
    }
}

impl FalconAiSystem {}

pub const FALCON_FLY_HEIGHT: f32 = 5.0;

impl<'a> specs::System<'a> for FalconAiSystem {
    type SystemData = (
        specs::Entities<'a>,
        specs::WriteStorage<'a, FalconComponent>,
        specs::WriteStorage<'a, SpriteRenderDescriptorComponent>,
        specs::WriteStorage<'a, CharacterStateComponent>,
        specs::ReadStorage<'a, ControllerComponent>,
        specs::WriteExpect<'a, SystemFrameDurations>,
        specs::WriteExpect<'a, PhysicEngine>,
        specs::WriteExpect<'a, SystemVariables>,
    );

    fn run(
        &mut self,
        (
            entities,
            mut falcon_storage,
            mut sprite_storage,
            mut char_storage,
            controller_storage,
            mut system_benchmark,
            mut physics_world,
            mut system_vars,
        ): Self::SystemData,
    ) {
        let _stopwatch = system_benchmark.start_measurement("FalconAiSystem");
        for (falcon_id, falcon, sprite) in
            (&entities, &mut falcon_storage, &mut sprite_storage).join()
        {
            match falcon.state {
                FalconState::Follow => {
                    if let Some(owner) = char_storage.get(falcon.owner_entity_id.0) {
                        let falcon_pos_2d = v3_to_v2(&falcon.pos);
                        let diff_v = owner.pos() - falcon_pos_2d;
                        let distance = diff_v.magnitude();
                        if distance > 2.0 {
                            let dir_3d =
                                Vector3::new(owner.pos().x, FALCON_FLY_HEIGHT, owner.pos().y)
                                    - falcon.pos;
                            falcon.acceleration = (falcon.acceleration + system_vars.dt.0 * 0.05)
                                .min(0.03 * owner.calculated_attribs().walking_speed.as_f32());
                            falcon.pos += dir_3d * falcon.acceleration;
                            sprite.direction = NextActionApplierSystem::determine_dir(
                                &owner.pos(),
                                &falcon_pos_2d,
                            );
                        } else {
                            if falcon.acceleration < 0.00001 || distance == 0.0 {
                                falcon.acceleration = 0.0;
                            } else {
                                falcon.acceleration -= system_vars.dt.0 * 0.1;
                                let dir = diff_v.normalize();
                                falcon.pos += v2_to_v3(&dir) * falcon.acceleration;
                            }
                        }
                    } else {
                        entities.delete(falcon_id);
                    }
                }
                FalconState::CarryOwner {
                    owner_controller_id,
                    started_at,
                    ends_at,
                    target_is_caught,
                    start_pos,
                } => {
                    let duration_percentage =
                        system_vars.time.percentage_between(started_at, ends_at);

                    let pick_duration = (system_vars.time.as_f32() - started_at.as_f32()) / 0.3;
                    if pick_duration <= 1.0 {
                        if let Some(target) = char_storage.get(falcon.owner_entity_id.0) {
                            let target_pos = target.pos();
                            let line = Vector3::new(target_pos.x, 2.0, target_pos.y) - start_pos;
                            falcon.pos = start_pos + line * pick_duration;
                        } else {
                            entities.delete(falcon_id);
                            return;
                        }
                    } else if duration_percentage < 1.0 {
                        if !target_is_caught {
                            sprite.action_index = CharActionIndex::Idle as usize;
                            system_vars.apply_statuses.push(
                                ApplyStatusComponent::from_secondary_status(
                                    falcon.owner_entity_id,
                                    falcon.owner_entity_id,
                                    Box::new(FalconCarryStatus {
                                        started_at,
                                        ends_at,
                                        carry_owner: true,
                                        end_pos: Vector2::zeros(),
                                    }),
                                ),
                            );
                            falcon.state = FalconState::CarryOwner {
                                owner_controller_id,
                                started_at,
                                ends_at,
                                target_is_caught: true,
                                start_pos,
                            }
                        }
                        pub const FALCON_CARRY_HEIGHT: f32 = 12.0;
                        let y = if duration_percentage < 0.4 {
                            (duration_percentage / 0.4) * (FALCON_CARRY_HEIGHT - 2.0) + 2.0
                        } else {
                            (1.0 - ((duration_percentage - 0.8).max(0.0) / 0.2))
                                * (FALCON_CARRY_HEIGHT - 2.0)
                                + 2.0
                        };
                        let target_pos = if let Some(controller) =
                            controller_storage.get(owner_controller_id.0)
                        {
                            match controller.next_action {
                                Some(PlayerIntention::MoveTo(pos)) => v3!(pos.x, y, pos.y),
                                Some(PlayerIntention::MoveTowardsMouse(pos)) => {
                                    v3!(pos.x, y, pos.y)
                                }
                                _ => match controller.last_action {
                                    Some(PlayerIntention::MoveTo(pos)) => v3!(pos.x, y, pos.y),
                                    Some(PlayerIntention::MoveTowardsMouse(pos)) => {
                                        v3!(pos.x, y, pos.y)
                                    }
                                    _ => v3!(falcon.pos.x, y, falcon.pos.z),
                                },
                            }
                        } else {
                            entities.delete(falcon_id);
                            return;
                        };
                        falcon.pos.y = y;
                        let diff_v = target_pos - falcon.pos;
                        let distance = diff_v.magnitude();
                        if distance > 2.0 {
                            let falcon_pos_2d = v3_to_v2(&falcon.pos);
                            let dir_3d = (diff_v).normalize();
                            falcon.acceleration =
                                (falcon.acceleration + system_vars.dt.0 * 0.05).min(0.03 * 2.0);
                            falcon.pos += dir_3d * falcon.acceleration;
                            sprite.direction = NextActionApplierSystem::determine_dir(
                                &v3_to_v2(&target_pos),
                                &falcon_pos_2d,
                            );
                        } else {
                            if falcon.acceleration < 0.00001 || distance == 0.0 {
                                falcon.acceleration = 0.0;
                            } else {
                                falcon.acceleration -= system_vars.dt.0 * 0.1;
                                let dir = diff_v.normalize();
                                falcon.pos += dir * falcon.acceleration;
                            }
                        }

                        if let Some(target) = char_storage.get_mut(falcon.owner_entity_id.0) {
                            let body = physics_world
                                .bodies
                                .rigid_body_mut(target.body_handle)
                                .unwrap();
                            body.set_position(Isometry2::translation(falcon.pos.x, falcon.pos.z));
                            target.set_y(falcon.pos.y - 2.5);
                        }
                    } else {
                        if let Some(target) = char_storage.get_mut(falcon.owner_entity_id.0) {
                            target.set_y(0.0);
                        }
                        falcon.state = FalconState::Follow;
                        sprite.action_index = CharActionIndex::Idle as usize;
                    }
                }
                FalconState::CarryAlly {
                    target_id,
                    start_pos,
                    started_at,
                    ends_at,
                    target_is_caught,
                    end_pos,
                } => {
                    let duration_percentage =
                        system_vars.time.percentage_between(started_at, ends_at);
                    // 30% of duration is to go for the ally
                    if duration_percentage <= 0.3 {
                        if let Some(target) = char_storage.get(target_id.0) {
                            let target_pos = target.pos();
                            let line = Vector3::new(target_pos.x, 2.0, target_pos.y) - start_pos;
                            let duration_percentage = duration_percentage / 0.3;
                            falcon.pos = start_pos + line * duration_percentage;
                        } else {
                            falcon.state = FalconState::Follow;
                            sprite.action_index = CharActionIndex::Idle as usize;
                        }
                    } else if duration_percentage < 1.0 {
                        if !target_is_caught {
                            if let Some(owner) = char_storage.get(falcon.owner_entity_id.0) {
                                let line = v3_to_v2(&falcon.pos) - owner.pos();
                                let ctrl =
                                    v3_to_v2(&falcon.pos) + (line * 0.2) + Vector2::new(5.0, 0.0);
                                let ctrl = vek::Vec3::new(ctrl.x, 20.0, ctrl.y);
                                let end_pos = owner.pos();
                                sprite.action_index = CharActionIndex::Idle as usize;
                                falcon.bezier = QuadraticBezier3 {
                                    start: vek::Vec3::new(falcon.pos.x, falcon.pos.y, falcon.pos.z),
                                    ctrl,
                                    end: vek::Vec3::new(end_pos.x, 2.0, end_pos.y),
                                };
                                system_vars.apply_statuses.push(
                                    ApplyStatusComponent::from_secondary_status(
                                        falcon.owner_entity_id,
                                        target_id,
                                        Box::new(FalconCarryStatus {
                                            started_at,
                                            ends_at,
                                            carry_owner: false,
                                            end_pos,
                                        }),
                                    ),
                                );
                                falcon.state = FalconState::CarryAlly {
                                    target_id,
                                    start_pos,
                                    started_at,
                                    ends_at,
                                    target_is_caught: true,
                                    end_pos,
                                }
                            } else {
                                entities.delete(falcon_id);
                                return;
                            }
                        }
                        let duration_percentage = (duration_percentage - 0.3) / 0.7;
                        let pos = falcon.bezier.evaluate(duration_percentage);
                        falcon.pos = v3!(pos.x, pos.y, pos.z);
                        if let Some(target) = char_storage.get_mut(target_id.0) {
                            let body = physics_world
                                .bodies
                                .rigid_body_mut(target.body_handle)
                                .unwrap();
                            body.set_position(Isometry2::translation(falcon.pos.x, falcon.pos.z));
                            target.set_y(falcon.pos.y - 2.5);
                        }
                        sprite.direction = NextActionApplierSystem::determine_dir(
                            &end_pos,
                            &v3_to_v2(&falcon.pos),
                        );
                    } else {
                        if let Some(target) = char_storage.get_mut(target_id.0) {
                            target.set_y(0.0);
                        }
                        falcon.state = FalconState::Follow;
                        sprite.action_index = CharActionIndex::Idle as usize;
                    }
                }
            }
        }
    }
}
