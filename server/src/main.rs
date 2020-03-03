//#![deny(
////missing_docs,
//warnings,
//anonymous_parameters,
//unused_extern_crates,
//unused_import_braces,
//trivial_casts,
//variant_size_differences,
////missing_debug_implementations,
//trivial_numeric_casts,
//unused_qualifications,
//clippy::all
//)]
#[macro_use]
extern crate specs_derive;

use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::str::FromStr;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use log::LevelFilter;
use serde::Deserialize;
use specs;
use specs::prelude::*;
use strum;

use rustarok_common::attack::{ApplyForceComponent, AreaAttackComponent, HpModificationRequest};
use rustarok_common::char_attr::CharAttributes;
use rustarok_common::common::{
    measure_time, v2, EngineTime, LocalTime, ServerTime, SimulationTick,
};
use rustarok_common::components::char::{
    create_common_player_entity, CharOutlook, CharType, ControllerEntityId, EntityTarget, JobId,
    LocalCharEntityId, LocalCharStateComp, ServerCharState, ServerEntityId, Sex,
    StaticCharDataComponent, Team,
};
use rustarok_common::components::controller::{
    ControllerComponent, PlayerIntention, ToServerPlayerIntention,
};
use rustarok_common::components::job_ids::JobSpriteId;
use rustarok_common::config::CommonConfigs;
use rustarok_common::console::CommandArguments;
use rustarok_common::grf::asset_loader::CommonAssetLoader;
use rustarok_common::map::MapWalkingInfo;
use rustarok_common::packets::from_server::{FromServerPacket, ServerEntityState};
use rustarok_common::packets::to_server::ToServerPacket;
use rustarok_common::packets::{NetworkTrafficEvent, PacketHandlerThread, SocketId};
use rustarok_common::systems::char_state_sys::CharacterStateUpdateSystem;

use crate::console_cmd::execute_console_cmd;
use crate::controller_intention_to_char_target::ControllerIntentionToCharTargetSystem;
use crate::server_config::{load_common_configs, ServerConfig};

mod components;
mod console_cmd;
mod controller_intention_to_char_target;
#[path = "config.rs"]
mod server_config;

pub const SIMULATION_FREQ: usize = 30;
pub const SIMULATION_DURATION_MS: usize = 1000 / SIMULATION_FREQ;
pub const MAX_SECONDS_ALLOWED_FOR_SINGLE_SIMULATION_FRAME: f32 =
    SIMULATION_DURATION_MS as f32 / 1000.0;
pub const MAX_DURATION_ALLOWED_FOR_SINGLE_SIMULATION_FRAME: Duration =
    Duration::from_millis(SIMULATION_DURATION_MS as u64);

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub map_name: String,
    pub log_level: String,
    pub start_pos_x: f32,
    pub start_pos_y: f32,
    pub grf_paths: Vec<String>,
    pub server_port: u16,
}

impl AppConfig {
    pub fn new(name: &str) -> Result<Self, config::ConfigError> {
        let mut s = config::Config::new();
        s.merge(config::File::with_name(name))?;
        return s.try_into();
    }
}

fn bind_server(port: u16) -> TcpListener {
    let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], port))).unwrap();
    listener
        .set_nonblocking(true)
        .expect("Cannot set non-blocking");
    return listener;
}

fn accept_connection(listener: &mut TcpListener) -> Option<(TcpStream, SocketAddr)> {
    return match listener.accept() {
        Ok(s) => Some(s),
        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => None,
        Err(e) => {
            log::error!("encountered IO error: {}", e);
            None
        }
    };
}

struct RemoteClient {
    socket_id: SocketId,
    controller_id: Option<ControllerEntityId>,
    sock_addr: SocketAddr,
    // latest input from client
    last_action_tick: u64,
    last_command_id: u32,
    name: String,
}

// only the server must implement it
fn prepare_charsnapshot_for_sending(snapshot: LocalCharStateComp) -> ServerCharState {
    unsafe { std::mem::transmute(snapshot) }
}

