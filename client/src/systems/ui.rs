use crate::components::char::{
    CharacterStateComponent, NpcComponent, SpriteRenderDescriptorComponent,
};
use crate::components::controller::{HumanInputComponent, LocalPlayerController, SkillKey};
use crate::grf::database::AssetDatabase;
use crate::render::render_command::{RenderCommandCollector, UiLayer2d};
use crate::runtime_assets::graphic::FONT_SIZE_SKILL_KEY;
use crate::runtime_assets::map::MapRenderData;
use crate::systems::{AssetResources, SystemVariables};
use crate::{ElapsedTime, SpriteResource};
use rustarok_common::common::{EngineTime, Vec2i, Vec3};
use rustarok_common::components::char::AuthorizedCharStateComponent;
use specs::prelude::*;
use specs::ReadStorage;

pub struct RenderUI {}

impl RenderUI {
    pub fn new() -> RenderUI {
        RenderUI {}
    }

    pub fn run(
        &mut self,
        self_auth_state: &AuthorizedCharStateComponent,
        input: &HumanInputComponent,
        local_player: &LocalPlayerController,
        render_commands: &mut RenderCommandCollector,
        sys_vars: &SystemVariables,
        time: &EngineTime,
        char_state_storage: &ReadStorage<AuthorizedCharStateComponent>,
        npc_storage: &ReadStorage<NpcComponent>,
        entities: &Entities,
        camera_pos: &Vec3,
        asset_db: &AssetDatabase,
        map_render_data: &MapRenderData,
    ) {
        // Draw casting bar
        // TODO2
        //        match self_char_state.state() {
        //            ClientCharState::CastingSkill(casting_info) => {
        //                // for really short skills, don't render it
        //                if casting_info
        //                    .cast_ends
        //                    .minus(casting_info.cast_started)
        //                    .as_f32()
        //                    > 0.1
        //                {
        //                    let mut draw_rect = |x: i32, y: i32, w: i32, h: i32, color: &[u8; 4]| {
        //                        let bar_w = 540;
        //                        let bar_x = ((sys_vars.matrices.resolution_w / 2) - (bar_w / 2) - 2) as i32;
        //                        render_commands
        //                            .rectangle_2d()
        //                            .screen_pos(bar_x + x, sys_vars.matrices.resolution_h as i32 - 200 + y)
        //                            .size(w as u16, h as u16)
        //                            .color(&color)
        //                            .layer(UiLayer2d::SelfCastingBar)
        //                            .add()
        //                    };
        //                    draw_rect(0, 0, 540, 30, &[36, 92, 201, 77]); // transparent blue background
        //                    draw_rect(2, 2, 536, 26, &[0, 0, 0, 255]); // black background
        //                    let percentage = time
        //                        .now()
        //                        .percentage_between(casting_info.cast_started, casting_info.cast_ends);
        //                    draw_rect(3, 3, (percentage * 543.0) as i32, 24, &[36, 92, 201, 255]);
        //                    // inner fill
        //                }
        //            }
        //            _ => {}
        //        }

        let main_skill_bar_top = RenderUI::draw_main_skill_bar(
            self_auth_state,
            input,
            local_player,
            render_commands,
            &sys_vars,
            time,
        );

        RenderUI::draw_secondary_skill_bar(
            self_auth_state,
            input,
            local_player,
            render_commands,
            &sys_vars,
            time,
            main_skill_bar_top,
        );

        // render targeting skill name
        RenderUI::draw_targeting_skill_name(
            self_auth_state,
            input,
            local_player,
            render_commands,
            &sys_vars.assets,
            &time,
        );

        //        self.draw_minimap(
        //            self_char_state,
        //            render_commands,
        //            &sys_vars.matrices,
        //            &sys_vars.assets,
        //            char_state_storage,
        //            npc_storage,
        //            entities,
        //            camera_pos,
        //            asset_db,
        //            map_render_data,
        //        );

        render_action_2d(
            time,
            &local_player.cursor_anim_descr,
            &sys_vars.assets.sprites.cursors,
            &Vec2i::new(input.last_mouse_x as i16, input.last_mouse_y as i16),
            &local_player.cursor_color,
            render_commands,
            UiLayer2d::Cursor,
            1.0,
            asset_db,
        )
    }

