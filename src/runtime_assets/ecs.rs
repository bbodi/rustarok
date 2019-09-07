use crate::components::char::{
    CharacterStateComponent, NpcComponent, PhysicsComponent, SpriteRenderDescriptorComponent,
};
use crate::components::controller::{CameraComponent, ControllerComponent, HumanInputComponent};
use crate::components::skills::skill::SkillManifestationComponent;
use crate::components::{
    BrowserClient, FlyingNumberComponent, MinionComponent, SoundEffectComponent, StrEffectComponent,
};
use crate::systems::console_system::ConsoleComponent;
use crate::systems::render::render_command::RenderCommandCollectorComponent;
use crate::systems::sound_sys::AudioCommandCollectorComponent;
use specs::World;

pub fn create_ecs_world() -> World {
    let mut ecs_world = specs::World::new();
    ecs_world.register::<BrowserClient>();
    ecs_world.register::<NpcComponent>();
    ecs_world.register::<HumanInputComponent>();
    ecs_world.register::<RenderCommandCollectorComponent>();
    ecs_world.register::<AudioCommandCollectorComponent>();
    ecs_world.register::<SpriteRenderDescriptorComponent>();
    ecs_world.register::<CharacterStateComponent>();
    ecs_world.register::<PhysicsComponent>();
    ecs_world.register::<FlyingNumberComponent>();
    ecs_world.register::<SoundEffectComponent>();
    ecs_world.register::<StrEffectComponent>();
    ecs_world.register::<SkillManifestationComponent>();
    ecs_world.register::<CameraComponent>();
    ecs_world.register::<ControllerComponent>();
    ecs_world.register::<MinionComponent>();
    ecs_world.register::<ConsoleComponent>();
    ecs_world
}
