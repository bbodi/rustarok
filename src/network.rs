use crate::asset::database::AssetDatabase;
use crate::components::char::{attach_human_player_components, Team};
use crate::components::controller::{CameraComponent, CharEntityId, ControllerEntityId};
use crate::components::BrowserClient;
use crate::configs::DevConfig;
use crate::consts::JobId;
use crate::effect::StrEffectType;
use crate::runtime_assets::map::PhysicEngine;
use crate::systems::{Sex, SystemVariables};
use crate::video::VIDEO_HEIGHT;
use crate::video::VIDEO_WIDTH;
use byteorder::{LittleEndian, WriteBytesExt};
use nalgebra::Vector2;
use specs::prelude::*;
use std::convert::TryInto;
use std::io::Write;
use std::net::TcpListener;
use std::str::FromStr;
use std::time::SystemTime;
use strum::IntoEnumIterator;
use websocket::server::{NoTlsAcceptor, WsServer};
use websocket::OwnedMessage;

pub fn handle_new_connections(
    map_name: &str,
    ecs_world: &mut specs::World,
    websocket_server: &mut WsServer<NoTlsAcceptor, TcpListener>,
) -> () {
    match websocket_server.accept() {
        Ok(wsupgrade) => {
            let browser_socket = wsupgrade.accept().unwrap();
            browser_socket.set_nonblocking(true).unwrap();

            let mut browser_client = BrowserClient::new(browser_socket);
            {
                let asset_db: &AssetDatabase = &ecs_world.read_resource();
                let system_vars = &ecs_world.read_resource::<SystemVariables>();
                let welcome_data = json!({
                    "screen_width": VIDEO_WIDTH,
                    "screen_height": VIDEO_HEIGHT,
                    "map_name": map_name,
                    "asset_database": serde_json::to_value(asset_db).unwrap(),
                    "effect_names": StrEffectType::iter().map(|it| it.to_string()).collect::<Vec<_>>(),
                    "ground": json!({
                        "light_dir" : system_vars.map_render_data.rsw.light.direction,
                        "light_ambient" : system_vars.map_render_data.rsw.light.ambient,
                        "light_diffuse" : system_vars.map_render_data.rsw.light.diffuse,
                        "light_opacity" : system_vars.map_render_data.rsw.light.opacity,
                    }),
                    "projection_mat": system_vars
                                        .matrices
                                        .projection.as_slice(),
                    "ortho_mat": system_vars
                                        .matrices
                                        .ortho.as_slice()
                });
                let welcome_msg = serde_json::to_vec(&welcome_data).unwrap();
                browser_client.send_message(&welcome_msg);
            };

            let browser_client_entity = ecs_world.create_entity().with(browser_client).build();
            log::info!("Client connected: {:?}", browser_client_entity);
        }
        _ => { /* Nobody tried to connect, move on.*/ }
    }
}

