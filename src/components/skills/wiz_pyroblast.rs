use nalgebra::{Isometry2, Vector2};
use specs::{Entity, LazyUpdate, ReadStorage};

use crate::components::char::{
    ActionPlayMode, CastingSkillData, CharacterStateComponent, SpriteRenderDescriptorComponent,
};
use crate::components::controller::{CharEntityId, WorldCoord};
use crate::components::skills::skills::{
    SkillDef, SkillManifestation, SkillManifestationComponent, SkillTargetType, WorldCollisions,
};
use crate::components::status::status::{ApplyStatusComponent, Status, StatusNature};
use crate::components::{
    AreaAttackComponent, AttackComponent, AttackType, DamageDisplayType, StrEffectComponent,
};
use crate::configs::{DevConfig, SkillConfigPyroBlastInner};
use crate::effect::StrEffectType;
use crate::runtime_assets::map::PhysicEngine;
use crate::systems::render::render_command::RenderCommandCollector;
use crate::systems::render_sys::{render_action, RenderDesktopClientSystem, COLOR_WHITE};
use crate::systems::sound_sys::AudioCommandCollectorComponent;
use crate::systems::{AssetResources, SystemVariables};
use crate::ElapsedTime;

pub struct WizPyroBlastSkill;

pub const WIZ_PYRO_BLAST_SKILL: &'static WizPyroBlastSkill = &WizPyroBlastSkill;

impl SkillDef for WizPyroBlastSkill {
    fn get_icon_path(&self) -> &'static str {
        "data\\texture\\À¯ÀúÀÎÅÍÆäÀÌ½º\\item\\ht_blastmine.bmp"
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
        let mut system_vars = ecs_world.write_resource::<SystemVariables>();
        let configs = ecs_world
            .read_resource::<DevConfig>()
            .skills
            .wiz_pyroblast
            .inner
            .clone();

        system_vars
            .apply_statuses
            .push(ApplyStatusComponent::from_secondary_status(
                caster_entity_id,
                target_entity.unwrap(),
                Box::new(PyroBlastTargetStatus {
                    caster_entity_id,
                    splash_radius: configs.splash_radius,
                }),
            ));
        Some(Box::new(PyroBlastManifest::new(
            caster_entity_id,
            caster_pos,
            target_entity.unwrap(),
            system_vars.time,
            &mut ecs_world.write_resource::<PhysicEngine>(),
            configs,
        )))
    }

    fn get_skill_target_type(&self) -> SkillTargetType {
        SkillTargetType::OnlyEnemy
    }

    fn render_casting(
        &self,
        char_pos: &Vector2<f32>,
        casting_state: &CastingSkillData,
        system_vars: &SystemVariables,
        dev_configs: &DevConfig,
        render_commands: &mut RenderCommandCollector,
        char_storage: &ReadStorage<CharacterStateComponent>,
    ) {
        RenderDesktopClientSystem::render_str(
            StrEffectType::Moonstar,
            casting_state.cast_started,
            char_pos,
            system_vars,
            render_commands,
            ActionPlayMode::Repeat,
        );
        let casting_percentage = system_vars
            .time
            .percentage_between(casting_state.cast_started, casting_state.cast_ends);

        if let Some(target_char) = char_storage.get(casting_state.target_entity.unwrap().0) {
            render_commands
                .horizontal_texture_3d()
                .pos(&target_char.pos())
                .rotation_rad(3.14 * casting_percentage)
                .fix_size(
                    (dev_configs.skills.wiz_pyroblast.inner.splash_radius
                        * 2.0
                        * casting_percentage)
                        .max(0.5),
                )
                .add(&system_vars.assets.sprites.magic_target)
        }
        let anim_descr = SpriteRenderDescriptorComponent {
            action_index: 16,
            animation_started: casting_state.cast_started,
            animation_ends_at: ElapsedTime(0.0),
            forced_duration: Some(dev_configs.skills.wiz_pyroblast.attributes.casting_time),
            direction: 0,
            fps_multiplier: 1.0,
        };
        render_action(
            system_vars.time,
            &anim_descr,
            &system_vars.assets.sprites.effect_sprites.plasma,
            &(char_pos + casting_state.char_to_skill_dir_when_casted),
            [0, 0],
            false,
            dev_configs.skills.wiz_pyroblast.inner.ball_size * casting_percentage,
            ActionPlayMode::Reverse,
            &COLOR_WHITE,
            render_commands,
        );
    }
}

