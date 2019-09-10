use crate::common::{rotate_vec2, v2_to_v3};
use crate::components::char::{CastingSkillData, CharacterStateComponent};
use crate::components::controller::{CharEntityId, WorldCoords};
use crate::components::skills::fire_bomb::FireBombStatus;
use crate::components::skills::lightning::{LightningManifest, LightningSkill};
use crate::components::status::absorb_shield::AbsorbStatus;
use crate::components::status::status::{
    ApplyStatusComponent, MainStatuses, RemoveStatusComponent, StatusType,
};
use crate::components::{
    ApplyForceComponent, AreaAttackComponent, AttackComponent, AttackType, SoundEffectComponent,
    StrEffectComponent,
};
use crate::effect::StrEffectType;
use crate::systems::render::render_command::RenderCommandCollectorComponent;
use crate::systems::render_sys::RenderDesktopClientSystem;
use crate::systems::sound_sys::AudioCommandCollectorComponent;
use crate::systems::{AssetResources, Collision, SystemVariables};
use crate::{ElapsedTime, PhysicEngine};
use nalgebra::{Isometry2, Vector2};
use nphysics2d::object::DefaultColliderHandle;
use specs::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use strum_macros::EnumIter;

pub type WorldCollisions = HashMap<(DefaultColliderHandle, DefaultColliderHandle), Collision>;

pub trait SkillManifestation {
    fn update(
        &mut self,
        entity_id: Entity,
        all_collisions_in_world: &WorldCollisions,
        system_vars: &mut SystemVariables,
        entities: &specs::Entities,
        char_storage: &specs::ReadStorage<CharacterStateComponent>,
        physics_world: &mut PhysicEngine,
        updater: &mut specs::Write<LazyUpdate>,
    );

    fn render(
        &self,
        now: ElapsedTime,
        tick: u64,
        assets: &AssetResources,
        render_commands: &mut RenderCommandCollectorComponent,
        audio_command_collector: &mut AudioCommandCollectorComponent,
    );
}

#[storage(HashMapStorage)]
#[derive(Component)]
pub struct SkillManifestationComponent {
    pub self_entity_id: Entity,
    pub skill: Arc<Mutex<Box<dyn SkillManifestation>>>,
}

impl SkillManifestationComponent {
    pub fn new(
        self_entity_id: Entity,
        skill: Box<dyn SkillManifestation>,
    ) -> SkillManifestationComponent {
        SkillManifestationComponent {
            self_entity_id,
            skill: Arc::new(Mutex::new(skill)),
        }
    }

    pub fn update(
        &mut self,
        self_entity_id: Entity,
        all_collisions_in_world: &WorldCollisions,
        system_vars: &mut SystemVariables,
        entities: &specs::Entities,
        char_storage: &specs::ReadStorage<CharacterStateComponent>,
        physics_world: &mut PhysicEngine,
        updater: &mut specs::Write<LazyUpdate>,
    ) {
        let mut skill = self.skill.lock().unwrap();
        skill.update(
            self_entity_id,
            all_collisions_in_world,
            system_vars,
            entities,
            char_storage,
            physics_world,
            updater,
        );
    }

    pub fn render(
        &self,
        now: ElapsedTime,
        tick: u64,
        assets: &AssetResources,
        render_commands: &mut RenderCommandCollectorComponent,
        audio_commands: &mut AudioCommandCollectorComponent,
    ) {
        let skill = self.skill.lock().unwrap();
        skill.render(now, tick, assets, render_commands, audio_commands);
    }
}

unsafe impl Sync for SkillManifestationComponent {}

unsafe impl Send for SkillManifestationComponent {}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash, EnumIter)]
pub enum Skills {
    FireWall,
    BrutalTestSkill,
    Lightning,
    Heal,
    Mounting,
    Poison,
    Cure,
    FireBomb,
    AbsorbShield,
}

