use nalgebra::{Matrix4, Vector2, Vector3};
use specs::prelude::*;

use crate::{ElapsedTime, SpriteResource};
use crate::components::BrowserClient;
use crate::components::char::{CharacterStateComponent, CharState, SpriteRenderDescriptorComponent};
use crate::components::controller::ControllerComponent;
use crate::cursor::{CURSOR_ATTACK, CURSOR_NORMAL, CURSOR_STOP, CURSOR_TARGET, CURSOR_CLICK};
use crate::systems::{SystemFrameDurations, SystemVariables};
use crate::video::{TEXTURE_0, VertexArray, VIDEO_HEIGHT, VIDEO_WIDTH, GlTexture};
use crate::video::VertexAttribDefinition;
use crate::components::skill::SkillTargetType;
use crate::components::skill::SkillDescriptor;

pub struct RenderUI {
    cursor_anim_descr: SpriteRenderDescriptorComponent,
    vao: VertexArray,
}

impl RenderUI {
    pub fn new() -> RenderUI {
        let s: Vec<[f32; 2]> = vec![
            [0.0, 1.0],
            [1.0, 1.0],
            [0.0, 0.0],
            [1.0, 0.0]
        ];
        RenderUI {
            cursor_anim_descr: SpriteRenderDescriptorComponent {
                action_index: 0,
                animation_started: ElapsedTime(0.0),
                animation_ends_at: ElapsedTime(0.0),
                forced_duration: None,
                direction: 0,
                fps_multiplier: 1.0,
            },
            vao: VertexArray::new(
                gl::TRIANGLE_STRIP,
                &s, 4, vec![
                    VertexAttribDefinition {
                        number_of_components: 2,
                        offset_of_first_element: 0,
                    }
                ]),
        }
    }
}

impl<'a> specs::System<'a> for RenderUI {
    type SystemData = (
        specs::Entities<'a>,
        specs::ReadStorage<'a, ControllerComponent>,
        specs::ReadStorage<'a, BrowserClient>,
        specs::ReadStorage<'a, CharacterStateComponent>,
        specs::ReadExpect<'a, SystemVariables>,
        specs::WriteExpect<'a, SystemFrameDurations>,
    );

    fn run(&mut self, (
        entities,
        input_storage,
        browser_client_storage,
        char_state_storage,
        system_vars,
        mut system_benchmark,
    ): Self::SystemData) {
        let stopwatch = system_benchmark.start_measurement("RenderUI");
        for (controller, _not_browser) in (&input_storage, !&browser_client_storage).join() {
            // Draw casting bar
            let char_state = char_state_storage.get(controller.char).unwrap();
            match char_state.state() {
                CharState::CastingSkill(casting_info) => {
                    // draw health bars etc
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
                    let percentage = system_vars.time.percentage_between(
                        casting_info.cast_started,
                        casting_info.cast_ends,
                    );
                    draw_rect(3, 3, (percentage * 543.0) as i32, 24, &[0.14, 0.36, 0.79, 1.0]); // inner fill
                }
                _ => {}
            }

            // Draw cursor
            let cursor = if let Some((skill_key, skill)) = controller.is_selecting_target() {
                render_texture_2d(
                    &system_vars,
                    &system_vars.texts.skill_name_texts[&skill],
                    &Vector2::new(
                        controller.last_mouse_x as f32,
                        controller.last_mouse_y as f32 + 32.0,
                    ),
                );
                if skill.get_skill_target_type() != SkillTargetType::Area {
                    CURSOR_TARGET
                } else {
                    CURSOR_CLICK
                }
            } else if let Some(entity_below_cursor) = controller.entity_below_cursor {
                if entity_below_cursor == controller.char { // self
                    CURSOR_NORMAL
                } else {
                    CURSOR_ATTACK
                }
            } else if !controller.cell_below_cursor_walkable {
                CURSOR_STOP
            } else {
                CURSOR_NORMAL
            };
            self.cursor_anim_descr.action_index = cursor.1;
            render_sprite_2d(&system_vars,
                             &self.cursor_anim_descr,
                             &system_vars.sprites.cursors,
                             &Vector2::new(controller.last_mouse_x as f32, controller.last_mouse_y as f32));
        }
    }
}

fn render_sprite_2d(system_vars: &SystemVariables,
                    animated_sprite: &SpriteRenderDescriptorComponent,
                    sprite_res: &SpriteResource,
                    pos: &Vector2<f32>,
) {
    let idx = animated_sprite.action_index;
    let action = &sprite_res.action.actions[idx];

    let frame_index = {
        let frame_count = action.frames.len();
        let time_needed_for_one_frame = action.delay as f32 / 1000.0 * 4.0;
        let elapsed_time = system_vars.time.elapsed_since(animated_sprite.animation_started);
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
        let offset = [
            offset[0] as f32,
            offset[1] as f32
        ];

        let mut matrix = Matrix4::<f32>::identity();
        let pos = Vector3::new(pos.x, pos.y, 0.0);
        matrix.prepend_translation_mut(&pos);

        let width = width as f32;
        let width = if layer.is_mirror { -width } else { width };

        let shader = system_vars.shaders.sprite2d_shader.gl_use();
        shader.set_mat4("projection", &system_vars.matrices.ortho);
        shader.set_int("model_texture", 0);
        shader.set_f32("alpha", 1.0);
        shader.set_mat4("model", &matrix);
        shader.set_vec2("offset", &offset);
        shader.set_vec2("size", &[
            width,
            -height as f32
        ]);
        shader.set_f32("alpha", 1.0);
        system_vars.map_render_data.sprite_vertex_array.bind().draw();
    }
}

fn render_texture_2d(system_vars: &SystemVariables,
                     texture: &GlTexture,
                     pos: &Vector2<f32>,
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
    shader.set_f32("alpha", 1.0);
    shader.set_mat4("model", &matrix);
    shader.set_vec2("offset", &[0.0, 0.0]);
    shader.set_vec2("size", &[
        width,
        -height as f32
    ]);
    shader.set_f32("alpha", 1.0);
    system_vars.map_render_data.sprite_vertex_array.bind().draw();
}