use crate::common::{v2, Vec2};
use crate::components::char::{AuthorizedCharStateComponent, CharDir, CharState};
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

    pub fn from_buffer(buf: &mut SocketBuffer) -> CharSnapshot {
        CharSnapshot {
            state: AuthorizedCharStateComponent::from_buffer(buf),
        }
    }

    pub fn write_into_buffer(&self, buf: &mut SocketBuffer) {
        self.state.write_into_buffer(buf);
    }
}

impl Default for CharSnapshot {
    fn default() -> Self {
        CharSnapshot {
            state: Default::default(),
        }
    }
}

#[derive(Default, Debug)]
pub struct WorldSnapshot {
    pub desktop_snapshot: CharSnapshot,
}

impl WorldSnapshot {
    pub fn from_buffer(buf: &mut SocketBuffer) -> WorldSnapshot {
        WorldSnapshot {
            desktop_snapshot: CharSnapshot::from_buffer(buf),
        }
    }

    pub fn write_into_buffer(&self, buf: &mut SocketBuffer) {
        self.desktop_snapshot.write_into_buffer(buf);
    }
}
