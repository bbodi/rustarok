use crate::components::char::{
    CharDir, CharEntityId, CharOutlook, CharType, JobId, ServerEntityId, Team,
};
use crate::components::snapshot::CharSnapshot;
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

#[derive(Debug, Serialize, Deserialize)]
pub struct ServerEntityState {
    pub id: ServerEntityId,
    pub char_snapshot: CharSnapshot,
}

#[derive(Debug, EnumDiscriminants, EnumCount, Serialize, Deserialize)]
pub enum FromServerPacket {
    Init {
        map_name: String,
        start_x: f32,
        start_y: f32,
    },
    Pong {
        server_tick: u64,
    },
    Ack {
        cid: u32,
        sent_at: u128,
        entries: Vec<ServerEntityState>,
    },
    NewEntity {
        id: ServerEntityId,
        name: String,
        team: Team,
        typ: CharType,
        outlook: CharOutlook,
        job_id: JobId,
        max_hp: i32,
        state: CharSnapshot,
    },
}

impl Packet for FromServerPacket {
    fn write_into(&self, buf: &mut SocketBuffer) {
        bincode::serialize_into(buf, self);
    }

    fn read_from(buf: &mut SocketBuffer) -> Result<Self, PacketReadErr> {
        return match bincode::deserialize_from(buf) {
            Ok(packet) => Ok(packet),
            Err(e) => Err(PacketReadErr::NotEnoughBytes),
        };
    }
}