// only the server must implement it
fn prepare_entity_id_for_sending(id: LocalCharEntityId) -> ServerEntityId {
    unsafe { std::mem::transmute(id) }
}

// only the server must implement it
fn prepare_time_for_sending(time: LocalTime) -> ServerTime {
    unsafe { std::mem::transmute(time) }
}

// only the server must implement it
fn client_id_to_server(id: ServerEntityId) -> LocalCharEntityId {
    unsafe { std::mem::transmute(id) }
}

fn init_connection(
    world: &mut specs::World,
    socket_handler: &mut PacketHandlerThread<ToServerPacket, FromServerPacket>,
    incoming_conn: (TcpStream, SocketAddr),
    configs: &AppConfig,
) -> RemoteClient {
    let socket_id = socket_handler.handle_socket(incoming_conn.0);

    RemoteClient {
        socket_id,
        controller_id: None,
        sock_addr: incoming_conn.1,
        last_action_tick: 1,
        last_command_id: 0,
        name: "unknown".to_owned(),
    }
}

type OutPacketCollector = Vec<(PacketTarget, FromServerPacket)>;

pub enum PacketTarget {
    All,
    Client(SocketId),
    Team(Team),
    Area,
}

fn main() {
    log::info!("Loading config file config.toml");
    let config = AppConfig::new("config").expect("Could not load config file ('config.toml')");

    simple_logging::log_to_stderr(
        LevelFilter::from_str(&config.log_level)
            .expect("Unknown log level. Please set one of the following values for 'log_level' in 'config.toml': \"OFF\", \"ERROR\", \"WARN\", \"INFO\", \"DEBUG\", \"TRACE\"")
    );
    log::info!(">>> Loading GRF files");
    let (elapsed, asset_loader) = measure_time(|| {
        CommonAssetLoader::new(config.grf_paths.as_slice())
            .expect("Could not open grf files. Please configure them in 'config.toml'")
    });
    log::info!("<<< GRF loading: {}ms", elapsed.as_millis());

    let mut ecs_world = create_ecs_world();
    ecs_world.insert(Vec::<HpModificationRequest>::with_capacity(128));
    ecs_world.insert(Vec::<AreaAttackComponent>::with_capacity(128));
    ecs_world.insert(Vec::<ApplyForceComponent>::with_capacity(128));
    ecs_world.insert(EngineTime::new(0));
    ecs_world.insert(OutPacketCollector::with_capacity(128));

    ecs_world.insert(ServerConfig::new("server-conf.toml").unwrap());
    ecs_world.insert(load_common_configs("config-runtime").unwrap());
    ecs_world.insert(MapWalkingInfo::new());
    ecs_world.insert(SimulationTick::new());

    let mut ecs_dispatcher = specs::DispatcherBuilder::new()
        .with(ControllerIntentionToCharTargetSystem, "char_control", &[])
        .with(CharacterStateUpdateSystem, "char_state", &["char_control"])
        .build();

    let mut packet_handler_thread =
        PacketHandlerThread::<ToServerPacket, FromServerPacket>::start_thread(64);

    //////// Execute init script
    {
        let file = File::open("init.cmd").unwrap();
        let reader = BufReader::new(file);

        for line in reader.lines() {
            let line = line.unwrap();
            if line.starts_with("//") || line.trim().is_empty() {
                continue;
            }
            execute_console_cmd(None, CommandArguments::new(&line), &mut ecs_world);
        }
    };

    let mut socket_listener = bind_server(config.server_port);
    log::info!("bind socket on port {}", config.server_port);

    log::info!("waiting for incoming connections...");
    let mut tmp_vec = Vec::with_capacity(256);
    const MAX_PLAYER_NUM: usize = 64;

    let mut remote_clients = Vec::<Option<RemoteClient>>::with_capacity(64);

    let mut next_player_team = Team::Left;
    let server_started = Instant::now();

    ////////////////////////////////////////////////////
    ////////////////////////////////////////////////////
    ////////////////////////////////////////////////////
    // MAIN LOOP
    ////////////////////////////////////////////////////
    ////////////////////////////////////////////////////
    ////////////////////////////////////////////////////

    loop {
        let start = Instant::now();

        accept_new_connections(
            &mut socket_listener,
            &mut packet_handler_thread,
            &mut remote_clients,
            &mut ecs_world,
            &config,
        );

        process_incoming_packets(
            &mut tmp_vec,
            &packet_handler_thread,
            &mut remote_clients,
            &mut ecs_world,
            &config,
            &mut next_player_team,
        );

        run_frame(&mut ecs_world, &mut ecs_dispatcher);

        send_snapshots(&packet_handler_thread, &mut remote_clients, &ecs_world);

        send_packets(&mut packet_handler_thread, &mut ecs_world, &remote_clients);

        let frame_duration = start.elapsed();
        if frame_duration < MAX_DURATION_ALLOWED_FOR_SINGLE_SIMULATION_FRAME {
            let to_sleep = MAX_DURATION_ALLOWED_FOR_SINGLE_SIMULATION_FRAME - frame_duration;
            std::thread::sleep(to_sleep);
        }

        ecs_world
            .write_resource::<EngineTime>()
            .tick(MAX_DURATION_ALLOWED_FOR_SINGLE_SIMULATION_FRAME);

        ecs_world.write_resource::<SimulationTick>().inc();
    }
}

