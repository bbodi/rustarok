use crate::components::char::{
    CharDir, CharEntityId, CharOutlook, CharType, JobId, ServerEntityId, Team,
};
use crate::components::snapshot::{CharSnapshot, WorldSnapshot};
use crate::packets::to_server::{Packet, PacketReadErr};
use crate::packets::SocketBuffer;
use crate::serde_remote::MyIoErrorKind;
use serde::export::TryFrom;
use serde::Deserialize;
use serde::Serialize;
use std::io::{Error, ErrorKind};
use strum::EnumCount;
use strum_macros::EnumCount;
use strum_macros::EnumDiscriminants;

#[derive(Debug, Serialize, Deserialize)]
pub enum AckEntry {
    EntityState {
        id: ServerEntityId,
        char_snapshot: CharSnapshot,
    }, // entity moved out, moved in, atk speed change, status change etc
}

#[derive(Debug, EnumDiscriminants, EnumCount, Serialize, Deserialize)]
pub enum FromServerPacket {
    LocalError(Option<MyIoErrorKind>),
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
        ack_tick: u64,
        entries: Vec<AckEntry>,
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

    fn new_error_packet(e: Option<Error>) -> Self {
        FromServerPacket::LocalError(e.map(|it| unsafe { std::mem::transmute(it.kind()) }))
    }
}
