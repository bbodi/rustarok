use crate::asset::gat::CellType;
use crate::common::p3_to_p2;
use crate::components::char::{
    attach_char_components, CharOutlook, CharState, CharType, CharacterStateComponent,
    ComponentRadius, NpcComponent, PhysicsComponent, SpriteRenderDescriptorComponent,
};
use crate::components::char::{Percentage, Team};
use crate::components::controller::{
    CameraComponent, CharEntityId, ControllerComponent, ControllerEntityId, HumanInputComponent,
};
use crate::components::skills::fire_bomb::FireBombStatus;
use crate::components::skills::skill::SkillManifestationComponent;
use crate::components::status::absorb_shield::AbsorbStatus;
use crate::components::status::attrib_mod::ArmorModifierStatus;
use crate::components::status::heal_area::HealApplierArea;
use crate::components::status::status::{
    ApplyStatusComponent, ApplyStatusComponentPayload, MainStatuses,
};
use crate::components::status::status_applier_area::StatusApplierArea;
use crate::components::{
    AttackComponent, AttackType, BrowserClient, DamageDisplayType, MinionComponent,
    StrEffectComponent,
};
use crate::consts::{JobId, MonsterId};
use crate::effect::StrEffectId;
use crate::systems::console_system::{
    AutocompletionProvider, AutocompletionProviderWithUsernameCompletion,
    BasicAutocompletionProvider, CommandDefinition, CommandParamType, ConsoleComponent,
    ConsoleEntry, ConsoleSystem, ConsoleWordType,
};
use crate::systems::{Sex, SystemVariables};
use crate::{CharActionIndex, CollisionGroup, ElapsedTime, PhysicEngine};
use nalgebra::{Isometry2, Point2};
use nalgebra::{Point3, Vector2};
use rand::Rng;
use specs::prelude::*;
use std::collections::HashMap;
use std::str::FromStr;
use strum::IntoEnumIterator;

struct SpawnEffectAutocompletion {
    effect_names: Vec<String>,
}

impl AutocompletionProvider for SpawnEffectAutocompletion {
    fn get_autocompletion_list(
        &self,
        _param_index: usize,
        _input_storage: &specs::ReadStorage<HumanInputComponent>,
    ) -> Option<Vec<String>> {
        Some(self.effect_names.clone())
    }
}

pub(super) fn cmd_set_class() -> CommandDefinition {
    let outlook_names = JobId::iter()
        .map(|it| format!("{:?}", it))
        .collect::<Vec<_>>();
    CommandDefinition {
        name: "set_class".to_string(),
        arguments: vec![
            ("class_name", CommandParamType::String, true),
            ("[username]", CommandParamType::String, false),
        ],
        autocompletion: AutocompletionProviderWithUsernameCompletion::new(
            move |index, username_completor, input_storage| {
                if index == 0 {
                    Some(
                        [
                            JobId::iter().map(|it| it.to_string()).collect::<Vec<_>>(),
                            MonsterId::iter()
                                .map(|it| it.to_string())
                                .collect::<Vec<_>>(),
                        ]
                        .concat(),
                    )
                } else {
                    Some(username_completor(input_storage))
                }
            },
        ),
        action: Box::new(|_self_controller_id, self_char_id, args, ecs_world| {
            let job_name = args.as_str(0).unwrap();
            let username = args.as_str(1);

            let target_char_id = if let Some(username) = username {
                ConsoleSystem::get_char_id_by_name(ecs_world, username)
            } else {
                Some(self_char_id)
            };
            if let Some(target_char_id) = target_char_id {
                if let Some(target_char) = ecs_world
                    .write_storage::<CharacterStateComponent>()
                    .get_mut(target_char_id.0)
                {
                    let job_id = JobId::from_str(job_name);
                    if let Ok(job_id) = job_id {
                        target_char.outlook = match target_char.outlook {
                            CharOutlook::Player {
                                job_id: _old_job_id,
                                head_index,
                                sex,
                            } => CharOutlook::Player {
                                job_id,
                                head_index,
                                sex,
                            },
                            CharOutlook::Monster(_) => CharOutlook::Player {
                                job_id,
                                head_index: 0,
                                sex: Sex::Male,
                            },
                        };
                        Ok(())
                    } else if let Ok(monster_id) = MonsterId::from_str(job_name) {
                        target_char.outlook = CharOutlook::Monster(monster_id);
                        Ok(())
                    } else {
                        Err("Invalid JobId/MonsterId".to_owned())
                    }
                } else {
                    Err(format!(
                        "The character component does not exist: {:?}",
                        target_char_id
                    ))
                }
            } else {
                Err("The user was not found".to_owned())
            }
        }),
    }
}

