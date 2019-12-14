use crate::common::Vec2;
use crate::components::char::CharEntityId;
use specs::prelude::*;

#[derive(Clone, Debug)]
pub enum PlayerIntention {
    /// param: direction vector between the char and the mouse world position, towards the mouse
    MoveTowardsMouse(Vec2),
    /// Move to the coordination, or if an enemy stands there, attack her.
    MoveTo(Vec2),
    Attack(CharEntityId),
    /// Move to the coordination, attack any enemy on the way.
    AttackTowards(Vec2),
    //    /// bool = is self cast
    //    Casting(Skills, bool, Vec2),
}

// It can be a player, an AI, script etc
#[derive(Component)]
pub struct ControllerComponent {
    pub next_action: Option<PlayerIntention>,
    pub controlled_entity: CharEntityId,
}
