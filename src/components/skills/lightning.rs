use nalgebra::{Isometry2, Vector2};
use specs::{Entity, LazyUpdate};

use crate::components::char::{ActionPlayMode, CharacterStateComponent};
use crate::components::controller::{CharEntityId, WorldCoord};
use crate::components::skills::skills::{
    SkillDef, SkillManifestation, SkillManifestationComponent, SkillTargetType, WorldCollisions,
};
use crate::components::{AreaAttackComponent, AttackType, DamageDisplayType, StrEffectComponent};
use crate::configs::DevConfig;
use crate::effect::StrEffectType;
use crate::systems::render::render_command::RenderCommandCollector;
use crate::systems::sound_sys::AudioCommandCollectorComponent;
use crate::systems::{AssetResources, SystemVariables};
use crate::{ElapsedTime, PhysicEngine};

pub struct LightningSkill;

pub const LIGHTNING_SKILL: &'static LightningSkill = &LightningSkill;

impl SkillDef for LightningSkill {
    fn get_icon_path(&self) -> &'static str {
        "data\\texture\\À¯ÀúÀÎÅÍÆäÀÌ½º\\item\\wl_chainlightning.bmp"
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
        let system_vars = ecs_world.read_resource::<SystemVariables>();
        Some(Box::new(LightningManifest::new(
            caster_entity_id,
            &skill_pos.unwrap(),
            char_to_skill_dir,
            system_vars.time,
            &ecs_world.entities(),
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
        _configs: &DevConfig,
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
    pub caster_entity_id: CharEntityId,
    pub effect_id: Entity,
    pub pos: Vector2<f32>,
    pub dir_vector: Vector2<f32>,
    pub created_at: ElapsedTime,
    pub next_action_at: ElapsedTime,
    pub next_damage_at: ElapsedTime,
    pub last_skill_pos: Vector2<f32>,
    pub action_count: u8,
}

impl LightningManifest {
    pub fn new(
        caster_entity_id: CharEntityId,
        skill_center: &Vector2<f32>,
        dir_vector: &Vector2<f32>,
        now: ElapsedTime,
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
    fn update(
        &mut self,
        self_entity_id: Entity,
        _all_collisions_in_world: &WorldCollisions,
        system_vars: &mut SystemVariables,
        _entities: &specs::Entities,
        _char_storage: &mut specs::WriteStorage<CharacterStateComponent>,
        _physics_world: &mut PhysicEngine,
        updater: &mut LazyUpdate,
    ) {
        if self
            .created_at
            .add_seconds(12.0)
            .has_already_passed(system_vars.time)
        {
            updater.remove::<SkillManifestationComponent>(self_entity_id);
            updater.remove::<StrEffectComponent>(self.effect_id);
        } else {
            if self.next_action_at.has_already_passed(system_vars.time) {
                updater.remove::<StrEffectComponent>(self.effect_id);
                let effect_comp = match self.action_count {
                    0 => StrEffectComponent {
                        effect_id: StrEffectType::Lightning.into(),
                        pos: self.pos,
                        start_time: system_vars.time.add_seconds(-0.5),
                        die_at: Some(system_vars.time.add_seconds(1.0)),
                        play_mode: ActionPlayMode::Repeat,
                    },
                    1 => {
                        let pos = self.pos + self.dir_vector * 2.2;
                        StrEffectComponent {
                            effect_id: StrEffectType::Lightning.into(),
                            pos,
                            start_time: system_vars.time.add_seconds(-0.5),
                            die_at: Some(system_vars.time.add_seconds(1.0)),
                            play_mode: ActionPlayMode::Repeat,
                        }
                    }
                    2 => {
                        let pos = self.pos + self.dir_vector * 2.0 * 2.2;
                        StrEffectComponent {
                            effect_id: StrEffectType::Lightning.into(),
                            pos,
                            start_time: system_vars.time.add_seconds(-0.5),
                            die_at: Some(system_vars.time.add_seconds(1.0)),
                            play_mode: ActionPlayMode::Repeat,
                        }
                    }
                    3 => {
                        let pos = self.pos + self.dir_vector * 2.0 * 2.2;
                        StrEffectComponent {
                            effect_id: StrEffectType::Lightning.into(),
                            pos,
                            start_time: system_vars.time.add_seconds(-0.5),
                            die_at: Some(system_vars.time.add_seconds(1.0)),
                            play_mode: ActionPlayMode::Repeat,
                        }
                    }
                    4 => {
                        let pos = self.pos + self.dir_vector * 2.2;
                        StrEffectComponent {
                            effect_id: StrEffectType::Lightning.into(),
                            pos,
                            start_time: system_vars.time.add_seconds(-0.5),
                            die_at: Some(system_vars.time.add_seconds(1.0)),
                            play_mode: ActionPlayMode::Repeat,
                        }
                    }
                    5 => StrEffectComponent {
                        effect_id: StrEffectType::Lightning.into(),
                        pos: self.pos,
                        start_time: system_vars.time.add_seconds(-0.5),
                        die_at: Some(system_vars.time.add_seconds(1.0)),
                        play_mode: ActionPlayMode::Repeat,
                    },
                    _ => {
                        return;
                    }
                };
                self.last_skill_pos = effect_comp.pos.clone();
                updater.insert(self.effect_id, effect_comp);
                self.action_count += 1;
                self.next_action_at = system_vars.time.add_seconds(1.5);
                self.next_damage_at = system_vars.time.add_seconds(1.0);
            }
            if self.next_damage_at.has_already_passed(system_vars.time) {
                system_vars.area_attacks.push(AreaAttackComponent {
                    area_shape: Box::new(ncollide2d::shape::Ball::new(1.0)),
                    area_isom: Isometry2::new(self.last_skill_pos, 0.0),
                    source_entity_id: self.caster_entity_id,
                    typ: AttackType::SpellDamage(120, DamageDisplayType::SingleNumber),
                    except: None,
                });
                self.next_damage_at = self.next_damage_at.add_seconds(0.6);
            }
        }
    }

    fn render(
        &self,
        _now: ElapsedTime,
        _tick: u64,
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
