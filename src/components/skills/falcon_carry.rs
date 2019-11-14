use specs::{Entities, LazyUpdate};

use crate::common::{ElapsedTime, Vec2};
use crate::components::char::{
    CharState, CharacterStateComponent, SpriteRenderDescriptorComponent,
};
use crate::components::controller::{CharEntityId, ControllerComponent, ControllerEntityId};
use crate::components::skills::skills::{
    FinishCast, SkillDef, SkillManifestation, SkillTargetType,
};
use crate::components::status::status::{
    Status, StatusNature, StatusStackingResult, StatusUpdateParams, StatusUpdateResult,
};
use crate::components::ApplyForceComponent;
use crate::configs::DevConfig;
use crate::runtime_assets::map::PhysicEngine;
use crate::systems::falcon_ai_sys::FalconComponent;
use crate::systems::render::render_command::RenderCommandCollector;
use crate::systems::SystemVariables;
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
        let sys_vars = ecs_world.read_resource::<SystemVariables>();
        let configs = &ecs_world.read_resource::<DevConfig>().skills.falcon_carry;
        let target_entity = params.target_entity.unwrap();
        let target_pos = {
            let char_storage = ecs_world.read_storage::<CharacterStateComponent>();
            if let Some(target) = char_storage.get(target_entity.0) {
                target.pos()
            } else {
                return None;
            }
        };
        {
            for (falcon, sprite) in (
                &mut ecs_world.write_storage::<FalconComponent>(),
                &mut ecs_world.write_storage::<SpriteRenderDescriptorComponent>(),
            )
                .join()
            {
                if falcon.owner_entity_id != params.caster_entity_id {
                    continue;
                }
                if target_entity == params.caster_entity_id {
                    // falcon.state = FalconState::CarryOwner
                    for (entity_id, controller) in (
                        &ecs_world.entities(),
                        &ecs_world.read_storage::<ControllerComponent>(),
                    )
                        .join()
                    {
                        if controller.controlled_entity == falcon.owner_entity_id {
                            falcon.carry_owner(
                                ControllerEntityId(entity_id),
                                &target_pos,
                                sys_vars.time,
                                configs.carry_owner_duration,
                                sprite,
                            );
                            break;
                        }
                    }
                } else {
                    falcon.carry_ally(
                        target_entity,
                        &target_pos,
                        sys_vars.time,
                        configs.carry_ally_duration,
                        sprite,
                    );
                };
                break;
            }
        }
        None
    }

    fn get_skill_target_type(&self) -> SkillTargetType {
        SkillTargetType::OnlyAllyAndSelf
    }
}

#[derive(Clone)]
pub struct FalconCarryStatus {
    pub started_at: ElapsedTime,
    pub ends_at: ElapsedTime,
    pub carry_owner: bool,
    pub end_pos: Vec2,
}

impl Status for FalconCarryStatus {
    fn dupl(&self) -> Box<dyn Status + Send> {
        Box::new(self.clone())
    }

    fn on_apply(
        &mut self,
        _self_entity_id: CharEntityId,
        target_char: &mut CharacterStateComponent,
        _entities: &Entities,
        _updater: &mut LazyUpdate,
        _sys_vars: &SystemVariables,
        physics_world: &mut PhysicEngine,
    ) {
        target_char.set_noncollidable(physics_world);
        target_char.set_state(CharState::StandBy, 0);
    }

    fn can_target_move(&self) -> bool {
        false
    }

    fn can_target_be_controlled(&self) -> bool {
        false
    }

    fn can_target_cast(&self) -> bool {
        false
    }

    fn update(&mut self, params: StatusUpdateParams) -> StatusUpdateResult {
        if self.ends_at.has_already_passed(params.sys_vars.time) {
            params.target_char.set_collidable(params.physics_world);
            StatusUpdateResult::RemoveIt
        } else {
            StatusUpdateResult::KeepIt
        }
    }

    fn allow_push(&self, _push: &ApplyForceComponent) -> bool {
        false
    }

    fn render(
        &self,
        _char_state: &CharacterStateComponent,
        sys_vars: &SystemVariables,
        render_commands: &mut RenderCommandCollector,
    ) {
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
                .add(sys_vars.assets.sprites.falcon.textures[2])
        }
    }

    fn get_status_completion_percent(&self, now: ElapsedTime) -> Option<(ElapsedTime, f32)> {
        if self.carry_owner {
            Some((
                self.ends_at,
                now.percentage_between(self.started_at, self.ends_at),
            ))
        } else {
            None
        }
    }

    fn stack(&self, _other: &Box<dyn Status>) -> StatusStackingResult {
        StatusStackingResult::Replace
    }

    fn typ(&self) -> StatusNature {
        StatusNature::Neutral
    }
}