pub(super) fn cmd_list_entities() -> CommandDefinition {
    CommandDefinition {
        name: "list_entities".to_string(),
        arguments: vec![],
        autocompletion: BasicAutocompletionProvider::new(|_index| None),
        action: Box::new(|self_controller_id, _self_char_id, _args, ecs_world| {
            let mut entities = HashMap::with_capacity(32);
            entities.insert("all", 0);
            entities.insert("left_team", 0);
            entities.insert("right_team", 0);
            entities.insert("merc_melee", 0);
            entities.insert("merc_range", 0);
            entities.insert("baphomet", 0);
            entities.insert("poring", 0);
            entities.insert("npc", 0);
            entities.insert("player", 0);
            for (entity_id, char_state) in (
                &ecs_world.entities(),
                &ecs_world.read_storage::<CharacterStateComponent>(),
            )
                .join()
            {
                *entities.get_mut("all").unwrap() += 1;
                if char_state.team == Team::Left {
                    *entities.get_mut("left_team").unwrap() += 1;
                } else {
                    *entities.get_mut("right_team").unwrap() += 1;
                }
                if let CharOutlook::Player {
                    job_id: JobId::SWORDMAN,
                    ..
                } = char_state.outlook
                {
                    *entities.get_mut("merc_melee").unwrap() += 1;
                } else if let CharOutlook::Player {
                    job_id: JobId::ARCHER,
                    ..
                } = char_state.outlook
                {
                    *entities.get_mut("merc_range").unwrap() += 1;
                } else if let CharOutlook::Monster(MonsterId::Baphomet) = char_state.outlook {
                    *entities.get_mut("baphomet").unwrap() += 1;
                } else if let CharOutlook::Monster(MonsterId::Poring) = char_state.outlook {
                    *entities.get_mut("poring").unwrap() += 1;
                }
                if ecs_world
                    .read_storage::<HumanInputComponent>()
                    .get(entity_id)
                    .is_none()
                {
                    *entities.get_mut("npc").unwrap() += 1;
                } else {
                    *entities.get_mut("player").unwrap() += 1;
                }
                for (name, count) in &entities {
                    ecs_world
                        .write_storage::<ConsoleComponent>()
                        .get_mut(self_controller_id.0)
                        .unwrap()
                        .add_entry(
                            ConsoleEntry::new()
                                .add(&format!("{:15} ", name), ConsoleWordType::Normal)
                                .add(&count.to_string(), ConsoleWordType::Param),
                        );
                }
            }
            Ok(())
        }),
    }
}

pub(super) fn cmd_kill_all() -> CommandDefinition {
    CommandDefinition {
        name: "kill_all".to_string(),
        arguments: vec![("type", CommandParamType::String, true)],
        autocompletion: BasicAutocompletionProvider::new(|index| match index {
            0 => Some(vec![
                "all".to_owned(),
                "left_team".to_owned(),
                "right_team".to_owned(),
                "merc_melee".to_owned(),
                "merc_range".to_owned(),
                "baphomet".to_owned(),
                "poring".to_owned(),
                "npc".to_owned(),
                "player".to_owned(),
            ]),
            _ => None,
        }),
        action: Box::new(|_self_controller_id, self_char_id, args, ecs_world| {
            let type_name = args.as_str(0).unwrap();
            let mut entity_ids = Vec::with_capacity(32);
            for (entity_id, char_state) in (
                &ecs_world.entities(),
                &ecs_world.read_storage::<CharacterStateComponent>(),
            )
                .join()
            {
                let entity_id = CharEntityId(entity_id);
                let need_delete = match type_name {
                    "all" => true,
                    "left_team" => char_state.team == Team::Left,
                    "right_team" => char_state.team == Team::Right,
                    "merc_melee" => match char_state.outlook {
                        CharOutlook::Player {
                            job_id: JobId::SWORDMAN,
                            ..
                        } => true,
                        _ => false,
                    },
                    "merc_range" => match char_state.outlook {
                        CharOutlook::Player {
                            job_id: JobId::ARCHER,
                            ..
                        } => true,
                        _ => false,
                    },
                    "baphomet" => {
                        if let CharOutlook::Monster(MonsterId::Baphomet) = char_state.outlook {
                            true
                        } else {
                            false
                        }
                    }
                    "poring" => {
                        if let CharOutlook::Monster(MonsterId::Poring) = char_state.outlook {
                            true
                        } else {
                            false
                        }
                    }
                    "npc" => ecs_world
                        .read_storage::<HumanInputComponent>()
                        .get(entity_id.0)
                        .is_none(),
                    "player" => ecs_world
                        .read_storage::<HumanInputComponent>()
                        .get(entity_id.0)
                        .is_some(),
                    _ => false,
                };
                if need_delete && entity_id != self_char_id {
                    entity_ids.push(entity_id);
                }
            }
            for entity_id in entity_ids {
                ecs_world
                    .write_storage::<CharacterStateComponent>()
                    .get_mut(entity_id.0)
                    .unwrap()
                    .hp = 0;
            }

            Ok(())
        }),
    }
}

