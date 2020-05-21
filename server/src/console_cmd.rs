use crate::server_config::load_common_configs;
use crate::OutPacketCollector;
use crate::PacketTarget;
use rand::Rng;
use rustarok_common::char_attr::CharAttributes;
use rustarok_common::common::{v2, v3_to_v2, Local, Vec2};
use rustarok_common::components::char::{
    CharOutlook, CharType, ControllerEntityId, EntityId, JobId, LocalCharStateComp, MonsterId, Sex,
    StaticCharDataComponent, Team,
};
use rustarok_common::components::controller::ControllerComponent;
use rustarok_common::components::job_ids::JobSpriteId;
use rustarok_common::config::CommonConfigs;
use rustarok_common::console::CommandArguments;
use rustarok_common::map::{CellType, MapWalkingInfo};
use rustarok_common::packets::from_server::FromServerPacket;
use rustarok_common::packets::SocketId;
use specs::world::Builder;
use specs::world::WorldExt;
use specs::Join;
use std::str::FromStr;

pub fn execute_console_cmd(
    controller_id: Option<ControllerEntityId>,
    args: CommandArguments,
    ecs_world: &mut specs::World,
) {
    match args.get_command_name() {
        Some("kill_all") => {
            cmd_kill_all(controller_id, args, ecs_world);
        }
        Some("reload_configs") => {
            cmd_reload_configs(controller_id, args, ecs_world);
        }
        Some("spawn_entity") => {
            cmd_spawn_entity(controller_id, args, ecs_world);
        }
        _ => {
            log::error!("Unknown command: {:?}", args.get_command_name());
        }
    }
}

fn get_client_char_id(
    controller_id: Option<ControllerEntityId>,
    ecs_world: &mut specs::World,
) -> Option<EntityId<Local>> {
    controller_id.and_then(|controller_id| {
        let controller_storage = ecs_world.read_storage::<ControllerComponent>();
        let controller: &ControllerComponent =
            controller_storage.get(controller_id.into()).unwrap();
        controller.controlled_entity
    })
}

fn cmd_spawn_entity(
    _controller_id: Option<ControllerEntityId>,
    args: CommandArguments,
    ecs_world: &mut specs::World,
) -> Result<(), String> {
    let type_name = args.as_str(0).unwrap();
    let team = match args.as_str(1).unwrap() {
        "left" => Team::Left,
        _ => Team::Right,
    };
    let count = args.as_int(2).unwrap_or(1);
    let pos2d = match (args.as_int(3), args.as_int(4)) {
        (Some(x), Some(y)) => v2(x as f32, y as f32),
        _ => {
            let gat = &ecs_world.read_resource::<MapWalkingInfo>();
            let hero_pos = ecs_world.read_resource::<LocalCharStateComp<Local>>().pos();
            //            let mut rng = rand::thread_rng();
            //            let (x, y) = loop {
            //                let x: f32 = rng.gen_range(hero_pos.x - 10.0, hero_pos.x + 10.0);
            //                let y: f32 = rng.gen_range(hero_pos.y - 10.0, hero_pos.y + 10.0).abs();
            //                let index =
            //                    y.max(0.0) as usize * gat.width as usize + x.max(0.0) as usize;
            //                let walkable =
            //                    (map_render_data.gat.cells[index].cell_type & CellType::Walkable as u8) != 0;
            //                if walkable {
            //                    break (x, y);
            //                }
            //            };
            //            v2(x, -y)
            hero_pos
        }
    };
    let outlook = args
        .as_str(5)
        .and_then(|outlook| get_outlook(outlook, None));
    let y = args.as_f32(6).unwrap_or(0.0);

    for _ in 0..count {
        match type_name {
            "minion_melee" | "minion_ranged" => {
                // TODO asd
                //                let job_id = if type_name == "minion_melee" {
                //                    JobId::MeleeMinion
                //                } else {
                //                    JobId::RangedMinion
                //                };
                //                let char_entity_id =
                //                    create_random_char_minion(ecs_world, pos2d, team, job_id, outlook.clone());
                //                ecs_world
                //                    .create_entity()
                //                    .with(ControllerComponent::new(char_entity_id))
                //                    .with(MinionComponent { fountain_up: false })
                //                    .build();
            }
            "guard" => {
                // TODO asd
                //                let _char_entity_id = create_client_guard_entity(ecs_world, pos2d, team, y);
            }
            "dummy_enemy" => {
                let _char_entity_id = create_dummy(ecs_world, pos2d, JobId::TargetDummy);
            }
            "dummy_ally" => {
                let _char_entity_id = create_dummy(ecs_world, pos2d, JobId::HealingDummy);
            }
            _ => {}
        }
    }
    Ok(())
}

