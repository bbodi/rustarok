use crate::common::{v2, Vec2};
use crate::grf::binary_reader::BinaryReader;
use crate::packets::to_server::{Packet, PacketReadErr};
use hexplay::{HexView, HexViewBuilder};
use serde::export::fmt::Debug;
use std::io::prelude::*;
use std::io::{Error, Read};
use std::net::TcpStream;
use std::sync::mpsc::RecvError;
use std::time::Duration;

pub mod from_server;
pub mod to_server;

pub struct SocketBuffer {
    buf: [u8; 2048],
    /// pointer at which the OS writes/reads the data during send/recv
    os_pointer: usize,
    /// pointer at which the application writes/reads the data during send/recv
    user_pointer: usize,
}

impl Write for SocketBuffer {
    fn write(&mut self, src: &[u8]) -> Result<usize, Error> {
        //        pub fn write_u8(&mut self, value: u8) {
        //            self.buff[self.user_pointer] = value;
        //            self.user_pointer += 1;
        //        }
        let from = self.user_pointer;
        let len = src.len();
        let to = self.user_pointer + len;
        self.buf[from..to].copy_from_slice(src);
        self.user_pointer += len;
        Ok(len)
    }

    fn flush(&mut self) -> Result<(), Error> {
        Ok(())
    }
}

impl Read for SocketBuffer {
    fn read(&mut self, dst: &mut [u8]) -> Result<usize, Error> {
        let from = self.user_pointer;
        let full_len = self.os_pointer - self.user_pointer;
        let copy_len = full_len.min(dst.len());
        let to = from + copy_len;

        dst[0..copy_len].copy_from_slice(&mut self.buf[from..to]);
        self.user_pointer += copy_len;
        return Ok(copy_len);
    }
}

impl SocketBuffer {
    pub fn new() -> SocketBuffer {
        SocketBuffer {
            buf: [0; 2048],
            os_pointer: 0,
            user_pointer: 0,
        }
    }

    pub fn read_incoming_data(
        &mut self,
        socket_stream: &mut TcpStream,
    ) -> Result<usize, std::io::Error> {
        let n = socket_stream.read(&mut self.buf[self.os_pointer..])?;
        self.os_pointer += n;
        Ok(n)
    }

    pub fn incoming_data_len(&self) -> usize {
        self.os_pointer - self.user_pointer
    }

    pub fn outgoing_data_len(&self) -> usize {
        self.user_pointer - self.os_pointer
    }

    pub fn eof(&self) -> bool {
        self.os_pointer == self.user_pointer
    }

    pub fn get_debug_string_for_incoming_data(&self) -> HexView {
        let data = &self.buf[self.user_pointer..self.os_pointer];
        return HexViewBuilder::new(data)
            .address_offset(40)
            .row_width(16)
            .finish();
    }

    pub fn get_debug_string_for_outgoing_data(&self) -> HexView {
        let data = &self.buf[self.os_pointer..self.user_pointer];
        return HexViewBuilder::new(data)
            .address_offset(40)
            .row_width(16)
            .finish();
    }

    #[inline]
    pub fn send_outgoing_data(
        &mut self,
        socket_stream: &mut TcpStream,
    ) -> Result<usize, std::io::Error> {
        let mut all_sent_data = 0;
        while !self.eof() {
            let sending_buf = &self.buf[self.os_pointer..self.user_pointer];
            let sent_data = socket_stream.write(sending_buf)?;
            self.os_pointer += sent_data;
            all_sent_data += sent_data;
        }
        Ok(all_sent_data)
    }

    pub fn read_u8(&mut self) -> u8 {
        let result = self.buf[self.user_pointer];
        self.user_pointer += 1;
        return result;
    }

    pub fn read_f32(&mut self) -> f32 {
        let result =
            unsafe { *(self.buf.as_ptr().offset(self.user_pointer as isize) as *const f32) };
        self.user_pointer += 4;
        return result;
    }

    pub fn read_v2(&mut self) -> Vec2 {
        v2(self.read_f32(), self.read_f32())
    }

