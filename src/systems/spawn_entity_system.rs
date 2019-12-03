use crate::common::Vec2;
use crate::components::char::{CharOutlook, CharacterEntityBuilder, NpcComponent, Team};
use crate::configs::DevConfig;
use crate::consts::{JobId, MonsterId};
use crate::runtime_assets::map::PhysicEngine;
use crate::systems::CharEntityId;
use nphysics2d::object::BodyStatus;
use specs::prelude::*;
use specs::LazyUpdate;

pub struct SpawnEntitySystem;

impl SpawnEntitySystem {
    pub fn create_barricade(
        entities: &Entities,
        updater: &LazyUpdate,
        physics: &mut PhysicEngine,
        dev_configs: &DevConfig,
        team: Team,
        pos2d: Vec2,
    ) {
        let barricade_entity_id = CharEntityId::from(entities.create());
        updater.insert(barricade_entity_id.into(), NpcComponent);
        CharacterEntityBuilder::new(barricade_entity_id, "barricade")
            .insert_sprite_render_descr_component(updater)
            .physics(pos2d, physics, |builder| {
                builder
                    .collision_group(team.get_barricade_collision_group())
                    .rectangle(1.0, 1.0)
                    .body_status(BodyStatus::Static)
            })
            .char_state(updater, dev_configs, |ch| {
                ch.outlook(CharOutlook::Monster(MonsterId::Barricade))
                    .job_id(JobId::Barricade)
                    .team(team)
            });
    }
}