fn send_packets(
    packet_handler_thread: &mut PacketHandlerThread<ToServerPacket, FromServerPacket>,
    ecs_world: &mut specs::World,
    remote_clients: &Vec<Option<RemoteClient>>,
) -> usize {
    let mut to_client = ecs_world.write_resource::<OutPacketCollector>();
    let sent = to_client.len();

    for (target, packet) in to_client.drain(..) {
        match target {
            PacketTarget::All => {
                for remote_client in remote_clients.iter() {
                    if let Some(remote_client) = remote_client {
                        packet_handler_thread.send(remote_client.socket_id, packet.clone());
                    }
                }
            }
            PacketTarget::Client(client_socket) => {
                packet_handler_thread.send(client_socket, packet);
            }
            PacketTarget::Team(team) => {
                // TODO
            }
            PacketTarget::Area => {
                // TODO
            }
        }
    }
    return sent;
}

fn run_frame(ecs_world: &mut specs::World, ecs_dispatcher: &mut specs::Dispatcher) {
    ecs_dispatcher.dispatch(ecs_world);
    ecs_world.maintain();
}

fn accept_new_connections(
    socket_listener: &mut TcpListener,
    packet_handler_thread: &mut PacketHandlerThread<ToServerPacket, FromServerPacket>,
    remote_clients: &mut Vec<Option<RemoteClient>>,
    ecs_world: &mut specs::World,
    config: &AppConfig,
) {
    if let Some(connecting_client) = accept_connection(socket_listener) {
        let remote_client =
            init_connection(ecs_world, packet_handler_thread, connecting_client, &config);
        log::info!(
            "{:?} - {} has joined",
            &remote_client.socket_id,
            &remote_client.sock_addr
        );
        if remote_client.socket_id.as_usize() >= remote_clients.len() {
            remote_clients.push(Some(remote_client));
        } else {
            let index = remote_client.socket_id.as_usize();
            remote_clients[index] = Some(remote_client);
        }
    }
}

