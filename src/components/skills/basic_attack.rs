use nalgebra::{Vector2, Vector3};
use specs::{Entity, LazyUpdate};

use crate::components::char::{
    ActionPlayMode, CharActionIndex, CharAttributes, CharacterStateComponent,
    SpriteRenderDescriptorComponent,
};
use crate::components::controller::{CharEntityId, WorldCoord};
use crate::components::skills::skills::{
    SkillManifestation, SkillManifestationComponent, WorldCollisions,
};

use crate::common::ElapsedTime;
use crate::components::char::Percentage;
use crate::components::{
    DamageDisplayType, HpModificationRequest, HpModificationRequestType, SoundEffectComponent,
};
use crate::runtime_assets::map::PhysicEngine;
use crate::systems::next_action_applier_sys::NextActionApplierSystem;
use crate::systems::render::render_command::RenderCommandCollector;
use crate::systems::render_sys::render_single_layer_action;
use crate::systems::sound_sys::AudioCommandCollectorComponent;
use crate::systems::{AssetResources, SystemVariables};

#[derive(Clone, Debug, PartialEq)]
pub enum BasicAttack {
    MeleeSimple,
    MeleeCombo {
        combo_count: u8,
        base_dmg_percentage_for_each_combo: Percentage,
    },
    Ranged {
        bullet_type: WeaponType,
    },
}

impl BasicAttack {
    pub fn finish_attack(
        &self,
        calculated_attribs: &CharAttributes,
        caster_entity_id: CharEntityId,
        caster_pos: WorldCoord,
        target_pos: Vector2<f32>,
        target_entity_id: CharEntityId,
        sys_vars: &mut SystemVariables,
    ) -> Option<Box<dyn SkillManifestation>> {
        match self {
            BasicAttack::MeleeSimple => {
                sys_vars.hp_mod_requests.push(HpModificationRequest {
                    src_entity: caster_entity_id,
                    dst_entity: target_entity_id,
                    typ: HpModificationRequestType::BasicDamage(
                        calculated_attribs.attack_damage as u32,
                        DamageDisplayType::SingleNumber,
                        WeaponType::Sword,
                    ),
                });
                None
            }
            BasicAttack::MeleeCombo {
                combo_count,
                base_dmg_percentage_for_each_combo,
            } => {
                let p = base_dmg_percentage_for_each_combo;
                let dmg =
                    (p.of(calculated_attribs.attack_damage as i32) * *combo_count as i32) as u32;
                sys_vars.hp_mod_requests.push(HpModificationRequest {
                    src_entity: caster_entity_id,
                    dst_entity: target_entity_id,
                    typ: HpModificationRequestType::BasicDamage(
                        dmg,
                        DamageDisplayType::Combo(*combo_count),
                        WeaponType::Sword,
                    ),
                });
                None
            }
            BasicAttack::Ranged { bullet_type } => Some(Box::new(BasicRangeAttackBullet::new(
                calculated_attribs.attack_speed.as_f32(),
                caster_pos,
                caster_entity_id,
                target_entity_id,
                target_pos,
                sys_vars.time,
                *bullet_type,
                sys_vars.tick,
            ))),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum WeaponType {
    Sword,
    Arrow,
    SilverBullet,
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
    weapon_type: WeaponType,
    started_tick: u64,
}

impl BasicRangeAttackBullet {
    fn new(
        attack_speed: f32,
        start_pos: WorldCoord,
        caster_id: CharEntityId,
        target_id: CharEntityId,
        target_pos: WorldCoord,
        now: ElapsedTime,
        bullet_type: WeaponType,
        now_tick: u64,
    ) -> BasicRangeAttackBullet {
        BasicRangeAttackBullet {
            attack_speed,
            start_pos,
            current_pos: Vector2::new(0.0, 0.0),
            target_pos: Vector2::new(0.0, 0.0),
            caster_id,
            target_id,
            started_at: now,
            ends_at: now.add_seconds(((target_pos - start_pos).magnitude() * 0.05).min(0.3)),
            weapon_type: bullet_type,
            started_tick: now_tick,
        }
    }
}

impl SkillManifestation for BasicRangeAttackBullet {
    fn update(
        &mut self,
        self_entity_id: Entity,
        all_collisions_in_world: &WorldCollisions,
        sys_vars: &mut SystemVariables,
        entities: &specs::Entities,
        char_storage: &mut specs::WriteStorage<CharacterStateComponent>,
        physics_world: &mut PhysicEngine,
        updater: &mut LazyUpdate,
    ) {
        let now = sys_vars.time;
        if sys_vars.tick == self.started_tick + 1 {
            match self.weapon_type {
                WeaponType::Arrow => {
                    updater.insert(
                        entities.create(),
                        SoundEffectComponent {
                            target_entity_id: self.caster_id,
                            sound_id: sys_vars.assets.sounds.arrow_attack,
                            pos: self.start_pos,
                            start_time: now,
                        },
                    );
                }
                WeaponType::SilverBullet => {
                    updater.insert(
                        entities.create(),
                        SoundEffectComponent {
                            target_entity_id: self.caster_id,
                            sound_id: sys_vars.assets.sounds.gun_attack,
                            pos: self.start_pos,
                            start_time: now,
                        },
                    );
                }
                WeaponType::Sword => {}
            }
        }

        let travel_duration_percentage = sys_vars
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
                sys_vars.hp_mod_requests.push(HpModificationRequest {
                    src_entity: self.caster_id,
                    dst_entity: self.target_id,
                    typ: HpModificationRequestType::BasicDamage(
                        caster.calculated_attribs().attack_damage as u32,
                        DamageDisplayType::SingleNumber,
                        self.weapon_type,
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
        let (spr, scale) = match self.weapon_type {
            WeaponType::Arrow => (&assets.sprites.arrow, 1.0),
            WeaponType::SilverBullet => (&assets.sprites.ginseng_bullet, 0.25),
            WeaponType::Sword => panic!(),
        };
        render_single_layer_action(
            now,
            &anim,
            spr,
            &Vector3::new(self.current_pos.x, 2.0, self.current_pos.y),
            [0, 0],
            false,
            scale,
            ActionPlayMode::FixFrame(0),
            &[255, 255, 255, 255],
            render_commands,
        );
    }
}