impl Skills {
    pub fn get_icon_path(&self) -> &'static str {
        match self {
            Skills::FireWall => {
                "data\\texture\\À¯ÀúÀÎÅÍÆäÀÌ½º\\item\\mg_firewall.bmp"
            }
            Skills::BrutalTestSkill => {
                "data\\texture\\À¯ÀúÀÎÅÍÆäÀÌ½º\\item\\wz_meteor.bmp"
            }
            Skills::Lightning => {
                "data\\texture\\À¯ÀúÀÎÅÍÆäÀÌ½º\\item\\wl_chainlightning.bmp"
            }
            Skills::Heal => "data\\texture\\À¯ÀúÀÎÅÍÆäÀÌ½º\\item\\al_heal.bmp",
            Skills::Mounting => {
                "data\\texture\\À¯ÀúÀÎÅÍÆäÀÌ½º\\item\\su_pickypeck.bmp"
            }
            Skills::Poison => "data\\texture\\À¯ÀúÀÎÅÍÆäÀÌ½º\\item\\tf_poison.bmp",
            Skills::Cure => "data\\texture\\À¯ÀúÀÎÅÍÆäÀÌ½º\\item\\so_el_cure.bmp",
            Skills::FireBomb => {
                "data\\texture\\À¯ÀúÀÎÅÍÆäÀÌ½º\\item\\gn_makebomb.bmp"
            }
            Skills::AbsorbShield => {
                "data\\texture\\À¯ÀúÀÎÅÍÆäÀÌ½º\\item\\cr_reflectshield.bmp"
            }
        }
    }

    pub fn limit_vector_into_range(
        char_pos: &Vector2<f32>,
        mouse_pos: &WorldCoords,
        range: f32,
    ) -> (Vector2<f32>, Vector2<f32>) {
        let dir2d = mouse_pos - char_pos;
        let dir_vector = dir2d.normalize();
        let pos = char_pos + dir_vector * dir2d.magnitude().min(range);
        return (pos, dir_vector);
    }

    pub fn render_casting_box(
        is_castable: bool,
        casting_area_size: &Vector2<u16>,
        skill_pos: &Vector2<f32>,
        char_to_skill_dir: &Vector2<f32>,
        render_commands: &mut RenderCommandCollectorComponent,
    ) {
        let angle = char_to_skill_dir.angle(&Vector2::y());
        let angle = if char_to_skill_dir.x > 0.0 {
            angle
        } else {
            -angle
        };
        let skill_pos = v2_to_v3(skill_pos);

        render_commands
            .rectangle_3d()
            .pos(&skill_pos)
            .rotation_rad(angle)
            .color(
                &(if is_castable {
                    [0, 255, 0, 255]
                } else {
                    [179, 179, 179, 255]
                }),
            )
            .size(casting_area_size.x, casting_area_size.y)
            .add()
    }
}

#[derive(Eq, PartialEq)]
pub enum SkillTargetType {
    /// casts immediately
    NoTarget,
    Area,
    AnyEntity,
    OnlyAllyButNoSelf,
    OnlyAllyAndSelf,
    OnlyEnemy,
    OnlySelf,
}

