use nalgebra::{Isometry2, Vector2};
use specs::{Entities, Entity, LazyUpdate};

use crate::common::rotate_vec2;
use crate::components::char::{ActionPlayMode, CharacterStateComponent};
use crate::components::controller::CharEntityId;
use crate::components::skills::skill::{
    SkillDef, SkillManifestation, SkillManifestationComponent, SkillTargetType, Skills,
    WorldCollisions,
};
use crate::components::{AreaAttackComponent, AttackType, DamageDisplayType, StrEffectComponent};
use crate::configs::DevConfig;
use crate::effect::StrEffectType;
use crate::runtime_assets::map::PhysicEngine;
use crate::systems::render::render_command::RenderCommandCollector;
use crate::systems::sound_sys::AudioCommandCollectorComponent;
use crate::systems::{AssetResources, SystemVariables};
use crate::ElapsedTime;

pub struct BrutalTestSkill;

pub const BRUTAL_TEST_SKILL: &'static BrutalTestSkill = &BrutalTestSkill;

impl SkillDef for BrutalTestSkill {
    fn get_icon_path(&self) -> &'static str {
        "data\\texture\\À¯ÀúÀÎÅÍÆäÀÌ½º\\item\\wz_meteor.bmp"
    }

    fn finish_cast(
        &self,
        caster_entity_id: CharEntityId,
        caster: &CharacterStateComponent,
        skill_pos: Option<Vector2<f32>>,
        char_to_skill_dir: &Vector2<f32>,
        target_entity: Option<CharEntityId>,
        physics_world: &mut PhysicEngine,
        system_vars: &mut SystemVariables,
        entities: &Entities,
        updater: &mut specs::Write<LazyUpdate>,
    ) -> Option<Box<dyn SkillManifestation>> {
        let angle_in_rad = char_to_skill_dir.angle(&Vector2::y());
        let angle_in_rad = if char_to_skill_dir.x > 0.0 {
            angle_in_rad
        } else {
            -angle_in_rad
        };
        Some(Box::new(BrutalSkillManifest::new(
            caster_entity_id,
            &skill_pos.unwrap(),
            angle_in_rad,
            system_vars.dev_configs.skills.brutal_test_skill.damage,
            system_vars.time,
            entities,
            updater,
        )))
    }

    fn get_skill_target_type(&self) -> SkillTargetType {
        SkillTargetType::Area
    }

    fn render_target_selection(
        &self,
        is_castable: bool,
        skill_pos: &Vector2<f32>,
        char_to_skill_dir: &Vector2<f32>,
        render_commands: &mut RenderCommandCollector,
        configs: &DevConfig,
    ) {
        Skills::render_casting_box(
            is_castable,
            &Vector2::new(
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
    pub extents: Vector2<u16>,
    pub half_extents: Vector2<f32>,
    pub pos: Vector2<f32>,
    pub rot_angle_in_rad: f32,
    pub created_at: ElapsedTime,
    pub die_at: ElapsedTime,
    pub next_damage_at: ElapsedTime,
    pub damage: u32,
}

impl BrutalSkillManifest {
    pub fn new(
        caster_entity_id: CharEntityId,
        skill_center: &Vector2<f32>,
        rot_angle_in_rad: f32,
        damage: u32,
        system_time: ElapsedTime,
        entities: &specs::Entities,
        updater: &mut specs::Write<LazyUpdate>,
    ) -> BrutalSkillManifest {
        let effect_ids = (0..11 * 11)
            .map(|i| {
                let x = -5.0 + (i % 10) as f32;
                let y = -5.0 + (i / 10) as f32;
                skill_center + rotate_vec2(rot_angle_in_rad, &v2!(x, y))
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
        //        let effect_comp = StrEffectComponent {
        //            effect: "StrEffect::LordOfVermilion".to_owned(),
        //            pos: *skill_center,
        //            start_time: system_time,
        //            die_at: system_time.add_seconds(3.0),
        //            duration: ElapsedTime(3.0),
        //        };
        //        let effect_entity = entities.create();
        //        updater.insert(effect_entity, effect_comp);
        //        let effect_ids = vec![effect_entity];
        BrutalSkillManifest {
            caster_entity_id,
            effect_ids,
            rot_angle_in_rad,
            pos: *skill_center,
            extents: Vector2::new(10, 10),
            half_extents: v2!(5.0, 5.0),
            created_at: system_time.clone(),
            die_at: system_time.add_seconds(30.0),
            next_damage_at: system_time,
            damage,
        }
    }
}

impl SkillManifestation for BrutalSkillManifest {
    fn update(
        &mut self,
        self_entity_id: Entity,
        _all_collisions_in_world: &WorldCollisions,
        system_vars: &mut SystemVariables,
        _entities: &specs::Entities,
        _char_storage: &mut specs::WriteStorage<CharacterStateComponent>,
        _physics_world: &mut PhysicEngine,
        updater: &mut specs::Write<LazyUpdate>,
    ) {
        if self.die_at.has_already_passed(system_vars.time) {
            updater.remove::<SkillManifestationComponent>(self_entity_id);
            for effect_id in &self.effect_ids {
                updater.remove::<StrEffectComponent>(*effect_id);
            }
        } else {
            if self.next_damage_at.has_not_passed_yet(system_vars.time) {
                return;
            }
            self.next_damage_at = system_vars.time.add_seconds(0.5);
            system_vars.area_attacks.push(AreaAttackComponent {
                area_shape: Box::new(ncollide2d::shape::Cuboid::new(self.half_extents)),
                area_isom: Isometry2::new(self.pos, self.rot_angle_in_rad),
                source_entity_id: self.caster_entity_id,
                typ: AttackType::SpellDamage(600, DamageDisplayType::Combo(10)),
                except: None,
            });
        }
    }

    fn render(
        &self,
        _now: ElapsedTime,
        _tick: u64,
        _assets: &AssetResources,
        _configs: &DevConfig,
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