pub(super) fn cmd_spawn_entity() -> CommandDefinition {
    CommandDefinition {
        name: "spawn_entity".to_string(),
        arguments: vec![
            ("type", CommandParamType::String, true),
            ("team", CommandParamType::String, true),
            ("[count:1]", CommandParamType::Int, false),
        ],
        autocompletion: BasicAutocompletionProvider::new(|index| match index {
            0 => Some(vec![
                "merc_melee".to_owned(),
                "merc_range".to_owned(),
                "baphomet".to_owned(),
                "poring".to_owned(),
            ]),
            1 => Some(vec!["left".to_owned(), "right".to_owned()]),
            _ => None,
        }),
        action: Box::new(|_self_controller_id, self_char_id, args, ecs_world| {
            let type_name = args.as_str(0).unwrap();
            let team = match args.as_str(1).unwrap() {
                "left" => Team::Left,
                _ => Team::Right,
            };
            let count = args.as_int(2).unwrap_or(1);
            for _ in 0..count {
                let pos = {
                    let map_render_data =
                        &ecs_world.read_resource::<SystemVariables>().map_render_data;
                    let hero_pos = {
                        let storage = ecs_world.read_storage::<CharacterStateComponent>();
                        let char_state = storage.get(self_char_id.0).unwrap();
                        char_state.pos()
                    };
                    let mut rng = rand::thread_rng();
                    let (x, y) = loop {
                        let x: f32 = rng.gen_range(hero_pos.x - 10.0, hero_pos.x + 10.0);
                        let y: f32 = rng.gen_range(hero_pos.y - 10.0, hero_pos.y + 10.0).abs();
                        let index = y.max(0.0) as usize * map_render_data.gat.width as usize
                            + x.max(0.0) as usize;
                        let walkable = (map_render_data.gat.cells[index].cell_type
                            & CellType::Walkable as u8)
                            != 0;
                        if walkable {
                            break (x, y);
                        }
                    };
                    p3!(x, 0.5, -y)
                };
                let pos2d = p3_to_p2(&pos);
                let mut rng = rand::thread_rng();
                match type_name {
                    "baphomet" | "poring" => {
                        let char_entity_id = create_monster(
                            ecs_world,
                            pos2d,
                            match type_name {
                                "baphomet" => MonsterId::Baphomet,
                                "poring" => MonsterId::Poring,
                                _ => MonsterId::Poring,
                            },
                            rng.gen_range(1, 3),
                            team,
                            CharType::Mercenary,
                            CollisionGroup::Player,
                            &[CollisionGroup::NonPlayer],
                        );
                        ecs_world
                            .create_entity()
                            .with(ControllerComponent::new(char_entity_id))
                            .with(MinionComponent { fountain_up: false })
                            .build();
                    }
                    _ => {
                        let job_id = if type_name == "merc_melee" {
                            JobId::SWORDMAN
                        } else {
                            JobId::ARCHER
                        };
                        let char_entity_id =
                            create_random_char_minion(ecs_world, pos2d, team, job_id);
                        ecs_world
                            .create_entity()
                            .with(ControllerComponent::new(char_entity_id))
                            .with(MinionComponent { fountain_up: false })
                            .build();
                    }
                }
            }

            Ok(())
        }),
    }
}

pub fn create_monster(
    ecs_world: &mut specs::world::World,
    pos2d: Point2<f32>,
    monster_id: MonsterId,
    radius: i32,
    team: Team,
    typ: CharType,
    collision_group: CollisionGroup,
    blacklist_coll_groups: &[CollisionGroup],
) -> CharEntityId {
    let entity_id = {
        let entity_id = CharEntityId(ecs_world.create_entity().build());
        ecs_world.write_storage().insert(
            entity_id.0,
            CharacterStateComponent::new(
                "monster".to_owned(),
                typ,
                CharOutlook::Monster(monster_id),
                team,
                &ecs_world.read_resource::<SystemVariables>().dev_configs,
            ),
        );
        ecs_world.write_storage().insert(
            entity_id.0,
            SpriteRenderDescriptorComponent {
                action_index: CharActionIndex::Idle as usize,
                animation_started: ElapsedTime(0.0),
                animation_ends_at: ElapsedTime(0.0),
                forced_duration: None,
                direction: 0,
                fps_multiplier: 1.0,
            },
        );
        ecs_world.write_storage().insert(entity_id.0, NpcComponent);
        entity_id
    };
    let mut storage = ecs_world.write_storage();
    let physics_world = &mut ecs_world.write_resource::<PhysicEngine>();
    let physics_component = PhysicsComponent::new(
        physics_world,
        pos2d.coords,
        ComponentRadius(radius),
        entity_id,
        collision_group,
        blacklist_coll_groups,
    );
    storage.insert(entity_id.0, physics_component).unwrap();
    return entity_id;
}

fn create_random_char_minion(
    ecs_world: &mut World,
    pos2d: Point2<f32>,
    team: Team,
    job_id: JobId,
) -> CharEntityId {
    let mut rng = rand::thread_rng();
    let sex = if rng.gen::<usize>() % 2 == 0 {
        Sex::Male
    } else {
        Sex::Female
    };

    let head_count = ecs_world
        .read_resource::<SystemVariables>()
        .assets
        .sprites
        .head_sprites[Sex::Male as usize]
        .len();
    let entity_id = CharEntityId(ecs_world.create_entity().build());
    ecs_world
        .read_resource::<LazyUpdate>()
        .insert(entity_id.0, NpcComponent);
    attach_char_components(
        "minion".to_owned(),
        entity_id,
        &ecs_world.read_resource::<LazyUpdate>(),
        &mut ecs_world.write_resource::<PhysicEngine>(),
        pos2d,
        sex,
        job_id,
        rng.gen::<usize>() % head_count,
        1,
        team,
        CharType::Minion,
        CollisionGroup::NonPlayer,
        &[
            //CollisionGroup::NonPlayer,
            CollisionGroup::Player,
            CollisionGroup::StaticModel,
        ],
        &ecs_world.read_resource::<SystemVariables>().dev_configs,
    );
    entity_id
}

