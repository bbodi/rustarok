use crate::components::char::{CharacterStateComponent, SpriteRenderDescriptorComponent};
use crate::components::controller::LocalPlayerControllerComponent;
use crate::components::skills::skills::{
    FinishCast, SkillDef, SkillManifestation, SkillTargetType,
};
use crate::components::status::status::{StatusUpdateParams, StatusUpdateResult};
use crate::configs::DevConfig;
use crate::render::render_command::RenderCommandCollector;
use crate::systems::falcon_ai_sys::FalconComponent;
use crate::systems::{AssetResources, SystemVariables};
use rustarok_common::common::{ElapsedTime, EngineTime, Vec2};
use specs::prelude::*;

pub struct FalconCarrySkill;

pub const FALCON_CARRY_SKILL: &'static FalconCarrySkill = &FalconCarrySkill;

impl SkillDef for FalconCarrySkill {
    fn get_icon_path(&self) -> &'static str {
        "data\\texture\\À¯ÀúÀÎÅÍÆäÀÌ½º\\item\\mer_scapegoat.bmp"
    }

    fn finish_cast(
        &self,
        params: &FinishCast,
        ecs_world: &mut World,
    ) -> Option<Box<dyn SkillManifestation>> {
        None
        // TODO2
        //        let now = ecs_world.read_resource::<EngineTime>().now();
        //        let configs = &ecs_world.read_resource::<DevConfig>().skills.falcon_carry;
        //        let target_entity = params.target_entity.unwrap();
        //        let target_pos = {
        //            let char_storage = ecs_world.read_storage::<CharacterStateComponent>();
        //            if let Some(target) = char_storage.get(target_entity.into()) {
        //                target.pos()
        //            } else {
        //                return None;
        //            }
        //        };
        //        {
        //            for (falcon, sprite) in (
        //                &mut ecs_world.write_storage::<FalconComponent>(),
        //                &mut ecs_world.write_storage::<SpriteRenderDescriptorComponent>(),
        //            )
        //                .join()
        //            {
        //                if falcon.owner_entity_id != params.caster_entity_id {
        //                    continue;
        //                }
        //                if target_entity == params.caster_entity_id {
        //                    // falcon.state = FalconState::CarryOwner
        //                    for (entity_id, controller) in (
        //                        &ecs_world.entities(),
        //                        &ecs_world.read_storage::<LocalPlayerControllerComponent>(),
        //                    )
        //                        .join()
        //                    {
        //                        if controller.controlled_entity == falcon.owner_entity_id {
        //                            falcon.carry_owner(
        //                                ControllerEntityId(entity_id),
        //                                &target_pos,
        //                                now,
        //                                configs.carry_owner_duration,
        //                                sprite,
        //                            );
        //                            break;
        //                        }
        //                    }
        //                } else {
        //                    falcon.carry_ally(
        //                        target_entity,
        //                        &target_pos,
        //                        now,
        //                        configs.carry_ally_duration,
        //                        sprite,
        //                    );
        //                };
        //                break;
        //            }
        //        }
        //        None
    }

    fn get_skill_target_type(&self) -> SkillTargetType {
        SkillTargetType::OnlyAllyAndSelf
    }
}

#[derive(Clone, Debug)]
pub struct FalconCarryStatus {
    pub started_at: ElapsedTime,
    pub ends_at: ElapsedTime,
    pub carry_owner: bool,
    pub end_pos: Vec2,
}

impl FalconCarryStatus {
    pub fn update(&mut self, params: StatusUpdateParams) -> StatusUpdateResult {
        if self.ends_at.has_already_passed(params.time.now()) {
            params.target_char.set_collidable(params.physics_world);
            StatusUpdateResult::RemoveIt
        } else {
            StatusUpdateResult::KeepIt
        }
    }

    pub fn render(&self, assets: &AssetResources, render_commands: &mut RenderCommandCollector) {
        if !self.carry_owner {
            render_commands
                .circle_3d()
                .radius(0.5)
                .color(&[0, 255, 0, 255])
                .pos_2d(&self.end_pos)
                .y(0.05)
                .add();

            render_commands
                .horizontal_texture_3d()
                .rotation_rad(3.14)
                .color_rgb(&[0, 255, 0])
                .scale(0.5)
                .pos(&self.end_pos)
                .add(assets.sprites.falcon.textures[2])
        }
    }

    pub fn get_status_completion_percent(&self, now: ElapsedTime) -> Option<(ElapsedTime, f32)> {
        if self.carry_owner {
            Some((
                self.ends_at,
                now.percentage_between(self.started_at, self.ends_at),
            ))
        } else {
            None
        }
    }
}
