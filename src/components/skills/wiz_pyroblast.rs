use nalgebra::{Isometry2, Vector2};
use specs::{Entities, Entity, LazyUpdate, ReadStorage};

use crate::components::char::{
    ActionPlayMode, CastingSkillData, CharacterStateComponent, SpriteRenderDescriptorComponent,
};
use crate::components::controller::{CharEntityId, WorldCoords};
use crate::components::skills::skill::{
    SkillDef, SkillManifestation, SkillManifestationComponent, SkillTargetType, WorldCollisions,
};
use crate::components::status::status::{ApplyStatusComponent, Status, StatusNature};
use crate::components::{
    AreaAttackComponent, AttackComponent, AttackType, DamageDisplayType, StrEffectComponent,
};
use crate::configs::DevConfig;
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
        caster: &CharacterStateComponent,
        skill_pos: Option<Vector2<f32>>,
        char_to_skill_dir: &Vector2<f32>,
        target_entity: Option<CharEntityId>,
        physics_world: &mut PhysicEngine,
        system_vars: &mut SystemVariables,
        entities: &Entities,
        updater: &mut specs::Write<LazyUpdate>,
    ) -> Option<Box<dyn SkillManifestation>> {
        system_vars
            .apply_statuses
            .push(ApplyStatusComponent::from_secondary_status(
                caster_entity_id,
                target_entity.unwrap(),
                Box::new(PyroBlastTargetStatus { caster_entity_id }),
            ));
        Some(Box::new(PyroBlastManifest::new(
            caster_entity_id,
            caster.pos(),
            target_entity.unwrap(),
            system_vars.time,
            system_vars.dev_configs.skills.wiz_pyroblast.damage,
            physics_world,
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
                    (system_vars.dev_configs.skills.wiz_pyroblast.splash_radius
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
            forced_duration: Some(
                system_vars
                    .dev_configs
                    .skills
                    .wiz_pyroblast
                    .attributes
                    .casting_time,
            ),
            direction: 0,
            fps_multiplier: 1.0,
        };
        render_action(
            system_vars.time,
            &anim_descr,
            &system_vars.assets.sprites.effect_sprites.plasma,
            0.0,
            &(char_pos + casting_state.char_to_skill_dir_when_casted),
            [0, 0],
            false,
            system_vars.dev_configs.skills.wiz_pyroblast.ball_size * casting_percentage,
            ActionPlayMode::Reverse,
            &COLOR_WHITE,
            render_commands,
        );
    }
}

pub struct PyroBlastManifest {
    pub caster_entity_id: CharEntityId,
    pub pos: WorldCoords,
    pub damage: u32,
    pub target_last_pos: WorldCoords,
    pub target_entity_id: CharEntityId,
    pub created_at: ElapsedTime,
}

impl PyroBlastManifest {
    pub fn new(
        caster_entity_id: CharEntityId,
        pos: WorldCoords,
        target_entity_id: CharEntityId,
        created_at: ElapsedTime,
        damage: u32,
        physics_world: &mut PhysicEngine,
    ) -> PyroBlastManifest {
        PyroBlastManifest {
            caster_entity_id,
            pos,
            target_last_pos: Vector2::new(0.0, 0.0),
            target_entity_id,
            created_at,
            damage,
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
        updater: &mut specs::Write<LazyUpdate>,
    ) {
        if let Some(target_char) = char_storage.get_mut(self.target_entity_id.0) {
            let target_pos = target_char.pos();
            let dir_vector = target_pos - self.pos;
            let distance = dir_vector.magnitude();
            let configs = &system_vars.dev_configs.skills.wiz_pyroblast;
            if distance > 2.0 {
                let dir_vector = dir_vector.normalize();
                self.pos = self.pos + (dir_vector * system_vars.dt.0 * configs.moving_speed);
            } else {
                updater.remove::<SkillManifestationComponent>(self_entity_id);
                system_vars.attacks.push(AttackComponent {
                    src_entity: self.caster_entity_id,
                    dst_entity: self.target_entity_id,
                    typ: AttackType::SpellDamage(configs.damage, DamageDisplayType::SingleNumber),
                });
                let area_shape = Box::new(ncollide2d::shape::Ball::new(configs.splash_radius));
                let area_isom = Isometry2::new(target_pos, 0.0);
                system_vars.area_attacks.push(AreaAttackComponent {
                    area_shape,
                    area_isom,
                    source_entity_id: self.caster_entity_id,
                    typ: AttackType::SpellDamage(
                        configs.secondary_damage,
                        DamageDisplayType::SingleNumber,
                    ),
                    except: Some(self.target_entity_id),
                });
                updater.insert(
                    entities.create(),
                    StrEffectComponent {
                        effect_id: StrEffectType::Explosion.into(),
                        pos: target_pos,
                        start_time: system_vars.time.add_seconds(0.0),
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
        configs: &DevConfig,
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
            0.0,
            &self.pos,
            [0, 0],
            false,
            configs.skills.wiz_pyroblast.ball_size,
            ActionPlayMode::Repeat,
            &COLOR_WHITE,
            render_commands,
        );
    }
}

#[derive(Clone)]
pub struct PyroBlastTargetStatus {
    pub caster_entity_id: CharEntityId,
}

impl Status for PyroBlastTargetStatus {
    fn dupl(&self) -> Box<dyn Status> {
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
            .fix_size(system_vars.dev_configs.skills.wiz_pyroblast.splash_radius * 2.0)
            .add(&system_vars.assets.sprites.magic_target);
    }

    fn typ(&self) -> StatusNature {
        StatusNature::Neutral
    }
}
