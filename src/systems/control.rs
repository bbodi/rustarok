use nalgebra::{Point3, Point2, Vector2, Vector3, Perspective3, Vector4, Matrix4, Isometry2};
use crate::systems::render::DIRECTION_TABLE;
use specs::prelude::*;
use rand::Rng;
use crate::systems::{SystemVariables, SystemFrameDurations};
use crate::video::{VIDEO_WIDTH, VIDEO_HEIGHT};
use sdl2::keyboard::Scancode;
use crate::{ActionIndex, RenderMatrices, PhysicsWorld, TICKS_PER_SECOND, Tick, ElapsedTime};
use crate::cam::Camera;
use ncollide2d::shape::{Cuboid, Ball};
use ncollide2d::query::point_internal::point_query::PointQuery;
use specs::world::EntitiesRes;
use specs::join::JoinIter;
use crate::components::{FlyingNumberType, FlyingNumberComponent};
use crate::components::char::{CharState, PhysicsComponent, CharacterStateComponent, PlayerSpriteComponent};
use crate::components::controller::ControllerComponent;
use crate::components::skill::{PushBackWallSkill, SkillManifestationComponent};
use nphysics2d::object::Body;

pub struct CharacterControlSystem;

impl<'a> specs::System<'a> for CharacterControlSystem {
    type SystemData = (
        specs::Entities<'a>,
        specs::WriteStorage<'a, PhysicsComponent>,
        specs::WriteStorage<'a, CharacterStateComponent>,
        specs::WriteStorage<'a, PlayerSpriteComponent>,
        specs::ReadStorage<'a, ControllerComponent>,
        specs::WriteExpect<'a, SystemVariables>,
        specs::WriteExpect<'a, PhysicsWorld>,
        specs::WriteExpect<'a, SystemFrameDurations>,
        specs::Write<'a, LazyUpdate>,
    );

    fn run(&mut self, (
        entities,
        mut physics_storage,
        mut char_state_storage,
        mut sprite_storage,
        controller_storage,
        mut system_vars,
        mut physics_world,
        mut system_benchmark,
        mut updater,
    ): Self::SystemData) {
        let stopwatch = system_benchmark.start_measurement("CharacterControlSystem");
        let mut rng = rand::thread_rng();
        let dt = system_vars.dt.0;
        for (controller) in (&controller_storage).join() {
            let char_pos = {
                let physics_comp = physics_storage.get(controller.char).unwrap();
                physics_comp.pos(&physics_world)
            };
            let mouse_world_pos = CharacterControlSystem::picking_2d_3d(controller.last_mouse_x,
                                                                        controller.last_mouse_y,
                                                                        &controller.camera,
                                                                        &system_vars.matrices);
            let mut entity_below_cursor: Option<Entity> = None;
            for (entity, other_char_state, other_physics) in (&entities, &char_state_storage, &physics_storage).join() {
                let bb = &other_char_state.bounding_rect;
                let mx = controller.last_mouse_x as i32;
                let my = controller.last_mouse_y as i32;
                if mx >= bb.bottom_left[0] && mx <= bb.top_right[0] &&
                    my <= bb.bottom_left[1] && my >= bb.top_right[1] {
                    entity_below_cursor = Some(entity);
                    break;
                }
            }
            system_vars.entity_below_cursor = entity_below_cursor;
            system_vars.cell_below_cursor_walkable = system_vars.map_render_data.gat.is_walkable(
                mouse_world_pos.x as usize,
                mouse_world_pos.y.abs() as usize,
            );
            if controller.right_mouse_released {
                let mut char_state = char_state_storage.get_mut(controller.char).unwrap();
                char_state.target = entity_below_cursor;
                char_state.target_pos = dbg!(Some(mouse_world_pos));
            } else {
                // follow target
                let mut char_state = char_state_storage.get_mut(controller.char).unwrap();
                if let Some(target) = char_state.target {
                    let target_pos = {
                        let physics_comp = physics_storage.get(target).unwrap();
                        physics_comp.pos(&physics_world)
                    };
                    char_state.target_pos = Some(Point2::new(target_pos.x, target_pos.y));
                }
            }

            //
            if controller.is_key_just_pressed(Scancode::Q) {
                let skill_entity_id = entities.create();
                let skill = PushBackWallSkill::new(
                    &mut physics_world,
                    mouse_world_pos.coords,
                    skill_entity_id,
                    &system_vars.time,
                );
                updater.insert(skill_entity_id, SkillManifestationComponent::new(skill));
            }
            //
            if controller.is_key_just_pressed(Scancode::R) {
                let physics_comp = physics_storage.get(controller.char).unwrap();
                let mut body = physics_world.rigid_body_mut(physics_comp.body_handle).unwrap();
                let mut new_pos = Isometry2::identity();
                new_pos.translation.vector = mouse_world_pos.coords;
                body.set_position(new_pos);
            }
            //
            let mut char_state = char_state_storage.get_mut(controller.char).unwrap();
            if char_state.cannot_control_until.has_not_passed(&system_vars.time) {
                continue;
            }


            let mut sprite = sprite_storage.get_mut(controller.char).unwrap();
            let mut physics_comp = physics_storage.get_mut(controller.char).unwrap();
            let body = physics_world.rigid_body_mut(physics_comp.body_handle).unwrap();
            let char_pos = body.position().translation.vector;

            if let CharState::Attacking { attack_ends } = char_state.state() {
                if attack_ends.has_passed(&system_vars.time) {
                    char_state.set_state(CharState::Idle,
                                         char_state.dir(),
                                         &mut sprite.descr,
                                         system_vars.time,
                                         None);
                    let damage = entities.create();
                    let mut rng = rand::thread_rng();
                    let typ = match rng.gen_range(1, 5) {
                        1 => FlyingNumberType::Damage,
                        2 => FlyingNumberType::Heal,
                        3 => FlyingNumberType::Mana,
                        _ => FlyingNumberType::Normal,
                    };
                    updater.insert(damage, FlyingNumberComponent::new(
                        typ,
                        rng.gen_range(1, 20000),
                        Point2::new(char_pos.x, char_pos.y),
                        system_vars.tick));
                }
            }
            if !char_state.state().is_attacking() {
                if let Some(target_pos) = char_state.target_pos {
                    let distance = nalgebra::distance(&nalgebra::Point::from(char_pos), &target_pos);
                    if char_state.target.is_some() && char_state.target.unwrap() != controller.char && distance <= char_state.attack_range {
                        let attack_anim_duration = ElapsedTime(1.0 / char_state.attack_speed);
                        let attack_ends = system_vars.time.add(&attack_anim_duration);
                        char_state.set_state(CharState::Attacking { attack_ends },
                                             CharacterControlSystem::determine_dir(&target_pos, &char_pos),
                                             &mut sprite.descr,
                                             system_vars.time,
                                             Some(attack_anim_duration));
                        body.set_linear_velocity(Vector2::new(0.0, 0.0));
                    } else if distance < 0.2 {
                        char_state.set_state(CharState::Idle,
                                             char_state.dir(),
                                             &mut sprite.descr, system_vars.time,
                                             None);
                        body.set_linear_velocity(Vector2::new(0.0, 0.0));
                        char_state.target_pos = None;
                    } else {
                        if !char_state.state().is_walking() {
                            char_state.set_state(CharState::Walking,
                                                 CharacterControlSystem::determine_dir(&target_pos, &char_pos),
                                                 &mut sprite.descr, system_vars.time,
                                                 None);
                        } else {
                            char_state.set_dir(CharacterControlSystem::determine_dir(&target_pos, &char_pos),
                                               &mut sprite);
                        }
                        let dir = (target_pos - nalgebra::Point::from(char_pos)).normalize();
                        let speed = dir * char_state.moving_speed * 0.01;
                        let force = speed;
                        body.set_linear_velocity(body.velocity().linear + force);
                    }
                }
            }
        }
    }
}


