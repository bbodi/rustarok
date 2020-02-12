use nalgebra::{Isometry2, Vector2};
use specs::{Entity, LazyUpdate, ReadStorage};

use crate::audio::sound_sys::AudioCommandCollectorComponent;
use crate::components::char::{ActionPlayMode, CharacterStateComponent};
use crate::components::skills::skills::{
    FinishCast, SkillDef, SkillManifestation, SkillManifestationComponent,
    SkillManifestationUpdateParam, SkillTargetType, Skills,
};
use crate::components::StrEffectComponent;
use crate::effect::StrEffectType;
use crate::render::render_command::RenderCommandCollector;
use crate::systems::{AssetResources, SystemVariables};
use crate::ElapsedTime;
use rustarok_common::attack::{AreaAttackComponent, DamageDisplayType, HpModificationType};
use rustarok_common::common::{rotate_vec2, v2};
use rustarok_common::common::{EngineTime, Vec2};
use rustarok_common::components::char::{CharEntityId, StaticCharDataComponent};
use rustarok_common::config::CommonConfigs;

pub struct BrutalTestSkill;

pub const BRUTAL_TEST_SKILL: &'static BrutalTestSkill = &BrutalTestSkill;

impl SkillDef for BrutalTestSkill {
    fn get_icon_path(&self) -> &'static str {
        "data\\texture\\À¯ÀúÀÎÅÍÆäÀÌ½º\\item\\wz_meteor.bmp"
    }

    fn finish_cast(
        &self,
        params: &FinishCast,
        ecs_world: &mut specs::world::World,
    ) -> Option<Box<dyn SkillManifestation>> {
        let angle_in_rad = params.char_to_skill_dir.angle(&Vector2::y());
        let angle_in_rad = if params.char_to_skill_dir.x > 0.0 {
            angle_in_rad
        } else {
            -angle_in_rad
        };
        let entities = &ecs_world.entities();
        let mut updater = ecs_world.write_resource::<LazyUpdate>();
        Some(Box::new(BrutalSkillManifest::new(
            params.caster_entity_id,
            &params.skill_pos.unwrap(),
            angle_in_rad,
            ecs_world
                .read_resource::<CommonConfigs>()
                .skills
                .brutal_test_skill
                .damage,
            ecs_world.read_resource::<EngineTime>().time,
            entities,
            &mut updater,
        )))
    }

    fn get_skill_target_type(&self) -> SkillTargetType {
        SkillTargetType::Area
    }

    fn render_target_selection(
        &self,
        is_castable: bool,
        skill_pos: &Vec2,
        char_to_skill_dir: &Vec2,
        render_commands: &mut RenderCommandCollector,
        configs: &CommonConfigs,
    ) {
        Skills::render_casting_box(
            is_castable,
            &v2(
                configs.skills.brutal_test_skill.width,
                configs.skills.brutal_test_skill.height,
            ),
            skill_pos,
            char_to_skill_dir,
            render_commands,
        );
    }
}

pub struct BrutalSkillManifest {
    pub caster_entity_id: CharEntityId,
    pub effect_ids: Vec<Entity>,
    pub extents: Vec2,
    pub half_extents: Vec2,
    pub pos: Vec2,
    pub rot_angle_in_rad: f32,
    pub created_at: ElapsedTime,
    pub die_at: ElapsedTime,
    pub next_damage_at: ElapsedTime,
    pub damage: u32,
}

impl BrutalSkillManifest {
    pub fn new(
        caster_entity_id: CharEntityId,
        skill_center: &Vec2,
        rot_angle_in_rad: f32,
        damage: u32,
        system_time: ElapsedTime,
        entities: &specs::Entities,
        updater: &mut LazyUpdate,
    ) -> BrutalSkillManifest {
        let effect_ids = (0..11 * 11)
            .map(|i| {
                let x = -5.0 + (i % 10) as f32;
                let y = -5.0 + (i / 10) as f32;
                skill_center + rotate_vec2(rot_angle_in_rad, &v2(x, y))
            })
            .map(|effect_coords| {
                let effect_comp = StrEffectComponent {
                    effect_id: StrEffectType::FireWall.into(),
                    pos: effect_coords,
                    start_time: system_time,
                    die_at: Some(system_time.add_seconds(30.0)),
                    play_mode: ActionPlayMode::Repeat,
                };
                let effect_entity = entities.create();
                updater.insert(effect_entity, effect_comp);
                effect_entity
            })
            .collect();
        BrutalSkillManifest {
            caster_entity_id,
            effect_ids,
            rot_angle_in_rad,
            pos: *skill_center,
            extents: v2(10.0, 10.0),
            half_extents: v2(5.0, 5.0),
            created_at: system_time.clone(),
            die_at: system_time.add_seconds(30.0),
            next_damage_at: system_time,
            damage,
        }
    }
}

impl SkillManifestation for BrutalSkillManifest {
    fn update(&mut self, mut params: SkillManifestationUpdateParam) {
        if self.die_at.has_already_passed(params.time().now()) {
            params.remove_component::<SkillManifestationComponent>(params.self_entity_id);
            for effect_id in &self.effect_ids {
                params.remove_component::<StrEffectComponent>(*effect_id);
            }
        } else {
            if self.next_damage_at.has_not_passed_yet(params.time().now()) {
                return;
            }
            self.next_damage_at = params.time().now().add_seconds(0.5);
            params.add_area_hp_mod_request(AreaAttackComponent {
                // TODO2
                //                area_shape: Box::new(ncollide2d::shape::Cuboid::new(self.half_extents)),
                //                area_isom: Isometry2::new(self.pos, self.rot_angle_in_rad),
                source_entity_id: self.caster_entity_id,
                typ: HpModificationType::SpellDamage(600, DamageDisplayType::Combo(10)),
                except: None,
            });
        }
    }

    fn render(
        &self,
        _char_entity_storage: &ReadStorage<StaticCharDataComponent>,
        _now: ElapsedTime,
        _tick: u64,
        _assets: &AssetResources,
        render_commands: &mut RenderCommandCollector,
        _audio_commands: &mut AudioCommandCollectorComponent,
    ) {
        render_commands
            .rectangle_3d()
            .pos_2d(&self.pos)
            .rotation_rad(self.rot_angle_in_rad)
            .color(&[0, 255, 0, 255])
            .size(self.extents.x, self.extents.y)
            .add();
    }
}
