use crate::asset::database::AssetDatabase;
use crate::cam::Camera;
use crate::common::Vec2;
use crate::components::char::{
    ActionPlayMode, CharOutlook, CharState, CharType, CharacterStateComponent, EntityTarget,
    NpcComponent, SpriteBoundingRect, SpriteRenderDescriptorComponent, Team,
};
use crate::components::controller::{
    CameraComponent, CharEntityId, ControllerComponent, ControllerEntityId, EntitiesBelowCursor,
    HumanInputComponent, PlayerIntention, SkillKey,
};
use crate::components::skills::skills::{SkillManifestationComponent, SkillTargetType, Skills};
use crate::components::{
    BrowserClient, FlyingNumberComponent, FlyingNumberType, SoundEffectComponent,
    StrEffectComponent,
};
use crate::configs::DevConfig;
use crate::cursor::CURSOR_TARGET;
use crate::effect::StrEffectId;
use crate::runtime_assets::map::{MapRenderData, PhysicEngine};
use crate::systems::render::render_command::{RenderCommandCollector, UiLayer2d};
use crate::systems::sound_sys::AudioCommandCollectorComponent;
use crate::systems::ui::RenderUI;
use crate::systems::{AssetResources, SystemFrameDurations, SystemVariables};
use crate::{ElapsedTime, SpriteResource};
use nalgebra::{Isometry2, Vector2, Vector3};
use specs::prelude::*;

pub const COLOR_WHITE: [u8; 4] = [255, 255, 255, 255];
/// The values that should be added to the sprite direction based on the camera
/// direction (the index is the camera direction, which is floor(angle/45)
pub const DIRECTION_TABLE: [usize; 8] = [6, 5, 4, 3, 2, 1, 0, 7];

// todo: Move it into GPU?
pub const ONE_SPRITE_PIXEL_SIZE_IN_3D: f32 = 1.0 / 35.0;

pub struct RenderDesktopClientSystem {
    damage_render_sys: DamageRenderSystem,
    render_ui_sys: RenderUI,
}

impl RenderDesktopClientSystem {
    pub fn new() -> RenderDesktopClientSystem {
        RenderDesktopClientSystem {
            damage_render_sys: DamageRenderSystem::new(),
            render_ui_sys: RenderUI::new(),
        }
    }

