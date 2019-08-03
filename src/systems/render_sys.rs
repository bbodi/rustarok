use crate::cam::Camera;
use crate::components::char::{
    CharOutlook, CharState, CharType, CharacterStateComponent, PhysicsComponent,
    SpriteBoundingRect, SpriteRenderDescriptorComponent,
};
use crate::components::controller::{ControllerComponent, WorldCoords};
use crate::components::skills::skill::{SkillManifestationComponent, SkillTargetType, Skills};
use crate::components::{
    BrowserClient, FlyingNumberComponent, FlyingNumberType, StrEffectComponent,
};
use crate::systems::render::render_command::{Layer2d, RenderCommandCollectorComponent};
use crate::systems::ui::RenderUI;
use crate::systems::{AssetResources, SystemFrameDurations, SystemVariables};
use crate::video::VertexAttribDefinition;
use crate::video::{VertexArray, TEXTURE_0, TEXTURE_1, TEXTURE_2, VIDEO_HEIGHT, VIDEO_WIDTH};
use crate::{ElapsedTime, MapRenderData, PhysicsWorld, Shaders, SpriteResource};
use nalgebra::{Matrix3, Matrix4, Vector2, Vector3};
use specs::prelude::*;
use std::collections::HashMap;

/// The values that should be added to the sprite direction based on the camera
/// direction (the index is the camera direction, which is floor(angle/45)
pub const DIRECTION_TABLE: [usize; 8] = [6, 5, 4, 3, 2, 1, 0, 7];

// todo: Move it into GPU?
pub const ONE_SPRITE_PIXEL_SIZE_IN_3D: f32 = 1.0 / 35.0;

pub struct RenderDesktopClientSystem {
    circle_vertex_arrays: HashMap<i32, VertexArray>,
    damage_render_sys: DamageRenderSystem,
    render_ui_sys: RenderUI,
}

impl RenderDesktopClientSystem {
    pub fn new() -> RenderDesktopClientSystem {
        let circle_vertex_arrays = (1..=100)
            .map(|i| {
                let nsubdivs = 100;
                let two_pi = std::f32::consts::PI * 2.0;
                let dtheta = two_pi / nsubdivs as f32;

                let mut pts = Vec::with_capacity((nsubdivs - i) as usize);
                let r = 12.0;
                ncollide2d::procedural::utils::push_xy_arc(r, nsubdivs - i, dtheta, &mut pts);
                (
                    i as i32,
                    VertexArray::new(
                        gl::LINE_STRIP,
                        &pts,
                        pts.len(),
                        vec![VertexAttribDefinition {
                            number_of_components: 2,
                            offset_of_first_element: 0,
                        }],
                    ),
                )
            })
            .collect();
        RenderDesktopClientSystem {
            damage_render_sys: DamageRenderSystem::new(),
            render_ui_sys: RenderUI::new(),
            circle_vertex_arrays,
        }
    }