pub struct PyroBlastManifest {
    pub caster_entity_id: CharEntityId,
    pub pos: WorldCoord,
    pub target_last_pos: WorldCoord,
    pub target_entity_id: CharEntityId,
    pub created_at: ElapsedTime,
    pub configs: SkillConfigPyroBlastInner,
}

impl PyroBlastManifest {
    pub fn new(
        caster_entity_id: CharEntityId,
        pos: WorldCoord,
        target_entity_id: CharEntityId,
        created_at: ElapsedTime,
        physics_world: &mut PhysicEngine,
        configs: SkillConfigPyroBlastInner,
    ) -> PyroBlastManifest {
        PyroBlastManifest {
            caster_entity_id,
            pos,
            target_last_pos: Vector2::new(0.0, 0.0),
            target_entity_id,
            created_at,
            configs,
        }
    }
}

impl SkillManifestation for PyroBlastManifest {
    fn update(
        &mut self,
        self_entity_id: Entity,
        _all_collisions_in_world: &WorldCollisions,
        system_vars: &mut SystemVariables,
        entities: &specs::Entities,
        char_storage: &mut specs::WriteStorage<CharacterStateComponent>,
        _physics_world: &mut PhysicEngine,
        updater: &mut LazyUpdate,
    ) {
        if let Some(target_char) = char_storage.get_mut(self.target_entity_id.0) {
            let target_pos = target_char.pos();
            let dir_vector = target_pos - self.pos;
            let distance = dir_vector.magnitude();
            if distance > 2.0 {
                let dir_vector = dir_vector.normalize();
                self.pos = self.pos + (dir_vector * system_vars.dt.0 * self.configs.moving_speed);
            } else {
                updater.remove::<SkillManifestationComponent>(self_entity_id);
                system_vars.attacks.push(AttackComponent {
                    src_entity: self.caster_entity_id,
                    dst_entity: self.target_entity_id,
                    typ: AttackType::SpellDamage(
                        self.configs.damage,
                        DamageDisplayType::SingleNumber,
                    ),
                });
                let area_shape = Box::new(ncollide2d::shape::Ball::new(self.configs.splash_radius));
                let area_isom = Isometry2::new(target_pos, 0.0);
                system_vars.area_attacks.push(AreaAttackComponent {
                    area_shape,
                    area_isom,
                    source_entity_id: self.caster_entity_id,
                    typ: AttackType::SpellDamage(
                        self.configs.secondary_damage,
                        DamageDisplayType::SingleNumber,
                    ),
                    except: Some(self.target_entity_id),
                });
                updater.insert(
                    entities.create(),
                    StrEffectComponent {
                        effect_id: StrEffectType::Explosion.into(),
                        pos: target_pos,
                        start_time: system_vars.time,
                        die_at: None,
                        play_mode: ActionPlayMode::Once,
                    },
                );
                target_char
                    .statuses
                    .remove::<PyroBlastTargetStatus, _>(|status| {
                        status.caster_entity_id == self.caster_entity_id
                    })
            }
        } else {
            updater.remove::<SkillManifestationComponent>(self_entity_id);
        }
    }

    fn render(
        &self,
        now: ElapsedTime,
        _tick: u64,
        assets: &AssetResources,
        render_commands: &mut RenderCommandCollector,
        _audio_commands: &mut AudioCommandCollectorComponent,
    ) {
        let anim_descr = SpriteRenderDescriptorComponent {
            action_index: 0,
            animation_started: ElapsedTime(0.0),
            animation_ends_at: ElapsedTime(0.0),
            forced_duration: None,
            direction: 0,
            fps_multiplier: 1.0,
        };
        render_action(
            now,
            &anim_descr,
            &assets.sprites.effect_sprites.plasma,
            &self.pos,
            [0, 0],
            false,
            self.configs.ball_size,
            ActionPlayMode::Repeat,
            &COLOR_WHITE,
            render_commands,
        );
    }
}

#[derive(Clone)]
pub struct PyroBlastTargetStatus {
    pub caster_entity_id: CharEntityId,
    pub splash_radius: f32,
}

impl Status for PyroBlastTargetStatus {
    fn dupl(&self) -> Box<dyn Status + Send> {
        Box::new(self.clone())
    }

    fn render(
        &self,
        char_state: &CharacterStateComponent,
        system_vars: &SystemVariables,
        render_commands: &mut RenderCommandCollector,
    ) {
        render_commands
            .horizontal_texture_3d()
            .pos(&char_state.pos())
            .rotation_rad(system_vars.time.0 % 6.28)
            .fix_size(self.splash_radius * 2.0)
            .add(&system_vars.assets.sprites.magic_target);
    }

    fn typ(&self) -> StatusNature {
        StatusNature::Neutral
    }
}
