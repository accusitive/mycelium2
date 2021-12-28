use std::io::{Cursor, Write};
use std::net::TcpStream;
use std::ops::{Shr, ShrAssign};
use std::{io::Read, net::TcpListener};

use byteorder::{ReadBytesExt, WriteBytesExt};
use minecraft_varint::{VarIntRead, VarIntWrite};

mod packet;
mod protocol;

fn main() {
    let s = Server {
        listener: TcpListener::bind("0.0.0.0:5005").unwrap(),
        motd: "1".to_string(),
    };
    for client in s.listener.incoming() {
        let mut client = client.unwrap();

        std::thread::spawn(move || handle_client(client));
    }
    // let stream = TcpStream::connect("test.geysermc.org:25565").unwrap();

    // let mut data = vec![];
    // data.write_var_i32(0);
}
fn handle_client(bclient: TcpStream) {
    let bserver = TcpStream::connect("localhost:25565").unwrap();
    // Server reading thread
    let mut client = bclient.try_clone().unwrap();
    let mut server = bserver.try_clone().unwrap();
    std::thread::spawn(move || loop {
        let len = server.read_var_u32().unwrap();
        let mut buf = vec![0; len as usize];
        server.read_exact(&mut buf).unwrap();
        {
            let mut b = Cursor::new(buf.clone());
            hexdump::hexdump(&b.get_ref());
            let id = b.read_var_u32().unwrap();
            dbg!(id);
        }
        client.write_var_u32(len).unwrap();
        client.write_all(&buf).unwrap();
    });

    // Client reading thread
    let mut client = bclient.try_clone().unwrap();
    let mut server = bserver.try_clone().unwrap();

    std::thread::spawn(move || {
        let mut state = 0;
        loop {
            let len = client.read_var_u32().unwrap();

            let mut buf = vec![0; len as usize];
            client.read_exact(&mut buf).unwrap();
            {
                let mut b = Cursor::new(buf.clone());
                hexdump::hexdump(&b.get_ref());
                let id = b.read_var_u32().unwrap();
                match id {
                    0 if state == 0 => {
                        // hand shake
                        let pv = b.read_var_u32().unwrap_or(335);
                        let address = b.read_string();
                        dbg!(pv);
                        
                    }
                    _ => {}
                }
                dbg!(id);
            }

            server.write_var_u32(len).unwrap();
            server.write_all(&buf).unwrap();
        }
    });
    // loop {}
}

pub struct Server {
    pub listener: TcpListener,
    pub motd: String,
}
mod z {
    use std::io::{Read, Write};

    use byteorder::WriteBytesExt;

    trait MyceliumRead {
        fn read_var_i32(&mut self) -> Option<i32>;
        fn read_string(&mut self) -> Option<String>;
    }
    trait MyceliumWrite {
        fn write_var_i32(&mut self, value: i32);
    }
    impl<X: Read> MyceliumRead for X {
        fn read_var_i32(&mut self) -> Option<i32> {
            let mut buf = [0];
            let mut ans = 0;
            for i in 0..5 {
                self.read_exact(&mut buf).ok()?;
                ans |= (buf[0] as i32 & 0x7F) << 7 * i;

                if buf[0] & 0x80 == 0 {
                    break;
                }
            }
            Some(ans)
        }

        fn read_string(&mut self) -> Option<String> {
            let len = self.read_var_i32()?;
            let mut buf = vec![0; (len).try_into().ok()?];
            self.read_exact(&mut buf).unwrap();
            Some(String::from_utf8(buf).unwrap())
        }
    }

    impl<W: Write> MyceliumWrite for W {
        fn write_var_i32(&mut self, mut value: i32) {
            loop {
                if (value & !0x7f) == 0 {
                    self.write_i8(value as i8).unwrap();
                    dbg!(value, value & !0x7f);
                    break;
                }
                self.write_i8(((value & 0x7f) | 0x80) as i8).unwrap();
                value = value >> 7
            }
            // let mut buf = [0; (i32::BITS as usize + 6) / 7];
            // let mut i = 0;

            // loop {
            //     buf[i] = (value & 0b0111_1111) as u8;
            //     value >>= 7;
            //     if value != 0 {
            //         buf[i] |= 0b1000_0000;
            //     }
            //     i += 1;

            //     if value == 0 {
            //         break;
            //     }
            // }

            // self.write_all(&buf[..i]).unwrap();
        }
    }
}