impl Skills {
    pub fn finish_cast(
        &self,
        caster_entity_id: CharEntityId,
        char_pos: &Vector2<f32>,
        skill_pos: Option<Vector2<f32>>,
        char_to_skill_dir: &Vector2<f32>,
        target_entity: Option<CharEntityId>,
        physics_world: &mut PhysicEngine,
        system_vars: &mut SystemVariables,
        entities: &specs::Entities,
        updater: &mut specs::Write<LazyUpdate>,
    ) -> Option<Box<dyn SkillManifestation>> {
        match self {
            Skills::FireWall => {
                let angle_in_rad = char_to_skill_dir.angle(&Vector2::y());
                let angle_in_rad = if char_to_skill_dir.x > 0.0 {
                    angle_in_rad
                } else {
                    -angle_in_rad
                };
                Some(Box::new(PushBackWallSkill::new(
                    caster_entity_id,
                    physics_world,
                    &skill_pos.unwrap(),
                    angle_in_rad,
                    system_vars.time,
                    system_vars.tick,
                    entities,
                    updater,
                )))
            }
            Skills::BrutalTestSkill => {
                let angle_in_rad = char_to_skill_dir.angle(&Vector2::y());
                let angle_in_rad = if char_to_skill_dir.x > 0.0 {
                    angle_in_rad
                } else {
                    -angle_in_rad
                };
                Some(Box::new(BrutalSkillManifest::new(
                    caster_entity_id,
                    &skill_pos.unwrap(),
                    angle_in_rad,
                    system_vars.time,
                    entities,
                    updater,
                )))
            }
            Skills::Lightning => Some(Box::new(LightningManifest::new(
                caster_entity_id,
                &skill_pos.unwrap(),
                char_to_skill_dir,
                system_vars.time,
                entities,
            ))),
            Skills::Heal => {
                let target_entity_id = target_entity.unwrap();
                let entity = entities.create();
                updater.insert(
                    entity,
                    SoundEffectComponent {
                        target_entity_id,
                        sound_id: system_vars.assets.sounds.heal,
                        pos: *char_pos,
                        start_time: system_vars.time,
                    },
                );
                system_vars.attacks.push(AttackComponent {
                    src_entity: caster_entity_id,
                    dst_entity: target_entity_id,
                    typ: AttackType::Heal(8000),
                });
                None
            }
            Skills::Mounting => {
                system_vars
                    .apply_statuses
                    .push(ApplyStatusComponent::from_main_status(
                        caster_entity_id,
                        caster_entity_id,
                        MainStatuses::Mounted,
                    ));
                updater.insert(
                    entities.create(),
                    StrEffectComponent {
                        effect_id: StrEffectType::Concentration.into(),
                        pos: *char_pos,
                        start_time: system_vars.time,
                        die_at: system_vars.time.add_seconds(0.7),
                    },
                );
                None
            }
            Skills::Poison => {
                updater.insert(
                    entities.create(),
                    StrEffectComponent {
                        effect_id: StrEffectType::Poison.into(),
                        pos: skill_pos.unwrap(),
                        start_time: system_vars.time,
                        die_at: system_vars.time.add_seconds(0.7),
                    },
                );
                system_vars
                    .apply_statuses
                    .push(ApplyStatusComponent::from_main_status(
                        caster_entity_id,
                        target_entity.unwrap(),
                        MainStatuses::Poison,
                    ));
                None
            }
            Skills::Cure => {
                system_vars
                    .remove_statuses
                    .push(RemoveStatusComponent::from_secondary_status(
                        caster_entity_id,
                        target_entity.unwrap(),
                        StatusType::Harmful,
                    ));
                None
            }
            Skills::FireBomb => {
                system_vars
                    .apply_statuses
                    .push(ApplyStatusComponent::from_secondary_status(
                        caster_entity_id,
                        target_entity.unwrap(),
                        Box::new(FireBombStatus {
                            caster_entity_id,
                            started: system_vars.time,
                            until: system_vars.time.add_seconds(2.0),
                        }),
                    ));
                None
            }
            Skills::AbsorbShield => {
                system_vars
                    .apply_statuses
                    .push(ApplyStatusComponent::from_secondary_status(
                        caster_entity_id,
                        target_entity.unwrap(),
                        Box::new(AbsorbStatus::new(caster_entity_id, system_vars.time, 3.0)),
                    ));
                None
            }
        }
    }

    pub fn get_casting_time(&self, char_state: &CharacterStateComponent) -> ElapsedTime {
        let t = match self {
            Skills::FireWall => 0.0,
            Skills::BrutalTestSkill => 0.0,
            Skills::Lightning => 0.0,
            Skills::Heal => 0.0,
            Skills::Mounting => {
                if char_state.statuses.is_mounted() {
                    0.0
                } else {
                    2.0
                }
            }
            Skills::Poison => 0.0,
            Skills::Cure => 0.0,
            Skills::FireBomb => 0.0,
            Skills::AbsorbShield => 0.0,
        };
        return ElapsedTime(t);
    }

    pub fn get_cast_delay(&self, char_state: &CharacterStateComponent) -> ElapsedTime {
        let t = match self {
            Skills::FireWall => 3.0,
            Skills::BrutalTestSkill => 3.0,
            Skills::Lightning => 3.0,
            Skills::Heal => 1.0,
            Skills::Mounting => {
                if char_state.statuses.is_mounted() {
                    0.0
                } else {
                    4.0
                }
            }
            Skills::Poison => 2.0,
            Skills::Cure => 2.0,
            Skills::FireBomb => 2.0,
            Skills::AbsorbShield => 2.0,
        };
        return ElapsedTime(t);
    }

