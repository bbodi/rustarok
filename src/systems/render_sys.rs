use crate::asset::database::AssetDatabase;
use crate::cam::Camera;
use crate::components::char::{
    ActionPlayMode, CharOutlook, CharState, CharType, CharacterStateComponent, EntityTarget,
    NpcComponent, PhysicsComponent, SpriteBoundingRect, SpriteRenderDescriptorComponent, Team,
};
use crate::components::controller::{
    CameraComponent, CharEntityId, ControllerComponent, ControllerEntityId, EntitiesBelowCursor,
    HumanInputComponent, PlayerIntention, SkillKey, WorldCoords,
};
use crate::components::skills::skill::{SkillManifestationComponent, SkillTargetType, Skills};
use crate::components::{
    FlyingNumberComponent, FlyingNumberType, SoundEffectComponent, StrEffectComponent,
};
use crate::cursor::CURSOR_TARGET;
use crate::effect::StrEffectId;
use crate::runtime_assets::map::{MapRenderData, PhysicEngine};
use crate::systems::render::render_command::{RenderCommandCollectorComponent, UiLayer2d};
use crate::systems::sound_sys::AudioCommandCollectorComponent;
use crate::systems::ui::RenderUI;
use crate::systems::{AssetResources, SystemFrameDurations, SystemVariables};
use crate::{ElapsedTime, SpriteResource};
use nalgebra::{Vector2, Vector3};
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
        render_commands: &mut RenderCommandCollectorComponent,
        audio_commands: &mut AudioCommandCollectorComponent,
        physics_storage: &specs::ReadStorage<'a, PhysicsComponent>,
        physics_world: &specs::ReadExpect<'a, PhysicEngine>,
        system_vars: &SystemVariables,
        char_state_storage: &specs::ReadStorage<'a, CharacterStateComponent>,
        entities: &specs::Entities<'a>,
        sprite_storage: &specs::ReadStorage<'a, SpriteRenderDescriptorComponent>,
        skill_storage: &specs::ReadStorage<'a, SkillManifestationComponent>, // TODO remove me
        str_effect_storage: &specs::ReadStorage<'a, StrEffectComponent>,
        updater: &specs::Write<'a, LazyUpdate>,
        system_benchmark: &mut SystemFrameDurations,
        asset_database: &AssetDatabase,
    ) {
        render_commands.set_view_matrix(&camera.view_matrix, &camera.normal_matrix);
        {
            let _stopwatch = system_benchmark.start_measurement("render.draw_physics_coll");
            // Draw physics colliders
            for physics in (&physics_storage).join() {
                if let Some(body) = physics_world.bodies.rigid_body(physics.body_handle) {
                    let pos = body.position().translation.vector;

                    render_commands
                        .circle_3d()
                        .radius(physics.radius.get())
                        .color(&[255, 0, 255, 255])
                        .pos_2d(&pos)
                        .y(0.05)
                        .add();
                }
            }
        }

        {
            let _stopwatch = system_benchmark.start_measurement("render.render_client");
            render_client(
                controller
                    .as_ref()
                    .map(|it| it.controlled_char.pos())
                    .as_ref(),
                &camera.camera,
                &system_vars.map_render_data,
                asset_database,
                render_commands,
            );
        }

        {
            if let Some(controller) = &controller {
                {
                    let _stopwatch = system_benchmark.start_measurement("render.casting");
                    let char_pos = controller.controlled_char.pos();
                    if let Some((_skill_key, skill)) = controller.controller.select_skill_target {
                        render_commands
                            .circle_3d()
                            .pos_2d(&char_pos)
                            .y(0.0)
                            .radius(skill.get_casting_range())
                            .color(&[0, 255, 0, 255])
                            .add();

                        if skill.get_skill_target_type() == SkillTargetType::Area {
                            let is_castable = controller
                                .controlled_char
                                .skill_cast_allowed_at
                                .get(&skill)
                                .unwrap_or(&ElapsedTime(0.0))
                                .is_earlier_than(system_vars.time);
                            let (skill_3d_pos, dir_vector) = Skills::limit_vector_into_range(
                                &char_pos,
                                &input.mouse_world_pos,
                                skill.get_casting_range(),
                            );
                            skill.render_target_selection(
                                is_castable,
                                &skill_3d_pos,
                                &dir_vector,
                                render_commands,
                            );
                        }
                    } else {
                        if let CharState::CastingSkill(casting_info) =
                            controller.controlled_char.state()
                        {
                            let skill = casting_info.skill;
                            skill.render_casting(
                                &char_pos,
                                &casting_info,
                                system_vars,
                                render_commands,
                            );
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
                                system_vars.time,
                                &cursor_anim_descr,
                                &system_vars.assets.sprites.cursors,
                                camera.yaw,
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

        {
            let _stopwatch = system_benchmark.start_measurement("render.draw_characters");
            self.draw_characters(
                &camera,
                controller,
                render_commands,
                &system_vars,
                char_state_storage,
                entities,
                sprite_storage,
            );
        }

        for skill in (&skill_storage).join() {
            skill.render(
                system_vars.time,
                system_vars.tick,
                &system_vars.assets,
                render_commands,
                audio_commands,
            );
        }

        // TODO: into a separate system
        {
            let _stopwatch = system_benchmark.start_measurement("render.str_effect");
            for (entity_id, str_effect) in (entities, str_effect_storage).join() {
                if str_effect.die_at.is_earlier_than(system_vars.time) {
                    updater.remove::<StrEffectComponent>(entity_id);
                } else {
                    RenderDesktopClientSystem::render_str(
                        str_effect.effect_id,
                        str_effect.start_time,
                        &str_effect.pos,
                        system_vars,
                        render_commands,
                    );
                }
            }
        }
        // TODO: into a separate system
        {
            //            let _stopwatch = system_benchmark.start_measurement("render.dyn_str_effect");
            //            for (entity_id, str_effect) in (entities, dynamic_str_effect_storage).join() {
            //                if str_effect.die_at.is_earlier_than(system_vars.time) {
            //                    updater.remove::<StrEffectComponent>(entity_id);
            //                } else {
            //                    RenderDesktopClientSystem::render_str(
            //                        str_effect.effect_type,
            //                        str_effect.start_time,
            //                        &str_effect.pos,
            //                        system_vars,
            //                        render_commands,
            //                    );
            //                }
            //            }
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
            match skill.get_skill_target_type() {
                SkillTargetType::AnyEntity => entities_below_cursor
                    .get_enemy_or_friend()
                    .map(|it| it == rendering_entity_id)
                    .unwrap_or(false),
                SkillTargetType::NoTarget => false,
                SkillTargetType::Area => false,
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
                SkillTargetType::OnlySelf => entities_below_cursor
                    .get_friend()
                    .map(|it| it == followed_char_id)
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
        render_commands: &mut RenderCommandCollectorComponent,
        system_vars: &SystemVariables,
        char_state_storage: &ReadStorage<CharacterStateComponent>,
        entities: &Entities,
        sprite_storage: &ReadStorage<SpriteRenderDescriptorComponent>,
    ) {
        // Draw players
        for (rendering_entity_id, animated_sprite, char_state) in
            (entities, sprite_storage, char_state_storage).join()
        {
            let rendering_entity_id = CharEntityId(rendering_entity_id);
            // for autocompletion
            let char_state: &CharacterStateComponent = char_state;

            let pos = char_state.pos();
            if !camera.camera.is_visible(pos) {
                continue;
            }

            let color = char_state.statuses.calc_render_color(system_vars.time);
            match char_state.outlook {
                CharOutlook::Player {
                    job_id,
                    head_index,
                    sex,
                } => {
                    let body_sprite = if char_state.statuses.is_mounted() {
                        let sprites = &system_vars.assets.sprites;
                        &sprites
                            .mounted_character_sprites
                            .get(&job_id)
                            .and_then(|it| it.get(sex as usize))
                            .unwrap_or_else(|| {
                                let sprites = &system_vars.assets.sprites.character_sprites;
                                &sprites[&job_id][sex as usize]
                            })
                    } else {
                        let sprites = &system_vars.assets.sprites.character_sprites;
                        &sprites[&job_id][sex as usize]
                    };
                    let play_mode = if char_state.state().is_dead() {
                        ActionPlayMode::PlayThenHold
                    } else {
                        ActionPlayMode::Repeat
                    };
                    let head_res = {
                        let sprites = &system_vars.assets.sprites.head_sprites;
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
                            let color = if controller.controlled_char.team == char_state.team {
                                &[0, 0, 255, 179]
                            } else {
                                &[255, 0, 0, 179]
                            };
                            let body_pos_offset = render_single_layer_action(
                                system_vars.time,
                                &animated_sprite,
                                body_sprite,
                                camera.yaw,
                                &pos,
                                [0, 0],
                                true,
                                1.2,
                                play_mode,
                                color,
                                render_commands,
                            );

                            let _head_pos_offset = render_single_layer_action(
                                system_vars.time,
                                &animated_sprite,
                                head_res,
                                camera.yaw,
                                &pos,
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
                        system_vars.time,
                        &animated_sprite,
                        body_sprite,
                        camera.yaw,
                        &pos,
                        [0, 0],
                        true,
                        1.0,
                        play_mode,
                        &color,
                        render_commands,
                    );

                    let mut body_bounding_rect = {
                        let render_command = render_commands.get_last_billboard_command();

                        render_command.project_to_screen(
                            &camera.view_matrix,
                            &system_vars.matrices.projection,
                        )
                    };
                    let _head_pos_offset = render_single_layer_action(
                        system_vars.time,
                        &animated_sprite,
                        head_res,
                        camera.yaw,
                        &pos,
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
                        let head_bounding_rect = render_command.project_to_screen(
                            &camera.view_matrix,
                            &system_vars.matrices.projection,
                        );
                        body_bounding_rect.merge(&head_bounding_rect);
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
                                .map(|it| it.controlled_char.team == char_state.team)
                                .unwrap_or(false),
                            &char_state,
                            system_vars.time,
                            &body_bounding_rect,
                            &system_vars.assets,
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
                        let sprites = &system_vars.assets.sprites.monster_sprites;
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
                            let color = if controller.controlled_char.team == char_state.team {
                                &[0, 0, 255, 179]
                            } else {
                                &[255, 0, 0, 179]
                            };
                            let _pos_offset = render_single_layer_action(
                                system_vars.time,
                                &animated_sprite,
                                body_res,
                                camera.yaw,
                                &pos,
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
                        system_vars.time,
                        &animated_sprite,
                        body_res,
                        camera.yaw,
                        &pos,
                        [0, 0],
                        true,
                        1.0,
                        play_mode,
                        &color,
                        render_commands,
                    );
                    let bounding_rect = {
                        let render_command = render_commands.get_last_billboard_command();

                        render_command.project_to_screen(
                            &camera.view_matrix,
                            &system_vars.matrices.projection,
                        )
                    };
                    if !char_state.state().is_dead() {
                        self.draw_health_bar(
                            controller
                                .as_ref()
                                .map(|it| it.controller.controlled_entity == rendering_entity_id)
                                .unwrap_or(false),
                            controller
                                .as_ref()
                                .map(|it| it.controlled_char.team == char_state.team)
                                .unwrap_or(false),
                            &char_state,
                            system_vars.time,
                            &bounding_rect,
                            &system_vars.assets,
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

            char_state
                .statuses
                .render(&char_state.pos(), system_vars, render_commands);
        }
    }
}

struct ControllerAndControlled<'a> {
    controller: &'a mut ControllerComponent,
    controlled_char: &'a CharacterStateComponent,
}

impl<'a> specs::System<'a> for RenderDesktopClientSystem {
    type SystemData = (
        specs::Entities<'a>,
        specs::ReadStorage<'a, HumanInputComponent>,
        specs::ReadStorage<'a, PhysicsComponent>,
        specs::ReadStorage<'a, SpriteRenderDescriptorComponent>,
        specs::ReadStorage<'a, CharacterStateComponent>,
        specs::WriteStorage<'a, ControllerComponent>, // mut: we have to store bounding rects of drawed entities :(
        specs::WriteExpect<'a, SystemVariables>,
        specs::WriteExpect<'a, SystemFrameDurations>,
        specs::ReadStorage<'a, SkillManifestationComponent>, // TODO remove me
        specs::ReadStorage<'a, StrEffectComponent>,
        specs::ReadStorage<'a, CameraComponent>,
        specs::ReadExpect<'a, PhysicEngine>,
        specs::Write<'a, LazyUpdate>,
        specs::ReadStorage<'a, FlyingNumberComponent>,
        specs::ReadStorage<'a, SoundEffectComponent>,
        specs::WriteStorage<'a, RenderCommandCollectorComponent>,
        specs::WriteStorage<'a, AudioCommandCollectorComponent>,
        specs::ReadExpect<'a, AssetDatabase>,
        specs::ReadStorage<'a, NpcComponent>,
    );

    fn run(
        &mut self,
        (
            entities,
            input_storage,
            physics_storage,
            sprite_storage,
            char_state_storage,
            mut controller_storage,
            mut system_vars,
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
            asset_database,
            npc_storage,
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
            let entity_id = ControllerEntityId(entity_id);
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

            {
                self.render_for_controller(
                    &mut controller_and_controlled,
                    camera,
                    &mut input,
                    &mut render_commands,
                    &mut audio_commands,
                    &physics_storage,
                    &physics_world,
                    &mut system_vars,
                    &char_state_storage,
                    &entities,
                    &sprite_storage,
                    &skill_storage,
                    &str_effect_storage,
                    &updater,
                    &mut system_benchmark,
                    &asset_database,
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
                        CharEntityId(entity_id.0), // entity_id is the controller id, so no character will match with it, ~dummy value
                    ),
                controller_and_controlled
                    .as_ref()
                    .map(|it| it.controlled_char.team),
                system_vars.time,
                &system_vars.assets,
                &updater,
                render_commands,
            );

            if let Some(controller_and_controlled) = controller_and_controlled.as_ref() {
                self.render_ui_sys.run(
                    &controller_and_controlled.controlled_char,
                    &input,
                    &controller_and_controlled.controller,
                    &mut render_commands,
                    &mut system_vars,
                    &char_state_storage,
                    &npc_storage,
                    &entities,
                );
            }
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
    play_mode: ActionPlayMode,
    color: &[u8; 4],
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
            ActionPlayMode::Repeat => real_index % frame_count,
            ActionPlayMode::PlayThenHold => real_index.min(frame_count - 1),
        }
    };
    let frame = &action.frames[frame_index];

    let layer = &frame.layers[0];

    let offset = if !frame.positions.is_empty() && !is_main {
        [
            pos_offset[0] - frame.positions[0][0],
            pos_offset[1] - frame.positions[0][1],
        ]
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

    let sprite_texture = &sprite_res.textures[layer.sprite_frame_index as usize];
    render_commands
        .sprite_3d()
        .pos_2d(&pos)
        .scale(layer.scale[0] * size_multiplier)
        .offset(offset)
        .color(&color)
        .flip_vertically(layer.is_mirror)
        .add(&sprite_texture);

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
    camera_yaw: f32,
    pos: &Vector2<f32>,
    pos_offset: [i32; 2],
    is_main: bool,
    size_multiplier: f32,
    play_mode: ActionPlayMode,
    color: &[u8; 4],
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
            ActionPlayMode::Repeat => real_index % frame_count,
            ActionPlayMode::PlayThenHold => real_index.min(frame_count - 1),
        }
    };
    let frame = &action.frames[frame_index];

    for layer in frame.layers.iter() {
        if layer.sprite_frame_index < 0 {
            continue;
        }

        let offset = if !frame.positions.is_empty() && !is_main {
            [
                pos_offset[0] - frame.positions[0][0],
                pos_offset[1] - frame.positions[0][1],
            ]
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

        let sprite_texture = &sprite_res.textures[layer.sprite_frame_index as usize];
        render_commands
            .sprite_3d()
            .pos_2d(&pos)
            .scale(layer.scale[0] * size_multiplier)
            .offset(offset)
            .color(&color)
            .flip_vertically(layer.is_mirror)
            .add(&sprite_texture);
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
    char_pos: Option<&Vector2<f32>>,
    camera: &Camera,
    map_render_data: &MapRenderData,
    asset_database: &AssetDatabase,
    render_commands: &mut RenderCommandCollectorComponent,
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
                continue;
            }
            let model_render_data = asset_database.get_model(model_instance.asset_db_model_index);
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
        entities: &specs::Entities,
        numbers: &specs::ReadStorage<FlyingNumberComponent>,
        char_state_storage: &specs::ReadStorage<CharacterStateComponent>,
        followed_char_id: CharEntityId,
        desktop_entity_team: Option<Team>,
        now: ElapsedTime,
        assets: &AssetResources,
        updater: &specs::Write<LazyUpdate>,
        render_commands: &mut RenderCommandCollectorComponent,
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

            if number.die_at.is_earlier_than(now) {
                updater.remove::<FlyingNumberComponent>(entity_id);
            }
        }
    }

    fn add_render_command(
        number: &FlyingNumberComponent,
        char_state_storage: &specs::ReadStorage<CharacterStateComponent>,
        desktop_entity_id: CharEntityId,
        desktop_entity_team: Option<Team>,
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
                    .get(number.target_entity_id.0)
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
        let is_friend = char_state_storage
            .get(number.target_entity_id.0)
            .and_then(|target| {
                desktop_entity_team.map(|controller_team| controller_team == target.team)
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
            | FlyingNumberType::SubCombo
            | FlyingNumberType::Mana
            | FlyingNumberType::Crit => {
                render_commands
                    .number_3d()
                    .scale(size * size_mult)
                    .pos(&pos)
                    .color_rgb(&color)
                    .alpha((alpha * 255.0).min(255.0) as u8)
                    .add(number_value, digit_count as u8);
            }
            FlyingNumberType::Block => {
                render_commands
                    .sprite_3d()
                    .pos(&pos)
                    .scale(size_mult)
                    .color_rgb(&color)
                    .alpha((alpha * 255.0).min(255.0) as u8)
                    .add(&assets.texts.attack_blocked);
            }
            FlyingNumberType::Absorb => {
                render_commands
                    .sprite_3d()
                    .pos(&pos)
                    .scale(size_mult)
                    .color_rgb(&color)
                    .alpha((alpha * 255.0).min(255.0) as u8)
                    .add(&assets.texts.attack_absorbed);
            }
        };
    }

    fn calc_damage_size_pos(
        number: &FlyingNumberComponent,
        perc: f32,
        speed: f32,
    ) -> (f32, Vector3<f32>) {
        let mut pos = Vector3::new(number.start_pos.x, 1.0, number.start_pos.y);
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
        render_commands: &mut RenderCommandCollectorComponent,
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
        if char_state.attrib_bonuses().attrs.armor.is_not_zero() {
            let armor_bonus = char_state.attrib_bonuses().attrs.armor.as_i16();
            let shield_icon_texture = &assets.status_icons["shield"];
            let x = bar_x + bar_w + 1;
            let y = bounding_rect_2d.top_right[1] - 30;
            render_commands
                .sprite_2d()
                .color(&COLOR_WHITE)
                .screen_pos(x, y)
                .layer(UiLayer2d::StatusIndicators)
                .offset(0, (-shield_icon_texture.height / 2) as i16)
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
            let x = bar_x + bar_w + shield_icon_texture.width / 2 + 1;
            let y = bounding_rect_2d.top_right[1] - 30;

            render_commands
                .partial_circle_2d()
                .color(&color)
                .screen_pos(x, y)
                .layer(UiLayer2d::StatusIndicators)
                .circumference_percentage(index)
                .add();

            let text_texture = &assets.texts.custom_texts[&armor_bonus.to_string()];

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
        world_pos: &WorldCoords,
        system_vars: &SystemVariables,
        render_commands: &mut RenderCommandCollectorComponent,
    ) where
        E: Into<StrEffectId>,
    {
        let effect_id = effect.into();
        let str_file = &system_vars.str_effects[effect_id.0];
        let seconds_needed_for_one_frame = 1.0 / str_file.fps as f32;
        let max_key = str_file.max_key;
        let key_index = system_vars
            .time
            .elapsed_since(start_time)
            .div(seconds_needed_for_one_frame) as i32
            % max_key as i32;

        for layer_index in 0..str_file.layers.len() {
            render_commands.add_effect_command(world_pos, effect_id, key_index, layer_index);
        }
    }
}