    pub fn render_for_controller<'a>(
        &self,
        controller: &mut ControllerComponent,
        render_commands: &mut RenderCommandCollectorComponent,
        physics_storage: &specs::ReadStorage<'a, PhysicsComponent>,
        physics_world: &specs::ReadExpect<'a, PhysicsWorld>,
        system_vars: &mut SystemVariables,
        char_state_storage: &specs::ReadStorage<'a, CharacterStateComponent>,
        entities: &specs::Entities<'a>,
        sprite_storage: &specs::ReadStorage<'a, SpriteRenderDescriptorComponent>,
        skill_storage: &specs::ReadStorage<'a, SkillManifestationComponent>, // TODO remove me
        str_effect_storage: &specs::ReadStorage<'a, StrEffectComponent>,
        updater: &specs::Write<'a, LazyUpdate>,
    ) {
        render_commands.set_view_matrix(&controller.view_matrix, &controller.normal_matrix);
        // Draw physics colliders
        for physics in (&physics_storage).join() {
            let body = physics_world.rigid_body(physics.body_handle);
            if body.is_none() {
                continue;
            }
            let pos = body.unwrap().position().translation.vector;

            render_commands
                .prepare_for_3d()
                .radius(physics.radius.get())
                .color(&[1.0, 0.0, 1.0, 1.0])
                .pos_2d(&pos)
                .y(0.05)
                .add_circle_command();
        }

        let char_pos = char_state_storage
            .get(controller.char_entity_id)
            .unwrap()
            .pos();
        render_client(
            &char_pos,
            &controller.camera,
            &controller.view_matrix,
            &controller.normal_matrix,
            &system_vars.assets.shaders,
            &system_vars.matrices.projection,
            &system_vars.map_render_data,
            render_commands,
        );

        if let Some((_skill_key, skill)) = controller.is_selecting_target() {
            let char_state = char_state_storage.get(controller.char_entity_id).unwrap();
            render_commands
                .prepare_for_3d()
                .pos_2d(&char_state.pos())
                .y(0.0)
                .radius(skill.get_casting_range())
                .color(&[0.0, 1.0, 0.0, 1.0])
                .add_circle_command();

            if skill.get_skill_target_type() == SkillTargetType::Area {
                let (skill_3d_pos, dir_vector) = Skills::limit_vector_into_range(
                    &char_pos,
                    &controller.mouse_world_pos,
                    skill.get_casting_range(),
                );
                skill.render_target_selection(&skill_3d_pos, &dir_vector, render_commands);
            }
        } else {
            let char_state = char_state_storage.get(controller.char_entity_id).unwrap();
            if let CharState::CastingSkill(casting_info) = char_state.state() {
                let skill = casting_info.skill;
                skill.render_casting(&char_pos, casting_info, system_vars, render_commands);
            }
        }

        // Draw players
        for (entity_id, animated_sprite, char_state) in
            (entities, sprite_storage, char_state_storage).join()
        {
            // for autocompletion
            let char_state: &CharacterStateComponent = char_state;

            let pos = char_state.pos();
            let is_dead = char_state.state().is_dead();
            let color = char_state.statuses.calc_render_color();
            match char_state.outlook {
                CharOutlook::Player {
                    job_id,
                    head_index,
                    sex,
                } => {
                    let body_sprite = char_state.statuses.calc_render_sprite(
                        job_id,
                        head_index,
                        sex,
                        &system_vars.assets.sprites,
                    );
                    let head_res = {
                        let sprites = &system_vars.assets.sprites.head_sprites;
                        &sprites[sex as usize][head_index]
                    };
                    if controller
                        .entity_below_cursor
                        .filter(|it| *it == entity_id)
                        .is_some()
                    {
                        let body_pos_offset = render_single_layer_action(
                            system_vars.time,
                            &animated_sprite,
                            body_sprite,
                            controller.yaw,
                            &pos,
                            [0, 0],
                            true,
                            1.1,
                            is_dead,
                            &[0.0, 0.0, 1.0, 0.4],
                            render_commands,
                        );

                        let _head_pos_offset = render_single_layer_action(
                            system_vars.time,
                            &animated_sprite,
                            head_res,
                            controller.yaw,
                            &pos,
                            body_pos_offset,
                            false,
                            1.1,
                            is_dead,
                            &[0.0, 0.0, 1.0, 0.5],
                            render_commands,
                        );
                    }

                    // todo: kell a body_pos_offset még mindig? (bounding rect)
                    let body_pos_offset = render_single_layer_action(
                        system_vars.time,
                        &animated_sprite,
                        body_sprite,
                        controller.yaw,
                        &pos,
                        [0, 0],
                        true,
                        1.0,
                        is_dead,
                        &color,
                        render_commands,
                    );

                    let mut body_bounding_rect = {
                        let render_command = render_commands.get_last_billboard_command();

                        render_command.project_to_screen(
                            &controller.view_matrix,
                            &system_vars.matrices.projection,
                        )
                    };
                    let _head_pos_offset = render_single_layer_action(
                        system_vars.time,
                        &animated_sprite,
                        head_res,
                        controller.yaw,
                        &pos,
                        body_pos_offset,
                        false,
                        1.0,
                        is_dead,
                        &color,
                        render_commands,
                    );
                    // TODO: heads are quite similar, use fixed pixel size for it and remove this projection?
                    {
                        let render_command = render_commands.get_last_billboard_command();
                        let head_bounding_rect = render_command.project_to_screen(
                            &controller.view_matrix,
                            &system_vars.matrices.projection,
                        );
                        body_bounding_rect.merge(&head_bounding_rect);
                    };

                    if !is_dead {
                        self.draw_health_bar(
                            controller.char_entity_id == entity_id,
                            &char_state,
                            system_vars.time,
                            &body_bounding_rect,
                            &system_vars.assets,
                            render_commands,
                        );
                    }

                    controller
                        .bounding_rect_2d
                        .insert(entity_id, body_bounding_rect);
                }
                CharOutlook::Monster(monster_id) => {
                    let body_res = {
                        let sprites = &system_vars.assets.sprites.monster_sprites;
                        &sprites[&monster_id]
                    };
                    if controller
                        .entity_below_cursor
                        .filter(|it| *it == entity_id)
                        .is_some()
                    {
                        let _pos_offset = render_single_layer_action(
                            system_vars.time,
                            &animated_sprite,
                            body_res,
                            controller.yaw,
                            &pos,
                            [0, 0],
                            true,
                            1.1,
                            is_dead,
                            &[0.0, 0.0, 1.0, 0.5],
                            render_commands,
                        );
                    }
                    let _pos_offset = render_single_layer_action(
                        system_vars.time,
                        &animated_sprite,
                        body_res,
                        controller.yaw,
                        &pos,
                        [0, 0],
                        true,
                        1.0,
                        is_dead,
                        &color,
                        render_commands,
                    );
                    let bounding_rect = {
                        let render_command = render_commands.get_last_billboard_command();

                        render_command.project_to_screen(
                            &controller.view_matrix,
                            &system_vars.matrices.projection,
                        )
                    };
                    if !is_dead {
                        self.draw_health_bar(
                            controller.char_entity_id == entity_id,
                            &char_state,
                            system_vars.time,
                            &bounding_rect,
                            &system_vars.assets,
                            render_commands,
                        );
                    }

                    controller.bounding_rect_2d.insert(entity_id, bounding_rect);
                }
            }

            char_state
                .statuses
                .render(&char_state.pos(), system_vars, render_commands);
        }

        for skill in (&skill_storage).join() {
            skill.render(system_vars.time, &system_vars.assets, render_commands);
        }

        // TODO: into a separate system
        for (entity_id, str_effect) in (entities, str_effect_storage).join() {
            if str_effect.die_at.has_passed(system_vars.time) {
                updater.remove::<StrEffectComponent>(entity_id);
            } else {
                RenderDesktopClientSystem::render_str(
                    &str_effect.effect,
                    str_effect.start_time,
                    &str_effect.pos,
                    system_vars,
                    render_commands,
                );
            }
        }
    }
}