    pub fn read_i32(&mut self) -> i32 {
        let result =
            unsafe { *(self.buf.as_ptr().offset(self.user_pointer as isize) as *const i32) };
        self.user_pointer += 4;
        return result;
    }

    pub fn read_u32(&mut self) -> u32 {
        let result =
            unsafe { *(self.buf.as_ptr().offset(self.user_pointer as isize) as *const u32) };
        self.user_pointer += 4;
        return result;
    }

    pub fn read_u64(&mut self) -> u64 {
        let result =
            unsafe { *(self.buf.as_ptr().offset(self.user_pointer as isize) as *const u64) };
        self.user_pointer += 8;
        return result;
    }

    pub fn read_u16(&mut self) -> u16 {
        let result =
            unsafe { *(self.buf.as_ptr().offset(self.user_pointer as isize) as *const u16) };
        self.user_pointer += 2;
        return result;
    }

    pub fn write_u8(&mut self, value: u8) {
        self.buf[self.user_pointer] = value;
        self.user_pointer += 1;
    }

    pub fn write_f32(&mut self, value: f32) {
        unsafe {
            *(self.buf.as_mut_ptr().offset(self.user_pointer as isize) as *mut f32) = value;
        }
        self.user_pointer += 4;
    }

    pub fn write_v2(&mut self, value: &Vec2) {
        self.write_f32(value.x);
        self.write_f32(value.y);
    }

    pub fn write_i32(&mut self, value: i32) {
        unsafe {
            *(self.buf.as_mut_ptr().offset(self.user_pointer as isize) as *mut i32) = value;
        }
        self.user_pointer += 4;
    }

    pub fn write_u32(&mut self, value: u32) {
        unsafe {
            *(self.buf.as_mut_ptr().offset(self.user_pointer as isize) as *mut u32) = value;
        }
        self.user_pointer += 4;
    }

    pub fn write_u64(&mut self, value: u64) {
        unsafe {
            *(self.buf.as_mut_ptr().offset(self.user_pointer as isize) as *mut u64) = value;
        }
        self.user_pointer += 8;
    }

    pub fn write_u16(&mut self, value: u16) {
        unsafe {
            *(self.buf.as_mut_ptr().offset(self.user_pointer as isize) as *mut u16) = value;
        }
        self.user_pointer += 2;
    }

    pub fn read_str(&mut self) -> Result<&str, PacketReadErr> {
        let len = self.read_u16();
        if len > 1024 {
            return Err(PacketReadErr::InvalidValues);
        }
        self.ensure_size(len as usize)?;
        let start_index = self.user_pointer;
        let end_index = self.user_pointer + len as usize;
        self.user_pointer = end_index;
        return match std::str::from_utf8(&self.buf[start_index..end_index]) {
            Ok(str) => Ok(str),
            _ => Err(PacketReadErr::InvalidValues),
        };
    }

    pub fn write_str(&mut self, text: &str) {
        let len = text.len();
        self.write_u16(len as u16);
        let start = self.user_pointer;
        let end = self.user_pointer + text.len();
        self.buf[start..end].clone_from_slice(text.as_bytes());
        self.user_pointer = end;
    }

    pub fn reset(&mut self) {
        self.os_pointer = 0;
        self.user_pointer = 0;
    }

    #[inline]
    pub fn ensure_size(&self, requried_size: usize) -> Result<(), PacketReadErr> {
        if self.os_pointer - self.user_pointer >= requried_size {
            Ok(())
        } else {
            Err(PacketReadErr::NotEnoughBytes)
        }
    }
}

pub struct RemoteSocket {
    socket_stream: TcpStream,
    out_buff: SocketBuffer,
    in_buff: SocketBuffer,
}

