use crate::components::char::{CharOutlook, CharacterEntityBuilder, NpcComponent, Team};
use crate::components::controller::{CharEntityId, WorldCoord};
use crate::configs::DevConfig;
use crate::consts::{JobId, MonsterId};
use crate::runtime_assets::map::PhysicEngine;
use crate::systems::SystemVariables;
use nphysics2d::object::BodyStatus;
use specs::prelude::*;
use specs::LazyUpdate;

pub enum SpawnEntityType {
    Barricade { pos: WorldCoord, team: Team },
}

#[derive(Component)]
pub struct SpawnEntityComponent {
    typ: SpawnEntityType,
}

impl SpawnEntityComponent {
    pub fn new(typ: SpawnEntityType) -> SpawnEntityComponent {
        SpawnEntityComponent { typ }
    }
}

pub struct SpawnEntitySystem;

impl<'a> specs::System<'a> for SpawnEntitySystem {
    type SystemData = (
        specs::Entities<'a>,
        specs::ReadStorage<'a, SpawnEntityComponent>,
        specs::ReadExpect<'a, SystemVariables>,
        specs::WriteExpect<'a, PhysicEngine>,
        specs::ReadExpect<'a, DevConfig>,
        specs::Write<'a, LazyUpdate>,
    );

    fn run(
        &mut self,
        (entities, spawn_entity_storage, sys_vars, mut physics, dev_configs, updater): Self::SystemData,
    ) {
        for (spawn_entity_comp_id, spawn_entity_comp) in (&entities, &spawn_entity_storage).join() {
            match spawn_entity_comp.typ {
                SpawnEntityType::Barricade { pos, team } => {
                    SpawnEntitySystem::create_barricade(
                        &entities,
                        &updater,
                        &mut physics,
                        &dev_configs,
                        team,
                        pos,
                    );
                }
            }

            updater.remove::<SpawnEntityComponent>(spawn_entity_comp_id);
        }
    }
}

impl SpawnEntitySystem {
    pub fn create_barricade(
        entities: &Entities,
        updater: &LazyUpdate,
        physics: &mut PhysicEngine,
        dev_configs: &DevConfig,
        team: Team,
        pos2d: WorldCoord,
    ) {
        let barricade_entity_id = CharEntityId(entities.create());
        updater.insert(barricade_entity_id.0, NpcComponent);
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