impl<'a> specs::System<'a> for RenderDesktopClientSystem {
    type SystemData = (
        specs::Entities<'a>,
        specs::WriteStorage<'a, ControllerComponent>,
        specs::WriteStorage<'a, BrowserClient>,
        specs::ReadStorage<'a, PhysicsComponent>,
        specs::ReadStorage<'a, SpriteRenderDescriptorComponent>,
        specs::ReadStorage<'a, CharacterStateComponent>,
        specs::WriteExpect<'a, SystemVariables>,
        specs::WriteExpect<'a, SystemFrameDurations>,
        specs::ReadStorage<'a, SkillManifestationComponent>, // TODO remove me
        specs::ReadStorage<'a, StrEffectComponent>,
        specs::ReadExpect<'a, PhysicsWorld>,
        specs::Write<'a, LazyUpdate>,
        specs::ReadStorage<'a, FlyingNumberComponent>,
        specs::WriteStorage<'a, RenderCommandCollectorComponent>,
    );

    fn run(
        &mut self,
        (
            entities,
            mut controller_storage,
            mut browser_client_storage,
            physics_storage,
            sprite_storage,
            char_state_storage,
            mut system_vars,
            mut system_benchmark,
            skill_storage,
            str_effect_storage,
            physics_world,
            updater,
            numbers,
            mut render_commands_storage,
        ): Self::SystemData,
    ) {
        let _stopwatch = system_benchmark.start_measurement("RenderDesktopClientSystem");
        unsafe {
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
        }
        for render_commands in (&mut render_commands_storage).join() {
            render_commands.clear();
        }

        for (mut controller, browser, mut render_commands) in (
            &mut controller_storage,
            &mut browser_client_storage,
            &mut render_commands_storage,
        )
            .join()
        {
            self.render_for_controller(
                &mut controller,
                &mut render_commands,
                &physics_storage,
                &physics_world,
                &mut system_vars,
                &char_state_storage,
                &entities,
                &sprite_storage,
                &skill_storage,
                &str_effect_storage,
                &updater,
            );

            self.damage_render_sys.run(
                &entities,
                &numbers,
                &char_state_storage,
                controller.char_entity_id,
                system_vars.time,
                &system_vars.assets,
                &updater,
                render_commands,
            );

            self.render_ui_sys.run(
                &entities,
                &mut controller,
                &mut render_commands,
                &char_state_storage,
                &system_vars,
            );

            // now the back buffer contains the rendered image for this client
            unsafe {
                gl::ReadBuffer(gl::BACK);
                gl::ReadPixels(
                    0,
                    0,
                    VIDEO_WIDTH as i32,
                    VIDEO_HEIGHT as i32,
                    gl::RGBA,
                    gl::UNSIGNED_BYTE,
                    browser.offscreen.as_mut_ptr() as *mut gl::types::GLvoid,
                );
            }
            let message = websocket::Message::binary(browser.offscreen.as_slice());
            //                sent_bytes_per_second_counter += client.offscreen.len();
            // it is ok if it fails, the client might have disconnected but
            // ecs_world.maintain has not been executed yet to remove it from the world
            let _result = browser.websocket.lock().unwrap().send_message(&message);
        }

        unsafe {
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
        }

        for (mut controller, _not_browser, mut render_commands) in (
            &mut controller_storage,
            !&browser_client_storage,
            &mut render_commands_storage,
        )
            .join()
        {
            self.render_for_controller(
                &mut controller,
                &mut render_commands,
                &physics_storage,
                &physics_world,
                &mut system_vars,
                &char_state_storage,
                &entities,
                &sprite_storage,
                &skill_storage,
                &str_effect_storage,
                &updater,
            );

            self.damage_render_sys.run(
                &entities,
                &numbers,
                &char_state_storage,
                controller.char_entity_id,
                system_vars.time,
                &system_vars.assets,
                &updater,
                render_commands,
            );

            self.render_ui_sys.run(
                &entities,
                &mut controller,
                &mut render_commands,
                &char_state_storage,
                &system_vars,
            );
        }
    }
}