    //    fn draw_minimap(
    //        &mut self,
    //        self_char_state: &CharacterStateComponent,
    //        render_commands: &mut RenderCommandCollector,
    //        matrices: &RenderMatrices,
    //        assets: &AssetResources,
    //        char_state_storage: &ReadStorage<CharacterStateComponent>,
    //        npc_storage: &ReadStorage<NpcComponent>,
    //        entities: &Entities,
    //        camera_pos: &Vec3,
    //        asset_db: &AssetDatabase,
    //        map_render_data: &MapRenderData,
    //    ) {
    //        // prontera minimaps has empty spaces: left 52, right 45 pixels
    //        // 6 pixel padding on left and bottom, 7 on top and right
    //        let minimap_texture = asset_db.get_texture(map_render_data.minimap_texture_id);
    //        let scale = (matrices.resolution_h as i32 / 4).min(minimap_texture.height) as f32
    //            / minimap_texture.height as f32;
    //        let all_minimap_w = (minimap_texture.width as f32 * scale) as i32;
    //        let minimap_render_x = matrices.resolution_w as i32 - all_minimap_w - 20;
    //        let offset_x = ((52.0 + 6.0) * scale) as i32;
    //        let minimap_x = minimap_render_x + offset_x;
    //        let minimap_h = (minimap_texture.height as f32 * scale) as i32;
    //        let minimap_y = matrices.resolution_h as i32 - minimap_h - 20;
    //        render_commands
    //            .sprite_2d()
    //            .scale(scale)
    //            .screen_pos(minimap_render_x, minimap_y)
    //            .layer(UiLayer2d::Minimap)
    //            .add(map_render_data.minimap_texture_id);
    //        let minimap_w = (minimap_texture.width as f32 * scale) as i32
    //            - ((52.0 + 45.0 + 6.0 + 7.0) * scale) as i32;
    //        let real_to_map_scale_w = minimap_w as f32 / (map_render_data.ground_width * 2) as f32;
    //        let real_to_map_scale_h = minimap_h as f32 / (map_render_data.ground_height * 2) as f32;
    //        for (entity_id, char_state) in (entities, char_state_storage).join() {
    //            let entity_id = CharEntityId::from(entity_id);
    //            let head_index = if npc_storage.get(entity_id.into()).is_none() {
    //                if let CharOutlook::Player {
    //                    head_index, sex, ..
    //                } = char_state.outlook
    //                {
    //                    Some((sex, head_index))
    //                } else {
    //                    None
    //                }
    //            } else {
    //                None
    //            };
    //            let color = if self_char_state.team.is_ally_to(char_state.team) {
    //                &[0, 0, 255, 255]
    //            } else {
    //                &[255, 0, 0, 255]
    //            };
    //
    //            let (char_x, char_y) = {
    //                let char_pos = char_state.pos();
    //                let char_y = (map_render_data.ground_height * 2) as f32 + char_pos.y;
    //                (char_pos.x, char_y)
    //            };
    //
    //            if let Some((sex, head_index)) = head_index {
    //                let head_texture_id = {
    //                    let sprites = &assets.sprites.head_sprites;
    //                    sprites[sex as usize][head_index].textures[0]
    //                };
    //
    //                let head_texture = asset_db.get_texture(head_texture_id);
    //                let center_offset_x = (head_texture.width as f32 * 0.7 / 2.0) as i32;
    //                let center_offset_y = (head_texture.height as f32 * 0.7 / 2.0) as i32;
    //                render_commands
    //                    .sprite_2d()
    //                    .screen_pos(
    //                        minimap_x + (char_x * real_to_map_scale_w) as i32 - center_offset_x,
    //                        minimap_y + (char_y * real_to_map_scale_h) as i32 - center_offset_y,
    //                    )
    //                    .color(color)
    //                    .scale(0.7)
    //                    .layer(UiLayer2d::MinimapImportantEntities)
    //                    .add(head_texture_id);
    //
    //                let center_offset_x = (head_texture.width as f32 * 0.5 / 2.0) as i32;
    //                let center_offset_y = (head_texture.height as f32 * 0.5 / 2.0) as i32;
    //                render_commands
    //                    .sprite_2d()
    //                    .screen_pos(
    //                        minimap_x + (char_x * real_to_map_scale_w) as i32 - center_offset_x,
    //                        minimap_y + (char_y * real_to_map_scale_h) as i32 - center_offset_y,
    //                    )
    //                    .scale(0.5)
    //                    .layer(UiLayer2d::MinimapImportantEntities)
    //                    .add(head_texture_id);
    //            } else {
    //                render_commands
    //                    .point_2d()
    //                    .screen_pos(
    //                        minimap_x + (char_x * real_to_map_scale_w) as i32,
    //                        minimap_y + (char_y * real_to_map_scale_h) as i32,
    //                    )
    //                    .color(color)
    //                    .layer(UiLayer2d::MinimapSimpleEntities)
    //                    .add();
    //            }
    //        }
    //
    //        // draw camera rectangle
    //        let right_top = InputConsumerSystem::project_screen_pos_to_world_pos(
    //            matrices.resolution_w as u16,
    //            0,
    //            camera_pos,
    //            &matrices.projection,
    //            &render_commands.view_matrix,
    //            matrices.resolution_w,
    //            matrices.resolution_h,
    //        );
    //        let left_bottom = InputConsumerSystem::project_screen_pos_to_world_pos(
    //            0,
    //            matrices.resolution_h as u16,
    //            camera_pos,
    //            &matrices.projection,
    //            &render_commands.view_matrix,
    //            matrices.resolution_w,
    //            matrices.resolution_h,
    //        );
    //        let right_bottom = InputConsumerSystem::project_screen_pos_to_world_pos(
    //            matrices.resolution_w as u16,
    //            matrices.resolution_h as u16,
    //            camera_pos,
    //            &matrices.projection,
    //            &render_commands.view_matrix,
    //            matrices.resolution_w,
    //            matrices.resolution_h,
    //        );
    //
    //        let h = right_bottom.y - right_top.y;
    //        let letf_bottom_y = (map_render_data.ground_height * 2) as f32 + left_bottom.y;
    //        render_commands
    //            .rectangle_2d()
    //            .screen_pos(
    //                minimap_x + (left_bottom.x * real_to_map_scale_w) as i32,
    //                minimap_y + ((letf_bottom_y - h) * real_to_map_scale_h) as i32,
    //            )
    //            .size(
    //                ((right_bottom.x - left_bottom.x) * real_to_map_scale_w) as u16,
    //                (h * real_to_map_scale_h) as u16,
    //            )
    //            .color(&[0, 0, 255, 75])
    //            .layer(UiLayer2d::MinimapVisibleRegionRectangle)
    //            .add()
    //    }

