use std::{
    io::{Cursor, Read, Write, BufRead},
    net::TcpStream,
    sync::mpsc::{Receiver, Sender, TryRecvError},
};

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use hexdump::hexdump;
use minecraft_varint::{VarIntRead, VarIntWrite};
use nbt::Blob;

use crate::{
    handlers,
    packets::{self, MC1_17_1},
    MyceliumRead, MyceliumWrite, DEFAULT_SERVER_NAME, SERVERS,
};

pub fn handle_client(bclient: TcpStream) {
    let bserver = TcpStream::connect(SERVERS.get(DEFAULT_SERVER_NAME).unwrap()).unwrap();
    // Server reading thread

    // Incoming
    let client = bclient.try_clone().unwrap();
    let server = bserver.try_clone().unwrap();
    // *client.unwrap() = TcpStream::connect("").unwrap();
    let (tx, rx) = std::sync::mpsc::channel::<DataChange>();
    let mut threads = vec![];
    threads.push((
        0,
        std::thread::spawn(move || handle_server(rx, server, client)),
    ));

    // Client reading thread
    let mut player = PlayerConnection {
        client: bclient.try_clone().unwrap(),
        current_server: bserver.try_clone().unwrap(),
        tx: tx,
    };
    threads.push((1, std::thread::spawn(move || player.handle())));
    'f: for (id, thread) in threads {
        // thread.join().unwrap();
        match thread.join() {
            _ => {
                println!("JOINED!");
                if id == 0 {
                    bserver.shutdown(std::net::Shutdown::Both).unwrap();
                } else {
                    bclient.shutdown(std::net::Shutdown::Both).unwrap();
                }
                break 'f;
            }
        }
    }
}
pub enum DataChange {
    TcpStream(TcpStream),
    HandShook(
        /// pv
        u32,
    ),
}
struct PlayerConnection {
    current_server: TcpStream,
    tx: Sender<DataChange>,
    client: TcpStream,
}

impl PlayerConnection {
    fn handle(&mut self) {
        // let mut current_server = server;
        let mut current_server_name = DEFAULT_SERVER_NAME.to_string();
        println!("New client connected.");
        let mut state = 0;
        let mut pv = 0;

        loop {
            if self.client.read(&mut []).is_err() {
                println!("Braking.");
                break;
            }
            let len = match { self.client.read_var_u32() } {
                Ok(l) => l,
                Err(_) => continue,
            };

            let mut buf = vec![0; len as usize];
            self.client.read(&mut buf).unwrap();
            {
                // Copy of packet data to inspect without modifying the original buffer
                let mut read_only_copy = Cursor::new(buf.clone());
                let id = read_only_copy.read_var_u32().unwrap();
                // dbg!(id);
                if id == packets::get_chat_c2s(pv) {
                    println!("AAA");
                    let msg = read_only_copy.read_string().unwrap();
                    dbg!(&msg);
                    if msg.starts_with("/server ") {
                        let args = msg.split_ascii_whitespace().collect::<Vec<_>>();

                        let address = SERVERS.get(args[1]);
                        if let Some(address) = address {
                            if args[1] != current_server_name {
                                let new_server = TcpStream::connect(address);
                                if new_server.is_err() {
                                    self.send_chat_message(format!(
                                        "§4Failed to connect to server, §r{}! (Tell the admins!).",
                                        new_server.unwrap_err()
                                    ));
                                    continue;
                                }
                                let mut new_server = new_server.unwrap();
                                current_server_name = args[1].to_string();

                                // self.write_handshake(&mut new_server, 340, "localhost", 8001, 2);
                                self.write_handshake(&mut new_server, pv, "localhost", 8001, 2);

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
                                            let disconnect_reason =
                                                new_server.read_string().unwrap();
                                            self.send_chat_message(format!(
                                                "§4Disconnected. reason: §r{}!",
                                                disconnect_reason
                                            ));

                                            new_server.shutdown(std::net::Shutdown::Both).unwrap();
                                            continue;
                                        }
                                        2 => {
                                            // let uuid = new_server.read_string().unwrap();
                                            if pv > packets::MC1_15_2 {
                                                let uuid =
                                                    new_server.read_i128::<BigEndian>().unwrap();
                                                dbg!(uuid);
                                            } else {
                                                let uuid = new_server.read_string().unwrap();
                                                dbg!(uuid);
                                            }

                                            // let uuid = new_server.read_i128::<BigEndian>().unwrap();
                                            let username = new_server.read_string().unwrap();
                                            

                                            self.current_server
                                                .shutdown(std::net::Shutdown::Both)
                                                .unwrap();
                                            dbg!(username);

                                            // dbg!("read", new_server.read(&mut []).unwrap());
                                        }
                                        _ => unimplemented!(),
                                    }
                                }
                                self.tx
                                    .send(handlers::DataChange::TcpStream(
                                        new_server.try_clone().unwrap(),
                                    ))
                                    .unwrap();
                                self.current_server = new_server;

                                println!("Finished connecting");
                                // This stops it from forwarding to the server that you executed the /join command
                                continue;
                            } else {
                                self.send_chat_message(format!(
                                    "Already connected to §a{}!",
                                    args[1]
                                ));
                                continue;
                            }
                        }
                    }
                }

