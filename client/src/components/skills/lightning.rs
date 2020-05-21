use nalgebra::Isometry2;
use specs::{Entity, ReadStorage};

use crate::audio::sound_sys::AudioCommandCollectorComponent;
use crate::components::char::{ActionPlayMode, CharacterStateComponent};
use crate::components::skills::skills::{
    FinishCast, SkillDef, SkillManifestation, SkillManifestationComponent,
    SkillManifestationUpdateParam, SkillTargetType,
};
use crate::components::StrEffectComponent;
use crate::effect::StrEffectType;
use crate::render::render_command::RenderCommandCollector;
use crate::systems::{AssetResources, SystemVariables};
use crate::GameTime;
use rustarok_common::attack::{AreaAttackComponent, DamageDisplayType, HpModificationType};
use rustarok_common::common::{EngineTime, Local, Vec2};
use rustarok_common::components::char::{EntityId, StaticCharDataComponent};
use rustarok_common::config::CommonConfigs;
use specs::world::WorldExt;

pub struct LightningSkill;

pub const LIGHTNING_SKILL: &'static LightningSkill = &LightningSkill;

impl SkillDef for LightningSkill {
    fn get_icon_path(&self) -> &'static str {
        "data\\texture\\À¯ÀúÀÎÅÍÆäÀÌ½º\\item\\wl_chainlightning.bmp"
    }

    fn finish_cast(
        &self,
        params: &FinishCast,
        ecs_world: &mut specs::world::World,
    ) -> Option<Box<dyn SkillManifestation>> {
        let sys_vars = ecs_world.read_resource::<SystemVariables>();
        Some(Box::new(LightningManifest::new(
            params.caster_entity_id,
            &params.skill_pos.unwrap(),
            &params.char_to_skill_dir,
            ecs_world.read_resource::<EngineTime>().now(),
            &ecs_world.entities(),
        )))
    }

    fn get_skill_target_type(&self) -> SkillTargetType {
        SkillTargetType::Area
    }

    fn render_target_selection(
        &self,
        _is_castable: bool,
        skill_pos: &Vec2,
        char_to_skill_dir: &Vec2,
        render_commands: &mut RenderCommandCollector,
        _configs: &CommonConfigs,
    ) {
        for i in 0..3 {
            let pos = skill_pos + char_to_skill_dir * i as f32 * 2.2;
            render_commands
                .circle_3d()
                .pos_2d(&pos)
                .y(0.0)
                .radius(1.0)
                .color(&[0, 255, 0, 255])
                .add()
        }
    }
}

pub struct LightningManifest {
    pub caster_entity_id: EntityId<Local>,
    pub effect_id: Entity,
    pub pos: Vec2,
    pub dir_vector: Vec2,
    pub created_at: GameTime<Local>,
    pub next_action_at: GameTime<Local>,
    pub next_damage_at: GameTime<Local>,
    pub last_skill_pos: Vec2,
    pub action_count: u8,
}

impl LightningManifest {
    pub fn new(
        caster_entity_id: EntityId<Local>,
        skill_center: &Vec2,
        dir_vector: &Vec2,
        now: GameTime<Local>,
        entities: &specs::Entities,
    ) -> LightningManifest {
        LightningManifest {
            caster_entity_id,
            effect_id: entities.create(),
            pos: *skill_center,
            created_at: now,
            next_action_at: now,
            next_damage_at: now,
            last_skill_pos: *skill_center,
            action_count: 0,
            dir_vector: *dir_vector,
        }
    }
}

