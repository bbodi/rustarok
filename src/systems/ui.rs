use nalgebra::{Matrix4, Vector2, Vector3};

use crate::components::char::{
    CharState, CharacterStateComponent, SpriteRenderDescriptorComponent,
};
use crate::components::controller::{ControllerComponent, SkillKey};
use crate::components::skills::skill::SkillTargetType;
use crate::cursor::{CURSOR_ATTACK, CURSOR_CLICK, CURSOR_NORMAL, CURSOR_STOP, CURSOR_TARGET};
use crate::systems::SystemVariables;
use crate::video::VertexAttribDefinition;
use crate::video::{GlTexture, VertexArray, TEXTURE_0, VIDEO_HEIGHT, VIDEO_WIDTH};
use crate::SpriteResource;

pub struct RenderUI {
    vao: VertexArray,
}

impl RenderUI {
    pub fn new() -> RenderUI {
        let s: Vec<[f32; 2]> = vec![[0.0, 1.0], [1.0, 1.0], [0.0, 0.0], [1.0, 0.0]];
        RenderUI {
            vao: VertexArray::new(
                gl::TRIANGLE_STRIP,
                &s,
                4,
                vec![VertexAttribDefinition {
                    number_of_components: 2,
                    offset_of_first_element: 0,
                }],
            ),
        }
    }

    pub fn run(
        &self,
        _entities: &specs::Entities,
        controller: &mut ControllerComponent,
        char_state_storage: &specs::ReadStorage<CharacterStateComponent>,
        system_vars: &specs::WriteExpect<SystemVariables>,
    ) {
        // Draw casting bar
        let char_state = char_state_storage.get(controller.char_entity_id).unwrap();
        match char_state.state() {
            CharState::CastingSkill(casting_info) => {
                // for really short skills, don't render it
                if casting_info
                    .cast_ends
                    .minus(casting_info.cast_started)
                    .as_f32()
                    > 0.1
                {
                    let shader = system_vars.shaders.trimesh2d_shader.gl_use();
                    shader.set_mat4("projection", &system_vars.matrices.ortho);
                    let vao = self.vao.bind();
                    let draw_rect = |x: i32, y: i32, w: i32, h: i32, color: &[f32; 4]| {
                        let mut matrix = Matrix4::<f32>::identity();
                        let bar_w = 540.0;
                        let bar_x = (VIDEO_WIDTH as f32 / 2.0) - (bar_w / 2.0) - 2.0;
                        let pos = Vector3::new(
                            bar_x + x as f32,
                            VIDEO_HEIGHT as f32 - 200.0 + y as f32,
                            0.0,
                        );
                        matrix.prepend_translation_mut(&pos);
                        shader.set_mat4("model", &matrix);
                        shader.set_vec4("color", color);
                        shader.set_vec2("size", &[w as f32, h as f32]);
                        vao.draw();
                    };
                    draw_rect(0, 0, 540, 30, &[0.14, 0.36, 0.79, 0.3]); // transparent blue background
                    draw_rect(2, 2, 536, 26, &[0.0, 0.0, 0.0, 1.0]); // black background
                    let percentage = system_vars
                        .time
                        .percentage_between(casting_info.cast_started, casting_info.cast_ends);
                    draw_rect(
                        3,
                        3,
                        (percentage * 543.0) as i32,
                        24,
                        &[0.14, 0.36, 0.79, 1.0],
                    ); // inner fill
                }
            }
            _ => {}
        }

        let selecting_target = controller.is_selecting_target();

        // draw skill bar
        let single_icon_size = 48;
        let inner_border = 3;
        let outer_border = 6;
        let space = 4;
        let skill_bar_width =
            (outer_border * 2) + 4 * single_icon_size + inner_border * 4 * 2 + 3 * space;
        let start_x = VIDEO_WIDTH / 2 - skill_bar_width / 2;
        let y = VIDEO_HEIGHT - single_icon_size - 20 - outer_border * 2 - inner_border * 2;

        // blueish background
        render_rect(
            &system_vars,
            &Vector2::new(start_x as f32, y as f32),
            &[
                skill_bar_width as f32,
                single_icon_size as f32 + (outer_border * 2 + inner_border * 2) as f32,
            ],
            &[0.11, 0.25, 0.48, 1.0],
        );

        let mut x = start_x + outer_border;
        for (i, skill_key) in [SkillKey::Q, SkillKey::W, SkillKey::E, SkillKey::R]
            .iter()
            .enumerate()
        {
            if let Some(skill) = controller.get_skill_for_key(*skill_key) {
                // inner border
                let border_color = selecting_target
                    .filter(|it| it.0 == *skill_key)
                    .map(|_it| [0.0, 1.0, 0.0, 1.0])
                    .unwrap_or([0.0, 0.0, 0.0, 1.0]);
                render_rect(
                    &system_vars,
                    &Vector2::new(x as f32, (y + outer_border) as f32),
                    &[
                        (single_icon_size + inner_border * 2) as f32,
                        (single_icon_size + inner_border * 2) as f32,
                    ],
                    &border_color,
                );

                x += inner_border;
                let icon_y = (y + outer_border + inner_border) as f32;
                // blueish background
                render_rect(
                    &system_vars,
                    &Vector2::new(x as f32, icon_y),
                    &[(single_icon_size) as f32, (single_icon_size) as f32],
                    &[0.11, 0.25, 0.48, 1.0],
                );
                let texture = &system_vars.skill_icons[&skill];
                render_texture_2d(&system_vars, texture, &Vector2::new(x as f32, icon_y), 2.0);
                let skill_key_texture = &system_vars.texts.skill_key_texts[&skill_key];
                let center_x =
                    -2.0 + x as f32 + single_icon_size as f32 - skill_key_texture.width as f32;
                let center_y =
                    -2.0 + icon_y + single_icon_size as f32 - skill_key_texture.height as f32;
                render_texture_2d(
                    &system_vars,
                    skill_key_texture,
                    &Vector2::new(center_x, center_y),
                    1.0,
                );
                x += single_icon_size + inner_border + space;
            }
        }

        // Draw cursor
        let cursor = if let Some((skill_key, skill)) = selecting_target {
            let texture = &system_vars.texts.skill_name_texts[&skill];
            render_texture_2d(
                &system_vars,
                texture,
                &Vector2::new(
                    controller.last_mouse_x as f32 - texture.width as f32 / 2.0,
                    controller.last_mouse_y as f32 + 32.0,
                ),
                1.0,
            );
            if skill.get_skill_target_type() != SkillTargetType::Area {
                CURSOR_TARGET
            } else {
                CURSOR_CLICK
            }
        } else if let Some(entity_below_cursor) = controller.entity_below_cursor {
            let ent_below_cursor_state = char_state_storage.get(entity_below_cursor).unwrap();
            let ent_is_dead = char_state_storage
                .get(entity_below_cursor)
                .map(|it| !it.state().is_alive())
                .unwrap_or(false);
            if entity_below_cursor == controller.char_entity_id || ent_is_dead {
                // self or dead
                CURSOR_NORMAL
            } else {
                CURSOR_ATTACK
            }
        } else if !controller.cell_below_cursor_walkable {
            CURSOR_STOP
        } else {
            CURSOR_NORMAL
        };
        controller.cursor_anim_descr.action_index = cursor.1;
        render_sprite_2d(
            &system_vars,
            &controller.cursor_anim_descr,
            &system_vars.sprites.cursors,
            &Vector2::new(
                controller.last_mouse_x as f32,
                controller.last_mouse_y as f32,
            ),
        );
    }
}