                if state == 0 && id == 0 {
                    pv = read_only_copy.read_var_u32().unwrap();
                    let address = read_only_copy.read_string().unwrap();
                    let port = read_only_copy.read_u16::<BigEndian>().unwrap();
                    let next_state = read_only_copy.read_var_u32().unwrap();
                    state = next_state;

                    self.tx.send(DataChange::HandShook(pv)).unwrap();

                    // dbg!(pv,address,port,next_state);
                    println!("Zero id");
                }
                if state == 2 && id == 0x26   {
                    let mut v: Vec<u8> = vec![];
                    v.write_var_u32(0x0a).unwrap(); // Plugin channel c2s

                    v.write_string("minecraft:register".to_string());
                    v.write_string("mycelium:data".to_string());
                    self.current_server.write_var_u32(v.len() as u32).unwrap();
                    self.current_server.write_all(&v).unwrap();
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
        println!("Shouldve logged {}", chat.to_string());
        // let mut buf = vec![];

        // buf.write_var_u32(0x0f).unwrap();
        // buf.write_string(format!(r#"{{"text": "{}"}}"#, chat.to_string()));
        // buf.write_i8(0).unwrap();

        // self.client.write_var_u32(buf.len() as u32).unwrap();
        // self.client.write_all(&buf).unwrap();
    }
    fn write_handshake(
        &self,
        stream: &mut TcpStream,
        pv: u32,
        address: &str,
        port: u16,
        next_state: u32,
    ) {
        let mut v: Vec<u8> = vec![];

        v.write_var_u32(0x0).unwrap();
        v.write_var_u32(pv).unwrap();
        v.write_string(address.to_string());
        v.write_u16::<BigEndian>(port).unwrap();
        v.write_var_u32(next_state).unwrap();

        stream.write_var_u32(v.len() as u32).unwrap();
        stream.write_all(&v).unwrap();
    }
    fn transfer(&mut self) {
        
    }
}
pub fn handle_server(rx: Receiver<DataChange>, mut server: TcpStream, mut client: TcpStream) {
    {
        let mut recently_transferred = false;
        let mut pv = 0;
        let mut has_sent_plugins = false;
        loop {
            match rx.try_recv() {
                Ok(data_change) => {
                    // dbg!(&new_server);
                    match data_change {
                        DataChange::TcpStream(new_server) => {
                            server = new_server;
                            recently_transferred = true;
                            has_sent_plugins = false;
                        }
                        DataChange::HandShook(npv) => pv = npv,
                    }
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
                let mut read_only_copy = Cursor::new(buf.clone());

                let id = read_only_copy.read_var_u32().unwrap();
                // println!("{:X}", id);
                if pv != 0 && !has_sent_plugins {
                    {
                        let mut v: Vec<u8> = vec![];
                        v.write_var_u32(0x0a).unwrap(); // Plugin channel c2s

                        v.write_string("minecraft:register".to_string());
                        v.write_all(b"bungeecord:main\0\0").unwrap();
                        server.write_var_u32(v.len() as u32).unwrap();
                        server.write_all(&v).unwrap();
                    }
                    has_sent_plugins = true;
                }
                if id == 0x18 {
                    println!("PLUGIN");
                    let id = read_only_copy.read_string().unwrap();
                    dbg!(&id);
                    
                    let mut bytes = vec![0; (len - (id.len() as u32)).try_into().unwrap()];
                    read_only_copy.read(&mut bytes).unwrap();
                    
                    if id == "bungeecord:main" {
                        let segments = bytes.split(|c| *c == 0u8).skip(1).collect::<Vec<_>>();
                        for segment in segments {
                            if segment.len() == 0 {
                                continue
                            }
                            let len = segment[0];
                            let text = std::str::from_utf8(&segment[1..]).unwrap();
                            dbg!(text);
                        }
                        // dbg!(segments);
                    }
                    
                }
                match id {
                    // Join game
                    // 0x18 => {
                    //     println!("PLUGIN");
                    // }
                    i if pv != 0 && i == packets::get_join_game(pv) => {
                        client.write_var_u32(len).unwrap();
                        client.write_all(&buf).unwrap();
                        println!("JOINING GAME");
                        if recently_transferred {
                            // let x= vec![];

                            // hexdump(&b.get_ref());
                            // let dim: nbt::Map<String, String> = nbt::from_reader(b).unwrap();
                            // dbg!(&dim);
                            println!("pv {}", pv);
                            match pv {
                                packets::MC1_18_1 | packets::MC1_17_1 | packets::MC1_16_5 => {
                                    let _eid = read_only_copy.read_i32::<BigEndian>().unwrap();
                                    let is_hardcore = read_only_copy.read_i8().unwrap() != 0;
                                    let gamemode = read_only_copy.read_u8().unwrap();
                                    let prevgamemode = read_only_copy.read_u8().unwrap();
                                    let world_count = read_only_copy.read_var_u32().unwrap();
                                    let worlds = (0..world_count)
                                        .map(|_| read_only_copy.read_string().unwrap())
                                        .collect::<Vec<_>>();
                                    let codec: Blob =
                                        nbt::from_reader(&mut read_only_copy).unwrap();
                                    let dimension: Blob =
                                        nbt::from_reader(&mut read_only_copy).unwrap();
                                    let world_name = read_only_copy.read_string().unwrap();
                                    let hashed_seed =
                                        read_only_copy.read_i64::<BigEndian>().unwrap();

                                    dbg!(worlds, codec, &dimension);
                                    let mut respawn = |dim| {
                                        let mut v = vec![];
                                        let rid = packets::get_respawn_id(pv);
                                        dbg!(&rid);
                                        v.write_var_u32(rid).unwrap();

                                        nbt::to_writer(&mut v, &dim, None).unwrap();
                                        v.write_string("mycelium:transfer".to_string());
                                        v.write_i64::<BigEndian>(hashed_seed).unwrap();
                                        v.write_u8(gamemode).unwrap();
                                        v.write_u8(prevgamemode).unwrap();
                                        // is debug
                                        v.write_u8(false as u8).unwrap();
                                        // is flat
                                        v.write_u8(false as u8).unwrap();
                                        // copy metadata
                                        v.write_u8(false as u8).unwrap();

                                        client.write_var_u32(v.len() as u32).unwrap();
                                        client.write_all(&v).unwrap();
                                    };
                                    respawn(dimension);
                                }
                                packets::MC1_15_2 | packets::MC1_14_4 => {
                                    let _eid = read_only_copy.read_i32::<BigEndian>().unwrap();
                                    let gamemode = read_only_copy.read_u8().unwrap();
                                    let dimension = read_only_copy.read_i32::<BigEndian>().unwrap();
                                    let hashed_seed = if pv > packets::MC1_14_4 {
                                        Some(read_only_copy.read_i64::<BigEndian>().unwrap())
                                    } else {
                                        None
                                    };

                                    read_only_copy.read_u8().unwrap();
                                    let level_type = read_only_copy.read_string().unwrap();

                                    let mut respawn = |dim| {
                                        let mut v = vec![];
                                        let rid = packets::get_respawn_id(pv);
                                        dbg!(&rid);
                                        v.write_var_u32(rid).unwrap();

                                        v.write_i32::<BigEndian>(dim).unwrap();
                                        if hashed_seed.is_some() {
                                            v.write_i64::<BigEndian>(hashed_seed.unwrap()).unwrap();
                                        }
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
                                    respawn(fake_dimension);
                                    respawn(dimension);
                                }

                                _ => todo!("Unknown protocol {}", pv),
                            }

                            // respawn!(dimension);

                            // let fake_dimension = match dimension {
                            //     -1 => 1,
                            //     0 => -1,
                            //     1 => 0,
                            //     _ => panic!(),
                            // };
                            // send_respawn(fake_dimension);
                            // send_respawn(dimension);
                            recently_transferred = false;
                        }
                    }

                    _ => {
                        client.write_var_u32(len).unwrap();
                        client.write_all(&buf).unwrap();
                    }
                }
            }
        }
    }
}