impl RemoteSocket {
    pub fn new(stream: TcpStream) -> RemoteSocket {
        RemoteSocket {
            socket_stream: stream,
            out_buff: SocketBuffer::new(),
            in_buff: SocketBuffer::new(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SocketId(usize);

impl SocketId {
    pub fn as_usize(&self) -> usize {
        self.0
    }
}

pub struct PacketHandlerThread<I, O>
where
    I: Send + Packet + Debug + 'static,
    O: Send + Packet + Debug + 'static,
{
    incoming_channel: std::sync::mpsc::Receiver<(SocketId, NetworkTrafficEvent<I>)>,
    outgoing_channel: std::sync::mpsc::Sender<NetworkTrafficHandlerMsg<O>>,
    sockets: Vec<Option<()>>,
}

impl<I, O> PacketHandlerThread<I, O>
where
    I: Send + Packet + Debug + 'static,
    O: Send + Packet + Debug + 'static,
{
    pub fn start_thread(socket_capacity: usize) -> PacketHandlerThread<I, O> {
        let (send_to_incoming_ch, read_from_incoming_ch) =
            std::sync::mpsc::channel::<(SocketId, NetworkTrafficEvent<I>)>();
        let (send_to_outgoing_ch, read_from_outgoing_ch) =
            std::sync::mpsc::channel::<NetworkTrafficHandlerMsg<O>>();
        std::thread::spawn(move || {
            network_traffic_handler(send_to_incoming_ch, read_from_outgoing_ch)
        });
        PacketHandlerThread {
            incoming_channel: read_from_incoming_ch,
            outgoing_channel: send_to_outgoing_ch,
            sockets: Vec::with_capacity(socket_capacity),
        }
    }

    pub fn handle_socket(&mut self, socket_stream: TcpStream) -> SocketId {
        socket_stream.set_nonblocking(true);
        socket_stream.set_nodelay(true);
        // TODO: what is it? is it increasing infinitely??
        let id = SocketId(self.sockets.len());
        self.sockets.push(Some(()));
        self.outgoing_channel
            .send(NetworkTrafficHandlerMsg::NewConnection(id, socket_stream))
            .unwrap();
        return id;
    }

    pub fn send(&self, socket_id: SocketId, packet: O) {
        self.outgoing_channel
            .send(NetworkTrafficHandlerMsg::SendPacket(socket_id, packet))
            .unwrap();
    }

    pub fn receive_into(&self, out: &mut Vec<(SocketId, NetworkTrafficEvent<I>)>) {
        loop {
            if let Ok(id_and_packet) = self.incoming_channel.try_recv() {
                out.push(id_and_packet)
            } else {
                break;
            }
        }
    }

    pub fn receive_exact_into(
        &self,
        out: &mut Vec<(SocketId, NetworkTrafficEvent<I>)>,
        count: usize,
    ) {
        for i in 0..count {
            if let Ok(id_and_packet) = self.incoming_channel.try_recv() {
                out.push(id_and_packet)
            } else {
                break;
            }
        }
    }
}

#[derive(Debug)]
enum NetworkTrafficHandlerMsg<O>
where
    O: Send + Packet + Debug + 'static,
{
    NewConnection(SocketId, TcpStream),
    RemoveConnection(SocketId),
    SendPacket(SocketId, O),
}

#[derive(Debug)]
pub enum NetworkTrafficEvent<P: Send + Packet + Debug + 'static> {
    LocalError(std::io::Error),
    Disconnected,
    OutgoingTraffic { sent_data_len: usize },
    IncomingTraffic { received_data_len: usize },
    Packet(P),
}

fn network_traffic_handler<I, O>(
    // this is the channel the client app reads for incoming packets
    send_to_incoming_ch: std::sync::mpsc::Sender<(SocketId, NetworkTrafficEvent<I>)>,
    // the channel which is filled by the client app with outgoing packets
    read_from_outgoing_ch: std::sync::mpsc::Receiver<NetworkTrafficHandlerMsg<O>>,
) where
    I: Send + Packet + Debug + 'static,
    O: Send + Packet + Debug + 'static,
{
    let mut sockets: Vec<Option<RemoteSocket>> = Vec::with_capacity(64);
    loop {
        let command = read_from_outgoing_ch.try_recv();
        match command {
            Ok(NetworkTrafficHandlerMsg::NewConnection(socket_id, socket_stream)) => {
                log::info!("adding new connection: {:?}", socket_id);
                if socket_id.0 >= sockets.len() {
                    sockets.push(Some(RemoteSocket::new(socket_stream)));
                } else {
                    sockets[socket_id.0] = Some(RemoteSocket::new(socket_stream));
                }
            }
            Ok(NetworkTrafficHandlerMsg::RemoveConnection(socket_id)) => {
                sockets[socket_id.0] = None;
            }
            Ok(NetworkTrafficHandlerMsg::SendPacket(socket_id, packet)) => {
                if let Some(socket) = sockets[socket_id.0].as_mut() {
                    let socket_buffer = &mut socket.out_buff;
                    log::trace!("Outgoing Packet: {:?}", packet);
                    if let Err(e) = packet.write_into(socket_buffer) {
                        sockets[socket_id.0] = None;
                        send_to_incoming_ch.send((
                            SocketId(socket_id.0),
                            NetworkTrafficEvent::LocalError(std::io::Error::new(
                                std::io::ErrorKind::Other,
                                e.to_string(),
                            )),
                        ));
                    }
                }
            }
            Err(e) => {}
        }
        for i in 0..sockets.len() {
            if let Some(ref mut socket) = &mut sockets[i] {
                if !socket.out_buff.eof() {
                    log::trace!(
                        "OUTGOING\n{}",
                        socket.out_buff.get_debug_string_for_outgoing_data()
                    );
                    let send_result = socket
                        .out_buff
                        .send_outgoing_data(&mut socket.socket_stream);
                    match send_result {
                        Err(e) => {
                            sockets[i] = None;
                            send_to_incoming_ch
                                .send((SocketId(i), NetworkTrafficEvent::LocalError(e)));
                        }
                        Ok(sent_bytes) => {
                            if socket.out_buff.eof() {
                                // all the data has been sent, the buffer is empty
                                socket.out_buff.reset();
                            }
                            send_to_incoming_ch.send((
                                SocketId(i),
                                NetworkTrafficEvent::OutgoingTraffic {
                                    sent_data_len: sent_bytes,
                                },
                            ));
                        }
                    }
                }
            }
        }
        'sockets_loop: for i in 0..sockets.len() {
            if let Some(ref mut socket) = &mut sockets[i] {
                let len = match socket.in_buff.read_incoming_data(&mut socket.socket_stream) {
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        continue;
                    }
                    Err(e) => {
                        log::error!("Error during socket reading: {}", e);
                        send_to_incoming_ch.send((SocketId(i), NetworkTrafficEvent::LocalError(e)));
                        sockets[i] = None;
                        continue;
                    }
                    Ok(0) => {
                        let socket_id = SocketId(i);
                        send_to_incoming_ch.send((socket_id, NetworkTrafficEvent::Disconnected));
                        sockets[i] = None;
                        continue;
                    }
                    Ok(len) => len,
                };
                log::trace!(
                    "INCOMING\n{}",
                    socket.in_buff.get_debug_string_for_incoming_data()
                );
                send_to_incoming_ch.send((
                    SocketId(i),
                    NetworkTrafficEvent::IncomingTraffic {
                        received_data_len: len,
                    },
                ));

                let socket_id = SocketId(i);
                while !socket.in_buff.eof() {
                    match I::read_from(&mut socket.in_buff) {
                        Ok(packet) => {
                            log::trace!("Incoming Packet: {:?}", packet);
                            send_to_incoming_ch
                                .send((socket_id, NetworkTrafficEvent::Packet(packet)));
                        }
                        Err(err) => {
                            match err {
                                PacketReadErr::NotEnoughBytes => {
                                    // ok
                                }
                                PacketReadErr::InvalidValues => {
                                    log::error!("Socket({}) sent invalid values, close it", i);
                                    sockets[i] = None;
                                }
                            }
                            continue 'sockets_loop;
                        }
                    }
                }
                if socket.in_buff.eof() {
                    // all the data has read out, the buffer is empty
                    socket.in_buff.reset();
                }
            }
        } // 'sockets_loop
        std::thread::sleep(Duration::from_millis(10));
    }
}