pub(super) fn cmd_spawn_effect(effect_names: Vec<String>) -> CommandDefinition {
    CommandDefinition {
        name: "spawn_effect".to_string(),
        arguments: vec![("effect_name", CommandParamType::String, true)],
        autocompletion: Box::new(SpawnEffectAutocompletion { effect_names }),
        action: Box::new(|_self_controller_id, self_char_id, args, ecs_world| {
            let new_str_name = args.as_str(0).unwrap();
            let effect_id = {
                let system_vars = &mut ecs_world.write_resource::<SystemVariables>();
                system_vars
                    .asset_loader
                    .load_effect(
                        &system_vars.gl,
                        new_str_name,
                        &mut ecs_world.write_resource(),
                    )
                    .and_then(|str_file| {
                        let new_id = StrEffectId(system_vars.str_effects.len());
                        system_vars.str_effects.push(str_file);
                        Ok(new_id)
                    })
            };
            effect_id.and_then(|effect_id| {
                let hero_pos = {
                    let storage = ecs_world.read_storage::<CharacterStateComponent>();
                    let char_state = storage.get(self_char_id.0).unwrap();
                    char_state.pos()
                };
                ecs_world
                    .create_entity()
                    .with(StrEffectComponent {
                        effect_id,
                        pos: hero_pos,
                        start_time: ElapsedTime(0.0),
                        die_at: ElapsedTime(20000.0),
                    })
                    .build();
                Ok(())
            })
        }),
    }
}

pub(super) fn cmd_list_players() -> CommandDefinition {
    CommandDefinition {
        name: "list_players".to_string(),
        arguments: vec![],
        autocompletion: BasicAutocompletionProvider::new(|_index| None),
        action: Box::new(|self_controller_id, _self_char_id, _args, ecs_world| {
            print_console(
                &mut ecs_world.write_storage::<ConsoleComponent>(),
                self_controller_id,
                ConsoleEntry::new().add(
                    &format!(
                        "{:<15}{:>15}{:>17}{:>15}{:>15}",
                        "name", "traffic(sum)", "traffic/sec[KB]", "ping[ms]", "server fps",
                    ),
                    ConsoleWordType::CommandName,
                ),
            );
            for (entity_id, human) in (
                &ecs_world.entities(),
                &ecs_world.read_storage::<HumanInputComponent>(),
            )
                .join()
            {
                let (prev_bytes_per_second, sum_sent_bytes, ping, sending_fps) = ecs_world
                    .read_storage::<BrowserClient>()
                    .get(entity_id)
                    .map(|it| {
                        (
                            it.prev_bytes_per_second,
                            it.sum_sent_bytes,
                            it.ping,
                            it.sending_fps,
                        )
                    })
                    .unwrap_or((0, 0, 0, 1.0));
                print_console(
                    &mut ecs_world.write_storage::<ConsoleComponent>(),
                    self_controller_id,
                    ConsoleEntry::new().add(
                        &format!(
                            "{:<15}{:>15}{:>17}{:>15}{:>15}",
                            &human.username,
                            humanize_bytes(sum_sent_bytes),
                            format!("{:>8.2}", prev_bytes_per_second as f32 / KIB as f32),
                            ping,
                            (1.0 / sending_fps).round() as u32
                        ),
                        ConsoleWordType::Normal,
                    ),
                );
            }
            Ok(())
        }),
    }
}

/// bytes size for 1 kibibyte
const KIB: u64 = 1_024;
/// bytes size for 1 mebibyte
const MIB: u64 = 1_048_576;
/// bytes size for 1 gibibyte
const GIB: u64 = 1_073_741_824;
/// bytes size for 1 tebibyte
const TIB: u64 = 1_099_511_627_776;
/// bytes size for 1 pebibyte
const PIB: u64 = 1_125_899_906_842_624;
fn humanize_bytes(bytes: u64) -> String {
    if bytes / PIB > 0 {
        format!("{:.2} PB", bytes as f32 / PIB as f32)
    } else if bytes / TIB > 0 {
        format!("{:.2} TB", bytes as f32 / TIB as f32)
    } else if bytes / GIB > 0 {
        format!("{:.2} GB", bytes as f32 / GIB as f32)
    } else if bytes / MIB > 0 {
        format!("{:.2} MB", bytes as f32 / MIB as f32)
    } else if bytes / KIB > 0 {
        format!("{:.2} KB", bytes as f32 / KIB as f32)
    } else {
        format!("{}  B", bytes)
    }
}

fn print_console(
    console_storage: &mut WriteStorage<ConsoleComponent>,
    self_controller_id: ControllerEntityId,
    entry: ConsoleEntry,
) {
    console_storage
        .get_mut(self_controller_id.0)
        .unwrap()
        .add_entry(entry);
}