pub fn render_single_layer_action<'a>(
    now: ElapsedTime,
    animation: &SpriteRenderDescriptorComponent,
    sprite_res: &SpriteResource,
    camera_yaw: f32,
    pos: &Vector2<f32>,
    pos_offset: [i32; 2],
    is_main: bool,
    size_multiplier: f32,
    is_dead: bool,
    color: &[f32; 4],
    render_commands: &'a mut RenderCommandCollectorComponent,
) -> [i32; 2] {
    let idx = {
        let cam_dir = (((camera_yaw / 45.0) + 0.5) as usize) % 8;
        animation.action_index + (animation.direction + DIRECTION_TABLE[cam_dir]) % 8
    };

    // TODO: if debug
    let action = sprite_res
        .action
        .actions
        .get(idx)
        .or_else(|| {
            log::error!(
                "Invalid action action index: {} idx: {}",
                animation.action_index,
                idx
            );
            Some(&sprite_res.action.actions[0])
        })
        .unwrap();
    let frame_index = if is_dead {
        action.frames.len() - 1
    } else {
        let frame_count = action.frames.len();
        let mut time_needed_for_one_frame = if let Some(duration) = animation.forced_duration {
            duration.div(frame_count as f32)
        } else {
            action.delay as f32 * (1.0 / animation.fps_multiplier) / 1000.0
        };
        time_needed_for_one_frame = if time_needed_for_one_frame == 0.0 {
            0.1
        } else {
            time_needed_for_one_frame
        };
        let elapsed_time = now.elapsed_since(animation.animation_started);
        ((elapsed_time.div(time_needed_for_one_frame)) as usize % frame_count) as usize
    };
    let frame = &action.frames[frame_index];

    let layer = &frame.layers[0];

    let offset = if !frame.positions.is_empty() && !is_main {
        [
            (pos_offset[0] - frame.positions[0][0]) as f32,
            (pos_offset[1] - frame.positions[0][1]) as f32,
        ]
    } else {
        [0.0, 0.0]
    };
    let offset = [
        layer.pos[0] as f32 + offset[0],
        layer.pos[1] as f32 + offset[1],
    ];

    let mut color = color.clone();
    for i in 0..4 {
        color[i] *= layer.color[i];
    }

    let sprite_texture = &sprite_res.textures[layer.sprite_frame_index as usize];
    render_commands
        .prepare_for_3d()
        .pos_2d(&pos)
        .scale(layer.scale[0] * size_multiplier)
        .offset(offset)
        .color(&color)
        .add_billboard_command(&sprite_texture.texture, layer.is_mirror);

    // TODO: put 0,0 manually on startup if it is empty
    let anim_pos = frame
        .positions
        .get(0)
        .map(|it| it.clone())
        .unwrap_or([0, 0]);

    return [
        (anim_pos[0] as f32 * size_multiplier) as i32,
        (anim_pos[1] as f32 * size_multiplier) as i32,
    ];
}

pub fn render_action(
    now: ElapsedTime,
    animation: &SpriteRenderDescriptorComponent,
    sprite_res: &SpriteResource,
    camera_yaw: f32,
    pos: &Vector2<f32>,
    pos_offset: [i32; 2],
    is_main: bool,
    size_multiplier: f32,
    is_dead: bool,
    color: &[f32; 4],
    render_commands: &mut RenderCommandCollectorComponent,
) -> [i32; 2] {
    let idx = {
        let cam_dir = (((camera_yaw / 45.0) + 0.5) as usize) % 8;
        animation.action_index + (animation.direction + DIRECTION_TABLE[cam_dir]) % 8
    };

    // TODO: if debug
    let action = sprite_res
        .action
        .actions
        .get(idx)
        .or_else(|| {
            log::error!(
                "Invalid action action index: {} idx: {}",
                animation.action_index,
                idx
            );
            Some(&sprite_res.action.actions[0])
        })
        .unwrap();
    let frame_index = if is_dead {
        action.frames.len() - 1
    } else {
        let frame_count = action.frames.len();
        let mut time_needed_for_one_frame = if let Some(duration) = animation.forced_duration {
            duration.div(frame_count as f32)
        } else {
            action.delay as f32 * (1.0 / animation.fps_multiplier) / 1000.0
        };
        time_needed_for_one_frame = if time_needed_for_one_frame == 0.0 {
            0.1
        } else {
            time_needed_for_one_frame
        };
        let elapsed_time = now.elapsed_since(animation.animation_started);
        ((elapsed_time.div(time_needed_for_one_frame)) as usize % frame_count) as usize
    };
    let frame = &action.frames[frame_index];

    for layer in frame.layers.iter() {
        if layer.sprite_frame_index < 0 {
            continue;
        }

        let offset = if !frame.positions.is_empty() && !is_main {
            [
                (pos_offset[0] - frame.positions[0][0]) as f32,
                (pos_offset[1] - frame.positions[0][1]) as f32,
            ]
        } else {
            [0.0, 0.0]
        };
        let offset = [
            layer.pos[0] as f32 + offset[0],
            layer.pos[1] as f32 + offset[1],
        ];

        let mut color = color.clone();
        for i in 0..4 {
            color[i] *= layer.color[i];
        }

        let sprite_texture = &sprite_res.textures[layer.sprite_frame_index as usize];
        render_commands
            .prepare_for_3d()
            .pos_2d(&pos)
            .scale(layer.scale[0] * size_multiplier)
            .offset(offset)
            .color(&color)
            .add_billboard_command(&sprite_texture.texture, layer.is_mirror);
    }
    // TODO: put 0,0 manually on startup if it is empty
    let anim_pos = frame
        .positions
        .get(0)
        .map(|it| it.clone())
        .unwrap_or([0, 0]);

    return [
        (anim_pos[0] as f32 * size_multiplier) as i32,
        (anim_pos[1] as f32 * size_multiplier) as i32,
    ];
}

