use crate::common::{v2, Vec2};
use crate::components::char::{AuthorizedCharStateComponent, CharDir, CharState, ServerEntityId};
use crate::packets::from_server::ServerEntityState;
use crate::packets::SocketBuffer;
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CharSnapshot {
    pub state: AuthorizedCharStateComponent,
}

impl CharSnapshot {
    pub fn from(char_state: &AuthorizedCharStateComponent) -> CharSnapshot {
        CharSnapshot {
            state: char_state.clone(),
        }
    }
}

impl Default for CharSnapshot {
    fn default() -> Self {
        CharSnapshot {
            state: Default::default(),
        }
    }
}