pub(super) fn cmd_heal() -> CommandDefinition {
    CommandDefinition {
        name: "heal".to_string(),
        arguments: vec![
            ("value", CommandParamType::Int, true),
            ("[username]", CommandParamType::String, false),
        ],
        autocompletion: AutocompletionProviderWithUsernameCompletion::new(
            |index, username_completor, input_storage| {
                if index == 1 {
                    Some(username_completor(input_storage))
                } else {
                    None
                }
            },
        ),
        action: Box::new(|_self_controller_id, self_char_id, args, ecs_world| {
            let value = args.as_int(0).unwrap().max(0);
            let username = args.as_str(1);
            let entity_id = if let Some(username) = username {
                ConsoleSystem::get_char_id_by_name(ecs_world, username)
            } else {
                Some(self_char_id)
            };

            if let Some(entity_id) = entity_id {
                let mut system_vars = ecs_world.write_resource::<SystemVariables>();
                system_vars.attacks.push(AttackComponent {
                    src_entity: self_char_id,
                    dst_entity: entity_id,
                    typ: AttackType::Heal(value as u32),
                });
                Ok(())
            } else {
                Err("The user was not found".to_owned())
            }
        }),
    }
}

pub(super) fn cmd_spawn_area() -> CommandDefinition {
    CommandDefinition {
        name: "spawn_area".to_string(),
        arguments: vec![
            ("name", CommandParamType::String, true),
            ("[value]", CommandParamType::Int, false),
            ("[width:2]", CommandParamType::Int, false),
            ("[height:3]", CommandParamType::Int, false),
            ("[interval(ms):500]", CommandParamType::Int, false),
            ("[time(ms):500]", CommandParamType::Int, false),
        ],
        autocompletion: BasicAutocompletionProvider::new(|index| {
            if index == 0 {
                let mut names = STATUS_NAMES
                    .iter()
                    .map(|it| (*it).to_owned())
                    .collect::<Vec<_>>();
                names.append(&mut vec!["heal".to_owned(), "damage".to_owned()]);
                Some(names)
            } else {
                None
            }
        }),
        action: Box::new(|_self_controller_id, self_char_id, args, ecs_world| {
            let name = args.as_str(0).unwrap();
            let value = args.as_int(1).unwrap_or(0);
            let width = args.as_int(2).unwrap_or(2).max(0) as u16;
            let height = args.as_int(3).unwrap_or(3).max(0) as u16;
            let interval = args.as_int(4).unwrap_or(500) as f32 / 1000.0;
            let time = args.as_int(5).unwrap_or(500);
            let x = args.as_int(6).map(|it| it as f32);
            let y = args.as_int(7).map(|it| it as f32);

            let pos = {
                let hero_pos = {
                    let storage = ecs_world.read_storage::<CharacterStateComponent>();
                    let char_state = storage.get(self_char_id.0).unwrap();
                    char_state.pos()
                };
                v2!(x.unwrap_or(hero_pos.x), y.unwrap_or(hero_pos.y))
            };
            let area_status_id = ecs_world.create_entity().build();
            ecs_world
                .write_storage()
                .insert(
                    area_status_id,
                    SkillManifestationComponent::new(
                        area_status_id,
                        match name {
                            "heal" => Box::new(HealApplierArea::new(
                                "Heal",
                                AttackType::Heal(value.max(0) as u32),
                                &pos,
                                Vector2::new(width, height),
                                interval,
                                self_char_id,
                                &mut ecs_world.write_resource::<PhysicEngine>(),
                            )),
                            "damage" => Box::new(HealApplierArea::new(
                                "Damage",
                                AttackType::Basic(
                                    value.max(0) as u32,
                                    DamageDisplayType::SingleNumber,
                                ),
                                &pos,
                                Vector2::new(width, height),
                                interval,
                                self_char_id,
                                &mut ecs_world.write_resource::<PhysicEngine>(),
                            )),
                            _ => {
                                let name = name.to_owned();
                                Box::new(StatusApplierArea::new(
                                    name.to_owned(),
                                    move |now| {
                                        create_status_payload(&name, self_char_id, now, time, value)
                                            .unwrap()
                                    },
                                    &pos,
                                    Vector2::new(width, height),
                                    self_char_id,
                                    &mut ecs_world.write_resource::<PhysicEngine>(),
                                ))
                            }
                        },
                    ),
                )
                .unwrap();
            Ok(())
        }),
    }
}

fn create_status_payload(
    name: &str,
    self_controller_id: CharEntityId,
    now: ElapsedTime,
    time: i32,
    value: i32,
) -> Result<ApplyStatusComponentPayload, String> {
    dbg!(name);
    dbg!(time);
    dbg!(value);
    match name {
        "absorb" => Ok(ApplyStatusComponentPayload::from_secondary(Box::new(
            AbsorbStatus::new(self_controller_id, now, time as f32 / 1000.0),
        ))),
        "firebomb" => Ok(ApplyStatusComponentPayload::from_secondary(Box::new(
            FireBombStatus {
                caster_entity_id: self_controller_id,
                started: now,
                until: now.add_seconds(time as f32 / 1000.0),
                damage: value.max(1) as u32,
            },
        ))),
        "poison" => Ok(ApplyStatusComponentPayload::from_main_status(
            MainStatuses::Poison(value.max(1) as u32),
        )),
        "armor" => Ok(ApplyStatusComponentPayload::from_secondary(Box::new(
            ArmorModifierStatus::new(now, Percentage(value)),
        ))),
        _ => Err("Status not found".to_owned()),
    }
}

