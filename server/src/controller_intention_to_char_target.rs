use rustarok_common::components::char::{EntityTarget, LocalCharEntityId, LocalCharStateComp};
use rustarok_common::components::controller::{ControllerComponent, PlayerIntention};
use rustarok_common::systems::intention_applier::ControllerIntentionToCharTarget;
use specs::Join;

pub struct ControllerIntentionToCharTargetSystem;

impl<'a> specs::System<'a> for ControllerIntentionToCharTargetSystem {
    type SystemData = (
        specs::WriteStorage<'a, LocalCharStateComp>,
        specs::WriteStorage<'a, ControllerComponent>,
    );

    fn run(&mut self, (mut char_state_storage, mut controller_storage): Self::SystemData) {
        for mut controller in (&mut controller_storage).join() {
            ControllerIntentionToCharTarget::controller_intention_to_char_target(
                controller,
                &mut char_state_storage,
            );
            controller.intention = None;
        }
    }
}