    pub fn get_casting_range(&self) -> f32 {
        match self {
            Skills::FireWall => 10.0,
            Skills::BrutalTestSkill => 20.,
            Skills::Lightning => 7.0,
            Skills::Heal => 10.0,
            Skills::Mounting => 0.0,
            Skills::Poison => 10.0,
            Skills::Cure => 10.0,
            Skills::FireBomb => 10.0,
            Skills::AbsorbShield => 10.0,
        }
    }

    pub fn get_skill_target_type(&self) -> SkillTargetType {
        match self {
            Skills::FireWall => SkillTargetType::Area,
            Skills::BrutalTestSkill => SkillTargetType::Area,
            Skills::Lightning => SkillTargetType::Area,
            Skills::Heal => SkillTargetType::OnlyAllyAndSelf,
            Skills::Mounting => SkillTargetType::NoTarget,
            Skills::Poison => SkillTargetType::OnlyEnemy,
            Skills::Cure => SkillTargetType::OnlyAllyAndSelf,
            Skills::FireBomb => SkillTargetType::OnlyEnemy,
            Skills::AbsorbShield => SkillTargetType::OnlyAllyAndSelf,
        }
    }

    pub fn render_casting(
        &self,
        char_pos: &Vector2<f32>,
        casting_state: &CastingSkillData,
        system_vars: &SystemVariables,
        render_commands: &mut RenderCommandCollectorComponent,
    ) {
        match self {
            _ => {
                RenderDesktopClientSystem::render_str(
                    StrEffectType::Moonstar,
                    casting_state.cast_started,
                    char_pos,
                    system_vars,
                    render_commands,
                );
                if let Some(target_area_pos) = casting_state.target_area_pos {
                    self.render_target_selection(
                        true,
                        &target_area_pos,
                        &casting_state.char_to_skill_dir_when_casted,
                        render_commands,
                    );
                }
            }
        }
    }

    pub fn is_casting_allowed_based_on_target(
        &self,
        caster_id: CharEntityId,
        target_entity: Option<CharEntityId>,
        target_distance: f32,
    ) -> bool {
        match self.get_skill_target_type() {
            SkillTargetType::Area => true,
            SkillTargetType::NoTarget => true,
            SkillTargetType::AnyEntity => {
                target_entity.is_some() && self.get_casting_range() >= target_distance
            }
            SkillTargetType::OnlyAllyButNoSelf => {
                target_entity.map(|it| it != caster_id).unwrap_or(false)
                    && self.get_casting_range() >= target_distance
            }
            SkillTargetType::OnlyAllyAndSelf => {
                target_entity.is_some() && self.get_casting_range() >= target_distance
            }
            SkillTargetType::OnlyEnemy => {
                target_entity.is_some() && self.get_casting_range() >= target_distance
            }
            SkillTargetType::OnlySelf => target_entity.map(|it| it == caster_id).unwrap_or(false),
        }
    }

    pub fn render_target_selection(
        &self,
        is_castable: bool,
        skill_pos: &Vector2<f32>,
        char_to_skill_dir: &Vector2<f32>,
        render_commands: &mut RenderCommandCollectorComponent,
    ) {
        match self {
            Skills::FireWall => {
                Skills::render_casting_box(
                    is_castable,
                    &Vector2::new(3, 1),
                    skill_pos,
                    char_to_skill_dir,
                    render_commands,
                );
            }
            Skills::BrutalTestSkill => {
                Skills::render_casting_box(
                    is_castable,
                    &Vector2::new(10, 10),
                    skill_pos,
                    char_to_skill_dir,
                    render_commands,
                );
            }
            Skills::Lightning => {
                LightningSkill::render_target_selection(
                    skill_pos,
                    char_to_skill_dir,
                    render_commands,
                );
            }
            Skills::Heal => {}
            Skills::Mounting => {}
            Skills::Poison => {}
            Skills::Cure => {}
            Skills::FireBomb => {}
            Skills::AbsorbShield => {}
        }
    }
}