    // TODO: wtf is this argument list
    fn render_for_controller<'a>(
        &self,
        controller: &mut Option<ControllerAndControlled>, // mut: we have to store bounding rects of drawed entities :(
        camera: &CameraComponent,
        input: &HumanInputComponent,
        render_commands: &mut RenderCommandCollector,
        audio_commands: &mut AudioCommandCollectorComponent,
        physics_world: &ReadExpect<'a, PhysicEngine>,
        sys_vars: &SystemVariables,
        dev_configs: &DevConfig,
        char_state_storage: &ReadStorage<'a, CharacterStateComponent>,
        entities: &Entities<'a>,
        sprite_storage: &ReadStorage<'a, SpriteRenderDescriptorComponent>,
        skill_storage: &ReadStorage<'a, SkillManifestationComponent>, // TODO remove me
        str_effect_storage: &ReadStorage<'a, StrEffectComponent>,
        updater: &Write<'a, LazyUpdate>,
        system_benchmark: &mut SystemFrameDurations,
        asset_db: &AssetDatabase,
        render_only_chars: bool,
        map_render_data: &MapRenderData,
    ) {
        render_commands.set_view_matrix(&camera.view_matrix, &camera.normal_matrix, camera.yaw);
        {
            let _stopwatch = system_benchmark.start_measurement("render.draw_characters");
            self.draw_characters(
                &camera,
                controller,
                render_commands,
                sys_vars,
                dev_configs,
                char_state_storage,
                entities,
                sprite_storage,
                asset_db,
                //                &map_render_data.gat,
            );
        }

        if render_only_chars {
            return;
        }

        {
            let _stopwatch = system_benchmark.start_measurement("render.draw_physics_coll");
            // Draw physics colliders
            for physics in (&char_state_storage).join() {
                if let Some(collider) = physics_world.colliders.get(physics.collider_handle) {
                    if collider.shape().is_shape::<ncollide2d::shape::Ball<f32>>() {
                        let radius = collider
                            .shape()
                            .bounding_sphere(&Isometry2::new(Vector2::zeros(), 0.0))
                            .radius();
                        let pos = physics_world
                            .bodies
                            .rigid_body(physics.body_handle)
                            .unwrap()
                            .position()
                            .translation
                            .vector;
                        render_commands
                            .circle_3d()
                            .radius(radius)
                            .color(&[255, 0, 255, 255])
                            .pos_2d(&pos)
                            .y(0.05)
                            .add();
                    } else {
                        let extents = collider
                            .shape()
                            .aabb(&Isometry2::new(Vector2::zeros(), 0.0))
                            .extents();
                        let pos = physics_world
                            .bodies
                            .rigid_body(physics.body_handle)
                            .unwrap()
                            .position()
                            .translation
                            .vector;
                        render_commands
                            .rectangle_3d()
                            .pos_2d(&pos)
                            .color(&[255, 0, 255, 255])
                            .y(0.05)
                            .size(extents.x, extents.y)
                            .add();
                    }
                };
            }
        }

        {
            let _stopwatch = system_benchmark.start_measurement("render.models");
            render_models(
                controller
                    .as_ref()
                    .map(|it| it.controlled_char.pos())
                    .as_ref(),
                &camera.camera,
                &map_render_data,
                asset_db,
                render_commands,
            );
        }

        {
            if let Some(controller) = &controller {
                {
                    let _stopwatch =
                        system_benchmark.start_measurement("render.select_skill_target");
                    let char_pos = controller.controlled_char.pos();
                    if let Some((_skill_key, skill)) = controller.controller.select_skill_target {
                        let skill_def = skill.get_definition();
                        let skill_cast_attr =
                            skill.get_cast_attributes(&dev_configs, controller.controlled_char);
                        let (skill_3d_pos, dir_vector) = Skills::limit_vector_into_range(
                            &char_pos,
                            &input.mouse_world_pos,
                            skill_cast_attr.casting_range,
                        );
                        if skill_def.get_skill_target_type() != SkillTargetType::Directional {
                            render_commands
                                .circle_3d()
                                .pos_2d(&char_pos)
                                .y(0.0)
                                .radius(skill_cast_attr.casting_range)
                                .color(&[0, 255, 0, 255])
                                .add();
                            if skill_def.get_skill_target_type() == SkillTargetType::Area {
                                let is_castable = controller
                                    .controlled_char
                                    .skill_cast_allowed_at
                                    .get(&skill)
                                    .unwrap_or(&ElapsedTime(0.0))
                                    .has_already_passed(sys_vars.time);
                                skill_def.render_target_selection(
                                    is_castable,
                                    &skill_3d_pos,
                                    &dir_vector,
                                    render_commands,
                                    &dev_configs,
                                );
                            }
                        } else {
                            let center =
                                char_pos + dir_vector * (skill_cast_attr.casting_range / 2.0);
                            let angle = dir_vector.angle(&Vector2::y());
                            let angle = if dir_vector.x > 0.0 { angle } else { -angle };
                            render_commands
                                .rectangle_3d()
                                .pos_2d(&center)
                                .rotation_rad(angle)
                                .color(&[0, 255, 0, 255])
                                .size(
                                    skill_cast_attr.width.unwrap_or(1.0),
                                    skill_cast_attr.casting_range,
                                )
                                .add();
                        }
                    }
                }
                {
                    // render target position
                    // if there is a valid controller, there is char_state as well
                    if let Some(PlayerIntention::MoveTo(pos)) = controller.controller.last_action {
                        if CharState::Idle != *controller.controlled_char.state() {
                            let cursor_anim_descr = SpriteRenderDescriptorComponent {
                                action_index: CURSOR_TARGET.1,
                                animation_started: ElapsedTime(0.0),
                                animation_ends_at: ElapsedTime(0.0),
                                forced_duration: None,
                                direction: 0,
                                fps_multiplier: 2.0,
                            };
                            render_action(
                                sys_vars.time,
                                &cursor_anim_descr,
                                &sys_vars.assets.sprites.cursors,
                                &pos,
                                [0, 0],
                                false,
                                1.0,
                                ActionPlayMode::Repeat,
                                &COLOR_WHITE,
                                render_commands,
                            );
                        }
                    }
                }
            }
        }

        for skill in (&skill_storage).join() {
            skill.render(
                sys_vars.time,
                sys_vars.tick,
                &sys_vars.assets,
                render_commands,
                audio_commands,
            );
        }

        // TODO: into a separate system
        {
            let _stopwatch = system_benchmark.start_measurement("render.str_effect");
            for (entity_id, str_effect) in (entities, str_effect_storage).join() {
                if str_effect
                    .die_at
                    .map(|it| it.has_already_passed(sys_vars.time))
                    .unwrap_or(false)
                {
                    updater.remove::<StrEffectComponent>(entity_id);
                } else {
                    let remove = RenderDesktopClientSystem::render_str(
                        str_effect.effect_id,
                        str_effect.start_time,
                        &str_effect.pos,
                        sys_vars,
                        render_commands,
                        str_effect.play_mode,
                    );
                    if remove {
                        updater.remove::<StrEffectComponent>(entity_id);
                    }
                }
            }
        }
    }

    fn need_entity_highlighting(
        followed_char_id: CharEntityId,
        select_skill_target: Option<(SkillKey, Skills)>,
        rendering_entity_id: CharEntityId,
        entities_below_cursor: &EntitiesBelowCursor,
        desktop_target: &Option<EntityTarget>,
    ) -> bool {
        return if let Some((_skill_key, skill)) = select_skill_target {
            match skill.get_definition().get_skill_target_type() {
                SkillTargetType::AnyEntity => entities_below_cursor
                    .get_enemy_or_friend()
                    .map(|it| it == rendering_entity_id)
                    .unwrap_or(false),
                SkillTargetType::NoTarget => false,
                SkillTargetType::Area => false,
                SkillTargetType::Directional => false,
                SkillTargetType::OnlyAllyButNoSelf => entities_below_cursor
                    .get_friend_except(followed_char_id)
                    .map(|it| it == rendering_entity_id)
                    .unwrap_or(false),
                SkillTargetType::OnlyAllyAndSelf => entities_below_cursor
                    .get_friend()
                    .map(|it| it == rendering_entity_id)
                    .unwrap_or(false),
                SkillTargetType::OnlyEnemy => entities_below_cursor
                    .get_enemy()
                    .map(|it| it == rendering_entity_id)
                    .unwrap_or(false),
            }
        } else {
            let ret = entities_below_cursor
                .get_enemy_or_friend()
                .map(|it| it == rendering_entity_id)
                .unwrap_or(false);
            ret || match desktop_target {
                Some(EntityTarget::OtherEntity(target_entity_id)) => {
                    rendering_entity_id == *target_entity_id
                }
                _ => false,
            }
        };
    }

    fn draw_characters(
        &self,
        camera: &CameraComponent,
        controller: &mut Option<ControllerAndControlled>,
        render_commands: &mut RenderCommandCollector,
        sys_vars: &SystemVariables,
        dev_configs: &DevConfig,
        char_state_storage: &ReadStorage<CharacterStateComponent>,
        entities: &Entities,
        sprite_storage: &ReadStorage<SpriteRenderDescriptorComponent>,
        asset_db: &AssetDatabase,
        //        gat: &Gat,
    ) {
        // Draw players
        for (rendering_entity_id, animated_sprite, char_state) in
            (entities, sprite_storage, char_state_storage).join()
        {
            let rendering_entity_id = CharEntityId(rendering_entity_id);
            // for autocompletion
            let char_state: &CharacterStateComponent = char_state;

            let pos_2d = char_state.pos();
            if !camera.camera.is_visible(pos_2d) {
                continue;
            }

            // gat height calculation
            //            let w = gat.width as usize;
            //            let x = pos_2d.x;
            //            let y = -pos_2d.y;
            //            let index = (x.floor() as usize + y.floor() as usize * w);
            //            let x = x - x.floor();
            //            let y = y - y.floor();
            //
            //            let cell = &gat.cells[index];
            //            let x1 = cell.cells[0] + (cell.cells[1] - cell.cells[0]) * x;
            //            let x2 = cell.cells[2] + (cell.cells[3] - cell.cells[2]) * x;
            //            let h = -(x1 + (x2 - x1) * y);
            //            dbg!(h);
            //            let pos_3d = Vector3::new(pos_2d.x, h + char_state.get_y(), pos_2d.y);

            let pos_3d = Vector3::new(pos_2d.x, char_state.get_y(), pos_2d.y);

            let color = char_state.statuses.calc_render_color(sys_vars.time);
            match char_state.outlook {
                CharOutlook::Player {
                    job_sprite_id,
                    head_index,
                    sex,
                } => {
                    // for spectators, left team is red, right is blue
                    let viewer_team = controller
                        .as_ref()
                        .map(|it| it.controlled_char.team)
                        .unwrap_or(Team::Right);
                    let body_sprite = char_state
                        .statuses
                        .calc_body_sprite(sys_vars, char_state.job_id, sex)
                        .unwrap_or(
                            &sys_vars.assets.sprites.character_sprites[&job_sprite_id]
                                [viewer_team.get_palette_index(char_state.team)]
                                [sex as usize],
                        );

                    let play_mode = if char_state.state().is_dead() {
                        ActionPlayMode::PlayThenHold
                    } else {
                        ActionPlayMode::Repeat
                    };
                    let head_res = {
                        let sprites = &sys_vars.assets.sprites.head_sprites;
                        &sprites[sex as usize][head_index]
                    };

                    if let Some(controller) = &controller {
                        if RenderDesktopClientSystem::need_entity_highlighting(
                            controller.controller.controlled_entity,
                            controller.controller.select_skill_target,
                            rendering_entity_id,
                            &controller.controller.entities_below_cursor,
                            &controller.controlled_char.target,
                        ) {
                            let color =
                                if controller.controlled_char.team.is_ally_to(char_state.team) {
                                    &[0, 0, 255, 179]
                                } else {
                                    &[255, 0, 0, 179]
                                };
                            let body_pos_offset = render_single_layer_action(
                                sys_vars.time,
                                &animated_sprite,
                                body_sprite,
                                &pos_3d,
                                [0, 0],
                                true,
                                1.2,
                                play_mode,
                                color,
                                render_commands,
                            );

                            let _head_pos_offset = render_single_layer_action(
                                sys_vars.time,
                                &animated_sprite,
                                head_res,
                                &pos_3d,
                                body_pos_offset,
                                false,
                                1.2,
                                play_mode,
                                color,
                                render_commands,
                            );
                        }
                    }
                    // todo: kell a body_pos_offset még mindig? (bounding rect)
                    let body_pos_offset = render_single_layer_action(
                        sys_vars.time,
                        &animated_sprite,
                        body_sprite,
                        &pos_3d,
                        [0, 0],
                        true,
                        1.0,
                        play_mode,
                        &color,
                        render_commands,
                    );

                    let mut body_bounding_rect = {
                        let render_command = render_commands.get_last_billboard_command();
                        if let Some(render_command) = render_command {
                            render_command.project_to_screen(
                                &camera.view_matrix,
                                &sys_vars.matrices.projection,
                                &asset_db,
                            )
                        } else {
                            continue;
                        }
                    };
                    let _head_pos_offset = render_single_layer_action(
                        sys_vars.time,
                        &animated_sprite,
                        head_res,
                        &pos_3d,
                        body_pos_offset,
                        false,
                        1.0,
                        play_mode,
                        &color,
                        render_commands,
                    );
                    // TODO: heads are quite similar, use fixed pixel size for it and remove this projection?
                    {
                        let render_command = render_commands.get_last_billboard_command();
                        if let Some(render_command) = render_command {
                            let head_bounding_rect = render_command.project_to_screen(
                                &camera.view_matrix,
                                &sys_vars.matrices.projection,
                                asset_db,
                            );
                            body_bounding_rect.merge(&head_bounding_rect);
                        }
                    };

                    // TODO: create a has_hp component and draw this on them only?
                    if !char_state.state().is_dead() {
                        self.draw_health_bar(
                            controller
                                .as_ref()
                                .map(|it| it.controller.controlled_entity == rendering_entity_id)
                                .unwrap_or(false),
                            controller
                                .as_ref()
                                .map(|it| it.controlled_char.team.is_ally_to(char_state.team))
                                .unwrap_or(false),
                            &char_state,
                            sys_vars.time,
                            &body_bounding_rect,
                            &sys_vars.assets,
                            render_commands,
                        );
                    }

                    if let Some(controller) = controller {
                        controller
                            .controller
                            .bounding_rect_2d
                            .insert(rendering_entity_id, (body_bounding_rect, char_state.team));
                    }
                }
                CharOutlook::Monster(monster_id) => {
                    let body_res = {
                        let sprites = &sys_vars.assets.sprites.monster_sprites;
                        &sprites[&monster_id]
                    };
                    let play_mode = if char_state.state().is_dead() {
                        ActionPlayMode::PlayThenHold
                    } else {
                        ActionPlayMode::Repeat
                    };
                    if let Some(controller) = controller {
                        if RenderDesktopClientSystem::need_entity_highlighting(
                            controller.controller.controlled_entity,
                            controller.controller.select_skill_target,
                            rendering_entity_id,
                            &controller.controller.entities_below_cursor,
                            &controller.controlled_char.target,
                        ) {
                            let color =
                                if controller.controlled_char.team.is_ally_to(char_state.team) {
                                    &[0, 0, 255, 179]
                                } else {
                                    &[255, 0, 0, 179]
                                };
                            let _pos_offset = render_single_layer_action(
                                sys_vars.time,
                                &animated_sprite,
                                body_res,
                                &pos_3d,
                                [0, 0],
                                true,
                                1.2,
                                play_mode,
                                color,
                                render_commands,
                            );
                        }
                    }
                    let _pos_offset = render_single_layer_action(
                        sys_vars.time,
                        &animated_sprite,
                        body_res,
                        &pos_3d,
                        [0, 0],
                        true,
                        1.0,
                        play_mode,
                        &color,
                        render_commands,
                    );
                    let bounding_rect = {
                        let render_command = render_commands.get_last_billboard_command();
                        if let Some(render_command) = render_command {
                            render_command.project_to_screen(
                                &camera.view_matrix,
                                &sys_vars.matrices.projection,
                                asset_db,
                            )
                        } else {
                            continue;
                        }
                    };
                    if !char_state.state().is_dead() {
                        self.draw_health_bar(
                            controller
                                .as_ref()
                                .map(|it| it.controller.controlled_entity == rendering_entity_id)
                                .unwrap_or(false),
                            controller
                                .as_ref()
                                .map(|it| it.controlled_char.team.is_ally_to(char_state.team))
                                .unwrap_or(false),
                            &char_state,
                            sys_vars.time,
                            &bounding_rect,
                            &sys_vars.assets,
                            render_commands,
                        );
                    }

                    if let Some(controller) = controller {
                        controller
                            .controller
                            .bounding_rect_2d
                            .insert(rendering_entity_id, (bounding_rect, char_state.team));
                    }
                }
            }

            if let CharState::CastingSkill(casting_info) = char_state.state() {
                let skill = casting_info.skill;
                skill.get_definition().render_casting(
                    &char_state.pos(),
                    &casting_info,
                    sys_vars,
                    dev_configs,
                    render_commands,
                    &char_state_storage,
                );
            }

            char_state
                .statuses
                .render(&char_state, sys_vars, render_commands);
        }
    }
}

