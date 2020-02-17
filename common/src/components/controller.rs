use crate::common::Vec2;
use crate::components::char::{LocalCharEntityId, ServerEntityId};
use serde::Deserialize;
use serde::Serialize;
use specs::prelude::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PlayerIntention {
    /// param: direction vector between the char and the mouse world position, towards the mouse
    MoveTowardsMouse(Vec2),
    /// Move to the coordination, or if an enemy stands there, attack her.
    MoveTo(Vec2),
    Attack(LocalCharEntityId),
    /// Move to the coordination, attack any enemy on the way.
    AttackTowards(Vec2),
    //    /// bool = is self cast
    //    Casting(Skills, bool, Vec2),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ToServerPlayerIntention {
    /// param: direction vector between the char and the mouse world position, towards the mouse
    MoveTowardsMouse(Vec2),
    /// Move to the coordination, or if an enemy stands there, attack her.
    MoveTo(Vec2),
    Attack(ServerEntityId),
    /// Move to the coordination, attack any enemy on the way.
    AttackTowards(Vec2),
    //    /// bool = is self cast
    //    Casting(Skills, bool, Vec2),
}

// It can be a player, an AI, script etc
#[derive(Component)]
pub struct ControllerComponent {
    pub intention: Option<PlayerIntention>,
    pub controlled_entity: Option<LocalCharEntityId>,
}

impl ControllerComponent {
    pub fn new(controlled_entity: LocalCharEntityId) -> ControllerComponent {
        ControllerComponent {
            intention: None,
            controlled_entity: Some(controlled_entity),
        }
    }
}
