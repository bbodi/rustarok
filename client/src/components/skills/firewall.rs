use std::collections::HashMap;

use nalgebra::Vector2;
use nphysics2d::object::{DefaultBodyHandle, DefaultColliderHandle};
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
use crate::runtime_assets::map::PhysicEngine;
use crate::systems::{AssetResources, SystemVariables};
use crate::ElapsedTime;
use rustarok_common::common::{rotate_vec2, v2, EngineTime, Vec2, Vec2i};
use rustarok_common::components::char::{CharEntityId, StaticCharDataComponent, Team};
use rustarok_common::config::CommonConfigs;

pub struct FireWallSkill;

pub const FIRE_WALL_SKILL: &'static FireWallSkill = &FireWallSkill;

impl SkillDef for FireWallSkill {
    fn get_icon_path(&self) -> &'static str {
        "data\\texture\\À¯ÀúÀÎÅÍÆäÀÌ½º\\item\\mg_firewall.bmp"
    }

    fn finish_cast(
        &self,
        params: &FinishCast,
        ecs_world: &mut specs::world::World,
    ) -> Option<Box<dyn SkillManifestation>> {
        if let Some(caster) = ecs_world
            .read_storage::<StaticCharDataComponent>()
            .get(params.caster_entity_id.into())
        {
            let angle_in_rad = params.char_to_skill_dir.angle(&Vector2::y());
            let angle_in_rad = if params.char_to_skill_dir.x > 0.0 {
                angle_in_rad
            } else {
                -angle_in_rad
            };
            let time = ecs_world.read_resource::<EngineTime>();
            let entities = &ecs_world.entities();
            let mut updater = ecs_world.write_resource::<LazyUpdate>();
            let configs = &ecs_world.read_resource::<CommonConfigs>().skills.firewall;
            Some(Box::new(PushBackWallSkill::new(
                params.caster_entity_id,
                caster.team,
                configs.damage,
                configs.pushback_force,
                configs.force_duration_seconds,
                &mut ecs_world.write_resource::<PhysicEngine>(),
                &params.skill_pos.unwrap(),
                angle_in_rad,
                time.now(),
                time.simulation_frame,
                entities,
                &mut updater,
                configs.duration_seconds,
                configs.width,
            )))
        } else {
            None
        }
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
            &v2(configs.skills.firewall.width as f32, 1.0),
            skill_pos,
            char_to_skill_dir,
            render_commands,
        );
    }
}

pub struct PushBackWallSkill {
    caster_entity_id: CharEntityId,
    collider_handle: DefaultColliderHandle,
    effect_ids: Vec<Entity>,
    extents: Vec2i,
    pos: Vec2,
    rot_angle_in_rad: f32,
    die_at: ElapsedTime,
    cannot_damage_until: HashMap<CharEntityId, ElapsedTime>,
    born_tick: u64,
    team: Team,
    damage: u32,
    pushback_force: f32,
    force_duration_seconds: f32,
}

impl PushBackWallSkill {
    pub fn new(
        caster_entity_id: CharEntityId,
        team: Team,
        damage: u32,
        pushback_force: f32,
        force_duration_seconds: f32,
        physics_world: &mut PhysicEngine,
        skill_center: &Vec2,
        rot_angle_in_rad: f32,
        system_time: ElapsedTime,
        tick: u64,
        entities: &specs::Entities,
        updater: &mut LazyUpdate,
        duration_seconds: f32,
        width: u16,
    ) -> PushBackWallSkill {
        let effect_ids: Vec<Entity> = (0..width)
            .map(|x| {
                let x = x as f32;
                let x = x - (width as f32 / 2.0);
                skill_center + rotate_vec2(rot_angle_in_rad, &v2(x, 0.0))
            })
            .map(|effect_coords| {
                let effect_comp = StrEffectComponent {
                    effect_id: StrEffectType::FireWall.into(),
                    pos: effect_coords,
                    start_time: system_time,
                    die_at: Some(system_time.add_seconds(duration_seconds)),
                    play_mode: ActionPlayMode::Repeat,
                };
                let effect_entity = entities.create();
                updater.insert(effect_entity, effect_comp);
                effect_entity
            })
            .collect();

        let extents = Vec2i::new(3, 1);
        let (collider_handle, _body_handle) =
            physics_world.add_cuboid_skill_area(*skill_center, rot_angle_in_rad, v2(3.0, 1.0));

        PushBackWallSkill {
            caster_entity_id,
            effect_ids,
            collider_handle,
            rot_angle_in_rad,
            pos: *skill_center,
            extents,
            die_at: system_time.add_seconds(duration_seconds),
            cannot_damage_until: HashMap::new(),
            born_tick: tick,
            team,
            damage,
            pushback_force,
            force_duration_seconds,
        }
    }
}