pub const STATUS_NAMES: &'static [&'static str] = &["absorb", "poison", "firebomb", "armor"];

pub(super) fn cmd_add_status() -> CommandDefinition {
    CommandDefinition {
        name: "add_status".to_string(),
        arguments: vec![
            ("status_name", CommandParamType::String, true),
            ("time(ms)", CommandParamType::Int, true),
            ("[value]", CommandParamType::Int, false),
            ("[username]", CommandParamType::String, false),
        ],
        autocompletion: AutocompletionProviderWithUsernameCompletion::new(
            |index, username_completor, input_storage| {
                if index == 0 {
                    Some(STATUS_NAMES.iter().map(|it| (*it).to_owned()).collect())
                } else if index == 3 {
                    Some(username_completor(input_storage))
                } else {
                    None
                }
            },
        ),
        action: Box::new(|_self_controller_id, self_char_id, args, ecs_world| {
            let status_name = args.as_str(0).unwrap();
            let time = args.as_int(1).unwrap();
            let value = args.as_int(2).unwrap_or(0);

            let username = args.as_str(3);
            let entity_id = if let Some(username) = username {
                ConsoleSystem::get_char_id_by_name(ecs_world, username)
            } else {
                Some(self_char_id)
            };

            if let Some(entity_id) = entity_id {
                let mut system_vars = ecs_world.write_resource::<SystemVariables>();
                let now = system_vars.time;
                system_vars.apply_statuses.push(ApplyStatusComponent {
                    source_entity_id: self_char_id,
                    target_entity_id: entity_id,
                    status: create_status_payload(status_name, entity_id, now, time, value)?,
                });
                Ok(())
            } else {
                Err("The user was not found".to_owned())
            }
        }),
    }
}

pub(super) fn cmd_set_team() -> CommandDefinition {
    CommandDefinition {
        name: "set_team".to_string(),
        arguments: vec![
            ("team", CommandParamType::String, true),
            ("[charname]", CommandParamType::String, false),
        ],
        autocompletion: AutocompletionProviderWithUsernameCompletion::new(
            |index, username_completor, input_storage| {
                if index == 0 {
                    Some(vec!["left".to_owned(), "right".to_owned()])
                } else if index == 1 {
                    Some(username_completor(input_storage))
                } else {
                    None
                }
            },
        ),
        action: Box::new(|_self_controller_id, self_char_id, args, ecs_world| {
            let team = match args.as_str(0).unwrap() {
                "left" => Team::Left,
                _ => Team::Right,
            };
            let username = args.as_str(1);

            let target_entity_id = if let Some(username) = username {
                ConsoleSystem::get_char_id_by_name(ecs_world, username)
            } else {
                Some(self_char_id)
            };

            if let Some(target_char_id) = target_entity_id {
                let mut char_storage = ecs_world.write_storage::<CharacterStateComponent>();
                let char_state = char_storage.get_mut(target_char_id.0).unwrap();
                char_state.team = team;

                Ok(())
            } else {
                Err("The user was not found".to_owned())
            }
        }),
    }
}

pub(super) fn cmd_resurrect() -> CommandDefinition {
    CommandDefinition {
        name: "resurrect".to_string(),
        arguments: vec![("charname", CommandParamType::String, true)],
        autocompletion: AutocompletionProviderWithUsernameCompletion::new(
            |index, username_completor, input_storage| {
                if index == 0 {
                    Some(username_completor(input_storage))
                } else {
                    None
                }
            },
        ),
        action: Box::new(|_self_controller_id, _self_char_id, args, ecs_world| {
            let username = args.as_str(0).unwrap();
            let target_entity_id = ConsoleSystem::get_char_id_by_name(ecs_world, username);
            if let Some(target_char_id) = target_entity_id {
                let pos2d = {
                    // remove death status (that is the only status a death character has)
                    let mut char_storage = ecs_world.write_storage::<CharacterStateComponent>();
                    let char_state = char_storage.get_mut(target_char_id.0).unwrap();
                    char_state.statuses.remove_all();
                    char_state.set_state(CharState::Idle, char_state.dir());

                    // give him max hp/sp
                    char_state.hp = char_state.calculated_attribs().max_hp;
                    char_state.pos()
                };

                // give him back it's physic component
                let physics_component = PhysicsComponent::new(
                    &mut ecs_world.write_resource::<PhysicEngine>(),
                    pos2d,
                    ComponentRadius(1),
                    target_char_id,
                    CollisionGroup::Player,
                    &[CollisionGroup::NonPlayer],
                );
                ecs_world
                    .write_storage::<PhysicsComponent>()
                    .insert(target_char_id.0, physics_component);

                Ok(())
            } else {
                Err("The user was not found".to_owned())
            }
        }),
    }
}