fn render_sprite_2d(
    system_vars: &SystemVariables,
    animated_sprite: &SpriteRenderDescriptorComponent,
    sprite_res: &SpriteResource,
    pos: &Vector2<f32>,
) {
    let idx = animated_sprite.action_index;
    let action = &sprite_res.action.actions[idx];

    let frame_index = {
        let frame_count = action.frames.len();
        let time_needed_for_one_frame = action.delay as f32 / 1000.0 * 4.0;
        let elapsed_time = system_vars
            .time
            .elapsed_since(animated_sprite.animation_started);
        ((elapsed_time.div(time_needed_for_one_frame)) as usize % frame_count) as usize
    };
    let animation = &action.frames[frame_index];
    for layer in &animation.layers {
        if layer.sprite_frame_index < 0 {
            continue;
        }
        let texture = &sprite_res.textures[layer.sprite_frame_index as usize];

        let width = texture.original_width as f32 * layer.scale[0];
        let height = texture.original_height as f32 * layer.scale[1];
        texture.texture.bind(TEXTURE_0);

        let offset = [0, 0];
        let offset = [layer.pos[0] + offset[0], layer.pos[1] + offset[1]];
        let offset = [offset[0] as f32, offset[1] as f32];

        let mut matrix = Matrix4::<f32>::identity();
        let pos = Vector3::new(pos.x, pos.y, 0.0);
        matrix.prepend_translation_mut(&pos);

        let width = width as f32;
        let width = if layer.is_mirror { -width } else { width };

        let shader = system_vars.shaders.sprite2d_shader.gl_use();
        shader.set_mat4("projection", &system_vars.matrices.ortho);
        shader.set_int("model_texture", 0);
        shader.set_mat4("model", &matrix);
        shader.set_vec2("offset", &offset);
        shader.set_vec4("color", &[1.0, 1.0, 1.0, 1.0]);
        shader.set_vec2("size", &[width, -height as f32]);
        system_vars
            .map_render_data
            .centered_sprite_vertex_array
            .bind()
            .draw();
    }
}

fn render_rect(
    system_vars: &SystemVariables,
    pos: &Vector2<f32>,
    size: &[f32; 2],
    color: &[f32; 4],
) {
    let mut matrix = Matrix4::<f32>::identity();
    let pos = Vector3::new(pos.x, pos.y, 0.0);
    matrix.prepend_translation_mut(&pos);

    let shader = system_vars.shaders.rectangle_2d_shader.gl_use();
    shader.set_mat4("projection", &system_vars.matrices.ortho);
    shader.set_mat4("model", &matrix);
    shader.set_vec4("color", color);
    shader.set_vec2("size", size);
    system_vars
        .map_render_data
        .rectangle_vertex_array
        .bind()
        .draw();
}

fn render_texture_2d(
    system_vars: &SystemVariables,
    texture: &GlTexture,
    pos: &Vector2<f32>,
    size: f32,
) {
    let width = texture.width as f32;
    let height = texture.height as f32;
    texture.bind(TEXTURE_0);

    let mut matrix = Matrix4::<f32>::identity();
    let pos = Vector3::new(pos.x, pos.y, 0.0);
    matrix.prepend_translation_mut(&pos);

    let shader = system_vars.shaders.sprite2d_shader.gl_use();
    shader.set_mat4("projection", &system_vars.matrices.ortho);
    shader.set_int("model_texture", 0);
    shader.set_mat4("model", &matrix);
    shader.set_vec2("offset", &[0.0, 0.0]);
    shader.set_vec2("size", &[width * size, height as f32 * size]);
    shader.set_vec4("color", &[1.0, 1.0, 1.0, 1.0]);
    system_vars
        .map_render_data
        .sprite_vertex_array
        .bind()
        .draw();
}