impl SkillManifestation for PushBackWallSkill {
    fn update(&mut self, mut params: SkillManifestationUpdateParam) {
        // TODO2
        //        let now = params.time().now();
        //        let self_collider_handle = self.collider_handle;
        //        if self.die_at.has_already_passed(now) {
        //            params.physics_world.colliders.remove(self_collider_handle);
        //            params.remove_component::<SkillManifestationComponent>(params.self_entity_id);
        //            for effect_id in &self.effect_ids {
        //                params.remove_component::<StrEffectComponent>(*effect_id);
        //            }
        //        } else {
        //            // TODO: wouldn't it be better to use the area push functionality?
        //            let my_collisions = params
        //                .all_collisions_in_world
        //                .iter()
        //                .filter(|(_key, coll)| coll.other_coll_handle == self_collider_handle);
        //            for (_key, coll) in my_collisions {
        //                let collider: Option<(CharEntityId, DefaultBodyHandle)> = params
        //                    .physics_world
        //                    .colliders
        //                    .get(coll.character_coll_handle)
        //                    .map(|char_collider| {
        //                        (
        //                            *char_collider
        //                                .user_data()
        //                                .map(|v| v.downcast_ref().unwrap())
        //                                .unwrap(),
        //                            char_collider.body(),
        //                        )
        //                    });
        //                if let Some((target_char_entity_id, body_handle)) = collider {
        //                    let target = params
        //                        .char_storage
        //                        .get(target_char_entity_id.into())
        //                        .map(|target| (target.pos(), target.team));
        //                    if let Some((target_pos, target_team)) = target {
        //                        if !self.team.can_attack(target_team)
        //                            || !self
        //                                .cannot_damage_until
        //                                .get(&target_char_entity_id)
        //                                .unwrap_or(&now)
        //                                .has_already_passed(now)
        //                        {
        //                            continue;
        //                        }
        //                        let push_dir = self.pos - target_pos;
        //                        let push_dir = if push_dir.x == 0.0 && push_dir.y == 0.0 {
        //                            v2(1.0, 0.0) // "random"
        //                        } else {
        //                            -push_dir.normalize()
        //                        };
        //                        params.add_hp_mod_request(HpModificationRequest {
        //                            src_entity: self.caster_entity_id,
        //                            dst_entity: target_char_entity_id,
        //                            typ: HpModificationType::SpellDamage(
        //                                self.damage,
        //                                DamageDisplayType::SingleNumber,
        //                            ),
        //                        });
        //                        params.apply_force(ApplyForceComponent {
        //                            src_entity: self.caster_entity_id,
        //                            dst_entity: target_char_entity_id,
        //                            force: push_dir * self.pushback_force,
        //                            body_handle,
        //                            duration: self.force_duration_seconds,
        //                        });
        //                        self.cannot_damage_until.insert(
        //                            target_char_entity_id,
        //                            now.add_seconds(self.force_duration_seconds),
        //                        );
        //                    }
        //                }
        //            }
        //        }
    }

    fn render(
        &self,
        _char_entity_storage: &ReadStorage<StaticCharDataComponent>,
        _now: ElapsedTime,
        tick: u64,
        assets: &AssetResources,
        render_commands: &mut RenderCommandCollector,
        audio_command_collector: &mut AudioCommandCollectorComponent,
    ) {
        if self.born_tick + 1 == tick {
            audio_command_collector.add_sound_command(assets.sounds.firewall);
        }
        render_commands
            .rectangle_3d()
            .pos_2d(&self.pos)
            .rotation_rad(self.rot_angle_in_rad)
            .color(&[0, 255, 0, 255])
            .size(self.extents.x as f32, self.extents.y as f32)
            .add();
    }
}