pub(super) fn cmd_set_server_fps() -> CommandDefinition {
    CommandDefinition {
        name: "set_server_fps".to_string(),
        arguments: vec![
            ("username", CommandParamType::String, true),
            ("fps", CommandParamType::Int, true),
        ],
        autocompletion: AutocompletionProviderWithUsernameCompletion::new(
            |index, username_completor, input_storage| {
                if index == 0 {
                    Some(username_completor(input_storage))
                } else {
                    None
                }
            },
        ),
        action: Box::new(|self_controller_id, _self_char_id, args, ecs_world| {
            let username = args.as_str(0).unwrap();
            let fps = args.as_int(1).unwrap().max(1);

            let target_entity_id = ConsoleSystem::get_user_id_by_name(ecs_world, username);
            if let Some(target_entity_id) = target_entity_id {
                if let Some(browser) = ecs_world
                    .write_storage::<BrowserClient>()
                    .get_mut(target_entity_id.0)
                {
                    browser.set_sending_fps(fps as u32);
                    Ok(())
                } else {
                    Err("User is not a browser".to_owned())
                }
            } else {
                Err("The user was not found".to_owned())
            }
        }),
    }
}

pub(super) fn cmd_get_server_fps() -> CommandDefinition {
    CommandDefinition {
        name: "get_server_fps".to_string(),
        arguments: vec![("username", CommandParamType::String, true)],
        autocompletion: AutocompletionProviderWithUsernameCompletion::new(
            |index, username_completor, input_storage| Some(username_completor(input_storage)),
        ),
        action: Box::new(|self_controller_id, _self_char_id, args, ecs_world| {
            let username = args.as_str(0).unwrap();

            let target_entity_id = ConsoleSystem::get_user_id_by_name(ecs_world, username);
            if let Some(target_entity_id) = target_entity_id {
                if let Some(browser) = ecs_world
                    .read_storage::<BrowserClient>()
                    .get(target_entity_id.0)
                {
                    print_console(
                        &mut ecs_world.write_storage::<ConsoleComponent>(),
                        self_controller_id,
                        ConsoleEntry::new().add(
                            &format!("{}", (1.0 / browser.sending_fps).round() as u32),
                            ConsoleWordType::Normal,
                        ),
                    );
                    Ok(())
                } else {
                    Err("User is not a browser".to_owned())
                }
            } else {
                Err("The user was not found".to_owned())
            }
        }),
    }
}

pub(super) fn cmd_follow_char() -> CommandDefinition {
    CommandDefinition {
        name: "follow_char".to_string(),
        arguments: vec![("username", CommandParamType::String, true)],
        autocompletion: AutocompletionProviderWithUsernameCompletion::new(
            |_index, username_completor, input_storage| Some(username_completor(input_storage)),
        ),
        action: Box::new(|self_controller_id, _self_char_id, args, ecs_world| {
            let username = args.as_str(0).unwrap();

            let target_entity_id = ConsoleSystem::get_user_id_by_name(ecs_world, username);
            if let Some(target_entity_id) = target_entity_id {
                // remove controller from self
                ecs_world
                    .write_storage::<ControllerComponent>()
                    .remove(self_controller_id.0);

                // set camera to follow target
                ecs_world
                    .write_storage::<CameraComponent>()
                    .get_mut(self_controller_id.0)
                    .unwrap()
                    .followed_controller = Some(target_entity_id);
                Ok(())
            } else {
                Err("The user was not found".to_owned())
            }
        }),
    }
}

pub(super) fn cmd_control_char() -> CommandDefinition {
    CommandDefinition {
        name: "control_char".to_string(),
        arguments: vec![("charname", CommandParamType::String, true)],
        autocompletion: AutocompletionProviderWithUsernameCompletion::new(
            |_index, username_completor, input_storage| Some(username_completor(input_storage)),
        ),
        action: Box::new(|self_controller_id, _self_char_id, args, ecs_world| {
            let charname = args.as_str(0).unwrap();

            let target_char_id = ConsoleSystem::get_char_id_by_name(ecs_world, charname);
            if let Some(target_char_id) = target_char_id {
                // remove current controller and add a new one
                // TODO: skills should be reassigned as well
                ecs_world
                    .write_storage::<ControllerComponent>()
                    .remove(self_controller_id.0);

                ecs_world.write_storage::<ControllerComponent>().insert(
                    self_controller_id.0,
                    ControllerComponent::new(target_char_id),
                );

                // set camera to follow target
                ecs_world
                    .write_storage::<CameraComponent>()
                    .get_mut(self_controller_id.0)
                    .unwrap()
                    .followed_controller = Some(self_controller_id);
                Ok(())
            } else {
                Err("The user was not found".to_owned())
            }
        }),
    }
}

