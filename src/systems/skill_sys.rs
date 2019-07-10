use std::collections::HashMap;

use nalgebra::{Matrix3, Matrix4, Point2, Point3, Rotation3, Vector2, Vector3};
use specs::prelude::*;

use crate::{MapRenderData, Shaders, SpriteResource, Tick, PhysicsWorld};
use crate::cam::Camera;
use crate::cursor::{CURSOR_NORMAL, CURSOR_ATTACK, CURSOR_STOP, CURSOR_LOCK};
use crate::systems::{SystemFrameDurations, SystemVariables};
use crate::video::{draw_circle_inefficiently, draw_lines_inefficiently, draw_lines_inefficiently2, VertexArray, VIDEO_HEIGHT, VIDEO_WIDTH};
use crate::video::VertexAttribDefinition;
use crate::components::controller::ControllerComponent;
use crate::components::BrowserClient;
use crate::components::char::{PhysicsComponent, PlayerSpriteComponent, CharacterStateComponent, MonsterSpriteComponent};
use crate::components::skill::SkillManifestationComponent;

pub struct SkillSystem;

impl<'a> specs::System<'a> for SkillSystem {
    type SystemData = (
        specs::Entities<'a>,
        specs::ReadStorage<'a, ControllerComponent>,
        specs::ReadStorage<'a, BrowserClient>,
        specs::ReadStorage<'a, PhysicsComponent>,
        specs::ReadStorage<'a, PlayerSpriteComponent>,
        specs::ReadStorage<'a, CharacterStateComponent>,
        specs::ReadExpect<'a, SystemVariables>,
        specs::WriteExpect<'a, SystemFrameDurations>,
        specs::WriteExpect<'a, PhysicsWorld>,
        specs::WriteStorage<'a, SkillManifestationComponent>,
        specs::Write<'a, LazyUpdate>,
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
        mut physics_world,
        mut skill_storage,
        mut updater,
    ): Self::SystemData) {
        let stopwatch = system_benchmark.start_measurement("SkillSystem");
        for (entity_id, skill) in (&entities, &mut skill_storage).join() {
            skill.update(entity_id,
                         &system_vars,
                         &entities,
                         &mut physics_world,
                         &mut updater);
        }
    }
}