fn render_client(
    char_pos: &Vector2<f32>,
    camera: &Camera,
    view: &Matrix4<f32>,
    normal_matrix: &Matrix3<f32>,
    shaders: &Shaders,
    projection_matrix: &Matrix4<f32>,
    map_render_data: &MapRenderData,
    render_commands: &mut RenderCommandCollectorComponent,
) {
    if map_render_data.draw_ground {
        render_ground(
            shaders,
            projection_matrix,
            map_render_data,
            &view,
            &normal_matrix,
        );
    }

    // cam area is [-20;20] width and [70;5] height
    if map_render_data.draw_models {
        for model_instance in &map_render_data.model_instances {
            let model_render_data = &map_render_data.models[&model_instance.name];

            let min = model_instance.bottom_left_front;
            let max = model_instance.top_right_back;

            let cam_pos = camera.pos();
            if ((max.x < cam_pos.x - 40.0 || max.x > cam_pos.x + 40.0)
                && (min.x < cam_pos.x - 40.0 || min.x > cam_pos.x + 40.0))
                || ((max.z < cam_pos.z - 70.0 || max.z > cam_pos.z + 5.0)
                    && (min.z < cam_pos.z - 70.0 || min.z > cam_pos.z + 5.0))
            {
                continue;
            }
            let alpha = if (max.x > char_pos.x && min.x < char_pos.x)
                && char_pos.y <= min.z // character is behind
                && max.y > 2.0
            {
                0.3
            } else {
                model_render_data.alpha
            };

            render_commands
                .prepare_for_3d()
                .alpha(alpha)
                .add_model_command(&model_instance.name, &model_instance.matrix);
        }
    }
}

fn render_ground(
    shaders: &Shaders,
    projection_matrix: &Matrix4<f32>,
    map_render_data: &MapRenderData,
    model_view: &Matrix4<f32>,
    normal_matrix: &Matrix3<f32>,
) {
    let shader = shaders.ground_shader.gl_use();
    shader.set_mat4("projection", &projection_matrix);
    shader.set_mat4("model_view", &model_view);
    shader.set_mat3("normal_matrix", &normal_matrix);
    shader.set_vec3("light_dir", &map_render_data.rsw.light.direction);
    shader.set_vec3("light_ambient", &map_render_data.rsw.light.ambient);
    shader.set_vec3("light_diffuse", &map_render_data.rsw.light.diffuse);
    shader.set_f32("light_opacity", map_render_data.rsw.light.opacity);
    shader.set_vec3("in_lightWheight", &map_render_data.light_wheight);
    map_render_data.texture_atlas.bind(TEXTURE_0);
    shader.set_int("gnd_texture_atlas", 0);
    map_render_data.tile_color_texture.bind(TEXTURE_1);
    shader.set_int("tile_color_texture", 1);
    map_render_data.lightmap_texture.bind(TEXTURE_2);
    shader.set_int("lightmap_texture", 2);
    shader.set_int(
        "use_tile_color",
        if map_render_data.use_tile_colors {
            1
        } else {
            0
        },
    );
    shader.set_int(
        "use_lightmap",
        if map_render_data.use_lightmaps { 1 } else { 0 },
    );
    shader.set_int(
        "use_lighting",
        if map_render_data.use_lighting { 1 } else { 0 },
    );
    map_render_data.ground_vertex_array.bind().draw();
}

pub struct DamageRenderSystem {}

impl DamageRenderSystem {
    pub fn new() -> DamageRenderSystem {
        DamageRenderSystem {}
    }

    pub fn get_digits(n: u32) -> Vec<u8> {
        let mut digits = Vec::new();
        let mut n = n;
        while n > 9 {
            digits.push((n % 10) as u8);
            n = n / 10;
        }
        digits.push(n as u8);
        digits.reverse();
        return digits;
    }
}

impl DamageRenderSystem {
    const COMBO_DELAY_BETWEEN_SUBS: f32 = 0.1;

    pub fn run(
        &self,
        entities: &specs::Entities,
        numbers: &specs::ReadStorage<FlyingNumberComponent>,
        char_state_storage: &specs::ReadStorage<CharacterStateComponent>,
        desktop_entity_id: Entity,
        now: ElapsedTime,
        assets: &AssetResources,
        updater: &specs::Write<LazyUpdate>,
        render_commands: &mut RenderCommandCollectorComponent,
    ) {
        for (entity_id, number) in (entities, numbers).join() {
            DamageRenderSystem::add_render_command(
                number,
                char_state_storage,
                desktop_entity_id,
                now,
                assets,
                render_commands,
            );

            if number.die_at.has_passed(now) {
                updater.remove::<FlyingNumberComponent>(entity_id);
            }
        }
    }

