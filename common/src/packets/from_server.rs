use crate::common::{LocalTime, ServerTime, SimulationTick};
use crate::components::char::{
    CharDir, CharOutlook, CharType, JobId, LocalCharEntityId, LocalCharStateComp, ServerCharState,
    ServerEntityId, Team,
};
use crate::config::CommonConfigs;
use crate::packets::to_server::{Packet, PacketReadErr};
use crate::packets::SocketBuffer;
use serde::export::TryFrom;
use serde::Deserialize;
use serde::Serialize;
use std::io::{Error, ErrorKind};
use std::time::Instant;
use strum::EnumCount;
use strum_macros::EnumCount;
use strum_macros::EnumDiscriminants;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServerEntityState {
    pub id: ServerEntityId,
    pub char_snapshot: ServerCharState,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServerEntityStateLocal {
    pub id: ServerEntityId,
    pub char_snapshot: LocalCharStateComp,
}

#[derive(Clone, Debug, EnumDiscriminants, EnumCount, Serialize, Deserialize)]
pub enum FromServerPacket {
    Init {
        map_name: String,
        start_x: f32,
        start_y: f32,
    },
    Configs(CommonConfigs),
    Pong {
        server_time: ServerTime,
        server_tick: SimulationTick,
    },
    Ack {
        cid: u32,
        entries: Vec<ServerEntityState>,
    },
    NewEntity {
        id: ServerEntityId,
        name: String,
        team: Team,
        typ: CharType,
        outlook: CharOutlook,
        job_id: JobId,
        state: ServerCharState,
    },
}

impl Packet for FromServerPacket {
    fn write_into(&self, buf: &mut SocketBuffer) -> bincode::Result<()> {
        bincode::serialize_into(buf, self)
    }

    fn read_from(buf: &mut SocketBuffer) -> Result<Self, PacketReadErr> {
        return match bincode::deserialize_from(buf) {
            Ok(packet) => Ok(packet),
            Err(e) => Err(PacketReadErr::NotEnoughBytes),
        };
    }
}
