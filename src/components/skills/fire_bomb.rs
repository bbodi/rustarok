use nalgebra::{Isometry2, Vector2};
use specs::LazyUpdate;

use crate::components::char::{ActionPlayMode, CharacterStateComponent, Team};
use crate::components::controller::{CharEntityId, WorldCoord};
use crate::components::skills::skills::{SkillDef, SkillManifestation, SkillTargetType};
use crate::components::status::status::{
    ApplyStatusComponent, ApplyStatusComponentPayload, ApplyStatusInAreaComponent, Status,
    StatusNature, StatusUpdateResult,
};
use crate::components::{
    AreaAttackComponent, DamageDisplayType, HpModificationType, StrEffectComponent,
};
use crate::configs::DevConfig;
use crate::effect::StrEffectType;
use crate::runtime_assets::map::PhysicEngine;
use crate::systems::render::render_command::RenderCommandCollector;
use crate::systems::render_sys::RenderDesktopClientSystem;
use crate::systems::SystemVariables;
use crate::ElapsedTime;

pub struct FireBombSkill;

pub const FIRE_BOMB_SKILL: &'static FireBombSkill = &FireBombSkill;

impl SkillDef for FireBombSkill {
    fn get_icon_path(&self) -> &'static str {
        "data\\texture\\À¯ÀúÀÎÅÍÆäÀÌ½º\\item\\gn_makebomb.bmp"
    }

    fn finish_cast(
        &self,
        caster_entity_id: CharEntityId,
        caster_pos: WorldCoord,
        skill_pos: Option<Vector2<f32>>,
        char_to_skill_dir: &Vector2<f32>,
        target_entity: Option<CharEntityId>,
        ecs_world: &mut specs::world::World,
    ) -> Option<Box<dyn SkillManifestation>> {
        if let Some(caster) = ecs_world
            .read_storage::<CharacterStateComponent>()
            .get(caster_entity_id.0)
        {
            let mut sys_vars = ecs_world.write_resource::<SystemVariables>();
            let now = sys_vars.time;
            sys_vars
                .apply_statuses
                .push(ApplyStatusComponent::from_secondary_status(
                    caster_entity_id,
                    target_entity.unwrap(),
                    Box::new(FireBombStatus {
                        caster_entity_id,
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

#[derive(Clone)]
pub struct FireBombStatus {
    pub caster_entity_id: CharEntityId,
    pub caster_team: Team,
    pub damage: u32,
    pub started: ElapsedTime,
    pub until: ElapsedTime,
    pub spread_count: u8,
}

impl Status for FireBombStatus {
    fn dupl(&self) -> Box<dyn Status + Send> {
        Box::new(self.clone())
    }

    fn update(
        &mut self,
        self_char_id: CharEntityId,
        char_state: &mut CharacterStateComponent,
        _physics_world: &mut PhysicEngine,
        sys_vars: &mut SystemVariables,
        entities: &specs::Entities,
        updater: &mut LazyUpdate,
    ) -> StatusUpdateResult {
        if self.until.has_already_passed(sys_vars.time) {
            let area_shape = Box::new(ncollide2d::shape::Ball::new(2.0));
            let area_isom = Isometry2::new(char_state.pos(), 0.0);
            sys_vars.area_hp_mod_requests.push(AreaAttackComponent {
                area_shape: area_shape.clone(),
                area_isom: area_isom.clone(),
                source_entity_id: self.caster_entity_id,
                typ: HpModificationType::SpellDamage(self.damage, DamageDisplayType::Combo(10)),
                except: None,
            });
            if self.spread_count < 1 {
                sys_vars
                    .apply_area_statuses
                    .push(ApplyStatusInAreaComponent {
                        source_entity_id: self.caster_entity_id,
                        status: ApplyStatusComponentPayload::from_secondary(Box::new(
                            FireBombStatus {
                                caster_entity_id: self.caster_entity_id,
                                started: sys_vars.time,
                                until: sys_vars.time.add_seconds(2.0),
                                damage: self.damage,
                                spread_count: self.spread_count + 1,
                                caster_team: self.caster_team,
                            },
                        )),
                        area_shape: area_shape.clone(),
                        area_isom: area_isom.clone(),
                        except: Some(self_char_id),
                        nature: StatusNature::Harmful,
                        caster_team: self.caster_team,
                    });
            }
            let effect_comp = StrEffectComponent {
                effect_id: StrEffectType::FirePillarBomb.into(),
                pos: char_state.pos(),
                start_time: sys_vars.time.add_seconds(-0.5),
                die_at: Some(sys_vars.time.add_seconds(1.0)),
                play_mode: ActionPlayMode::Repeat,
            };
            updater.insert(entities.create(), effect_comp);

            StatusUpdateResult::RemoveIt
        } else {
            StatusUpdateResult::KeepIt
        }
    }

    fn render(
        &self,
        char_state: &CharacterStateComponent,
        sys_vars: &SystemVariables,
        render_commands: &mut RenderCommandCollector,
    ) {
        RenderDesktopClientSystem::render_str(
            StrEffectType::FireWall,
            self.started,
            &char_state.pos(),
            sys_vars,
            render_commands,
            ActionPlayMode::Repeat,
        );
    }

    fn get_status_completion_percent(&self, now: ElapsedTime) -> Option<(ElapsedTime, f32)> {
        Some((self.until, now.percentage_between(self.started, self.until)))
    }

    fn typ(&self) -> StatusNature {
        StatusNature::Harmful
    }
}