    fn add_render_command(
        number: &FlyingNumberComponent,
        char_state_storage: &specs::ReadStorage<CharacterStateComponent>,
        desktop_entity_id: Entity,
        now: ElapsedTime,
        assets: &AssetResources,
        render_commands: &mut RenderCommandCollectorComponent,
    ) {
        let (number_value, digit_count) = match number.typ {
            FlyingNumberType::Combo {
                single_attack_damage,
                attack_count,
            } => {
                let index = ((now
                    .elapsed_since(number.start_time)
                    .div(DamageRenderSystem::COMBO_DELAY_BETWEEN_SUBS)
                    as u32)
                    + 1)
                .min(attack_count as u32);
                let number = index * single_attack_damage;
                (number, DamageRenderSystem::get_digits(number).len())
            }
            _ => (
                number.value,
                DamageRenderSystem::get_digits(number.value).len(),
            ),
        };

        let width = match number.typ {
            FlyingNumberType::Poison
            | FlyingNumberType::Heal
            | FlyingNumberType::Damage
            | FlyingNumberType::SubCombo
            | FlyingNumberType::Combo { .. }
            | FlyingNumberType::Mana
            | FlyingNumberType::Crit => digit_count as f32,
            FlyingNumberType::Block => assets.texts.attack_blocked.width as f32,
            FlyingNumberType::Absorb => assets.texts.attack_absorbed.width as f32,
        };

        let perc = now
            .elapsed_since(number.start_time)
            .div(number.duration as f32);

        // render sub damages for combo
        if let FlyingNumberType::Combo {
            single_attack_damage,
            attack_count,
        } = number.typ
        {
            let elapsed_attack_count = ((now
                .elapsed_since(number.start_time)
                .div(DamageRenderSystem::COMBO_DELAY_BETWEEN_SUBS)
                as i32)
                + 1)
            .min(attack_count as i32);
            for i in 0..elapsed_attack_count {
                let sub_number = FlyingNumberComponent {
                    value: single_attack_damage,
                    target_entity_id: number.target_entity_id,
                    typ: FlyingNumberType::SubCombo,
                    start_pos: number.start_pos,
                    start_time: number
                        .start_time
                        .add_seconds(DamageRenderSystem::COMBO_DELAY_BETWEEN_SUBS * i as f32),
                    die_at: ElapsedTime(0.0), // it is ignored
                    duration: 3.0,
                };
                DamageRenderSystem::add_render_command(
                    &sub_number,
                    char_state_storage,
                    desktop_entity_id,
                    now,
                    assets,
                    render_commands,
                );
            }
        }

        // TODO: don't render more than 1 damage in a single frame for the same target
        let (size, pos) = match number.typ {
            FlyingNumberType::Heal | FlyingNumberType::Mana => {
                DamageRenderSystem::calc_heal_size_pos(char_state_storage, number, width, perc)
            }
            FlyingNumberType::Combo { .. } => {
                let size = 1.0;
                let mut pos = Vector3::new(number.start_pos.x, 1.0, number.start_pos.y);
                pos.x -= width * size / 2.0;
                let y_offset = perc * 1.2;
                pos.y += 4.0 + y_offset;
                // a small hack to mitigate the distortion effect of perspective projection
                // at the edge of the screens
                pos.z -= y_offset;
                (size, pos)
            }
            FlyingNumberType::Damage => DamageRenderSystem::calc_damage_size_pos(number, perc, 1.0),
            FlyingNumberType::SubCombo => {
                DamageRenderSystem::calc_damage_size_pos(number, perc, 2.0)
            }
            FlyingNumberType::Poison => {
                DamageRenderSystem::calc_poison_size_pos(char_state_storage, number, width, perc)
            }
            FlyingNumberType::Block | FlyingNumberType::Absorb => {
                let real_pos = char_state_storage
                    .get(number.target_entity_id)
                    .map(|it| it.pos())
                    .unwrap_or(number.start_pos);
                let mut pos = Vector3::new(real_pos.x, 1.0, real_pos.y);
                let y_offset = (perc - 0.3) * 3.0;
                pos.y += 2.0 + y_offset;
                pos.z -= y_offset;
                (1.0, pos)
            }
            FlyingNumberType::Crit => {
                let mut pos = Vector3::new(number.start_pos.x, 1.0, number.start_pos.y);
                pos.y += 4.0 * perc;
                pos.z -= 2.0 * perc;
                pos.x += 2.0 * perc;
                let size = (1.0 - perc) * 4.0;
                (size, pos)
            }
        };
        let alpha = match number.typ {
            FlyingNumberType::Combo { .. } => {
                //                let y_offset = if perc < 0.3 { 0.0 } else { (perc - 0.3) * 3.0 };
                1.6 - (perc + 0.6 * perc)
            }
            _ => 1.3 - (perc + 0.3 * perc),
        };
        match number.typ {
            FlyingNumberType::Poison
            | FlyingNumberType::Heal
            | FlyingNumberType::Damage
            | FlyingNumberType::Combo { .. }
            | FlyingNumberType::SubCombo
            | FlyingNumberType::Mana
            | FlyingNumberType::Crit => {
                render_commands
                    .prepare_for_3d()
                    .scale(size)
                    .pos(&pos)
                    .color_rgb(
                        &number
                            .typ
                            .color(desktop_entity_id == number.target_entity_id),
                    )
                    .alpha(alpha)
                    .add_number_command(number_value, digit_count as u8);
            }
            FlyingNumberType::Block => {
                render_commands
                    .prepare_for_3d()
                    .pos(&pos)
                    .color_rgb(
                        &number
                            .typ
                            .color(desktop_entity_id == number.target_entity_id),
                    )
                    .alpha(alpha)
                    .add_billboard_command(&assets.texts.attack_blocked, false);
            }
            FlyingNumberType::Absorb => {
                render_commands
                    .prepare_for_3d()
                    .pos(&pos)
                    .color_rgb(
                        &number
                            .typ
                            .color(desktop_entity_id == number.target_entity_id),
                    )
                    .alpha(alpha)
                    .add_billboard_command(&assets.texts.attack_absorbed, false);
            }
        };
    }

