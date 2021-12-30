use std::{
    io::{Cursor, Read, Write},
    net::TcpStream,
    sync::mpsc::{Receiver, Sender, TryRecvError},
};

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use minecraft_varint::{VarIntRead, VarIntWrite};

use crate::{MyceliumRead, MyceliumWrite, DEFAULT_SERVER_NAME, SERVERS};

pub fn handle_client(bclient: TcpStream) {
    let bserver = TcpStream::connect(SERVERS.get(DEFAULT_SERVER_NAME).unwrap()).unwrap();
    // Server reading thread

    // Incoming
    let client = bclient.try_clone().unwrap();
    let server = bserver.try_clone().unwrap();
    // *client.unwrap() = TcpStream::connect("").unwrap();
    let (tx, rx) = std::sync::mpsc::channel::<TcpStream>();
    std::thread::spawn(move || handle_server(rx, server, client));

    // Client reading thread
    let mut player = PlayerConnection {
        client: bclient.try_clone().unwrap(),
        current_server: bserver.try_clone().unwrap(),
        tx: tx,
    };
    std::thread::spawn(move || player.handle());
}

struct PlayerConnection {
    current_server: TcpStream,
    tx: Sender<TcpStream>,
    client: TcpStream,
}

impl PlayerConnection {
    fn handle(&mut self) {
        // let mut current_server = server;
        let mut current_server_name = DEFAULT_SERVER_NAME.to_string();
        println!("New client connected.");

        loop {
            let len = match { self.client.read_var_u32() } {
                Ok(l) => l,
                Err(_) => continue,
            };

            let mut buf = vec![0; len as usize];
            self.client.read(&mut buf).unwrap();
            {
                // Copy of packet data to inspect without modifying the original buffer
                let mut copy_of_packet = Cursor::new(buf.clone());
                let id = copy_of_packet.read_var_u32().unwrap();
                // Tab complete
                if id == 0x01 {
                    let text = copy_of_packet.read_string().unwrap();
                    if text.starts_with("/join ") {
                        let mut buf = vec![];
                        buf.write_var_u32(0x0E).unwrap();
                        buf.write_var_u32(SERVERS.len() as u32).unwrap();
                        for server in SERVERS.iter() {
                            buf.write_string(server.0.to_string());
                        }
                        self.client.write_var_u32(buf.len() as u32).unwrap();
                        self.client.write_all(&buf).unwrap();
                    }
                    continue;
                }
                // Send message
                if id == 0x02 {
                    let msg = copy_of_packet.read_string().unwrap();
                    if msg.starts_with("/join ") {
                        let args = msg.split_ascii_whitespace().collect::<Vec<_>>();

                        let address = SERVERS.get(args[1]);
                        if let Some(address) = address {
                            if args[1] != current_server_name {
                                let new_server = TcpStream::connect(address);
                                if new_server.is_err() {
                                    self.send_chat_message(format!("§4Failed to connect to server, §r{}! (Tell the admins!).", new_server.unwrap_err()));
                                    continue;
                                }
                                let mut new_server = new_server.unwrap();
                                current_server_name = args[1].to_string();
                            
                                self.write_handshake(&mut new_server, 340, "localhost", 8001, 2);
                                {
                                    let mut v: Vec<u8> = vec![];
                                    v.write_var_u32(0x0).unwrap();
                                    v.write_string("F8H".to_string());
                                    new_server.write_var_u32(v.len() as u32).unwrap();
                                    new_server.write_all(&v).unwrap();
                                }
                                {
                                    let len = new_server.read_var_u32().unwrap();
                                    let id = new_server.read_var_u32().unwrap();
                                    dbg!(len, id);
                                    match id {
                                        0 => {
                                            let disconnect_reason = new_server.read_string().unwrap();
                                            self.send_chat_message(format!("§4Disconnected. reason: §r{}!", disconnect_reason));


                                            new_server.shutdown(std::net::Shutdown::Both).unwrap();
                                            continue;
                                        }
                                        2 => {
                                            let uuid = new_server.read_string().unwrap();
                                            let username = new_server.read_string().unwrap();
                                            self.current_server
                                                .shutdown(std::net::Shutdown::Both)
                                                .unwrap();

                                            dbg!(uuid, username);

                                            // dbg!("read", new_server.read(&mut []).unwrap());
                                        }
                                        _ => unimplemented!(),
                                    }
                                }
                                self.tx.send(new_server.try_clone().unwrap()).unwrap();
                                self.current_server = new_server;

                                println!("Finished connecting");
                                // This stops it from forwarding to the server that you executed the /join command
                                continue;
                            } else {
                                self.send_chat_message(format!("Already connected to §a{}!", args[1]));
                                continue;
                            }
                        }
                    }
                }
                // For intercepting normal sending.
                match id {
                    _ => {
                        self.current_server.write_var_u32(len).unwrap();
                        self.current_server.write_all(&buf).unwrap();
                        // dbg!(id);
                        // println!("-> ID: 0x{:X} {}", id, id);
                    }
                }
            }
        }
    }
    fn send_chat_message<T: ToString>(&mut self, chat: T) {
        let mut buf = vec![];

        buf.write_var_u32(0x0f).unwrap();
        buf.write_string(format!(r#"{{"text": "{}"}}"#, chat.to_string()));
        buf.write_i8(0).unwrap();

        self.client.write_var_u32(buf.len() as u32).unwrap();
        self.client.write_all(&buf).unwrap();  
    }
    fn write_handshake(&self, stream: &mut TcpStream, pv: u32, address: &str, port: u16, next_state: u32) {
        let mut v: Vec<u8> = vec![];

        v.write_var_u32(0x0).unwrap();
        v.write_var_u32(pv).unwrap();
        v.write_string(address.to_string());
        v.write_u16::<BigEndian>(port).unwrap();
        v.write_var_u32(next_state).unwrap();

        stream.write_var_u32(v.len() as u32).unwrap();
        stream.write_all(&v).unwrap();
    }
  

}
pub fn handle_server(rx: Receiver<TcpStream>, mut server: TcpStream, mut client: TcpStream) {
    {
        let mut recently_transferred = false;
        loop {
            match rx.try_recv() {
                Ok(new_server) => {
                    dbg!(&new_server);

                    server = new_server;
                    recently_transferred = true;
                }
                // Not really an error
                Err(TryRecvError::Empty) => {}
                Err(e) => panic!("Sender error; {}", e),
            }

            let len = match { server.read_var_u32() } {
                Ok(l) => l,
                Err(_) => continue,
            };

            let mut buf = vec![0; len as usize];
            server.read_exact(&mut buf).unwrap();
            {
                let mut b = Cursor::new(buf.clone());

                let id = b.read_var_u32().unwrap();
                // Printing takes too long when printing chunk data. Its useless to the human eye anyway.
                if id == 0x23 {
                    client.write_var_u32(len).unwrap();
                    client.write_all(&buf).unwrap();
                    if recently_transferred {
                        let _eid = b.read_i32::<BigEndian>().unwrap();
                        let gamemode = b.read_u8().unwrap();
                        let dimension = b.read_i32::<BigEndian>().unwrap();
                        let difficulty = b.read_u8().unwrap();
                        let _maxplayers = b.read_u8().unwrap();
                        let level_type = b.read_string().unwrap_or("default".to_string());
                        let mut send_respawn = |dimension| {
                            // let difficulty = 1;
                            // let gamemode = 1;
                            // let level_type = "flat";

                            let mut v = vec![];
                            v.write_var_u32(0x35).unwrap();
                            v.write_i32::<BigEndian>(dimension).unwrap();
                            v.write_u8(difficulty).unwrap();
                            v.write_u8(gamemode).unwrap();
                            v.write_string(level_type.to_string());

                            client.write_var_u32(v.len() as u32).unwrap();
                            client.write_all(&v).unwrap();
                        };
                        let fake_dimension = match dimension {
                            -1 => 1,
                            0 => -1,
                            1 => 0,
                            _ => panic!(),
                        };
                        send_respawn(fake_dimension);
                        send_respawn(dimension);
                        recently_transferred = false;
                    }
                }

                match id {
                    // Join game
                    0x23 => {}
                    _ => {
                        client.write_var_u32(len).unwrap();
                        client.write_all(&buf).unwrap();
                    }
                }
            }
        }
    }
}