pub struct PushBackWallSkill {
    pub caster_entity_id: CharEntityId,
    pub collider_handle: DefaultColliderHandle,
    pub effect_ids: Vec<Entity>,
    pub extents: Vector2<u16>,
    pub pos: Vector2<f32>,
    pub rot_angle_in_rad: f32,
    pub created_at: ElapsedTime,
    pub die_at: ElapsedTime,
    cannot_damage_until: HashMap<CharEntityId, ElapsedTime>,
    born_tick: u64,
}

impl PushBackWallSkill {
    const DAMAGE_DURATION_SECONDS: f32 = 1.0;

    pub fn new(
        caster_entity_id: CharEntityId,
        physics_world: &mut PhysicEngine,
        skill_center: &Vector2<f32>,
        rot_angle_in_rad: f32,
        system_time: ElapsedTime,
        tick: u64,
        entities: &specs::Entities,
        updater: &mut specs::Write<LazyUpdate>,
    ) -> PushBackWallSkill {
        let effect_ids = [
            skill_center + rotate_vec2(rot_angle_in_rad, &v2!(-1.0, 0.0)),
            *skill_center,
            skill_center + rotate_vec2(rot_angle_in_rad, &v2!(1.0, 0.0)),
        ]
        .iter()
        .map(|effect_coords| {
            let effect_comp = StrEffectComponent {
                effect_id: StrEffectType::FireWall.into(),
                pos: *effect_coords,
                start_time: system_time,
                die_at: system_time.add_seconds(3.0),
            };
            let effect_entity = entities.create();
            updater.insert(effect_entity, effect_comp);
            effect_entity
        })
        .collect();

        let extents = Vector2::new(3, 1);
        let collider_handle =
            physics_world.add_cuboid_skill(*skill_center, rot_angle_in_rad, v2!(3, 1));

        PushBackWallSkill {
            caster_entity_id,
            effect_ids,
            collider_handle,
            rot_angle_in_rad,
            pos: *skill_center,
            extents,
            created_at: system_time.clone(),
            die_at: system_time.add_seconds(2.0),
            cannot_damage_until: HashMap::new(),
            born_tick: tick,
        }
    }
}