impl CharacterControlSystem {
    fn picking_2d_3d(x2d: u16, y2d: u16, camera: &Camera, matrices: &RenderMatrices) -> Point2<f32> {
        let screen_point = Point2::new(x2d as f32, y2d as f32);

        let ray_clip = Vector4::new(2.0 * screen_point.x as f32 / VIDEO_WIDTH as f32 - 1.0,
                                    1.0 - (2.0 * screen_point.y as f32) / VIDEO_HEIGHT as f32,
                                    -1.0,
                                    1.0);
        let ray_eye = matrices.projection.try_inverse().unwrap() * ray_clip;
        let ray_eye = Vector4::new(ray_eye.x, ray_eye.y, -1.0, 0.0);
        let ray_world = matrices.view.try_inverse().unwrap() * ray_eye;
        let ray_world = Vector3::new(ray_world.x, ray_world.y, ray_world.z).normalize();

        let line_location = camera.pos();
        let line_direction: Vector3<f32> = ray_world;
        let plane_normal = Vector3::new(0.0, 1.0, 0.0);
        let plane_point = Vector3::new(0.0, 0.0, 0.0);
        let t = (plane_normal.dot(&plane_point) - plane_normal.dot(&line_location.coords)) / plane_normal.dot(&line_direction);
        let world_pos = line_location + (line_direction.scale(t));
        return Point2::new(world_pos.x, world_pos.z);
    }

    fn determine_dir(&target_pos: &Point2<f32>, pos: &Vector2<f32>) -> usize {
        let dir_vec = target_pos - pos;
// "- 90.0"
// The calculated yaw for the camera are 90 at [0;1] and 180 at [1;0] etc,
// this calculation gives a different result which is shifted 90 degrees clockwise,
// so it is 90 at [1;0].
        let dd = dir_vec.x.atan2(dir_vec.y).to_degrees() - 90.0;
        let dd = if dd < 0.0 { dd + 360.0 } else if dd > 360.0 { dd - 360.0 } else { dd };
        let dir_index = (dd / 45.0 + 0.5) as usize % 8;
        return DIRECTION_TABLE[dir_index];
    }
}