impl SkillManifestation for LightningManifest {
    fn update(&mut self, mut params: SkillManifestationUpdateParam) {
        let now = params.time().now();
        if self.created_at.add_seconds(12.0).has_already_passed(now) {
            params.remove_component::<SkillManifestationComponent>(params.self_entity_id);
            params.remove_component::<StrEffectComponent>(params.self_entity_id);
        } else {
            if self.next_action_at.has_already_passed(now) {
                params.remove_component::<StrEffectComponent>(self.effect_id);
                let effect_comp = match self.action_count {
                    0 => StrEffectComponent {
                        effect_id: StrEffectType::Lightning.into(),
                        pos: self.pos,
                        start_time: now.add_seconds(-0.5),
                        die_at: Some(now.add_seconds(1.0)),
                        play_mode: ActionPlayMode::Repeat,
                    },
                    1 => {
                        let pos = self.pos + self.dir_vector * 2.2;
                        StrEffectComponent {
                            effect_id: StrEffectType::Lightning.into(),
                            pos,
                            start_time: now.add_seconds(-0.5),
                            die_at: Some(now.add_seconds(1.0)),
                            play_mode: ActionPlayMode::Repeat,
                        }
                    }
                    2 => {
                        let pos = self.pos + self.dir_vector * 2.0 * 2.2;
                        StrEffectComponent {
                            effect_id: StrEffectType::Lightning.into(),
                            pos,
                            start_time: now.add_seconds(-0.5),
                            die_at: Some(now.add_seconds(1.0)),
                            play_mode: ActionPlayMode::Repeat,
                        }
                    }
                    3 => {
                        let pos = self.pos + self.dir_vector * 2.0 * 2.2;
                        StrEffectComponent {
                            effect_id: StrEffectType::Lightning.into(),
                            pos,
                            start_time: now.add_seconds(-0.5),
                            die_at: Some(now.add_seconds(1.0)),
                            play_mode: ActionPlayMode::Repeat,
                        }
                    }
                    4 => {
                        let pos = self.pos + self.dir_vector * 2.2;
                        StrEffectComponent {
                            effect_id: StrEffectType::Lightning.into(),
                            pos,
                            start_time: now.add_seconds(-0.5),
                            die_at: Some(now.add_seconds(1.0)),
                            play_mode: ActionPlayMode::Repeat,
                        }
                    }
                    5 => StrEffectComponent {
                        effect_id: StrEffectType::Lightning.into(),
                        pos: self.pos,
                        start_time: now.add_seconds(-0.5),
                        die_at: Some(now.add_seconds(1.0)),
                        play_mode: ActionPlayMode::Repeat,
                    },
                    _ => {
                        return;
                    }
                };
                self.last_skill_pos = effect_comp.pos.clone();
                params.insert_comp(self.effect_id, effect_comp);
                self.action_count += 1;
                self.next_action_at = now.add_seconds(1.5);
                self.next_damage_at = now.add_seconds(1.0);
            }
            if self.next_damage_at.has_already_passed(now) {
                params.add_area_hp_mod_request(AreaAttackComponent {
                    // TODO2
                    //                    area_shape: Box::new(ncollide2d::shape::Ball::new(1.0)),
                    //                    area_isom: Isometry2::new(self.last_skill_pos, 0.0),
                    source_entity_id: self.caster_entity_id,
                    typ: HpModificationType::SpellDamage(120, DamageDisplayType::SingleNumber),
                    except: None,
                });
                self.next_damage_at = self.next_damage_at.add_seconds(0.6);
            }
        }
    }

    fn render(
        &self,
        _char_entity_storage: &ReadStorage<StaticCharDataComponent>,
        _now: GameTime<Local>,
        _assets: &AssetResources,
        render_commands: &mut RenderCommandCollector,
        _audio_commands: &mut AudioCommandCollectorComponent,
    ) {
        for i in self.action_count..3 {
            let pos = self.pos + self.dir_vector * i as f32 * 2.2;
            render_commands
                .circle_3d()
                .pos_2d(&pos)
                .y(0.0)
                .radius(1.0)
                .color(&[0, 255, 0, 255])
                .add();
        }
        // backwards
        if self.action_count >= 4 {
            for i in self.action_count..6 {
                let pos = self.pos + self.dir_vector * (5 - i) as f32 * 2.2;
                render_commands
                    .circle_3d()
                    .pos_2d(&pos)
                    .y(0.0)
                    .radius(1.0)
                    .color(&[0, 255, 0, 255])
                    .add();
            }
        }
    }
}