    fn calc_damage_size_pos(
        number: &FlyingNumberComponent,
        perc: f32,
        speed: f32,
    ) -> (f32, Vector3<f32>) {
        let mut pos = Vector3::new(number.start_pos.x, 1.0, number.start_pos.y);
        pos.x += perc * 6.0;
        pos.z -= perc * 4.0;
        pos.y += 2.0
            + (-std::f32::consts::FRAC_PI_2 + (std::f32::consts::PI * (0.5 + perc * 1.5 * speed)))
                .sin()
                * 5.0;
        let size = (1.0 - perc * speed) * 1.0;
        return (size.max(0.0), pos);
    }

    fn calc_poison_size_pos(
        char_state_storage: &ReadStorage<CharacterStateComponent>,
        number: &FlyingNumberComponent,
        width: f32,
        perc: f32,
    ) -> (f32, Vector3<f32>) {
        let real_pos = char_state_storage
            .get(number.target_entity_id)
            .map(|it| it.pos())
            .unwrap_or(number.start_pos);
        let mut pos = Vector3::new(real_pos.x, 1.0, real_pos.y);
        let size = 0.4;
        pos.x -= width * size / 2.0;
        let y_offset = (perc - 0.3) * 3.0;
        pos.y += 2.0 + y_offset;
        pos.z -= y_offset;
        return (size, pos);
    }

    fn calc_heal_size_pos(
        char_state_storage: &ReadStorage<CharacterStateComponent>,
        number: &FlyingNumberComponent,
        width: f32,
        perc: f32,
    ) -> (f32, Vector3<f32>) {
        // follow the target
        let real_pos = char_state_storage
            .get(number.target_entity_id)
            .map(|it| it.pos())
            .unwrap_or(number.start_pos);
        // the bigger the heal, the bigger the number and stays big longer
        let heal_value_factor = number.value as f32 / 10_000.0;
        let size_decrease_speed = (4.0 - heal_value_factor * 2.0).max(2.0);
        let initial_size = 1.0 + heal_value_factor * 1.0;
        let size_mult = 0.2 + heal_value_factor * 0.2;
        let size = ((1.0 - perc * size_decrease_speed) * initial_size).max(size_mult);
        let mut pos = Vector3::new(real_pos.x, 1.0, real_pos.y);
        pos.x -= width * size / 2.0;
        let y_offset = if perc < 0.3 { 0.0 } else { (perc - 0.3) * 3.0 };
        pos.y += 2.0 + y_offset;
        // a small hack to mitigate the distortion effect of perspective projection
        // at the edge of the screens
        pos.z -= y_offset;
        return (size, pos);
    }

    //    pub value: u32,
    //    pub target_entity_id: Entity,
    //    pub typ: FlyingNumberType,
    //    pub start_pos: Vector2<f32>,
    //    pub start_time: ElapsedTime,
    //    pub die_at: ElapsedTime,
    //    pub duration: f32,
    pub fn render_single_number(
        value: u32,
        typ: FlyingNumberType,
        target_entity_id: Entity,
        start_time: ElapsedTime,
    ) {
    }
}

//impl<'a> specs::System<'a> for DamageRenderSystem {
//    type SystemData = (
//        specs::Entities<'a>,
//        specs::ReadStorage<'a, FlyingNumberComponent>,
//        &'a specs::ReadStorage<'a, CharacterStateComponent>,
//        specs::ReadStorage<'a, ControllerComponent>,
//        specs::ReadExpect<'a, SystemVariables>,
//        specs::WriteExpect<'a, SystemFrameDurations>,
//        specs::Write<'a, LazyUpdate>,
//    );
//    fn run(&mut self, (
//        entities,
//        numbers,
//        char_state_storage,
//        controller_storage,
//        system_vars,
//        mut system_benchmark,
//        updater,
//    ): Self::SystemData) {
//    }
//}

