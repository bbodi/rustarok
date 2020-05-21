use crate::attack::HpModificationResultType;
use crate::common::{GameTime, Local, NetworkedObj, Remote, SimulationTick};
use crate::components::char::{
    CharDir, CharOutlook, CharType, EntityId, JobId, LocalCharStateComp, Team,
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
pub struct ServerEntityState<T: 'static + NetworkedObj> {
    pub id: EntityId<Remote>,
    pub char_snapshot: LocalCharStateComp<T>,
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
        server_time: GameTime<Remote>,
        server_tick: SimulationTick,
    },
    Ack {
        cid: u32,
        entries: Vec<ServerEntityState<Remote>>,
    },
    NewEntity {
        id: EntityId<Remote>,
        name: String,
        team: Team,
        typ: CharType,
        outlook: CharOutlook,
        job_id: JobId,
        state: LocalCharStateComp<Remote>,
    },
    PlayerDisconnected(EntityId<Remote>),
    Damage {
        src_id: EntityId<Remote>,
        dst_id: EntityId<Remote>,
        typ: HpModificationResultType,
    }, // EntityDisappeared {
       //     id: EntityId<Remote>,
       // },
       // EntityAppeared {
       //     id: EntityId<Remote>,
       // },
       // died?
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
