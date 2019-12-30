use crate::components::controller::PlayerIntention;
use crate::packets::SocketBuffer;
use serde::Deserialize;
use serde::Serialize;
use std::io::Error;
use strum_macros::EnumCount;
use strum_macros::EnumDiscriminants;

pub trait Packet: Sized {
    fn write_into(&self, buf: &mut SocketBuffer);
    fn read_from(buf: &mut SocketBuffer) -> Result<Self, PacketReadErr>;
}

#[derive(Debug, EnumDiscriminants, EnumCount, Serialize, Deserialize)]
pub enum ToServerPacket {
    Welcome {
        name: String,
        //        job: JobId
    },
    Ping,
    ReadyForGame,
    Intention {
        cid: u32,
        client_tick: u64,
        intention: PlayerIntention,
    },
}

pub enum PacketReadErr {
    NotEnoughBytes,
    InvalidValues,
}

impl Packet for ToServerPacket {
    fn write_into(&self, buf: &mut SocketBuffer) {
        bincode::serialize_into(buf, self);
        //        let discr: ToServerPacketDiscriminants = self.into();
        //        let packet_id = discr as u8 + 1;
        //        buf.write_u8(packet_id);
        //        match self {
        //            ToServerPacket::LocalError(_) => panic!("Pseudo packet is sent to client"),
        //            ToServerPacket::Welcome { name } => buf.write_str(name),
        //            ToServerPacket::Ping => {}
        //            ToServerPacket::ReadyForGame => {}
        //            ToServerPacket::Intention {
        //                cid,
        //                client_tick,
        //                intention,
        //            } => {
        //                buf.write_u32(*cid);
        //                buf.write_u64(*client_tick);
        //                match intention {
        //                    PlayerIntention::MoveTowardsMouse(vec2) => {
        //                        buf.write_u8(0);
        //                        buf.write_v2(vec2);
        //                    }
        //                    PlayerIntention::MoveTo(vec2) => {
        //                        buf.write_u8(1);
        //                        buf.write_v2(vec2);
        //                    }
        //                    PlayerIntention::AttackTowards(vec2) => {
        //                        buf.write_u8(3);
        //                        buf.write_v2(vec2);
        //                    }
        //                    PlayerIntention::Attack(_) => {}
        //                }
        //            }
        //        };
    }
    fn read_from(buf: &mut SocketBuffer) -> Result<ToServerPacket, PacketReadErr> {
        return match bincode::deserialize_from(buf) {
            Ok(packet) => Ok(packet),
            Err(e) => Err(PacketReadErr::NotEnoughBytes),
        };
        //        let packet_id = buf.read_u8() - 1;
        //        if packet_id >= ToServerPacket::count() as u8 {
        //            return Err(PacketReadErr::InvalidValues);
        //        }
        //        let packet = match unsafe { std::mem::transmute(packet_id) } {
        //            ToServerPacketDiscriminants::LocalError => {
        //                return Err(PacketReadErr::InvalidValues);
        //            }
        //            ToServerPacketDiscriminants::Ping => ToServerPacket::Ping,
        //            ToServerPacketDiscriminants::ReadyForGame => ToServerPacket::ReadyForGame,
        //            ToServerPacketDiscriminants::Welcome => {
        //                buf.ensure_size(2)?;
        //                ToServerPacket::Welcome {
        //                    name: buf.read_str()?.to_owned(),
        //                }
        //            }
        //            ToServerPacketDiscriminants::Intention => {
        //                buf.ensure_size(4 + 8 + 1 + 4 + 4)?;
        //                let cid = buf.read_u32();
        //                let client_tick = buf.read_u64();
        //                let intention_type = buf.read_u8();
        //                let intention = match intention_type {
        //                    0 => PlayerIntention::MoveTowardsMouse(buf.read_v2()),
        //                    1 => PlayerIntention::MoveTo(buf.read_v2()),
        //                    //                    2 => PlayerIntention::Attack(buf.read_v2()),
        //                    _ => PlayerIntention::AttackTowards(buf.read_v2()),
        //                };
        //                ToServerPacket::Intention {
        //                    cid,
        //                    client_tick,
        //                    intention,
        //                }
        //            }
        //        };
        //        Ok(packet)
    }
}