impl SkillManifestation for PushBackWallSkill {
    fn update(
        &mut self,
        self_entity_id: Entity,
        all_collisions_in_world: &WorldCollisions,
        system_vars: &mut SystemVariables,
        _entities: &specs::Entities,
        char_storage: &specs::ReadStorage<CharacterStateComponent>,
        physics_world: &mut PhysicEngine,
        updater: &mut specs::Write<LazyUpdate>,
    ) {
        let now = system_vars.time;
        let self_collider_handle = self.collider_handle;
        if self.die_at.is_earlier_than(now) {
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
                    let char_entity_id = *char_collider
                        .user_data()
                        .map(|v| v.downcast_ref().unwrap())
                        .unwrap();
                    if !self
                        .cannot_damage_until
                        .get(&char_entity_id)
                        .unwrap_or(&now)
                        .is_earlier_than(now)
                    {
                        continue;
                    }
                    if let Some(char_state) = char_storage.get(char_entity_id.0) {
                        let push_dir = self.pos - char_state.pos();
                        let push_dir = if push_dir.x == 0.0 && push_dir.y == 0.0 {
                            v2!(1, 0) // "random"
                        } else {
                            -push_dir.normalize()
                        };
                        dbg!("Firewall push attack");
                        system_vars.attacks.push(AttackComponent {
                            src_entity: self.caster_entity_id,
                            dst_entity: char_entity_id,
                            typ: AttackType::SpellDamage(600),
                        });
                        system_vars.pushes.push(ApplyForceComponent {
                            src_entity: self.caster_entity_id,
                            dst_entity: char_entity_id,
                            force: push_dir * 20.0,
                            body_handle: char_collider.body(),
                            duration: PushBackWallSkill::DAMAGE_DURATION_SECONDS,
                        });
                        self.cannot_damage_until.insert(
                            char_entity_id,
                            now.add_seconds(PushBackWallSkill::DAMAGE_DURATION_SECONDS),
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
        render_commands: &mut RenderCommandCollectorComponent,
        audio_command_collector: &mut AudioCommandCollectorComponent,
    ) {
        if self.born_tick + 1 == tick {
            audio_command_collector.add_sound_command(assets.sounds.firewall);
        }
        render_commands
            .rectangle_3d()
            .pos_2d(&self.pos)
            .rotation_rad(self.rot_angle_in_rad)
            .color(&[0, 255, 0, 25])
            .size(self.extents.x, self.extents.y)
            .add();
    }
}

pub struct BrutalSkillManifest {
    pub caster_entity_id: CharEntityId,
    pub effect_ids: Vec<Entity>,
    pub extents: Vector2<u16>,
    pub half_extents: Vector2<f32>,
    pub pos: Vector2<f32>,
    pub rot_angle_in_rad: f32,
    pub created_at: ElapsedTime,
    pub die_at: ElapsedTime,
    pub next_damage_at: ElapsedTime,
}

impl BrutalSkillManifest {
    pub fn new(
        caster_entity_id: CharEntityId,
        skill_center: &Vector2<f32>,
        rot_angle_in_rad: f32,
        system_time: ElapsedTime,
        entities: &specs::Entities,
        updater: &mut specs::Write<LazyUpdate>,
    ) -> BrutalSkillManifest {
        let effect_ids = (0..11 * 11)
            .map(|i| {
                let x = -5.0 + (i % 10) as f32;
                let y = -5.0 + (i / 10) as f32;
                skill_center + rotate_vec2(rot_angle_in_rad, &v2!(x, y))
            })
            .map(|effect_coords| {
                let effect_comp = StrEffectComponent {
                    effect_id: StrEffectType::FireWall.into(),
                    pos: effect_coords,
                    start_time: system_time,
                    die_at: system_time.add_seconds(30.0),
                };
                let effect_entity = entities.create();
                updater.insert(effect_entity, effect_comp);
                effect_entity
            })
            .collect();
        //        let effect_comp = StrEffectComponent {
        //            effect: "StrEffect::LordOfVermilion".to_owned(),
        //            pos: *skill_center,
        //            start_time: system_time,
        //            die_at: system_time.add_seconds(3.0),
        //            duration: ElapsedTime(3.0),
        //        };
        //        let effect_entity = entities.create();
        //        updater.insert(effect_entity, effect_comp);
        //        let effect_ids = vec![effect_entity];
        BrutalSkillManifest {
            caster_entity_id,
            effect_ids,
            rot_angle_in_rad,
            pos: *skill_center,
            extents: Vector2::new(10, 10),
            half_extents: v2!(5.0, 5.0),
            created_at: system_time.clone(),
            die_at: system_time.add_seconds(30.0),
            next_damage_at: system_time,
        }
    }
}

impl SkillManifestation for BrutalSkillManifest {
    fn update(
        &mut self,
        self_entity_id: Entity,
        _all_collisions_in_world: &WorldCollisions,
        system_vars: &mut SystemVariables,
        _entities: &specs::Entities,
        _char_storage: &specs::ReadStorage<CharacterStateComponent>,
        _physics_world: &mut PhysicEngine,
        updater: &mut specs::Write<LazyUpdate>,
    ) {
        if self.die_at.is_earlier_than(system_vars.time) {
            updater.remove::<SkillManifestationComponent>(self_entity_id);
            for effect_id in &self.effect_ids {
                updater.remove::<StrEffectComponent>(*effect_id);
            }
        } else {
            if self.next_damage_at.is_later_than(system_vars.time) {
                return;
            }
            self.next_damage_at = system_vars.time.add_seconds(0.5);
            system_vars.area_attacks.push(AreaAttackComponent {
                area_shape: Box::new(ncollide2d::shape::Cuboid::new(self.half_extents)),
                area_isom: Isometry2::new(self.pos, self.rot_angle_in_rad),
                source_entity_id: self.caster_entity_id,
                typ: AttackType::SpellDamage(600),
            });
        }
    }

    fn render(
        &self,
        _now: ElapsedTime,
        _tick: u64,
        _assets: &AssetResources,
        render_commands: &mut RenderCommandCollectorComponent,
        _audio_commands: &mut AudioCommandCollectorComponent,
    ) {
        render_commands
            .rectangle_3d()
            .pos_2d(&self.pos)
            .rotation_rad(self.rot_angle_in_rad)
            .color(&[0, 255, 0, 255])
            .size(self.extents.x, self.extents.y)
            .add();
    }
}
