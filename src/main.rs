use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use handlers::handle_client;
use minecraft_varint::{VarIntRead, VarIntWrite};
use std::collections::HashMap;
use std::io::{Cursor, Write};
use std::net::TcpStream;
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::{io::Read, net::TcpListener};

lazy_static::lazy_static! {
    static ref SERVERS: HashMap<&'static str, &'static str> = {
        let mut m =  HashMap::new();
        m.insert("lobby", "127.0.0.1:25565");
        m.insert("survival", "127.0.0.1:25566");

        // m.insert("flat", "127.0.0.1:35565");
        // m.insert("a", "127.0.0.1:8001");
        // m.insert("b", "127.0.0.1:8002");

        m
    };
}
const DEFAULT_SERVER_NAME: &str = "lobby";
mod packets;
fn main() {
    // let s = Server {
    //     listener: TcpListener::bind("0.0.0.0:5005").unwrap(),
    // };
    let initial_listener = TcpListener::bind("0.0.0.0:5005").unwrap();
    for client in initial_listener.incoming() {
        let client = client.unwrap();

        std::thread::spawn(move || handle_client(client));
    }
}
mod handlers;

// pub struct Server {
//     pub listener: TcpListener,
//     pub motd: String,
// }
pub struct ProxiedPlayer {
    current_server: TcpStream,
    stream: TcpStream,
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
