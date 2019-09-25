use nalgebra::{Vector2, Vector3};
use specs::{Entity, LazyUpdate};

use crate::components::char::{
    ActionPlayMode, CharActionIndex, CharacterStateComponent, SpriteRenderDescriptorComponent,
};
use crate::components::controller::{CharEntityId, WorldCoord};
use crate::components::skills::skills::{
    SkillDef, SkillManifestation, SkillManifestationComponent, SkillTargetType, WorldCollisions,
};

use crate::common::ElapsedTime;
use crate::components::{AttackComponent, AttackType, DamageDisplayType};
use crate::runtime_assets::map::PhysicEngine;
use crate::systems::next_action_applier_sys::NextActionApplierSystem;
use crate::systems::render::render_command::RenderCommandCollector;
use crate::systems::render_sys::render_single_layer_action;
use crate::systems::sound_sys::AudioCommandCollectorComponent;
use crate::systems::{AssetResources, SystemVariables};

pub struct BasicRangedAttackSkill;

pub const BASIC_RANGED_ATTACK_SKILL: &'static BasicRangedAttackSkill = &BasicRangedAttackSkill;

impl SkillDef for BasicRangedAttackSkill {
    fn get_icon_path(&self) -> &'static str {
        ""
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
        let target_entity_id = target_entity.unwrap();
        if let Some(caster) = ecs_world
            .read_storage::<CharacterStateComponent>()
            .get(caster_entity_id.0)
        {
            Some(Box::new(BasicRangeAttackBullet::new(
                caster.calculated_attribs().attack_speed.as_f32(),
                caster.pos(),
                caster_entity_id,
                target_entity_id,
                skill_pos.unwrap(),
                ecs_world.read_resource::<SystemVariables>().time,
            )))
        } else {
            None
        }
    }

    fn get_skill_target_type(&self) -> SkillTargetType {
        SkillTargetType::OnlyEnemy
    }
}

struct BasicRangeAttackBullet {
    attack_speed: f32,
    start_pos: WorldCoord,
    target_pos: WorldCoord,
    current_pos: WorldCoord,
    caster_id: CharEntityId,
    target_id: CharEntityId,
    started_at: ElapsedTime,
    ends_at: ElapsedTime,
}

impl BasicRangeAttackBullet {
    fn new(
        attack_speed: f32,
        start_pos: WorldCoord,
        caster_id: CharEntityId,
        target_id: CharEntityId,
        target_pos: WorldCoord,
        now: ElapsedTime,
    ) -> BasicRangeAttackBullet {
        let duration_between_attacks = 1.0 / (attack_speed);
        BasicRangeAttackBullet {
            attack_speed,
            start_pos,
            current_pos: Vector2::new(0.0, 0.0),
            target_pos: Vector2::new(0.0, 0.0),
            caster_id,
            target_id,
            started_at: now,
            ends_at: now.add_seconds(duration_between_attacks),
        }
    }
}

impl SkillManifestation for BasicRangeAttackBullet {
    fn update(
        &mut self,
        self_entity_id: Entity,
        all_collisions_in_world: &WorldCollisions,
        system_vars: &mut SystemVariables,
        entities: &specs::Entities,
        char_storage: &mut specs::WriteStorage<CharacterStateComponent>,
        physics_world: &mut PhysicEngine,
        updater: &mut LazyUpdate,
    ) {
        let now = system_vars.time;

        let travel_duration_percentage = system_vars
            .time
            .percentage_between(self.started_at, self.ends_at);
        if travel_duration_percentage < 1.0 {
            if let Some(target) = char_storage.get(self.target_id.0) {
                let dir = target.pos() - self.start_pos;
                self.current_pos = self.start_pos + dir * travel_duration_percentage;
                self.target_pos = target.pos();
            }
        } else {
            if let Some(caster) = char_storage.get(self.caster_id.0) {
                system_vars.attacks.push(AttackComponent {
                    src_entity: self.caster_id,
                    dst_entity: self.target_id,
                    typ: AttackType::Basic(
                        caster.calculated_attribs().attack_damage as u32,
                        DamageDisplayType::SingleNumber,
                    ),
                });
            }
            // TODO: return with KeepIt or RemoveMe
            updater.remove::<SkillManifestationComponent>(self_entity_id);
        }
    }

    fn render(
        &self,
        now: ElapsedTime,
        tick: u64,
        assets: &AssetResources,
        render_commands: &mut RenderCommandCollector,
        audio_command_collector: &mut AudioCommandCollectorComponent,
    ) {
        let dir = NextActionApplierSystem::determine_dir(&self.target_pos, &self.start_pos);
        let anim = SpriteRenderDescriptorComponent {
            action_index: CharActionIndex::Idle as usize,
            animation_started: ElapsedTime(0.0),
            animation_ends_at: ElapsedTime(0.0),
            forced_duration: None,
            direction: dir,
            fps_multiplier: 1.0,
        };
        render_single_layer_action(
            now,
            &anim,
            &assets.sprites.ginseng_bullet,
            &Vector3::new(self.current_pos.x, 2.0, self.current_pos.y),
            [0, 0],
            false,
            0.25,
            ActionPlayMode::FixFrame(0),
            &[255, 255, 255, 255],
            render_commands,
        );
    }
}