    fn draw_targeting_skill_name(
        char_state: &AuthorizedCharStateComponent,
        input: &HumanInputComponent,
        controller: &LocalPlayerController,
        render_commands: &mut RenderCommandCollector,
        assets: &AssetResources,
        time: &EngineTime,
    ) {
        if let Some((skill_key, skill)) = controller.select_skill_target {
            let texture = assets.texts.skill_name_texts[&skill];
            let not_castable =
                char_state.skill_cast_allowed_at[skill_key as usize].has_not_passed_yet(time.now());
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
                    // TODO: store the skill name w in the vec above
                    //                    input.last_mouse_x as i32 - texture.width / 2,
                    input.last_mouse_x as i32 - 30 / 2,
                    input.last_mouse_y as i32 + 32,
                )
                .layer(UiLayer2d::SelectingTargetSkillName)
                .add(texture);
        }
    }

    const SINGLE_MAIN_ICON_SIZE: i32 = 48;

    fn draw_secondary_skill_bar(
        char_state: &AuthorizedCharStateComponent,
        input: &HumanInputComponent,
        controller: &LocalPlayerController,
        render_commands: &mut RenderCommandCollector,
        sys_vars: &SystemVariables,
        time: &EngineTime,
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
        let start_x = sys_vars.matrices.resolution_w as i32 / 2 - skill_bar_width / 2;
        let y = main_skill_bar_top - single_icon_size - inner_border * 2 - outer_border * 2;

        let mut x = start_x + outer_border;
        for skill_key in main_keys.iter() {
            if let Some(skill) = input.get_skill_for_key(*skill_key) {
                // inner border
                let not_castable = char_state.skill_cast_allowed_at[*skill_key as usize]
                    .has_not_passed_yet(time.now());
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
                    .add(sys_vars.assets.skill_icons[&skill]);

                let skill_key_texture_id = sys_vars.assets.texts.skill_key_texts[&skill_key];
                let center_x = -2 + x + single_icon_size - FONT_SIZE_SKILL_KEY;
                let center_y = -2 + icon_y + single_icon_size - FONT_SIZE_SKILL_KEY;
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
                    .add(skill_key_texture_id);

                if input.mouse_pos().x > x as u16
                    && input.mouse_pos().x < (x + single_icon_size) as u16
                {
                    if input.mouse_pos().y > y as u16
                        && input.mouse_pos().y < (y + single_icon_size) as u16
                    {
                        let texture = sys_vars.assets.texts.skill_name_texts[&skill];
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
        char_state: &AuthorizedCharStateComponent,
        input: &HumanInputComponent,
        controller: &LocalPlayerController,
        render_commands: &mut RenderCommandCollector,
        sys_vars: &SystemVariables,
        time: &EngineTime,
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
        let start_x = sys_vars.matrices.resolution_w as i32 / 2 - skill_bar_width / 2;
        let y = sys_vars.matrices.resolution_h as i32
            - single_icon_size
            - 20
            - outer_border * 2
            - inner_border * 2;

        let mut x = start_x + outer_border;
        for skill_key in main_keys.iter() {
            if let Some(skill) = input.get_skill_for_key(*skill_key) {
                // inner border
                let not_castable = char_state.skill_cast_allowed_at[*skill_key as usize]
                    .has_not_passed_yet(time.now());
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
                    .add(sys_vars.assets.skill_icons[&skill]);

                let skill_key_texture = sys_vars.assets.texts.skill_key_texts[&skill_key];
                let center_x = -2 + x + single_icon_size - FONT_SIZE_SKILL_KEY;
                let center_y = -2 + icon_y + single_icon_size - FONT_SIZE_SKILL_KEY;
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
                        let texture = sys_vars.assets.texts.skill_name_texts[&skill];
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
    time: &EngineTime,
    animated_sprite: &SpriteRenderDescriptorComponent,
    sprite_res: &SpriteResource,
    pos: &Vec2i,
    color: &[u8; 3],
    render_commands: &mut RenderCommandCollector,
    ui_layer: UiLayer2d,
    scale: f32,
    asset_db: &AssetDatabase,
) {
    let idx = animated_sprite.action_index;
    let action = &sprite_res.action.actions[idx];

    let frame_index = {
        let frame_count = action.frames.len();
        let time_needed_for_one_frame = action.delay as f32 / 1000.0 * 4.0;
        let elapsed_time = time.now().elapsed_since(animated_sprite.animation_started);
        (elapsed_time.div(time_needed_for_one_frame)) as usize % frame_count
    };
    let animation = &action.frames[frame_index];
    for layer in &animation.layers {
        if layer.sprite_frame_index < 0 {
            continue;
        }
        let texture_id = sprite_res.textures[layer.sprite_frame_index as usize];

        let offset = {
            let texture = asset_db.get_texture(texture_id);
            let offset = [layer.pos[0], layer.pos[1]];
            [
                (offset[0] - (texture.width / 2)) as i16,
                (offset[1] - (texture.height / 2)) as i16,
            ]
        };

        render_commands
            .sprite_2d()
            .screen_pos(pos.x as i32, pos.y as i32)
            .scale(scale)
            .color_rgb(color)
            .flip_vertically(layer.is_mirror)
            .layer(ui_layer)
            .offset(offset[0], offset[1])
            .add(texture_id);
    }
}
