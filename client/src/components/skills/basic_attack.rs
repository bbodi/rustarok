use crate::components::char::{
    ActionPlayMode, CharActionIndex, CharacterStateComponent, SpriteRenderDescriptorComponent,
};
use crate::components::skills::skills::{
    SkillManifestation, SkillManifestationComponent, SkillManifestationUpdateParam,
};

use crate::audio::sound_sys::AudioCommandCollectorComponent;
use crate::components::SoundEffectComponent;
use crate::render::render_command::RenderCommandCollector;
use crate::render::render_sys::render_single_layer_action;
use crate::systems::{AssetResources, SystemVariables};
use rustarok_common::common::{v2, v3, ElapsedTime, EngineTime, Vec2};
use rustarok_common::components::char::{CharDir, CharEntityId};
use serde::Deserialize;
use serde::Serialize;
use specs::ReadStorage;

//impl SkillManifestation for BasicRangeAttackBullet {
//    fn update(&mut self, mut params: SkillManifestationUpdateParam) {
//        let now = params.time().now();
//        if params.time().simulation_frame == self.started_tick + 1 {
//            match self.weapon_type {
//                WeaponType::Arrow => {
//                    params.create_entity_with_comp(SoundEffectComponent {
//                        target_entity_id: self.caster_id,
//                        sound_id: params.assets().sounds.arrow_attack,
//                        pos: self.start_pos,
//                        start_time: now,
//                    });
//                }
//                WeaponType::SilverBullet => {
//                    params.create_entity_with_comp(SoundEffectComponent {
//                        target_entity_id: self.caster_id,
//                        sound_id: params.assets().sounds.gun_attack,
//                        pos: self.start_pos,
//                        start_time: now,
//                    });
//                }
//                WeaponType::Sword => {}
//            }
//        }
//
//        let travel_duration_percentage = params
//            .time()
//            .now()
//            .percentage_between(self.started_at, self.ends_at);
//        if travel_duration_percentage < 1.0 {
//            if let Some(target) = params.auth_state_storage.get(self.target_id.into()) {
//                let dir = target.pos() - self.start_pos;
//                self.current_pos = self.start_pos + dir * travel_duration_percentage;
//                self.target_pos = target.pos();
//            }
//        } else {
//            let attack_dmg = params
//                .char_storage
//                .get(self.caster_id.into())
//                .map(|caster| caster.calculated_attribs().attack_damage as u32);
//            if let Some(attack_dmg) = attack_dmg {
//                params.add_hp_mod_request(HpModificationRequest {
//                    src_entity: self.caster_id,
//                    dst_entity: self.target_id,
//                    typ: HpModificationType::BasicDamage(
//                        attack_dmg,
//                        DamageDisplayType::SingleNumber,
//                        self.weapon_type,
//                    ),
//                });
//            }
//            // TODO: return with KeepIt or RemoveMe
//            params.remove_component::<SkillManifestationComponent>(params.self_entity_id);
//        }
//    }
//
//    fn render(
//        &self,
//        _char_entity_storage: &ReadStorage<CharacterStateComponent>,
//        now: ElapsedTime,
//        _tick: u64,
//        assets: &AssetResources,
//        render_commands: &mut RenderCommandCollector,
//        _audio_command_collector: &mut AudioCommandCollectorComponent,
//    ) {
//        let dir = CharDir::determine_dir(&self.target_pos, &self.start_pos);
//        let anim = SpriteRenderDescriptorComponent {
//            action_index: CharActionIndex::Idle as usize,
//            animation_started: ElapsedTime(0.0),
//            animation_ends_at: ElapsedTime(0.0),
//            forced_duration: None,
//            direction: dir,
//            fps_multiplier: 1.0,
//        };
//        let (spr, scale) = match self.weapon_type {
//            WeaponType::Arrow => (&assets.sprites.arrow, 1.0),
//            WeaponType::SilverBullet => (&assets.sprites.ginseng_bullet, 0.25),
//            WeaponType::Sword => panic!(),
//        };
//        render_single_layer_action(
//            now,
//            &anim,
//            spr,
//            &v3(self.current_pos.x, 2.0, self.current_pos.y),
//            [0, 0],
//            false,
//            scale,
//            ActionPlayMode::FixFrame(0),
//            &[255, 255, 255, 255],
//            render_commands,
//        );
//    }
//}