fn send_snapshots(
    packet_handler_thread: &PacketHandlerThread<ToServerPacket, FromServerPacket>,
    remote_clients: &mut Vec<Option<RemoteClient>>,
    ecs_world: &specs::World,
) {
    let now = ecs_world.read_resource::<EngineTime>().now();
    for remote_client in remote_clients.iter_mut() {
        let remote_client = if let Some(remote_client) = remote_client {
            remote_client
        } else {
            continue;
        };
        if let Some(controller_id) = remote_client.controller_id {
            let controller_storage = ecs_world.read_storage::<ControllerComponent>();
            let controller = controller_storage.get(controller_id.into()).unwrap();
            if let Some(controlled_entity) = controller.controlled_entity {
                let auth_char_storage = ecs_world.read_storage::<LocalCharStateComp>();
                let char_state = auth_char_storage.get(controlled_entity.into()).unwrap();

                let mut entries = vec![ServerEntityState {
                    id: prepare_entity_id_for_sending(controlled_entity),
                    char_snapshot: prepare_charsnapshot_for_sending(char_state.clone()),
                }];
                for (other_char_id, other_char_state) in
                    (&ecs_world.entities(), &auth_char_storage).join()
                {
                    let other_char_id = LocalCharEntityId::from(other_char_id);
                    if other_char_id == controlled_entity.into() {
                        continue;
                    }
                    entries.push(ServerEntityState {
                        id: prepare_entity_id_for_sending(other_char_id),
                        char_snapshot: prepare_charsnapshot_for_sending(other_char_state.clone()),
                    })
                }
                packet_handler_thread.send(
                    remote_client.socket_id,
                    FromServerPacket::Ack {
                        cid: remote_client.last_command_id,
                        entries,
                    },
                );
                remote_client.last_action_tick += 1;
            }
        }
    }
}