impl RenderDesktopClientSystem {
    fn draw_health_bar(
        &self,
        is_self: bool,
        char_state: &CharacterStateComponent,
        now: ElapsedTime,
        bounding_rect_2d: &SpriteBoundingRect,
        assets: &AssetResources,
        render_commands: &mut RenderCommandCollectorComponent,
    ) {
        let bar_w = match char_state.typ {
            CharType::Player => 80,
            CharType::Minion => 70,
            _ => 100,
        };
        let spr_x = bounding_rect_2d.bottom_left[0];
        let spr_w = bounding_rect_2d.top_right[0] - bounding_rect_2d.bottom_left[0];
        let bar_x = spr_x as f32 + (spr_w as f32 / 2.0) - (bar_w as f32 / 2.0);
        let mut draw_rect = |x: i32, y: i32, w: i32, h: i32, color: &[f32; 4]| {
            render_commands
                .prepare_for_2d()
                .color(&color)
                .size2(w as f32, h as f32)
                .screen_pos(
                    bar_x + x as f32,
                    bounding_rect_2d.top_right[1] as f32 - 30.0 + y as f32,
                )
                .add_rectangle_command(Layer2d::Layer2);
        };

        let hp_percentage = char_state.hp as f32 / char_state.calculated_attribs().max_hp as f32;
        let health_color = if is_self {
            [0.29, 0.80, 0.11, 1.0] // for self, the health bar is green
        } else {
            [0.79, 0.00, 0.21, 1.0] // for enemies, red
                                    // [0.2, 0.46, 0.9] // for friends, blue
        };
        let mana_color = [0.23, 0.79, 0.88, 1.0];
        let bottom_bar_y = match char_state.typ {
            CharType::Player => {
                draw_rect(0, 0, bar_w, 9, &[0.0, 0.0, 0.0, 1.0]); // black border
                draw_rect(0, 0, bar_w, 5, &[0.0, 0.0, 0.0, 1.0]); // center separator
                let inner_w = ((bar_w - 2) as f32 * hp_percentage) as i32;
                draw_rect(1, 1, inner_w, 4, &health_color);
                draw_rect(1, 6, bar_w - 2, 2, &mana_color);
                9
            }
            _ => {
                draw_rect(0, 0, bar_w, 5, &[0.0, 0.0, 0.0, 1.0]); // black border
                let inner_w = ((bar_w - 2) as f32 * hp_percentage) as i32;
                draw_rect(1, 1, inner_w, 3, &health_color);
                5
            }
        };

        // draw status remaining time indicator
        if let Some(perc) = char_state
            .statuses
            .calc_largest_remaining_status_time_percent(now)
        {
            let orange = [1.0, 0.55, 0.0, 1.0];
            let w = bar_w - 4;
            draw_rect(2, bottom_bar_y + 2, w, 2, &[0.0, 0.0, 0.0, 1.0]); // black bg
            let inner_w = (w as f32 * (1.0 - perc)) as i32;
            draw_rect(2, bottom_bar_y + 2, inner_w, 2, &orange);
        }

        // draw status indicator icons
        if char_state.attrib_bonuses().attrs.armor.is_not_zero() {
            let armor_bonus = char_state.attrib_bonuses().attrs.armor.as_i16();
            let shield_icon_texture = &assets.status_icons["shield"];
            let x = bar_x + bar_w as f32 + 1.0;
            let y = bounding_rect_2d.top_right[1] as f32 - 30.0;
            render_commands
                .prepare_for_2d()
                .color(&[1.0, 1.0, 1.0, 1.0])
                .screen_pos(x, y)
                .add_sprite_command(
                    shield_icon_texture,
                    [0.0, (-shield_icon_texture.height / 2) as f32],
                    false,
                    Layer2d::Layer7,
                );

            // progress bar
            let color = if armor_bonus > 0 {
                [0.0, 1.0, 0.0, 1.0]
            } else {
                [1.0, 0.0, 0.0, 1.0]
            };

            let perc = (now.percentage_between(
                char_state.attrib_bonuses().durations.armor_bonus_started_at,
                char_state.attrib_bonuses().durations.armor_bonus_ends_at,
            ) * 100.0) as i32;
            let perc = perc.max(1);
            let x = bar_x + bar_w as f32 + shield_icon_texture.width as f32 / 2.0 + 1.0;
            let y = bounding_rect_2d.top_right[1] as f32 - 30.0;

            render_commands
                .prepare_for_2d()
                .color(&color)
                .screen_pos(x, y)
                .rotation_rad(-std::f32::consts::FRAC_PI_2)
                .add_trimesh_command(&self.circle_vertex_arrays[&perc], Layer2d::Layer2);

            let text_texture = &assets.texts.custom_texts[&armor_bonus.to_string()];

            render_commands
                .prepare_for_2d()
                .color(&color)
                .screen_pos(x, y)
                .add_sprite_command(text_texture, [0.0, 0.0], false, Layer2d::Layer7);
        }
    }

    pub fn render_str(
        effect_name: &str,
        start_time: ElapsedTime,
        world_pos: &WorldCoords,
        system_vars: &mut SystemVariables,
        render_commands: &mut RenderCommandCollectorComponent,
    ) {
        let str_file = &system_vars.map_render_data.str_effects[effect_name];
        let seconds_needed_for_one_frame = 1.0 / str_file.fps as f32;
        let max_key = str_file.max_key;
        let key_index = system_vars
            .time
            .elapsed_since(start_time)
            .div(seconds_needed_for_one_frame) as i32
            % max_key as i32;

        for layer_index in 0..str_file.layers.len() {
            render_commands.prepare_for_3d().add_effect_command(
                world_pos,
                effect_name,
                key_index,
                layer_index,
            );
        }
    }
}