struct ControllerAndControlled<'a> {
    controller: &'a mut ControllerComponent,
    controlled_char: &'a CharacterStateComponent,
}

impl<'a> System<'a> for RenderDesktopClientSystem {
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, HumanInputComponent>,
        ReadStorage<'a, SpriteRenderDescriptorComponent>,
        ReadStorage<'a, CharacterStateComponent>,
        WriteStorage<'a, ControllerComponent>, // mut: we have to store bounding rects of drawed entities :(
        WriteExpect<'a, SystemVariables>,
        ReadExpect<'a, DevConfig>,
        WriteExpect<'a, SystemFrameDurations>,
        ReadStorage<'a, SkillManifestationComponent>, // TODO remove me
        ReadStorage<'a, StrEffectComponent>,
        ReadStorage<'a, CameraComponent>,
        ReadExpect<'a, PhysicEngine>,
        Write<'a, LazyUpdate>,
        ReadStorage<'a, FlyingNumberComponent>,
        ReadStorage<'a, SoundEffectComponent>,
        WriteStorage<'a, RenderCommandCollector>,
        WriteStorage<'a, AudioCommandCollectorComponent>,
        ReadExpect<'a, AssetDatabase>,
        ReadStorage<'a, NpcComponent>,
        ReadStorage<'a, BrowserClient>,
        ReadExpect<'a, MapRenderData>,
    );

    fn run(
        &mut self,
        (
            entities,
            input_storage,
            sprite_storage,
            char_state_storage,
            mut controller_storage,
            mut sys_vars,
            dev_configs,
            mut system_benchmark,
            skill_storage,
            str_effect_storage,
            camera_storage,
            physics_world,
            updater,
            numbers,
            sound_effects,
            mut render_commands_storage,
            mut audio_commands_storage,
            asset_db,
            npc_storage,
            browser_storage,
            map_render_data,
        ): Self::SystemData,
    ) {
        let join = {
            let _stopwatch = system_benchmark.start_measurement("RenderDesktopClientSystem.join");
            (
                &entities,
                &input_storage,
                &mut render_commands_storage,
                &mut audio_commands_storage,
                &camera_storage,
            )
                .join()
        };
        for (entity_id, mut input, mut render_commands, mut audio_commands, camera) in join {
            let controller_id = ControllerEntityId(entity_id);

            let mut controller_and_controlled: Option<ControllerAndControlled> = camera
                .followed_controller
                .map(|controller_id| controller_storage.get_mut(controller_id.0).unwrap())
                .map(|controller| {
                    let entity = controller.controlled_entity;
                    ControllerAndControlled {
                        controller,
                        controlled_char: char_state_storage.get(entity.0).unwrap(),
                    }
                });

            let render_only_chars = if let Some(browser) = browser_storage.get(controller_id.0) {
                if browser.next_send_at.has_not_passed_yet(sys_vars.time) {
                    // commands won't be sent to the client in this frame, so render only characters
                    // for getting their bounding rects
                    true
                } else {
                    false
                }
            } else {
                false
            };

            {
                self.render_for_controller(
                    &mut controller_and_controlled,
                    camera,
                    &mut input,
                    &mut render_commands,
                    &mut audio_commands,
                    &physics_world,
                    &mut sys_vars,
                    &dev_configs,
                    &char_state_storage,
                    &entities,
                    &sprite_storage,
                    &skill_storage,
                    &str_effect_storage,
                    &updater,
                    &mut system_benchmark,
                    &asset_db,
                    render_only_chars,
                    &map_render_data,
                );
            }

            for (entity_id, sound) in (&entities, &sound_effects).join() {
                updater.remove::<SoundEffectComponent>(entity_id);
                if !camera.camera.is_visible(sound.pos) {
                    continue;
                }
                audio_commands.add_sound_command(sound.sound_id);
            }

            self.damage_render_sys.run(
                &entities,
                &numbers,
                &char_state_storage,
                controller_and_controlled
                    .as_ref()
                    .map(|it| it.controller.controlled_entity)
                    .unwrap_or(
                        CharEntityId(controller_id.0), // controller_id is the controller id, so no character will match with it, ~dummy value
                    ),
                controller_and_controlled
                    .as_ref()
                    .map(|it| it.controlled_char.team),
                sys_vars.time,
                &sys_vars.assets,
                &updater,
                render_commands,
            );

            if let Some(controller_and_controlled) = controller_and_controlled.as_ref() {
                self.render_ui_sys.run(
                    &controller_and_controlled.controlled_char,
                    &input,
                    &controller_and_controlled.controller,
                    &mut render_commands,
                    &mut sys_vars,
                    &char_state_storage,
                    &npc_storage,
                    &entities,
                    &camera.camera.pos(),
                    browser_storage.get(controller_id.0).is_some(),
                    &asset_db,
                    &map_render_data,
                );
            }
        }
    }
}