pub fn handle_client_handshakes(ecs_world: &mut World) {
    let projection_mat = ecs_world
        .read_resource::<SystemVariables>()
        .matrices
        .projection;
    let entities = &ecs_world.entities();
    let updater = ecs_world.read_resource::<LazyUpdate>();
    for (controller_id, client, _not_camera) in (
        &ecs_world.entities(),
        &mut ecs_world.write_storage::<BrowserClient>(),
        !&ecs_world.read_storage::<CameraComponent>(),
    )
        .join()
    {
        let controller_id = ControllerEntityId(controller_id);
        if let Ok(msg) = client.receive() {
            match msg {
                OwnedMessage::Binary(_buf) => {}
                OwnedMessage::Text(text) => {
                    if let Ok(deserialized) = serde_json::from_str::<serde_json::Value>(&text) {
                        if let Some(mismatched_textures) =
                            deserialized["mismatched_textures"].as_array()
                        {
                            log::trace!("mismatched_textures: {:?}", mismatched_textures);
                            let mut response_buf =
                                Vec::with_capacity(mismatched_textures.len() * 256 * 256);
                            for mismatched_texture in mismatched_textures {
                                ecs_world
                                    .read_resource::<AssetDatabase>()
                                    .copy_texture_into(
                                        &ecs_world.read_resource::<SystemVariables>().gl,
                                        mismatched_texture.as_str().unwrap_or(""),
                                        &mut response_buf,
                                    );
                                client.send_message(&response_buf);
                                response_buf.clear();
                            }
                        }
                        if let Some(mismatched_vertex_buffers) =
                            deserialized["mismatched_vertex_buffers"].as_array()
                        {
                            log::trace!(
                                "mismatched_vertex_buffers: {:?}",
                                mismatched_vertex_buffers
                            );
                            let mut response_buf = Vec::with_capacity(256 * 256 * 4);
                            for mismatched_vertex_buffer in mismatched_vertex_buffers {
                                if let Some("3d_ground") = mismatched_vertex_buffer.as_str() {
                                    response_buf.write_u8(1).unwrap();
                                    let ground_vao = &ecs_world
                                        .read_resource::<SystemVariables>()
                                        .map_render_data
                                        .ground_vertex_array;
                                    ground_vao.write_into(&mut response_buf);
                                    client.send_message(&response_buf);
                                    response_buf.clear();
                                }
                            }
                            // send closing message
                            {
                                response_buf.push(0xB1);
                                response_buf.push(0x6B);
                                response_buf.push(0x00);
                                response_buf.push(0xB5);
                                client.send_message(&response_buf);
                            }
                        }
                        if let Some(missing_effects) = deserialized["missing_effects"].as_array() {
                            log::trace!("missing_effects: {:?}", missing_effects);
                            let mut response_buf = Vec::with_capacity(256 * 256 * 4);
                            for missing_effect_name in missing_effects {
                                let missing_effect_name =
                                    missing_effect_name.as_str().unwrap_or("");
                                if let Ok(effect_type) =
                                    StrEffectType::from_str(missing_effect_name)
                                {
                                    let str_file = &ecs_world
                                        .read_resource::<SystemVariables>()
                                        .str_effects[effect_type as usize];

                                    response_buf
                                        .write_u16::<LittleEndian>(missing_effect_name.len() as u16)
                                        .unwrap();
                                    response_buf.write(missing_effect_name.as_bytes()).unwrap();

                                    str_file.write_into(&mut response_buf);
                                    client.send_message(&response_buf);
                                    response_buf.clear();
                                }
                            }
                            // send closing message
                            {
                                response_buf.push(0xB1);
                                response_buf.push(0x6B);
                                response_buf.push(0x00);
                                response_buf.push(0xB5);
                                client.send_message(&response_buf);
                            }
                        }
                        if deserialized["send_me_model_instances"].as_bool().is_some() {
                            let mut response_buf = Vec::with_capacity(256 * 256 * 4);
                            for model_instance in &ecs_world
                                .read_resource::<SystemVariables>()
                                .map_render_data
                                .model_instances
                            {
                                response_buf
                                    .write_u32::<LittleEndian>(
                                        model_instance.asset_db_model_index as u32,
                                    )
                                    .unwrap();
                                for v in &model_instance.matrix {
                                    response_buf.write_f32::<LittleEndian>(*v).unwrap();
                                }
                            }
                            client.send_message(&response_buf);
                        }
                        if let Some(missing_models) = deserialized["missing_models"].as_array() {
                            log::trace!("missing_models: {:?}", missing_models);
                            let mut response_buf =
                                Vec::with_capacity(missing_models.len() * 256 * 256);
                            for missing_model in missing_models {
                                ecs_world.read_resource::<AssetDatabase>().copy_model_into(
                                    missing_model.as_str().unwrap_or(""),
                                    &mut response_buf,
                                );
                                client.send_message(&response_buf);
                                response_buf.clear();
                            }
                            // send closing message
                            {
                                response_buf.push(0xB1);
                                response_buf.push(0x6B);
                                response_buf.push(0x00);
                                response_buf.push(0xB5);
                                client.send_message(&response_buf);
                            }
                        }
                        if deserialized["ready"].as_bool().is_some() {
                            let char_entity_id = CharEntityId(entities.create());
                            attach_human_player_components(
                                "browser",
                                char_entity_id,
                                controller_id,
                                &updater,
                                &mut ecs_world.write_resource::<PhysicEngine>(),
                                projection_mat,
                                v2!(
                                    ecs_world.read_resource::<DevConfig>().start_pos_x,
                                    ecs_world.read_resource::<DevConfig>().start_pos_y
                                ),
                                Sex::Male,
                                JobId::CRUSADER,
                                2,
                                Team::Right,
                                &ecs_world.read_resource::<DevConfig>(),
                            );
                        }
                    } else {
                        log::warn!("Invalid msg: {}", text);
                    }
                }
                OwnedMessage::Close(_) => {}
                OwnedMessage::Ping(_) => {}
                OwnedMessage::Pong(buf) => {
                    let now_ms = SystemTime::now()
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap()
                        .as_millis();
                    // TODO
                    if buf.len() >= 16 {
                        let (int_bytes, _rest) = buf.split_at(std::mem::size_of::<u128>());
                        let ping_sent = u128::from_le_bytes(int_bytes.try_into().unwrap());
                        client.set_ping(now_ms - ping_sent);
                    }
                }
            }
        }
    }
}
