use nalgebra::Isometry2;

use crate::components::char::{ActionPlayMode, CharacterStateComponent, Team};
use crate::components::skills::skills::{
    FinishCast, SkillDef, SkillManifestation, SkillTargetType,
};
use crate::components::status::status::{
    ApplyStatusComponent, ApplyStatusInAreaComponent, StatusEnum, StatusNature, StatusUpdateParams,
    StatusUpdateResult,
};
use crate::components::{
    AreaAttackComponent, DamageDisplayType, HpModificationType, StrEffectComponent,
};
use crate::configs::DevConfig;
use crate::effect::StrEffectType;
use crate::render::render_command::RenderCommandCollector;
use crate::render::render_sys::RenderDesktopClientSystem;
use crate::systems::{AssetResources, SystemVariables};
use crate::ElapsedTime;
use rustarok_common::common::{EngineTime, Vec2};
use rustarok_common::components::char::CharEntityId;

pub struct FireBombSkill;

pub const FIRE_BOMB_SKILL: &'static FireBombSkill = &FireBombSkill;

impl SkillDef for FireBombSkill {
    fn get_icon_path(&self) -> &'static str {
        "data\\texture\\À¯ÀúÀÎÅÍÆäÀÌ½º\\item\\gn_makebomb.bmp"
    }

    fn finish_cast(
        &self,
        params: &FinishCast,
        ecs_world: &mut specs::world::World,
    ) -> Option<Box<dyn SkillManifestation>> {
        if let Some(caster) = ecs_world
            .read_storage::<CharacterStateComponent>()
            .get(params.caster_entity_id.into())
        {
            let mut sys_vars = ecs_world.write_resource::<SystemVariables>();
            let now = ecs_world.read_resource::<EngineTime>().now();
            sys_vars
                .apply_statuses
                .push(ApplyStatusComponent::from_status(
                    params.caster_entity_id,
                    params.target_entity.unwrap(),
                    StatusEnum::FireBombStatus(FireBombStatus {
                        caster_entity_id: params.caster_entity_id,
                        started: now,
                        until: now.add_seconds(2.0),
                        damage: ecs_world
                            .read_resource::<DevConfig>()
                            .skills
                            .firebomb
                            .damage,
                        spread_count: 0,
                        caster_team: caster.team,
                    }),
                ));
        }
        None
    }

    fn get_skill_target_type(&self) -> SkillTargetType {
        SkillTargetType::OnlyEnemy
    }
}

#[derive(Clone, Debug)]
pub struct FireBombStatus {
    pub caster_entity_id: CharEntityId,
    pub caster_team: Team,
    pub damage: u32,
    pub started: ElapsedTime,
    pub until: ElapsedTime,
    pub spread_count: u8,
}

impl FireBombStatus {
    pub fn update(&mut self, params: StatusUpdateParams) -> StatusUpdateResult {
        let now = params.time.now();
        if self.until.has_already_passed(now) {
            let area_shape = Box::new(ncollide2d::shape::Ball::new(2.0));
            let area_isom = Isometry2::new(params.target_char.pos(), 0.0);
            params
                .sys_vars
                .area_hp_mod_requests
                .push(AreaAttackComponent {
                    area_shape: area_shape.clone(),
                    area_isom: area_isom.clone(),
                    source_entity_id: self.caster_entity_id,
                    typ: HpModificationType::SpellDamage(self.damage, DamageDisplayType::Combo(10)),
                    except: None,
                });
            if self.spread_count < 1 {
                params
                    .sys_vars
                    .apply_area_statuses
                    .push(ApplyStatusInAreaComponent {
                        source_entity_id: self.caster_entity_id,
                        status: StatusEnum::FireBombStatus(FireBombStatus {
                            caster_entity_id: self.caster_entity_id,
                            started: now,
                            until: now.add_seconds(2.0),
                            damage: self.damage,
                            spread_count: self.spread_count + 1,
                            caster_team: self.caster_team,
                        }),
                        area_shape: area_shape.clone(),
                        area_isom: area_isom.clone(),
                        except: Some(params.self_char_id),
                        nature: StatusNature::Harmful,
                        caster_team: self.caster_team,
                    });
            }
            let effect_comp = StrEffectComponent {
                effect_id: StrEffectType::FirePillarBomb.into(),
                pos: params.target_char.pos(),
                start_time: now.add_seconds(-0.5),
                die_at: Some(now.add_seconds(1.0)),
                play_mode: ActionPlayMode::Repeat,
            };
            params.updater.insert(params.entities.create(), effect_comp);

            StatusUpdateResult::RemoveIt
        } else {
            StatusUpdateResult::KeepIt
        }
    }

    pub fn render(
        &self,
        char_pos: Vec2,
        now: ElapsedTime,
        assets: &AssetResources,
        render_commands: &mut RenderCommandCollector,
    ) {
        RenderDesktopClientSystem::render_str(
            StrEffectType::FireWall,
            self.started,
            &char_pos,
            assets,
            now,
            render_commands,
            ActionPlayMode::Repeat,
        );
    }

    pub fn get_status_completion_percent(&self, now: ElapsedTime) -> Option<(ElapsedTime, f32)> {
        Some((self.until, now.percentage_between(self.started, self.until)))
    }
}
