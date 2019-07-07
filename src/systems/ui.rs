use std::collections::HashMap;

use nalgebra::{Matrix3, Matrix4, Point2, Point3, Rotation3, Vector2, Vector3};
use specs::prelude::*;

use crate::{MapRenderData, Shaders, SpriteResource, Tick};
use crate::cam::Camera;
use crate::components::{BrowserClient, ComponentRadius, ControllerComponent, CharacterStateComponent, PhysicsComponent, PlayerSpriteComponent, MonsterSpriteComponent};
use crate::cursor::CURSOR_NORMAL;
use crate::systems::{SystemFrameDurations, SystemVariables};
use crate::video::{draw_circle_inefficiently, draw_lines_inefficiently, draw_lines_inefficiently2, VertexArray, VIDEO_HEIGHT, VIDEO_WIDTH};
use crate::video::VertexAttribDefinition;

pub struct RenderUI;

impl<'a> specs::System<'a> for RenderUI {
    type SystemData = (
        specs::Entities<'a>,
        specs::ReadStorage<'a, ControllerComponent>,
        specs::ReadStorage<'a, BrowserClient>,
        specs::ReadStorage<'a, PhysicsComponent>,
        specs::ReadStorage<'a, PlayerSpriteComponent>,
        specs::ReadStorage<'a, CharacterStateComponent>,
        specs::ReadExpect<'a, SystemVariables>,
        specs::WriteExpect<'a, SystemFrameDurations>,
    );

    fn run(&mut self, (
        entities,
        input_storage,
        browser_client_storage,
        physics_storage,
        animated_sprite_storage,
        ai_storage,
        system_vars,
        mut system_benchmark,
    ): Self::SystemData) {
        let stopwatch = system_benchmark.start_measurement("RenderUI");
        for (controller, _not_browser) in (&input_storage, !&browser_client_storage).join() {
            let tick = system_vars.tick;
            render_sprite_2d(&system_vars,
                             tick,
                             &MonsterSpriteComponent {
                                 file_index: 0, //
                                 action_index: CURSOR_NORMAL.1,
                                 animation_start: Tick(0),
                                 direction: 0,
                             },
                             &system_vars.system_sprites.cursors,
                             &Vector2::new(controller.last_mouse_x as f32, controller.last_mouse_y as f32))
        }
    }
}

fn render_sprite_2d(system_vars: &SystemVariables,
                    tick: Tick,
                    animated_sprite: &MonsterSpriteComponent,
                    sprite_res: &SpriteResource,
                    pos: &Vector2<f32>,
) {
    // draw layer
    let animation_elapsed_tick = tick.0 - animated_sprite.animation_start.0;
    let idx = animated_sprite.action_index;

    let delay = sprite_res.action.actions[idx].delay;
    let frame_count = sprite_res.action.actions[idx].frames.len();
    let frame_index = ((animation_elapsed_tick / (delay / 20) as u64) % frame_count as u64) as usize;
    let animation = &sprite_res.action.actions[idx].frames[frame_index];
    for layer in &animation.layers {
        if layer.sprite_frame_index < 0 {
            continue;
        }
        let texture = &sprite_res.textures[layer.sprite_frame_index as usize];

        let width = texture.original_width as f32 * layer.scale[0];
        let height = texture.original_height as f32 * layer.scale[1];
        texture.texture.bind(gl::TEXTURE0);

        let mut offset = [0, 0];
        let offset = [layer.pos[0] + offset[0], layer.pos[1] + offset[1]];
        let offset = [
            offset[0] as f32,
            offset[1] as f32
        ];

        let mut matrix = Matrix4::<f32>::identity();
        let mut pos = Vector3::new(pos.x, pos.y, 0.0);
        matrix.prepend_translation_mut(&pos);

        let width = width as f32;
        let width = if layer.is_mirror { -width } else { width };

        system_vars.shaders.sprite2d_shader.gl_use();
        system_vars.shaders.sprite2d_shader.set_mat4("projection", &system_vars.matrices.ortho);
        system_vars.shaders.sprite2d_shader.set_int("model_texture", 0);
        system_vars.shaders.sprite2d_shader.set_f32("alpha", 1.0);
        system_vars.shaders.player_shader.set_mat4("model", &matrix);
        system_vars.shaders.player_shader.set_vec2("offset", &offset);
        system_vars.shaders.player_shader.set_vec3("size", &[
            width,
            -height as f32,
            0.0
        ]);
        system_vars.shaders.player_shader.set_f32("alpha", 1.0);
        system_vars.map_render_data.sprite_vertex_array.bind().draw();
    }
}