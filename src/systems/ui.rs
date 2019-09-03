use nalgebra::Vector2;

use crate::components::char::{
    CharState, CharacterStateComponent, SpriteRenderDescriptorComponent,
};
use crate::components::controller::{ControllerComponent, HumanInputComponent, SkillKey};
use crate::systems::render::render_command::{RenderCommandCollectorComponent, UiLayer2d};
use crate::systems::SystemVariables;
use crate::video::{VIDEO_HEIGHT, VIDEO_WIDTH};
use crate::{ElapsedTime, SpriteResource};

pub struct RenderUI {}

impl RenderUI {
    pub fn new() -> RenderUI {
        RenderUI {}
    }

    pub fn run(
        &self,
        char_state: &CharacterStateComponent,
        input: &HumanInputComponent,
        controller: &ControllerComponent,
        render_commands: &mut RenderCommandCollectorComponent,
        system_vars: &specs::WriteExpect<SystemVariables>,
    ) {
        // Draw casting bar
        match char_state.state() {
            CharState::CastingSkill(casting_info) => {
                // for really short skills, don't render it
                if casting_info
                    .cast_ends
                    .minus(casting_info.cast_started)
                    .as_f32()
                    > 0.1
                {
                    let mut draw_rect = |x: i32, y: i32, w: i32, h: i32, color: &[u8; 4]| {
                        let bar_w = 540;
                        let bar_x = ((VIDEO_WIDTH / 2) - (bar_w / 2) - 2) as i32;
                        // .trimesh_2d(&system_vars.map_render_data.rectangle_vertex_array)
                        render_commands
                            .prepare_for_2d()
                            .screen_pos(bar_x + x, VIDEO_HEIGHT as i32 - 200 + y)
                            .size2(w, h)
                            .color(&color)
                            .add_rectangle_command(UiLayer2d::SelfCastingBar)
                    };
                    draw_rect(0, 0, 540, 30, &[36, 92, 201, 77]); // transparent blue background
                    draw_rect(2, 2, 536, 26, &[0, 0, 0, 255]); // black background
                    let percentage = system_vars
                        .time
                        .percentage_between(casting_info.cast_started, casting_info.cast_ends);
                    draw_rect(3, 3, (percentage * 543.0) as i32, 24, &[36, 92, 201, 255]); // inner fill
                }
            }
            _ => {}
        }

        // draw skill bar
        let single_icon_size = 48;
        let inner_border = 3;
        let outer_border = 6;
        let space = 4;
        let skill_bar_width =
            (outer_border * 2) + 4 * single_icon_size + inner_border * 4 * 2 + 3 * space;
        let start_x = VIDEO_WIDTH as i32 / 2 - skill_bar_width / 2;
        let y = VIDEO_HEIGHT as i32 - single_icon_size - 20 - outer_border * 2 - inner_border * 2;

        // blueish background
        render_commands
            .prepare_for_2d()
            .screen_pos(start_x, y)
            .size2(
                skill_bar_width,
                single_icon_size + (outer_border * 2 + inner_border * 2),
            )
            .color(&[28, 64, 122, 255])
            .add_rectangle_command(UiLayer2d::SkillBar);

        let mut x = start_x + outer_border;
        for skill_key in [SkillKey::Q, SkillKey::W, SkillKey::E, SkillKey::R].iter() {
            if let Some(skill) = input.get_skill_for_key(*skill_key) {
                // inner border
                let not_castable = char_state
                    .skill_cast_allowed_at
                    .get(&skill)
                    .unwrap_or(&ElapsedTime(0.0))
                    .is_later_than(system_vars.time);
                let border_color = if not_castable {
                    [179, 179, 179, 255] // grey
                } else {
                    controller
                        .select_skill_target
                        .filter(|it| it.0 == *skill_key)
                        .map(|_it| [0, 255, 0, 255])
                        .unwrap_or([0, 0, 0, 255])
                };
                render_commands
                    .prepare_for_2d()
                    .screen_pos(x, y + outer_border)
                    .size2(
                        single_icon_size + inner_border * 2,
                        single_icon_size + inner_border * 2,
                    )
                    .color(&border_color)
                    .add_rectangle_command(UiLayer2d::SkillBar);

                x += inner_border;
                let icon_y = y + outer_border + inner_border;
                // blueish background
                render_commands
                    .prepare_for_2d()
                    .screen_pos(x, icon_y)
                    .size2(single_icon_size, single_icon_size)
                    .color(
                        &(if not_castable {
                            [179, 179, 179, 255] // grey if not castable
                        } else {
                            [28, 64, 122, 255]
                        }),
                    )
                    .add_rectangle_command(UiLayer2d::SkillBar);

                render_commands
                    .prepare_for_2d()
                    .screen_pos(x, icon_y)
                    .size(2.0)
                    .add_texture_command(
                        &system_vars.assets.skill_icons[&skill],
                        UiLayer2d::SkillBarIcon,
                    );

                let skill_key_texture = &system_vars.assets.texts.skill_key_texts[&skill_key];
                let center_x = -2 + x + single_icon_size - skill_key_texture.width;
                let center_y = -2 + icon_y + single_icon_size - skill_key_texture.height;
                render_commands
                    .prepare_for_2d()
                    .screen_pos(center_x, center_y)
                    .add_texture_command(skill_key_texture, UiLayer2d::SkillBarKey);
                x += single_icon_size + inner_border + space;
            }
        }

        // render targeting skill name
        if let Some((_skill_key, skill)) = controller.select_skill_target {
            let texture = &system_vars.assets.texts.skill_name_texts[&skill];
            let not_castable = char_state
                .skill_cast_allowed_at
                .get(&skill)
                .unwrap_or(&ElapsedTime(0.0))
                .is_later_than(system_vars.time);
            render_commands
                .prepare_for_2d()
                .color(
                    &(if not_castable {
                        [179, 179, 179, 255]
                    } else {
                        [255, 255, 255, 255]
                    }),
                )
                .screen_pos(
                    input.last_mouse_x as i32 - texture.width / 2,
                    input.last_mouse_y as i32 + 32,
                )
                .add_texture_command(texture, UiLayer2d::SelectingTargetSkillName);
        }

        render_action_2d(
            &system_vars,
            &controller.cursor_anim_descr,
            &system_vars.assets.sprites.cursors,
            &Vector2::new(input.last_mouse_x as f32, input.last_mouse_y as f32),
            &controller.cursor_color,
            render_commands,
        );
    }
}

fn render_action_2d(
    system_vars: &SystemVariables,
    animated_sprite: &SpriteRenderDescriptorComponent,
    sprite_res: &SpriteResource,
    pos: &Vector2<f32>,
    color: &[u8; 3],
    render_commands: &mut RenderCommandCollectorComponent,
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

        // todo: don't we need layer.scale[0]?

        let offset = [0, 0];
        let offset = [layer.pos[0] + offset[0], layer.pos[1] + offset[1]];
        let offset = [offset[0] as f32, offset[1] as f32];
        let offset = [
            offset[0] - (texture.texture.width / 2) as f32,
            offset[1] - (texture.texture.height / 2) as f32,
        ];

        render_commands
            .prepare_for_2d()
            .screen_pos(pos.x as i32, pos.y as i32)
            .color_rgb(color)
            .add_sprite_command(&texture.texture, offset, layer.is_mirror, UiLayer2d::Cursor);
    }
}
