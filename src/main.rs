use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use minecraft_varint::{VarIntRead, VarIntWrite};
use std::collections::HashMap;
use std::io::{Cursor, Write};
use std::net::TcpStream;
use std::sync::mpsc::{TryRecvError, Receiver, Sender};
use std::{io::Read, net::TcpListener};

lazy_static::lazy_static! {
    static ref SERVERS: HashMap<&'static str, &'static str> = {
        let mut m =  HashMap::new();
        m.insert("lobby", "127.0.0.1:25565");
        m.insert("flat", "127.0.0.1:35565");
        m.insert("a", "127.0.0.1:8001");
        m.insert("b", "127.0.0.1:8002");

        m
    };
}
const DEFAULT_SERVER_NAME: &str = "lobby";
fn main() {
    let s = Server {
        listener: TcpListener::bind("0.0.0.0:5005").unwrap(),
        motd: "1".to_string(),
    };
    for client in s.listener.incoming() {
        let client = client.unwrap();

        std::thread::spawn(move || handle_client(client));
    }
}
fn handle_client(bclient: TcpStream) {
    let bserver = TcpStream::connect(SERVERS.get(DEFAULT_SERVER_NAME).unwrap()).unwrap();
    // Server reading thread

    // Incoming
    let mut client = bclient.try_clone().unwrap();
    let mut server = bserver.try_clone().unwrap();
    // *client.unwrap() = TcpStream::connect("").unwrap();
    let (x, y) = std::sync::mpsc::channel::<TcpStream>();

    std::thread::spawn(move || handle_upstream(y, server ,client));

    // Client reading thread
    let mut client = bclient.try_clone().unwrap();
    let mut server = bserver.try_clone().unwrap();
    // Outgoing
    std::thread::spawn(move || handle_downstream(x, server, client));
    // loop {}
}
fn handle_downstream(x: Sender<TcpStream>, mut server: TcpStream, mut client: TcpStream) {
    let mut current_server = server;
    let mut current_server_name = DEFAULT_SERVER_NAME.to_string();
    println!("New client connected.");

    loop {
        let len = match { client.read_var_u32() } {
            Ok(l) => l,
            Err(_) => continue,
        };

        let mut buf = vec![0; len as usize];
        client.read_exact(&mut buf).unwrap();
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
                    client.write_var_u32(buf.len() as u32).unwrap();
                    client.write_all(&buf).unwrap();
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
                            let mut new_server = TcpStream::connect(address).unwrap();
                            current_server_name = args[1].to_string();
                            {
                                let mut v: Vec<u8> = vec![];
                                let pv = 340;
                                let address = "localhost";
                                let port = 8001;
                                let next_state = 2;

                                v.write_var_u32(0x0).unwrap();
                                v.write_var_u32(pv).unwrap();
                                v.write_string(address.to_string());
                                v.write_u16::<BigEndian>(port).unwrap();
                                v.write_var_u32(next_state).unwrap();

                                new_server.write_var_u32(v.len() as u32).unwrap();
                                new_server.write_all(&v).unwrap();
                            }
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
                                        let chat = new_server.read_string().unwrap();
                                        dbg!(&chat);
                                        let mut buf = vec![];

                                        buf.write_var_u32(0x0f).unwrap();
                                        buf.write_string(chat);
                                        buf.write_i8(0).unwrap();

                                        client.write_var_u32(buf.len() as u32).unwrap();
                                        client.write_all(&buf).unwrap();

                                        new_server.shutdown(std::net::Shutdown::Both).unwrap();
                                        continue;
                                    }
                                    2 => {
                                        let uuid = new_server.read_string().unwrap();
                                        let username = new_server.read_string().unwrap();
                                        current_server
                                            .shutdown(std::net::Shutdown::Both)
                                            .unwrap();

                                        dbg!(uuid, username);

                                        // dbg!("read", new_server.read(&mut []).unwrap());
                                    }
                                    _ => unimplemented!(),
                                }
                            }
                            x.send(new_server.try_clone().unwrap()).unwrap();
                            current_server = new_server;

                            println!("Finished connecting");
                            // This stops it from forwarding to the server that you executed the /join command
                            continue;
                        }
                    }
                }
            }
            // For intercepting normal sending.
            match id {
                _ => {
                    current_server.write_var_u32(len).unwrap();
                    current_server.write_all(&buf).unwrap();
                    // dbg!(id);
                    // println!("-> ID: 0x{:X} {}", id, id);
                }
            }
        }
    }
}
fn handle_upstream(y: Receiver<TcpStream>, mut server: TcpStream, mut client: TcpStream) {
    {
        let mut recently_transferred = false;
        loop {
            match y.try_recv() {
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
// pub struct Server {
//     pub listener: TcpListener,
//     pub motd: String,
// }
pub struct ProxiedPlayer {
    current_server: TcpStream,
    stream: TcpStream
}
trait MyceliumRead {
    fn read_string(&mut self) -> Option<String>;
}
trait MyceliumWrite {
    fn write_string(&mut self, s: String);
}

impl<X: Read> MyceliumRead for X {
    fn read_string(&mut self) -> Option<String> {
        let len = self.read_var_u32().ok()?;
        let mut buf = vec![0; (len).try_into().ok()?];
        self.read_exact(&mut buf).unwrap();
        Some(String::from_utf8(buf).unwrap())
    }
}

impl<X: Write> MyceliumWrite for X {
    fn write_string(&mut self, s: String) {
        self.write_var_u32(s.len().try_into().unwrap()).unwrap();
        self.write_all(s.as_bytes()).unwrap();
    }
}

#[test]
fn test_login() {
    let mut server = TcpStream::connect("127.0.0.1:35565").unwrap();
    // handshake
    {
        let mut v: Vec<u8> = vec![];
        let pv = 340;
        let address = "localhost";
        let port = 8001;
        let next_state = 2;

        v.write_var_u32(0x0).unwrap();
        v.write_var_u32(pv).unwrap();
        v.write_string(address.to_string());
        v.write_u16::<BigEndian>(port).unwrap();
        v.write_var_u32(next_state).unwrap();

        server.write_var_u32(v.len() as u32).unwrap();
        server.write_all(&v).unwrap();
    }
    // login start
    {
        let mut v: Vec<u8> = vec![];
        v.write_var_u32(0x0).unwrap();
        v.write_string("f8h".to_string());
        server.write_var_u32(v.len() as u32).unwrap();
        server.write_all(&v).unwrap();
    }
    {
        let len = server.read_var_u32().unwrap();
        let id = server.read_var_u32().unwrap();
        dbg!(len, id);
    }
}
