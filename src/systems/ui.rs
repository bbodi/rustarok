use nalgebra::{Point3, Vector2};

use crate::components::char::{
    CharOutlook, CharState, CharacterStateComponent, NpcComponent, SpriteRenderDescriptorComponent,
};
use crate::components::controller::{
    CharEntityId, ControllerComponent, HumanInputComponent, SkillKey,
};
use crate::systems::input_sys::InputConsumerSystem;
use crate::systems::render::render_command::{RenderCommandCollector, UiLayer2d};
use crate::systems::SystemVariables;
use crate::video::{VIDEO_HEIGHT, VIDEO_WIDTH};
use crate::{ElapsedTime, SpriteResource};
use specs::prelude::*;
use specs::ReadStorage;

pub struct RenderUI {}

impl RenderUI {
    pub fn new() -> RenderUI {
        RenderUI {}
    }

    pub fn run(
        &mut self,
        self_char_state: &CharacterStateComponent,
        input: &HumanInputComponent,
        controller: &ControllerComponent,
        render_commands: &mut RenderCommandCollector,
        system_vars: &mut SystemVariables,
        char_state_storage: &ReadStorage<CharacterStateComponent>,
        npc_storage: &ReadStorage<NpcComponent>,
        entities: &specs::Entities,
        camera_pos: &Point3<f32>,
    ) {
        // Draw casting bar
        match self_char_state.state() {
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
                        render_commands
                            .rectangle_2d()
                            .screen_pos(bar_x + x, VIDEO_HEIGHT as i32 - 200 + y)
                            .size(w as u16, h as u16)
                            .color(&color)
                            .layer(UiLayer2d::SelfCastingBar)
                            .add()
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

        let main_skill_bar_top = RenderUI::draw_main_skill_bar(
            self_char_state,
            input,
            controller,
            render_commands,
            &system_vars,
        );

        RenderUI::draw_secondary_skill_bar(
            self_char_state,
            input,
            controller,
            render_commands,
            &system_vars,
            main_skill_bar_top,
        );

        // render targeting skill name
        RenderUI::draw_targeting_skill_name(
            self_char_state,
            input,
            controller,
            render_commands,
            &system_vars,
        );

        self.draw_minimap(
            self_char_state,
            render_commands,
            system_vars,
            char_state_storage,
            npc_storage,
            entities,
            camera_pos,
        );

        render_action_2d(
            &system_vars,
            &controller.cursor_anim_descr,
            &system_vars.assets.sprites.cursors,
            &Vector2::new(input.last_mouse_x as i32, input.last_mouse_y as i32),
            &controller.cursor_color,
            render_commands,
            UiLayer2d::Cursor,
            1.0,
        );
    }

    fn draw_minimap(
        &mut self,
        self_char_state: &CharacterStateComponent,
        render_commands: &mut RenderCommandCollector,
        system_vars: &mut SystemVariables,
        char_state_storage: &ReadStorage<CharacterStateComponent>,
        npc_storage: &ReadStorage<NpcComponent>,
        entities: &specs::Entities,
        camera_pos: &Point3<f32>,
    ) {
        // prontera minimaps has empty spaces: left 52, right 45 pixels
        // 6 pixel padding on left and bottom, 7 on top and right
        let scale = (VIDEO_HEIGHT as i32 / 4)
            .min(system_vars.map_render_data.minimap_texture.height) as f32
            / system_vars.map_render_data.minimap_texture.height as f32;
        let all_minimap_w =
            (system_vars.map_render_data.minimap_texture.width as f32 * scale) as i32;
        let minimap_render_x = VIDEO_WIDTH as i32 - all_minimap_w - 20;
        let offset_x = ((52.0 + 6.0) * scale) as i32;
        let minimap_x = minimap_render_x + offset_x;
        let minimap_h = (system_vars.map_render_data.minimap_texture.height as f32 * scale) as i32;
        let minimap_y = VIDEO_HEIGHT as i32 - minimap_h - 20;
        render_commands
            .sprite_2d()
            .scale(scale)
            .screen_pos(minimap_render_x, minimap_y)
            .layer(UiLayer2d::Minimap)
            .add(&system_vars.map_render_data.minimap_texture);
        let minimap_w = (system_vars.map_render_data.minimap_texture.width as f32 * scale) as i32
            - ((52.0 + 45.0 + 6.0 + 7.0) * scale) as i32;
        let real_to_map_scale_w =
            minimap_w as f32 / (system_vars.map_render_data.gnd.width * 2) as f32;
        let real_to_map_scale_h =
            minimap_h as f32 / (system_vars.map_render_data.gnd.height * 2) as f32;
        for (entity_id, char_state) in (entities, char_state_storage).join() {
            let entity_id = CharEntityId(entity_id);
            let head_index = if npc_storage.get(entity_id.0).is_none() {
                if let CharOutlook::Player {
                    head_index, sex, ..
                } = char_state.outlook
                {
                    Some((sex, head_index))
                } else {
                    None
                }
            } else {
                None
            };
            let color = if self_char_state.team.is_ally_to(char_state.team) {
                &[0, 0, 255, 255]
            } else {
                &[255, 0, 0, 255]
            };

            let (char_x, char_y) = {
                let char_pos = char_state.pos();
                let char_y = (system_vars.map_render_data.gnd.height * 2) as f32 + char_pos.y;
                (char_pos.x, char_y)
            };

            if let Some((sex, head_index)) = head_index {
                let head_texture = {
                    let sprites = &system_vars.assets.sprites.head_sprites;
                    &sprites[sex as usize][head_index].textures[0]
                };

                let center_offset_x = (head_texture.width as f32 * 0.7 / 2.0) as i32;
                let center_offset_y = (head_texture.height as f32 * 0.7 / 2.0) as i32;
                render_commands
                    .sprite_2d()
                    .screen_pos(
                        minimap_x + (char_x * real_to_map_scale_w) as i32 - center_offset_x,
                        minimap_y + (char_y * real_to_map_scale_h) as i32 - center_offset_y,
                    )
                    .color(color)
                    .scale(0.7)
                    .layer(UiLayer2d::MinimapImportantEntities)
                    .add(head_texture);

                let center_offset_x = (head_texture.width as f32 * 0.5 / 2.0) as i32;
                let center_offset_y = (head_texture.height as f32 * 0.5 / 2.0) as i32;
                render_commands
                    .sprite_2d()
                    .screen_pos(
                        minimap_x + (char_x * real_to_map_scale_w) as i32 - center_offset_x,
                        minimap_y + (char_y * real_to_map_scale_h) as i32 - center_offset_y,
                    )
                    .scale(0.5)
                    .layer(UiLayer2d::MinimapImportantEntities)
                    .add(head_texture);
            } else {
                render_commands
                    .point_2d()
                    .screen_pos(
                        minimap_x + (char_x * real_to_map_scale_w) as i32,
                        minimap_y + (char_y * real_to_map_scale_h) as i32,
                    )
                    .color(color)
                    .layer(UiLayer2d::MinimapSimpleEntities)
                    .add();
            }
        }

        // draw camera rectangle
        let right_top = InputConsumerSystem::project_screen_pos_to_world_pos(
            VIDEO_WIDTH as u16,
            0,
            camera_pos,
            &system_vars.matrices.projection,
            &render_commands.view_matrix,
        );
        let left_bottom = InputConsumerSystem::project_screen_pos_to_world_pos(
            0,
            VIDEO_HEIGHT as u16,
            camera_pos,
            &system_vars.matrices.projection,
            &render_commands.view_matrix,
        );
        let right_bottom = InputConsumerSystem::project_screen_pos_to_world_pos(
            VIDEO_WIDTH as u16,
            VIDEO_HEIGHT as u16,
            camera_pos,
            &system_vars.matrices.projection,
            &render_commands.view_matrix,
        );

        let h = right_bottom.y - right_top.y;
        let letf_bottom_y = (system_vars.map_render_data.gnd.height * 2) as f32 + left_bottom.y;
        render_commands
            .rectangle_2d()
            .screen_pos(
                minimap_x + (left_bottom.x * real_to_map_scale_w) as i32,
                minimap_y + ((letf_bottom_y - h) * real_to_map_scale_h) as i32,
            )
            .size(
                ((right_bottom.x - left_bottom.x) * real_to_map_scale_w) as u16,
                (h * real_to_map_scale_h) as u16,
            )
            .color(&[0, 0, 255, 75])
            .layer(UiLayer2d::MinimapVisibleRegionRectangle)
            .add()
    }

    fn draw_targeting_skill_name(
        char_state: &CharacterStateComponent,
        input: &HumanInputComponent,
        controller: &ControllerComponent,
        render_commands: &mut RenderCommandCollector,
        system_vars: &SystemVariables,
    ) {
        if let Some((_skill_key, skill)) = controller.select_skill_target {
            let texture = &system_vars.assets.texts.skill_name_texts[&skill];
            let not_castable = char_state
                .skill_cast_allowed_at
                .get(&skill)
                .unwrap_or(&ElapsedTime(0.0))
                .has_not_passed_yet(system_vars.time);
            render_commands
                .sprite_2d()
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
                .layer(UiLayer2d::SelectingTargetSkillName)
                .add(texture);
        }
    }

    const SINGLE_MAIN_ICON_SIZE: i32 = 48;

    fn draw_secondary_skill_bar(
        char_state: &CharacterStateComponent,
        input: &HumanInputComponent,
        controller: &ControllerComponent,
        render_commands: &mut RenderCommandCollector,
        system_vars: &SystemVariables,
        main_skill_bar_top: i32,
    ) {
        let single_icon_size = 32;
        let inner_border = 2;
        let outer_border = 3;
        let space = 2;

        let main_keys = [SkillKey::Num1, SkillKey::Num2, SkillKey::Num3];
        let count = main_keys.len() as i32;
        let skill_bar_width = (outer_border * 2)
            + count * single_icon_size
            + inner_border * count * 2
            + (count - 1) * space;
        let start_x = VIDEO_WIDTH as i32 / 2 - skill_bar_width / 2;
        let y = main_skill_bar_top - single_icon_size - inner_border * 2 - outer_border * 2;

        let mut x = start_x + outer_border;
        for skill_key in main_keys.iter() {
            if let Some(skill) = input.get_skill_for_key(*skill_key) {
                // inner border
                let not_castable = char_state
                    .skill_cast_allowed_at
                    .get(&skill)
                    .unwrap_or(&ElapsedTime(0.0))
                    .has_not_passed_yet(system_vars.time);
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
                    .rectangle_2d()
                    .screen_pos(x, y + outer_border)
                    .size(
                        (single_icon_size + inner_border * 2) as u16,
                        (single_icon_size + inner_border * 2) as u16,
                    )
                    .color(&border_color)
                    .layer(UiLayer2d::SkillBar)
                    .add();

                x += inner_border;
                let icon_y = y + outer_border + inner_border;
                // blueish background
                render_commands
                    .rectangle_2d()
                    .screen_pos(x, icon_y)
                    .size(single_icon_size as u16, single_icon_size as u16)
                    .color(
                        &(if not_castable {
                            [233, 76, 76, 255] // red if not castable
                        } else {
                            [28, 64, 122, 255]
                        }),
                    )
                    .layer(UiLayer2d::SkillBar)
                    .add();

                render_commands
                    .sprite_2d()
                    .screen_pos(x, icon_y)
                    .scale(single_icon_size as f32 / RenderUI::SINGLE_MAIN_ICON_SIZE as f32 * 2.0)
                    .layer(UiLayer2d::SkillBarIcon)
                    .add(&system_vars.assets.skill_icons[&skill]);

                let skill_key_texture = &system_vars.assets.texts.skill_key_texts[&skill_key];
                let center_x = -2 + x + single_icon_size - skill_key_texture.width;
                let center_y = -2 + icon_y + single_icon_size - skill_key_texture.height;
                render_commands
                    .sprite_2d()
                    .screen_pos(center_x, center_y)
                    .color_rgb(if not_castable {
                        &[239, 76, 76]
                    } else {
                        &[255, 255, 255]
                    })
                    .scale(single_icon_size as f32 / RenderUI::SINGLE_MAIN_ICON_SIZE as f32)
                    .layer(UiLayer2d::SkillBarKey)
                    .add(skill_key_texture);

                if input.mouse_pos().x > x as u16
                    && input.mouse_pos().x < (x + single_icon_size) as u16
                {
                    if input.mouse_pos().y > y as u16
                        && input.mouse_pos().y < (y + single_icon_size) as u16
                    {
                        let texture = &system_vars.assets.texts.skill_name_texts[&skill];
                        render_commands
                            .sprite_2d()
                            .color(&[255, 255, 255, 255])
                            .screen_pos(input.last_mouse_x as i32, input.last_mouse_y as i32)
                            .layer(UiLayer2d::HoveringSkillBarName)
                            .add(texture);
                    }
                }

                x += single_icon_size + inner_border + space;
            }
        }
    }

    fn draw_main_skill_bar(
        char_state: &CharacterStateComponent,
        input: &HumanInputComponent,
        controller: &ControllerComponent,
        render_commands: &mut RenderCommandCollector,
        system_vars: &SystemVariables,
    ) -> i32 {
        let single_icon_size = RenderUI::SINGLE_MAIN_ICON_SIZE;
        let inner_border = 3;
        let outer_border = 6;
        let space = 4;

        let main_keys = [
            SkillKey::Q,
            SkillKey::W,
            SkillKey::E,
            SkillKey::R,
            SkillKey::D,
        ];
        let count = main_keys.len() as i32;
        let skill_bar_width = (outer_border * 2)
            + count * single_icon_size
            + inner_border * count * 2
            + (count - 1) * space;
        let start_x = VIDEO_WIDTH as i32 / 2 - skill_bar_width / 2;
        let y = VIDEO_HEIGHT as i32 - single_icon_size - 20 - outer_border * 2 - inner_border * 2;

        let mut x = start_x + outer_border;
        for skill_key in main_keys.iter() {
            if let Some(skill) = input.get_skill_for_key(*skill_key) {
                // inner border
                let not_castable = char_state
                    .skill_cast_allowed_at
                    .get(&skill)
                    .unwrap_or(&ElapsedTime(0.0))
                    .has_not_passed_yet(system_vars.time);
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
                    .rectangle_2d()
                    .screen_pos(x, y + outer_border)
                    .size(
                        (single_icon_size + inner_border * 2) as u16,
                        (single_icon_size + inner_border * 2) as u16,
                    )
                    .color(&border_color)
                    .layer(UiLayer2d::SkillBar)
                    .add();

                x += inner_border;
                let icon_y = y + outer_border + inner_border;
                // blueish background
                render_commands
                    .rectangle_2d()
                    .screen_pos(x, icon_y)
                    .size(single_icon_size as u16, single_icon_size as u16)
                    .color(
                        &(if not_castable {
                            [179, 179, 179, 255] // grey if not castable
                        } else {
                            [28, 64, 122, 255]
                        }),
                    )
                    .layer(UiLayer2d::SkillBar)
                    .add();

                render_commands
                    .sprite_2d()
                    .screen_pos(x, icon_y)
                    .scale(2.0)
                    .layer(UiLayer2d::SkillBarIcon)
                    .add(&system_vars.assets.skill_icons[&skill]);

                let skill_key_texture = &system_vars.assets.texts.skill_key_texts[&skill_key];
                let center_x = -2 + x + single_icon_size - skill_key_texture.width;
                let center_y = -2 + icon_y + single_icon_size - skill_key_texture.height;
                render_commands
                    .sprite_2d()
                    .screen_pos(center_x, center_y)
                    .layer(UiLayer2d::SkillBarKey)
                    .color_rgb(if not_castable {
                        &[239, 76, 76]
                    } else {
                        &[255, 255, 255]
                    })
                    .add(skill_key_texture);

                if input.mouse_pos().x > x as u16
                    && input.mouse_pos().x < (x + single_icon_size) as u16
                {
                    if input.mouse_pos().y > y as u16
                        && input.mouse_pos().y < (y + single_icon_size) as u16
                    {
                        let texture = &system_vars.assets.texts.skill_name_texts[&skill];
                        render_commands
                            .sprite_2d()
                            .color(&[255, 255, 255, 255])
                            .screen_pos(input.last_mouse_x as i32, input.last_mouse_y as i32)
                            .layer(UiLayer2d::HoveringSkillBarName)
                            .add(texture);
                    }
                }

                x += single_icon_size + inner_border + space;
            }
        }
        return y;
    }
}

fn render_action_2d(
    system_vars: &SystemVariables,
    animated_sprite: &SpriteRenderDescriptorComponent,
    sprite_res: &SpriteResource,
    pos: &Vector2<i32>,
    color: &[u8; 3],
    render_commands: &mut RenderCommandCollector,
    layer: UiLayer2d,
    scale: f32,
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

        let offset = [layer.pos[0], layer.pos[1]];
        let offset = [
            (offset[0] - (texture.width / 2)) as i16,
            (offset[1] - (texture.height / 2)) as i16,
        ];

        render_commands
            .sprite_2d()
            .screen_pos(pos.x, pos.y)
            .scale(scale)
            .color_rgb(color)
            .flip_vertically(layer.is_mirror)
            .layer(UiLayer2d::Cursor)
            .offset(offset[0], offset[1])
            .add(texture);
    }
}
