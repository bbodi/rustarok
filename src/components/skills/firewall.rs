use std::collections::HashMap;

use nalgebra::Vector2;
use nphysics2d::object::DefaultColliderHandle;
use specs::{Entity, LazyUpdate};

use crate::common::rotate_vec2;
use crate::components::char::{ActionPlayMode, CharacterStateComponent, Team};
use crate::components::controller::{CharEntityId, WorldCoord};
use crate::components::skills::skills::{
    SkillDef, SkillManifestation, SkillManifestationComponent, SkillTargetType, Skills,
    WorldCollisions,
};
use crate::components::{
    ApplyForceComponent, AttackComponent, AttackType, DamageDisplayType, StrEffectComponent,
};
use crate::configs::DevConfig;
use crate::effect::StrEffectType;
use crate::runtime_assets::map::PhysicEngine;
use crate::systems::render::render_command::RenderCommandCollector;
use crate::systems::sound_sys::AudioCommandCollectorComponent;
use crate::systems::{AssetResources, SystemVariables};
use crate::ElapsedTime;

pub struct FireWallSkill;

pub const FIRE_WALL_SKILL: &'static FireWallSkill = &FireWallSkill;

impl SkillDef for FireWallSkill {
    fn get_icon_path(&self) -> &'static str {
        "data\\texture\\À¯ÀúÀÎÅÍÆäÀÌ½º\\item\\mg_firewall.bmp"
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
        if let Some(caster) = ecs_world
            .read_storage::<CharacterStateComponent>()
            .get(caster_entity_id.0)
        {
            let angle_in_rad = char_to_skill_dir.angle(&Vector2::y());
            let angle_in_rad = if char_to_skill_dir.x > 0.0 {
                angle_in_rad
            } else {
                -angle_in_rad
            };
            let sys_vars = ecs_world.read_resource::<SystemVariables>();
            let entities = &ecs_world.entities();
            let mut updater = ecs_world.write_resource::<LazyUpdate>();
            let configs = ecs_world.read_resource::<DevConfig>();
            Some(Box::new(PushBackWallSkill::new(
                caster_entity_id,
                caster.team,
                configs.skills.firewall.damage,
                configs.skills.firewall.pushback_force,
                configs.skills.firewall.force_duration_seconds,
                &mut ecs_world.write_resource::<PhysicEngine>(),
                &skill_pos.unwrap(),
                angle_in_rad,
                sys_vars.time,
                sys_vars.tick,
                entities,
                &mut updater,
                configs.skills.firewall.duration_seconds,
                configs.skills.firewall.width,
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
        skill_pos: &Vector2<f32>,
        char_to_skill_dir: &Vector2<f32>,
        render_commands: &mut RenderCommandCollector,
        configs: &DevConfig,
    ) {
        Skills::render_casting_box(
            is_castable,
            &Vector2::new(configs.skills.firewall.width as f32, 1.0),
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
    extents: Vector2<u16>,
    pos: Vector2<f32>,
    rot_angle_in_rad: f32,
    created_at: ElapsedTime,
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
        skill_center: &Vector2<f32>,
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
                skill_center + rotate_vec2(rot_angle_in_rad, &v2!(x, 0.0))
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

        let extents = Vector2::new(3, 1);
        let (collider_handle, _body_handle) =
            physics_world.add_cuboid_skill_area(*skill_center, rot_angle_in_rad, v2!(3, 1));

        PushBackWallSkill {
            caster_entity_id,
            effect_ids,
            collider_handle,
            rot_angle_in_rad,
            pos: *skill_center,
            extents,
            created_at: system_time.clone(),
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
    fn update(
        &mut self,
        self_entity_id: Entity,
        all_collisions_in_world: &WorldCollisions,
        sys_vars: &mut SystemVariables,
        _entities: &specs::Entities,
        char_storage: &mut specs::WriteStorage<CharacterStateComponent>,
        physics_world: &mut PhysicEngine,
        updater: &mut LazyUpdate,
    ) {
        let now = sys_vars.time;
        let self_collider_handle = self.collider_handle;
        if self.die_at.has_already_passed(now) {
            physics_world.colliders.remove(self_collider_handle);
            updater.remove::<SkillManifestationComponent>(self_entity_id);
            for effect_id in &self.effect_ids {
                updater.remove::<StrEffectComponent>(*effect_id);
            }
        } else {
            // TODO: wouldn't it be better to use the area push functionality?
            let my_collisions = all_collisions_in_world
                .iter()
                .filter(|(_key, coll)| coll.other_coll_handle == self_collider_handle);
            for (_key, coll) in my_collisions {
                if let Some(char_collider) = physics_world.colliders.get(coll.character_coll_handle)
                {
                    let target_char_entity_id: CharEntityId = *char_collider
                        .user_data()
                        .map(|v| v.downcast_ref().unwrap())
                        .unwrap();
                    if let Some(target_char) = char_storage.get(target_char_entity_id.0) {
                        if !self.team.can_attack(target_char.team)
                            || !self
                                .cannot_damage_until
                                .get(&target_char_entity_id)
                                .unwrap_or(&now)
                                .has_already_passed(now)
                        {
                            continue;
                        }
                        let push_dir = self.pos - target_char.pos();
                        let push_dir = if push_dir.x == 0.0 && push_dir.y == 0.0 {
                            v2!(1, 0) // "random"
                        } else {
                            -push_dir.normalize()
                        };
                        sys_vars.attacks.push(AttackComponent {
                            src_entity: self.caster_entity_id,
                            dst_entity: target_char_entity_id,
                            typ: AttackType::SpellDamage(
                                self.damage,
                                DamageDisplayType::SingleNumber,
                            ),
                        });
                        sys_vars.pushes.push(ApplyForceComponent {
                            src_entity: self.caster_entity_id,
                            dst_entity: target_char_entity_id,
                            force: push_dir * self.pushback_force,
                            body_handle: char_collider.body(),
                            duration: self.force_duration_seconds,
                        });
                        self.cannot_damage_until.insert(
                            target_char_entity_id,
                            now.add_seconds(self.force_duration_seconds),
                        );
                    }
                }
            }
        }
    }

    fn render(
        &self,
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