pub fn render_single_layer_action<'a>(
    now: ElapsedTime,
    animation: &SpriteRenderDescriptorComponent,
    sprite_res: &SpriteResource,
    pos: &Vector3<f32>,
    pos_offset: [i32; 2],
    is_main: bool,
    size_multiplier: f32,
    play_mode: ActionPlayMode,
    color: &[u8; 4],
    render_commands: &'a mut RenderCommandCollector,
) -> [i32; 2] {
    let idx = {
        let cam_dir = (((render_commands.yaw / 45.0) + 0.5) as usize) % 8;
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
    let frame_index = {
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
        let real_index = (elapsed_time.div(time_needed_for_one_frame)) as usize;
        match play_mode {
            ActionPlayMode::Repeat | ActionPlayMode::Once => real_index % frame_count,
            ActionPlayMode::PlayThenHold => real_index.min(frame_count - 1),
            ActionPlayMode::Reverse => (frame_count - 1) - (real_index % frame_count),
            ActionPlayMode::FixFrame(frame_i) => frame_i,
        }
    };
    let frame = &action.frames[frame_index];
    // TODO collect the problematic sprites and remove the 'if' if possible
    if frame.layers.is_empty() {
        // e.g. not every sprite has death status
        return [0, 0];
    }

    let layer = &frame.layers[0];

    let offset = if !is_main {
        let positions = frame.positions.get(0).unwrap_or(&[0, 0]);
        [pos_offset[0] - positions[0], pos_offset[1] - positions[1]]
    } else {
        [0, 0]
    };
    let offset = [
        (layer.pos[0] + offset[0]) as i16,
        (layer.pos[1] + offset[1]) as i16,
    ];

    let mut color = color.clone();
    for i in 0..4 {
        color[i] = (color[i] as u32 * layer.color[i] as u32 / 255) as u8;
    }

    let sprite_texture = sprite_res.textures[layer.sprite_frame_index as usize];
    render_commands
        .sprite_3d()
        .pos(&pos)
        .scale(layer.scale[0] * size_multiplier)
        .rot_radian((-layer.angle as f32).to_radians())
        .offset(offset)
        .color(&color)
        .flip_vertically(layer.is_mirror)
        .add(sprite_texture);

    // TODO: put 0,0 manually on startup if it is empty
    let anim_pos = frame
        .positions
        .get(0)
        .map(|it| it.clone())
        .unwrap_or([0, 0]);

    return [(anim_pos[0] as f32) as i32, (anim_pos[1] as f32) as i32];
}

pub fn render_action(
    now: ElapsedTime,
    animation: &SpriteRenderDescriptorComponent,
    sprite_res: &SpriteResource,
    pos: &Vec2,
    pos_offset: [i32; 2],
    is_main: bool,
    size_multiplier: f32,
    play_mode: ActionPlayMode,
    color: &[u8; 4],
    render_commands: &mut RenderCommandCollector,
) -> [i32; 2] {
    let idx = {
        let cam_dir = (((render_commands.yaw / 45.0) + 0.5) as usize) % 8;
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
    let frame_index = {
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
        let real_index = (elapsed_time.div(time_needed_for_one_frame)) as usize;
        match play_mode {
            ActionPlayMode::Repeat | ActionPlayMode::Once => real_index % frame_count,
            ActionPlayMode::Reverse => (frame_count - 1) - (real_index % frame_count),
            ActionPlayMode::PlayThenHold => real_index.min(frame_count - 1),
            ActionPlayMode::FixFrame(frame_i) => frame_i,
        }
    };
    let frame = &action.frames[frame_index];

    for layer in frame.layers.iter() {
        if layer.sprite_frame_index < 0 {
            continue;
        }

        let offset = if !is_main {
            // TODO: check if there is any sprite whose frame.positions is not empty
            let positions = frame.positions.get(0).unwrap_or(&[0, 0]);
            [pos_offset[0] + positions[0], pos_offset[1] + positions[1]]
        } else {
            [0, 0]
        };
        let offset = [
            (layer.pos[0] + offset[0]) as i16,
            (layer.pos[1] + offset[1]) as i16,
        ];

        let mut color = color.clone();
        for i in 0..4 {
            color[i] = (color[i] as u32 * layer.color[i] as u32 / 255) as u8;
        }

        let sprite_texture = sprite_res.textures[layer.sprite_frame_index as usize];
        render_commands
            .sprite_3d()
            .pos_2d(&pos)
            .scale(layer.scale[0] * size_multiplier)
            .rot_radian((-layer.angle as f32).to_radians())
            .offset(offset)
            .color(&color)
            .flip_vertically(layer.is_mirror)
            .add(sprite_texture);
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

fn render_models(
    char_pos: Option<&Vec2>,
    camera: &Camera,
    map_render_data: &MapRenderData,
    asset_db: &AssetDatabase,
    render_commands: &mut RenderCommandCollector,
) {
    // cam area is [-20;20] width and [70;5] height
    if map_render_data.draw_models {
        for (model_instance_index, model_instance) in
            map_render_data.model_instances.iter().enumerate()
        {
            let min = model_instance.bottom_left_front;
            let max = model_instance.top_right_back;

            let cam_pos = camera.pos();
            if ((max.x < cam_pos.x - 40.0 || max.x > cam_pos.x + 40.0)
                && (min.x < cam_pos.x - 40.0 || min.x > cam_pos.x + 40.0))
                || ((max.z < cam_pos.z - 70.0 || max.z > cam_pos.z + 5.0)
                    && (min.z < cam_pos.z - 70.0 || min.z > cam_pos.z + 5.0))
            {
                //                continue;
            }
            let model_render_data = asset_db.get_model(model_instance.asset_db_model_index);
            let alpha = if let Some(char_pos) = char_pos {
                if (max.x > char_pos.x && min.x < char_pos.x)
                    && char_pos.y <= min.z // character is behind
                    && max.y > 2.0
                {
                    77
                } else {
                    model_render_data.alpha
                }
            } else {
                model_render_data.alpha
            };

            render_commands.add_model_command_3d(model_instance_index, alpha != 255);
        }
    }
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
        entities: &Entities,
        numbers: &ReadStorage<FlyingNumberComponent>,
        char_state_storage: &ReadStorage<CharacterStateComponent>,
        followed_char_id: CharEntityId,
        desktop_entity_team: Option<Team>,
        now: ElapsedTime,
        assets: &AssetResources,
        updater: &Write<LazyUpdate>,
        render_commands: &mut RenderCommandCollector,
    ) {
        for (entity_id, number) in (entities, numbers).join() {
            DamageRenderSystem::add_render_command(
                number,
                char_state_storage,
                followed_char_id,
                desktop_entity_team,
                now,
                assets,
                render_commands,
            );

            if number.die_at.has_already_passed(now) {
                updater.remove::<FlyingNumberComponent>(entity_id);
            }
        }
    }

    fn add_render_command(
        number: &FlyingNumberComponent,
        char_state_storage: &ReadStorage<CharacterStateComponent>,
        desktop_entity_id: CharEntityId,
        desktop_entity_team: Option<Team>,
        now: ElapsedTime,
        assets: &AssetResources,
        render_commands: &mut RenderCommandCollector,
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
            | FlyingNumberType::Combo { .. } => digit_count as f32,
            FlyingNumberType::Block => 100.0,
            FlyingNumberType::Absorb => 120.0,
        };

        let perc = now.elapsed_since(number.start_time).div(number.duration);

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
                    src_entity_id: number.src_entity_id,
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
                    desktop_entity_team,
                    now,
                    assets,
                    render_commands,
                );
            }
        }

        // TODO: don't render more than 1 damage in a single frame for the same target
        let (size, pos) = match number.typ {
            FlyingNumberType::Heal => {
                DamageRenderSystem::calc_heal_size_pos(char_state_storage, number, width, perc)
            }
            FlyingNumberType::Combo { .. } => {
                let real_pos = char_state_storage
                    .get(number.target_entity_id.0)
                    .map(|it| it.pos())
                    .unwrap_or(number.start_pos);
                let size = 1.0;
                let mut pos = Vector3::new(real_pos.x, 1.0, real_pos.y);
                pos.x -= width * size / 2.0;
                let y_offset = perc * 1.2;
                pos.y += 4.0 + y_offset;
                // a small hack to mitigate the distortion effect of perspective projection
                // at the edge of the screens
                pos.z -= y_offset;
                (size, pos)
            }
            FlyingNumberType::Damage => {
                DamageRenderSystem::calc_damage_size_pos(char_state_storage, number, perc, 1.0)
            }
            FlyingNumberType::SubCombo => {
                DamageRenderSystem::calc_damage_size_pos(char_state_storage, number, perc, 2.0)
            }
            FlyingNumberType::Poison => {
                DamageRenderSystem::calc_poison_size_pos(char_state_storage, number, width, perc)
            }
            FlyingNumberType::Block | FlyingNumberType::Absorb => {
                let real_pos = char_state_storage
                    .get(number.target_entity_id.0)
                    .map(|it| it.pos())
                    .unwrap_or(number.start_pos);
                let mut pos = Vector3::new(real_pos.x, 1.0, real_pos.y);
                let y_offset = (perc - 0.3) * 3.0;
                pos.y += 2.0 + y_offset;
                pos.z -= y_offset;
                (1.0, pos)
            }
        };
        let alpha = match number.typ {
            FlyingNumberType::Combo { .. } => {
                //                let y_offset = if perc < 0.3 { 0.0 } else { (perc - 0.3) * 3.0 };
                1.6 - (perc + 0.6 * perc)
            }
            _ => 1.3 - (perc + 0.3 * perc),
        };
        let is_friend = char_state_storage
            .get(number.target_entity_id.0)
            .and_then(|target| {
                desktop_entity_team.map(|controller_team| controller_team.is_ally_to(target.team))
            })
            .unwrap_or(true);
        let size_mult = if desktop_entity_id == number.target_entity_id
            || desktop_entity_id == number.src_entity_id
        {
            0.5
        } else {
            0.3
        };
        let color = number.typ.color(
            desktop_entity_id == number.target_entity_id,
            is_friend,
            desktop_entity_id == number.src_entity_id,
        );
        match number.typ {
            FlyingNumberType::Poison
            | FlyingNumberType::Heal
            | FlyingNumberType::Damage
            | FlyingNumberType::Combo { .. }
            | FlyingNumberType::SubCombo => {
                render_commands
                    .number_3d()
                    .scale(size * size_mult)
                    .pos(&pos)
                    .color_rgb(&color)
                    .alpha((alpha * 255.0).min(255.0) as u8)
                    .add(number_value);
            }
            FlyingNumberType::Block => {
                render_commands
                    .sprite_3d()
                    .pos(&pos)
                    .scale(size_mult)
                    .color_rgb(&color)
                    .alpha((alpha * 255.0).min(255.0) as u8)
                    .add(assets.texts.attack_blocked);
            }
            FlyingNumberType::Absorb => {
                render_commands
                    .sprite_3d()
                    .pos(&pos)
                    .scale(size_mult)
                    .color_rgb(&color)
                    .alpha((alpha * 255.0).min(255.0) as u8)
                    .add(assets.texts.attack_absorbed);
            }
        };
    }

    fn calc_damage_size_pos(
        char_state_storage: &ReadStorage<CharacterStateComponent>,
        number: &FlyingNumberComponent,
        perc: f32,
        speed: f32,
    ) -> (f32, Vector3<f32>) {
        let real_pos = char_state_storage
            .get(number.target_entity_id.0)
            .map(|it| it.pos())
            .unwrap_or(number.start_pos);
        let mut pos = Vector3::new(real_pos.x, 1.0, real_pos.y);
        pos.x += perc * 1.0;
        pos.z -= perc * 1.0;
        pos.y += 2.0
            + (-std::f32::consts::FRAC_PI_2 + (std::f32::consts::PI * (0.5 + perc * 1.5 * speed)))
                .sin()
                * 2.0;
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
            .get(number.target_entity_id.0)
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
            .get(number.target_entity_id.0)
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
}

