use crate::components::char::{CharacterStateComponent, PhysicsComponent};
use crate::components::controller::HumanInputComponent;
use crate::components::status::absorb_shield::AbsorbStatus;
use crate::components::status::status::{
    ApplyStatusComponent, ApplyStatusComponentPayload, MainStatuses,
};
use crate::components::StrEffectComponent;
use crate::systems::console_system::{
    AutocompletionProvider, BasicAutocompletionProvider, CommandDefinition, CommandParamType,
    ConsoleComponent, ConsoleEntry, ConsoleSystem, ConsoleWordType,
};
use crate::systems::SystemVariables;
use crate::{ElapsedTime, PhysicEngine};
use nalgebra::Isometry2;
use specs::prelude::*;

struct SpawnEffectAutocompletion {
    effect_names: Vec<String>,
}

impl AutocompletionProvider for SpawnEffectAutocompletion {
    fn get_autocompletion_list(&self, param_index: usize) -> Option<Vec<String>> {
        Some(self.effect_names.clone())
    }
}

pub(super) fn cmd_spawn_effect(effect_names: Vec<String>) -> CommandDefinition {
    CommandDefinition {
        name: "spawn_effect".to_string(),
        arguments: vec![("effect_name", CommandParamType::String, true)],
        autocompletion: Box::new(SpawnEffectAutocompletion { effect_names }),
        action: Box::new(|self_entity_id, args, ecs_world| {
            let new_str_name = args.as_str(0).unwrap();
            {
                let system_vars = &mut ecs_world.write_resource::<SystemVariables>();
                if !system_vars
                    .map_render_data
                    .str_effects
                    .contains_key(new_str_name)
                {
                    system_vars
                        .asset_loader
                        .load_effect(new_str_name)
                        .and_then(|str_file| {
                            Ok(system_vars
                                .map_render_data
                                .str_effects
                                .insert(new_str_name.to_owned(), str_file))
                        });
                }
            }
            let hero_pos = {
                let storage = ecs_world.read_storage::<CharacterStateComponent>();
                let char_state = storage.get(self_entity_id).unwrap();
                char_state.pos()
            };
            ecs_world
                .create_entity()
                .with(StrEffectComponent {
                    effect: new_str_name.to_owned(),
                    pos: hero_pos,
                    start_time: ElapsedTime(0.0),
                    die_at: ElapsedTime(20000.0),
                    duration: ElapsedTime(1.0),
                })
                .build();
            Ok(())
        }),
    }
}

pub(super) fn cmd_player_list() -> CommandDefinition {
    CommandDefinition {
        name: "player_list".to_string(),
        arguments: vec![],
        autocompletion: BasicAutocompletionProvider::new(|_index| None),
        action: Box::new(|self_entity_id, args, ecs_world| {
            for (entity_id, human) in (
                &ecs_world.entities(),
                &ecs_world.read_storage::<HumanInputComponent>(),
            )
                .join()
            {
                ecs_world
                    .write_storage::<ConsoleComponent>()
                    .get_mut(self_entity_id)
                    .unwrap()
                    .add_entry(ConsoleEntry::new().add(
                        &format!(
                            "id: {}, gen: {:?}, name: {}",
                            entity_id.id(),
                            entity_id.gen(),
                            &human.username
                        ),
                        ConsoleWordType::Normal,
                    ));
            }
            Ok(())
        }),
    }
}

pub(super) fn cmd_add_status() -> CommandDefinition {
    CommandDefinition {
        name: "add_status".to_string(),
        arguments: vec![
            ("status_name", CommandParamType::String, true),
            ("time(ms)", CommandParamType::Int, true),
            ("[username]", CommandParamType::String, false),
        ],
        autocompletion: BasicAutocompletionProvider::new(|index| {
            if index == 0 {
                Some(vec![
                    "absorb".to_owned(),
                    "posion".to_owned(),
                    "firebomb".to_owned(),
                ])
            } else {
                None
            }
        }),
        action: Box::new(|self_entity_id, args, ecs_world| {
            let status_name = args.as_str(0).unwrap();
            let time = args.as_int(1).unwrap();

            let username = args.as_str(2);
            let entity_id = if let Some(username) = username {
                ConsoleSystem::get_user_id_by_name(ecs_world, username)
            } else {
                Some(self_entity_id)
            };

            if let Some(entity_id) = entity_id {
                let mut system_vars = ecs_world.write_resource::<SystemVariables>();
                let now = system_vars.time;
                system_vars.apply_statuses.push(ApplyStatusComponent {
                    source_entity_id: self_entity_id,
                    target_entity_id: entity_id,
                    status: match status_name {
                        "absorb" => ApplyStatusComponentPayload::from_secondary(Box::new(
                            AbsorbStatus::new(self_entity_id, now),
                        )),
                        _ => ApplyStatusComponentPayload::from_main_status(MainStatuses::Poison),
                    },
                });
                Ok(())
            } else {
                Err("The user was not found".to_owned())
            }
        }),
    }
}

pub(super) fn cmd_set_pos() -> CommandDefinition {
    CommandDefinition {
        name: "set_pos".to_string(),
        arguments: vec![
            ("x", CommandParamType::Int, true),
            ("y", CommandParamType::Int, true),
            ("[username]", CommandParamType::String, false),
        ],
        autocompletion: BasicAutocompletionProvider::new(|_index| None),
        action: Box::new(|self_entity_id, args, ecs_world| {
            let x = args.as_int(0).unwrap();
            let y = args.as_int(1).unwrap();
            let username = args.as_str(2);

            let entity_id = if let Some(username) = username {
                ConsoleSystem::get_user_id_by_name(ecs_world, username)
            } else {
                Some(self_entity_id)
            };

            let body_handle = entity_id.and_then(|it| {
                ecs_world
                    .read_storage::<PhysicsComponent>()
                    .get(it)
                    .map(|it| it.body_handle)
            });

            if let Some(body_handle) = body_handle {
                let physics_world = &mut ecs_world.write_resource::<PhysicEngine>();
                if let Some(body) = physics_world.bodies.rigid_body_mut(body_handle) {
                    body.set_position(Isometry2::translation(x as f32, y as f32));
                    Ok(())
                } else {
                    Err("No rigid body was found for this user".to_owned())
                }
            } else {
                Err("The user was not found".to_owned())
            }
        }),
    }
}
