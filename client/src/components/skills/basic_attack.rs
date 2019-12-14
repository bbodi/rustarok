use crate::components::char::{
    ActionPlayMode, CharActionIndex, CharAttributes, CharacterStateComponent,
    SpriteRenderDescriptorComponent,
};
use crate::components::skills::skills::{
    SkillManifestation, SkillManifestationComponent, SkillManifestationUpdateParam,
};

use crate::audio::sound_sys::AudioCommandCollectorComponent;
use crate::components::char::Percentage;
use crate::components::{
    DamageDisplayType, HpModificationRequest, HpModificationType, SoundEffectComponent,
};
use crate::render::render_command::RenderCommandCollector;
use crate::render::render_sys::render_single_layer_action;
use crate::systems::next_action_applier_sys::NextActionApplierSystem;
use crate::systems::{AssetResources, CharEntityId, SystemVariables};
use rustarok_common::common::{v2, v3, ElapsedTime, Vec2};
use serde::Deserialize;
use serde::Serialize;
use specs::ReadStorage;

#[derive(Clone, Debug, PartialEq)]
#[allow(variant_size_differences)]
pub enum BasicAttackType {
    MeleeSimple,
    #[allow(dead_code)]
    MeleeCombo {
        combo_count: u8,
        base_dmg_percentage_for_each_combo: Percentage,
    },
    Ranged {
        bullet_type: WeaponType,
    },
}

impl BasicAttackType {
    pub fn finish_attack(
        &self,
        calculated_attribs: &CharAttributes,
        caster_entity_id: CharEntityId,
        caster_pos: Vec2,
        target_pos: Vec2,
        target_entity_id: CharEntityId,
        sys_vars: &mut SystemVariables,
    ) -> Option<Box<dyn SkillManifestation>> {
        match self {
            BasicAttackType::MeleeSimple => {
                sys_vars.hp_mod_requests.push(HpModificationRequest {
                    src_entity: caster_entity_id,
                    dst_entity: target_entity_id,
                    typ: HpModificationType::BasicDamage(
                        calculated_attribs.attack_damage as u32,
                        DamageDisplayType::SingleNumber,
                        WeaponType::Sword,
                    ),
                });
                None
            }
            BasicAttackType::MeleeCombo {
                combo_count,
                base_dmg_percentage_for_each_combo,
            } => {
                let p = base_dmg_percentage_for_each_combo;
                let dmg =
                    (p.of(calculated_attribs.attack_damage as i32) * *combo_count as i32) as u32;
                sys_vars.hp_mod_requests.push(HpModificationRequest {
                    src_entity: caster_entity_id,
                    dst_entity: target_entity_id,
                    typ: HpModificationType::BasicDamage(
                        dmg,
                        DamageDisplayType::Combo(*combo_count),
                        WeaponType::Sword,
                    ),
                });
                None
            }
            BasicAttackType::Ranged { bullet_type } => Some(Box::new(BasicRangeAttackBullet::new(
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

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum WeaponType {
    Sword,
    Arrow,
    SilverBullet,
}

struct BasicRangeAttackBullet {
    start_pos: Vec2,
    target_pos: Vec2,
    current_pos: Vec2,
    caster_id: CharEntityId,
    target_id: CharEntityId,
    started_at: ElapsedTime,
    ends_at: ElapsedTime,
    weapon_type: WeaponType,
    started_tick: u64,
}

impl BasicRangeAttackBullet {
    fn new(
        start_pos: Vec2,
        caster_id: CharEntityId,
        target_id: CharEntityId,
        target_pos: Vec2,
        now: ElapsedTime,
        bullet_type: WeaponType,
        now_tick: u64,
    ) -> BasicRangeAttackBullet {
        BasicRangeAttackBullet {
            start_pos,
            current_pos: v2(0.0, 0.0),
            target_pos: v2(0.0, 0.0),
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
    fn update(&mut self, mut params: SkillManifestationUpdateParam) {
        let now = params.now();
        if params.tick() == self.started_tick + 1 {
            match self.weapon_type {
                WeaponType::Arrow => {
                    params.create_entity_with_comp(SoundEffectComponent {
                        target_entity_id: self.caster_id,
                        sound_id: params.assets().sounds.arrow_attack,
                        pos: self.start_pos,
                        start_time: now,
                    });
                }
                WeaponType::SilverBullet => {
                    params.create_entity_with_comp(SoundEffectComponent {
                        target_entity_id: self.caster_id,
                        sound_id: params.assets().sounds.gun_attack,
                        pos: self.start_pos,
                        start_time: now,
                    });
                }
                WeaponType::Sword => {}
            }
        }

        let travel_duration_percentage = params
            .now()
            .percentage_between(self.started_at, self.ends_at);
        if travel_duration_percentage < 1.0 {
            if let Some(target) = params.char_storage.get(self.target_id.into()) {
                let dir = target.pos() - self.start_pos;
                self.current_pos = self.start_pos + dir * travel_duration_percentage;
                self.target_pos = target.pos();
            }
        } else {
            let attack_dmg = params
                .char_storage
                .get(self.caster_id.into())
                .map(|caster| caster.calculated_attribs().attack_damage as u32);
            if let Some(attack_dmg) = attack_dmg {
                params.add_hp_mod_request(HpModificationRequest {
                    src_entity: self.caster_id,
                    dst_entity: self.target_id,
                    typ: HpModificationType::BasicDamage(
                        attack_dmg,
                        DamageDisplayType::SingleNumber,
                        self.weapon_type,
                    ),
                });
            }
            // TODO: return with KeepIt or RemoveMe
            params.remove_component::<SkillManifestationComponent>(params.self_entity_id);
        }
    }

    fn render(
        &self,
        _char_entity_storage: &ReadStorage<CharacterStateComponent>,
        now: ElapsedTime,
        _tick: u64,
        assets: &AssetResources,
        render_commands: &mut RenderCommandCollector,
        _audio_command_collector: &mut AudioCommandCollectorComponent,
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
            &v3(self.current_pos.x, 2.0, self.current_pos.y),
            [0, 0],
            false,
            scale,
            ActionPlayMode::FixFrame(0),
            &[255, 255, 255, 255],
            render_commands,
        );
    }
}