impl RenderDesktopClientSystem {
    fn draw_health_bar(
        &self,
        is_self: bool,
        is_same_team: bool,
        char_state: &CharacterStateComponent,
        now: ElapsedTime,
        bounding_rect_2d: &SpriteBoundingRect,
        assets: &AssetResources,
        render_commands: &mut RenderCommandCollector,
    ) {
        let bar_w = match char_state.typ {
            CharType::Player => 80,
            CharType::Minion => 70,
            _ => 100,
        };
        let spr_x = bounding_rect_2d.bottom_left[0];
        let spr_w = bounding_rect_2d.top_right[0] - bounding_rect_2d.bottom_left[0];
        let bar_x = spr_x + (spr_w / 2) - (bar_w / 2);
        let mut draw_rect = |x: i32, y: i32, w: i32, h: i32, color: &[u8; 4]| {
            render_commands
                .rectangle_2d()
                .color(&color)
                .size(w as u16, h as u16)
                .screen_pos(bar_x + x, bounding_rect_2d.top_right[1] - 30 + y)
                .layer(UiLayer2d::HealthBars)
                .add();
        };

        let hp_percentage = char_state.hp as f32 / char_state.calculated_attribs().max_hp as f32;
        let health_color = if is_self {
            [74, 204, 28, 255] // for self, the health bar is green
        } else if is_same_team {
            [51, 117, 230, 255] // for friends, blue
        } else {
            [201, 0, 54, 255] // for enemies, red
        };
        let mana_color = [59, 201, 224, 255];
        let bottom_bar_y = match char_state.typ {
            CharType::Player => {
                draw_rect(0, 0, bar_w, 9, &[0, 0, 0, 255]); // black border
                draw_rect(0, 0, bar_w, 5, &[0, 0, 0, 255]); // center separator
                let inner_w = ((bar_w - 2) as f32 * hp_percentage) as i32;
                draw_rect(1, 1, inner_w, 4, &health_color);
                draw_rect(1, 6, bar_w - 2, 2, &mana_color);
                9
            }
            _ => {
                draw_rect(0, 0, bar_w, 5, &[0, 0, 0, 255]); // black border
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
            let orange = [255, 140, 0, 255];
            let w = bar_w - 4;
            draw_rect(2, bottom_bar_y + 2, w, 2, &[0, 0, 0, 255]); // black bg
            let inner_w = (w as f32 * (1.0 - perc)) as i32;
            draw_rect(2, bottom_bar_y + 2, inner_w, 2, &orange);
        }

        // draw status indicator icons
        const ICON_WIDTH: i32 = 24;
        if char_state.attrib_bonuses().attrs.armor.is_not_zero() {
            let armor_bonus = char_state.attrib_bonuses().attrs.armor.as_i16();
            let shield_icon_texture = assets.status_icons["shield"];
            let x = bar_x + bar_w + 1;
            let y = bounding_rect_2d.top_right[1] - 30;
            // icon size is 24x24
            render_commands
                .sprite_2d()
                .color(&COLOR_WHITE)
                .screen_pos(x, y)
                .layer(UiLayer2d::StatusIndicators)
                .offset(0, -(ICON_WIDTH as i16) / 2)
                .add(shield_icon_texture);

            // progress bar
            let color = if armor_bonus > 0 {
                [0, 255, 0, 255]
            } else {
                [255, 0, 0, 255]
            };

            let perc = (now.percentage_between(
                char_state.attrib_bonuses().durations.armor_bonus_started_at,
                char_state.attrib_bonuses().durations.armor_bonus_ends_at,
            ) * 100.0) as i32;
            let index = (100 - perc).max(1) as usize;
            let x = bar_x + bar_w + ICON_WIDTH / 2 + 1;
            let y = bounding_rect_2d.top_right[1] - 30;

            render_commands
                .partial_circle_2d()
                .color(&color)
                .screen_pos(x, y)
                .layer(UiLayer2d::StatusIndicators)
                .circumference_percentage(index)
                .add();

            let text_texture = assets.texts.custom_texts[&armor_bonus.to_string()];

            render_commands
                .sprite_2d()
                .color(&color)
                .screen_pos(x, y)
                .layer(UiLayer2d::StatusIndicators)
                .add(text_texture);
        }
    }

    pub fn render_str<E>(
        effect: E,
        start_time: ElapsedTime,
        world_pos: &Vec2,
        sys_vars: &SystemVariables,
        render_commands: &mut RenderCommandCollector,
        play_mode: ActionPlayMode,
    ) -> bool
    where
        E: Into<StrEffectId>,
    {
        let effect_id = effect.into();
        let str_file = &sys_vars.str_effects[effect_id.0];
        let seconds_needed_for_one_frame = 1.0 / str_file.fps as f32;
        let max_key = str_file.max_key as i32;
        let real_index = sys_vars
            .time
            .elapsed_since(start_time)
            .div(seconds_needed_for_one_frame) as i32;
        let key_index = match play_mode {
            ActionPlayMode::Repeat | ActionPlayMode::Once => real_index % max_key,
            ActionPlayMode::PlayThenHold => real_index.min(max_key - 1),
            ActionPlayMode::Reverse => (max_key - 1) - (real_index % max_key),
            ActionPlayMode::FixFrame(frame_i) => frame_i as i32,
        };

        render_commands.add_effect_command2(world_pos, effect_id, key_index);
        for layer_index in 0..str_file.layers.len() {
            render_commands.add_effect_command(world_pos, effect_id, key_index, layer_index);
        }
        return real_index >= max_key && play_mode == ActionPlayMode::Once;
    }
}