fn get_outlook(name: &str, current_outlook: Option<&CharOutlook>) -> Option<CharOutlook> {
    if let Ok(job_sprite_id) = JobSpriteId::from_str(name) {
        Some(match current_outlook {
            Some(CharOutlook::Human {
                job_sprite_id: _old_job_sprite_id,
                head_index,
                sex,
            }) => CharOutlook::Human {
                job_sprite_id,
                head_index: *head_index,
                sex: *sex,
            },
            _ => CharOutlook::Human {
                job_sprite_id,
                head_index: 0,
                sex: Sex::Male,
            },
        })
    } else if let Ok(monster_id) = MonsterId::from_str(name) {
        Some(CharOutlook::Monster(monster_id))
    } else {
        None
    }
}

fn create_dummy(ecs_world: &mut specs::World, pos2d: Vec2, job_id: JobId) {
    let base_attributes = {
        let dev_configs = &ecs_world.read_resource::<CommonConfigs>();
        CharAttributes::get_base_attributes(job_id, &dev_configs).clone()
    };
    ecs_world
        .create_entity()
        .with(LocalCharStateComp::new(pos2d, base_attributes))
        .with(StaticCharDataComponent::new(
            "Dummy".to_owned(),
            if job_id == JobId::HealingDummy {
                Team::AllyForAll
            } else {
                Team::EnemyForAll
            },
            CharType::Minion,
            job_id,
            if job_id == JobId::HealingDummy {
                CharOutlook::Monster(MonsterId::GEFFEN_MAGE_6)
            } else {
                CharOutlook::Monster(MonsterId::Barricade)
            },
        ))
        .build();
}

fn create_random_char_minion(
    ecs_world: &mut specs::World,
    pos2d: Vec2,
    team: Team,
    job_id: JobId,
    outlook: Option<CharOutlook>,
) {
    let mut rng = rand::thread_rng();
    let sex = if rng.gen::<usize>() % 2 == 0 {
        Sex::Male
    } else {
        Sex::Female
    };

    let head_index = rng.gen::<usize>() % 5;

    let base_attributes = {
        let dev_configs = &ecs_world.read_resource::<CommonConfigs>();
        CharAttributes::get_base_attributes(job_id, &dev_configs).clone()
    };

    let char_id = ecs_world
        .create_entity()
        .with(LocalCharStateComp::new(pos2d, base_attributes))
        .with(StaticCharDataComponent::new(
            "Minion".to_owned(),
            team,
            CharType::Minion,
            job_id,
            CharOutlook::Human {
                job_sprite_id: if job_id == JobId::MeleeMinion {
                    JobSpriteId::SWORDMAN
                } else {
                    JobSpriteId::ARCHER
                },
                head_index,
                sex,
            },
        ))
        .build();
}

fn cmd_kill_all(
    controller_id: Option<ControllerEntityId>,
    args: CommandArguments,
    ecs_world: &mut specs::World,
) -> Result<(), String> {
    let type_name = args.as_str(0).unwrap_or("all");
    let mut entity_ids = Vec::with_capacity(32);
    let self_char_id = get_client_char_id(controller_id, ecs_world);
    for (entity_id, char_state) in (
        &ecs_world.entities(),
        &ecs_world.read_storage::<StaticCharDataComponent>(),
    )
        .join()
    {
        let entity_id = EntityId::from(entity_id);
        let need_delete = match type_name {
            "all" => true,
            "left_team" => char_state.team == Team::Left,
            "right_team" => char_state.team == Team::Right,
            // TODO2
            //            "npc" => ecs_world
            //                .read_storage::<NpcComponent>()
            //                .get(entity_id.into())
            //                .is_some(),
            //            "player" => ecs_world
            //                .read_storage::<NpcComponent>()
            //                .get(entity_id.into())
            //                .is_none(),
            _ => {
                if let Ok(job_id) = JobId::from_str(type_name) {
                    char_state.job_id == job_id
                } else {
                    false
                }
            }
        };
        if need_delete && self_char_id.map(|it| it != entity_id).unwrap_or(true) {
            entity_ids.push(entity_id);
        }
    }
    for entity_id in entity_ids {
        ecs_world
            .write_storage::<LocalCharStateComp<Local>>()
            .get_mut(entity_id.into())
            .unwrap()
            .hp = 0;
    }

    Ok(())
}

fn cmd_reload_configs(
    controller_id: Option<ControllerEntityId>,
    args: CommandArguments,
    ecs_world: &mut specs::World,
) -> Result<(), String> {
    log::info!("Reloading configs");
    let configs = load_common_configs("config-runtime").unwrap();

    *ecs_world.write_resource::<CommonConfigs>() = configs.clone();

    for (state, static_info) in (
        &mut ecs_world.write_storage::<LocalCharStateComp<Local>>(),
        &ecs_world.read_storage::<StaticCharDataComponent>(),
    )
        .join()
    {
        state.recalc_attribs_based_on_statuses(static_info.job_id, &configs);
    }

    ecs_world
        .write_resource::<OutPacketCollector>()
        .push((PacketTarget::All, FromServerPacket::Configs(configs)));

    Ok(())
}