pub(super) fn cmd_set_mass() -> CommandDefinition {
    CommandDefinition {
        name: "set_mass".to_string(),
        arguments: vec![
            ("mass", CommandParamType::Float, true),
            ("[username]", CommandParamType::String, false),
        ],
        autocompletion: AutocompletionProviderWithUsernameCompletion::new(
            |index, username_completor, input_storage| {
                if index == 1 {
                    Some(username_completor(input_storage))
                } else {
                    None
                }
            },
        ),
        action: Box::new(|_self_controller_id, self_char_id, args, ecs_world| {
            let mass = args.as_f32(0).unwrap();
            let username = args.as_str(1);

            let entity_id = if let Some(username) = username {
                ConsoleSystem::get_char_id_by_name(ecs_world, username)
            } else {
                Some(self_char_id)
            };
            if let Some(entity_id) = entity_id {
                let body_handle = ecs_world
                    .read_storage::<PhysicsComponent>()
                    .get(entity_id.0)
                    .map(|it| it.body_handle)
                    .unwrap();
                let physics_world = &mut ecs_world.write_resource::<PhysicEngine>();
                if let Some(body) = physics_world.bodies.rigid_body_mut(body_handle) {
                    body.set_mass(mass);
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

pub(super) fn cmd_set_damping() -> CommandDefinition {
    CommandDefinition {
        name: "set_damping".to_string(),
        arguments: vec![
            ("damping", CommandParamType::Float, true),
            ("[username]", CommandParamType::String, false),
        ],
        autocompletion: AutocompletionProviderWithUsernameCompletion::new(
            |index, username_completor, input_storage| {
                if index == 1 {
                    Some(username_completor(input_storage))
                } else {
                    None
                }
            },
        ),
        action: Box::new(|_self_controller_id, self_char_id, args, ecs_world| {
            let damping = args.as_f32(0).unwrap();
            let username = args.as_str(1);

            let entity_id = if let Some(username) = username {
                ConsoleSystem::get_char_id_by_name(ecs_world, username)
            } else {
                Some(self_char_id)
            };
            if let Some(entity_id) = entity_id {
                let body_handle = ecs_world
                    .read_storage::<PhysicsComponent>()
                    .get(entity_id.0)
                    .map(|it| it.body_handle)
                    .unwrap();
                let physics_world = &mut ecs_world.write_resource::<PhysicEngine>();
                if let Some(body) = physics_world.bodies.rigid_body_mut(body_handle) {
                    body.set_linear_damping(damping);
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

pub(super) fn cmd_goto() -> CommandDefinition {
    CommandDefinition {
        name: "goto".to_string(),
        arguments: vec![("username", CommandParamType::String, true)],
        autocompletion: AutocompletionProviderWithUsernameCompletion::new(
            |_index, username_completor, input_storage| Some(username_completor(input_storage)),
        ),
        action: Box::new(|_self_controller_id, self_char_id, args, ecs_world| {
            let username = args.as_str(0).unwrap();

            let target_char_id = ConsoleSystem::get_char_id_by_name(ecs_world, username);
            if let Some(target_char_id) = target_char_id {
                let target_pos = {
                    let storage = ecs_world.read_storage::<CharacterStateComponent>();
                    let char_state = storage.get(target_char_id.0).unwrap();
                    char_state.pos()
                };
                let self_body_handle = ecs_world
                    .read_storage::<PhysicsComponent>()
                    .get(self_char_id.0)
                    .map(|it| it.body_handle)
                    .unwrap();
                let physics_world = &mut ecs_world.write_resource::<PhysicEngine>();
                if let Some(self_body) = physics_world.bodies.rigid_body_mut(self_body_handle) {
                    self_body.set_position(Isometry2::translation(target_pos.x, target_pos.y));
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

pub(super) fn cmd_get_pos() -> CommandDefinition {
    CommandDefinition {
        name: "get_pos".to_string(),
        arguments: vec![("[username]", CommandParamType::String, false)],
        autocompletion: AutocompletionProviderWithUsernameCompletion::new(
            |_index, username_completor, input_storage| Some(username_completor(input_storage)),
        ),
        action: Box::new(|self_controller_id, self_char_id, args, ecs_world| {
            let username = args.as_str(0);

            let entity_id = if let Some(username) = username {
                ConsoleSystem::get_char_id_by_name(ecs_world, username)
            } else {
                Some(self_char_id)
            };

            if let Some(entity_id) = entity_id {
                let hero_pos = {
                    let storage = ecs_world.read_storage::<CharacterStateComponent>();
                    let char_state = storage.get(entity_id.0).unwrap();
                    char_state.pos()
                };
                print_console(
                    &mut ecs_world.write_storage::<ConsoleComponent>(),
                    self_controller_id,
                    ConsoleEntry::new().add(
                        &format!("{}, {}", hero_pos.x as i32, hero_pos.y as i32),
                        ConsoleWordType::Normal,
                    ),
                );
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
        autocompletion: AutocompletionProviderWithUsernameCompletion::new(
            |index, username_completor, input_storage| {
                if index == 2 {
                    Some(username_completor(input_storage))
                } else {
                    None
                }
            },
        ),
        action: Box::new(|_self_controller_id, self_char_id, args, ecs_world| {
            let x = args.as_int(0).unwrap();
            let y = args.as_int(1).unwrap();
            let username = args.as_str(2);

            let entity_id = if let Some(username) = username {
                ConsoleSystem::get_char_id_by_name(ecs_world, username)
            } else {
                Some(self_char_id)
            };

            let body_handle = entity_id.and_then(|it| {
                ecs_world
                    .read_storage::<PhysicsComponent>()
                    .get(it.0)
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