fn process_incoming_packets(
    tmp_vec: &mut Vec<(SocketId, NetworkTrafficEvent<ToServerPacket>)>,
    packet_handler_thread: &PacketHandlerThread<ToServerPacket, FromServerPacket>,
    remote_clients: &mut Vec<Option<RemoteClient>>,
    ecs_world: &mut specs::World,
    config: &AppConfig,
    next_player_team: &mut Team,
) {
    tmp_vec.clear();
    packet_handler_thread.receive_into(tmp_vec);
    for (client_socket, packet) in tmp_vec.drain(..) {
        match packet {
            NetworkTrafficEvent::IncomingTraffic { received_data_len } => {
                //
            }
            NetworkTrafficEvent::OutgoingTraffic { sent_data_len } => {}
            NetworkTrafficEvent::Disconnected => {
                log::debug!("Client({:?}) has been disconnected", client_socket);
                disconnect_client(
                    remote_clients,
                    client_socket,
                    ecs_world,
                    packet_handler_thread,
                );
            }
            NetworkTrafficEvent::LocalError(e) => {
                log::error!("Client({:?}) has been disconnected: {:?}", client_socket, e);
                disconnect_client(
                    remote_clients,
                    client_socket,
                    ecs_world,
                    packet_handler_thread,
                );
            }
            NetworkTrafficEvent::Packet(p) => {
                match p {
                    ToServerPacket::Welcome { name } => {
                        log::info!("{} welcomed ^^", name);
                        let remote_client =
                            remote_clients[client_socket.as_usize()].as_mut().unwrap();
                        remote_client.name = name;
                        packet_handler_thread.send(
                            client_socket,
                            FromServerPacket::Init {
                                //    let map_name = "bat_a01"; // battle ground
                                map_name: "prontera".to_string(),
                                start_x: config.start_pos_x,
                                start_y: config.start_pos_y,
                            },
                        );
                        let configs = (*ecs_world.read_resource::<CommonConfigs>()).clone();
                        packet_handler_thread
                            .send(client_socket, FromServerPacket::Configs(configs));
                    }
                    ToServerPacket::Ping => {
                        packet_handler_thread.send(
                            client_socket,
                            FromServerPacket::Pong {
                                server_time: ServerTime(
                                    ecs_world.read_resource::<EngineTime>().now().as_millis(),
                                ),
                                server_tick: *ecs_world.read_resource::<SimulationTick>(),
                            },
                        );
                    }
                    ToServerPacket::ReadyForGame => {
                        let (connecting_char_id, connecting_char_snapshot) = {
                            let remote_client =
                                remote_clients[client_socket.as_usize()].as_mut().unwrap();
                            remote_client.name =
                                format!("{} {}", next_player_team.to_str(), remote_client.name);
                            log::info!("{} is ready to play", remote_client.name);

                            let char_id = LocalCharEntityId::from(
                                create_common_player_entity(
                                    remote_client.name.clone(),
                                    ecs_world,
                                    CharType::Player,
                                    JobId::CRUSADER,
                                    v2(config.start_pos_x, config.start_pos_y),
                                    *next_player_team,
                                    CharOutlook::Human {
                                        job_sprite_id: JobSpriteId::from_job_id(JobId::CRUSADER),
                                        head_index: 0,
                                        sex: Sex::Male,
                                    },
                                )
                                .build(),
                            );
                            *next_player_team = next_player_team.get_opponent_team();
                            let network_player_id = ecs_world
                                .create_entity()
                                .with(ControllerComponent::new(char_id))
                                .build();
                            remote_client.controller_id =
                                Some(ControllerEntityId::new(network_player_id));

                            let auth_char_storage = ecs_world.read_storage::<LocalCharStateComp>();
                            let char_state = auth_char_storage.get(char_id.into()).unwrap();
                            let static_data_storage =
                                ecs_world.read_storage::<StaticCharDataComponent>();
                            let static_char_state =
                                static_data_storage.get(char_id.into()).unwrap();
                            packet_handler_thread.send(
                                remote_client.socket_id,
                                FromServerPacket::NewEntity {
                                    id: prepare_entity_id_for_sending(char_id),
                                    name: static_char_state.name.clone(),
                                    team: static_char_state.team,
                                    typ: static_char_state.typ.clone(),
                                    outlook: static_char_state.outlook.clone(),
                                    job_id: static_char_state.job_id,
                                    state: prepare_charsnapshot_for_sending(char_state.clone()),
                                },
                            );

                            // send her the player list
                            {
                                for (other_char_id, other_char_state, other_static_data) in (
                                    &ecs_world.entities(),
                                    &auth_char_storage,
                                    &static_data_storage,
                                )
                                    .join()
                                {
                                    let other_char_id = LocalCharEntityId::new(other_char_id);
                                    if other_char_id == char_id {
                                        continue;
                                    }
                                    packet_handler_thread.send(
                                        remote_client.socket_id,
                                        FromServerPacket::NewEntity {
                                            id: prepare_entity_id_for_sending(other_char_id),
                                            name: other_static_data.name.to_owned(),
                                            team: other_static_data.team,
                                            typ: other_static_data.typ.clone(),
                                            outlook: other_static_data.outlook.clone(),
                                            job_id: other_static_data.job_id,
                                            state: prepare_charsnapshot_for_sending(
                                                other_char_state.clone(),
                                            ),
                                        },
                                    );
                                }
                            }

                            (char_id, char_state.clone())
                        };

                        // inform others about this player
                        let (packet, connecting_client_controller_id) = {
                            let static_data_storage =
                                ecs_world.read_storage::<StaticCharDataComponent>();
                            let static_data =
                                static_data_storage.get(connecting_char_id.into()).unwrap();
                            let remote_client =
                                remote_clients[client_socket.as_usize()].as_mut().unwrap();
                            (
                                FromServerPacket::NewEntity {
                                    id: prepare_entity_id_for_sending(connecting_char_id),
                                    name: remote_client.name.clone(),
                                    team: static_data.team,
                                    typ: static_data.typ.clone(),
                                    outlook: static_data.outlook.clone(),
                                    job_id: static_data.job_id,
                                    state: prepare_charsnapshot_for_sending(
                                        connecting_char_snapshot.clone(),
                                    ),
                                },
                                remote_client.controller_id.unwrap(),
                            )
                        };

                        send_to_all_except(
                            connecting_client_controller_id,
                            &packet,
                            remote_clients,
                            packet_handler_thread,
                        );
                    }
                    ToServerPacket::Intention {
                        cid,
                        client_tick,
                        intention,
                    } => {
                        let remote_client =
                            remote_clients[client_socket.as_usize()].as_mut().unwrap();
                        if let Some(controller_id) = remote_client.controller_id {
                            let mut controller_storage =
                                ecs_world.write_storage::<ControllerComponent>();
                            let controller: &mut ControllerComponent =
                                controller_storage.get_mut(controller_id.into()).unwrap();
                            controller.intention = Some(match intention {
                                ToServerPlayerIntention::MoveTo(v) => PlayerIntention::MoveTo(v),
                                ToServerPlayerIntention::MoveTowardsMouse(v) => {
                                    PlayerIntention::MoveTowardsMouse(v)
                                }
                                ToServerPlayerIntention::AttackTowards(v) => {
                                    PlayerIntention::AttackTowards(v)
                                }
                                ToServerPlayerIntention::Attack(id) => {
                                    PlayerIntention::Attack(client_id_to_server(id))
                                }
                            });
                            remote_client.last_command_id = cid;
                            log::debug!(
                                "client tick: {:?}, cid: {}, intention: {:?}",
                                client_tick,
                                cid,
                                &controller.intention
                            );
                        } else {
                            // TODO: close connection
                        }
                    }
                    ToServerPacket::ConsoleCommand(cmd) => {
                        let remote_client =
                            remote_clients[client_socket.as_usize()].as_mut().unwrap();
                        execute_console_cmd(remote_client.controller_id, cmd, ecs_world);
                    }
                }
            }
        }
    }
}

