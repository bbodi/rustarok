use rustarok_common::common::EngineTime;
use rustarok_common::components::char::{AuthorizedCharStateComponent, EntityTarget};
use rustarok_common::components::controller::{ControllerComponent, PlayerIntention};
use rustarok_common::systems::intention_applier::ControllerIntentionToCharTarget;
use specs::Join;

pub struct ControllerIntentionToCharTargetSystem;

impl<'a> specs::System<'a> for ControllerIntentionToCharTargetSystem {
    type SystemData = (
        specs::WriteStorage<'a, AuthorizedCharStateComponent>,
        specs::WriteStorage<'a, ControllerComponent>,
        specs::ReadExpect<'a, EngineTime>,
        //        ReadExpect<'a, DevConfig>,
    );

    fn run(&mut self, (mut char_state_storage, mut controller_storage, time): Self::SystemData) {
        let now = time.now();
        for mut controller in (&mut controller_storage).join() {
            ControllerIntentionToCharTarget::controller_intention_to_char_target(
                controller,
                &mut char_state_storage,
            );
            controller.intention = None;
        }
    }
}