fn send_to_all(
    packet: &FromServerPacket,
    remote_clients: &[Option<RemoteClient>],
    packet_handler_thread: &PacketHandlerThread<ToServerPacket, FromServerPacket>,
) {
    for other_remote_client in remote_clients.iter() {
        if let Some(other_client) = other_remote_client {
            if let Some(other_id) = other_client.controller_id {
                packet_handler_thread.send(other_client.socket_id, packet.clone());
            }
        }
    }
}

fn send_to_all_except(
    except_id: ControllerEntityId,
    packet: &FromServerPacket,
    remote_clients: &[Option<RemoteClient>],
    packet_handler_thread: &PacketHandlerThread<ToServerPacket, FromServerPacket>,
) {
    for other_remote_client in remote_clients.iter() {
        if let Some(other_client) = other_remote_client {
            if let Some(other_id) = other_client.controller_id {
                if other_id != except_id {
                    packet_handler_thread.send(other_client.socket_id, packet.clone());
                }
            }
        }
    }
}

fn disconnect_client(
    remote_clients: &mut [Option<RemoteClient>],
    socket_id: SocketId,
    ecs_world: &mut specs::World,
    packet_handler_thread: &PacketHandlerThread<ToServerPacket, FromServerPacket>,
) {
    let controller_id = remote_clients[socket_id.as_usize()]
        .as_ref()
        .unwrap()
        .controller_id;
    remote_clients[socket_id.as_usize()] = None;
    if let Some(controller_id) = controller_id {
        // TODO: remove its original char!

        let mut controlled_entity = ecs_world
            .read_storage::<ControllerComponent>()
            .get(controller_id.into())
            .unwrap()
            .controlled_entity;
        if let Some(controlled_entity) = controlled_entity {
            ecs_world.delete_entity(controlled_entity.into());
            send_to_all(
                &FromServerPacket::PlayerDisconnected(prepare_entity_id_for_sending(
                    controlled_entity,
                )),
                remote_clients,
                packet_handler_thread,
            )
        }
        ecs_world.delete_entity(controller_id.into());
    }
}

pub fn create_ecs_world() -> specs::World {
    let mut ecs_world = specs::World::new();
    ecs_world.register::<LocalCharStateComp>();
    ecs_world.register::<ControllerComponent>();
    ecs_world.register::<StaticCharDataComponent>();
    ecs_world
}
