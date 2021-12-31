// #[cfg(feature = "1.18.1")]
// pub mod ids {
//     pub const RESPAWN: u32 = 0x3d;
//     pub const JOIN_GAME: u32 = 0x26;
//     pub const PROTOCOL: u32 = 757;
//     pub const CHAT_MESSAGE_C2S: u32 = 0x03;

//     // pub const CHAT_MESSAGE_S: u32 = 0x03;
//     // pub const CHAT_MESSAGE_C: u32 = 0x0f;
// }

// #[cfg(feature = "1.16.5")]
// pub mod ids {
//     pub const RESPAWN: u32 = 0x39;
//     pub const JOIN_GAME: u32 = 0x24;
//     pub const CHAT_MESSAGE_C2S: u32 = 0x03;
//     pub const PROTOCOL: u32 = 754;

// }
pub const MC1_18_1: u32 = 757;
// pub const MC1_18: u32 = 757;
pub const MC1_17_1: u32 = 756;
// pub const MC1_17: u32 = 755;
pub const MC1_16_5: u32 = 754;
pub const MC1_15_2: u32 = 578;
pub const MC1_14_4: u32 = 498;
pub const MC1_13_2: u32 = 404;
pub const MC1_12_2: u32 = 340;

pub fn get_respawn_id(pv: u32) -> u32 {
    // dbg!(pv);

    match pv {
        self::MC1_18_1 | self::MC1_17_1 => 0x3d,
        self::MC1_16_5 => 0x39,
        self::MC1_15_2 => 0x3b,
        self::MC1_14_4 => 0x3a,
        self::MC1_13_2 => 0x38,
        self::MC1_12_2 => 0x35,
        0 => u32::MAX,

        // MC1_16_5 =>
        _ => todo!(),
    }
}
pub fn get_chat_c2s(pv: u32) -> u32 {
    // dbg!(pv);

    match pv {
        self::MC1_18_1 | self::MC1_17_1 => 0x3,
        self::MC1_16_5 | self::MC1_15_2 | self::MC1_14_4 => 0x03,
        self::MC1_13_2 | self::MC1_12_2 => 0x02,
        0 => u32::MAX,
        _ => todo!("todo: implement get chat for {} ", pv),
    }
}
pub fn get_join_game(pv: u32) -> u32 {
    // dbg!(pv);
    match pv {
        self::MC1_18_1 | self::MC1_17_1 => 0x26,
        self::MC1_16_5 => 0x24,
        self::MC1_15_2 => 0x26,
        self::MC1_14_4 => 0x25,
        self::MC1_13_2 => 0x25,
        self::MC1_12_2 => 0x23,
        0 => u32::MAX,

        _ => todo!("todo: implement join game for {} ", pv),
    }
}
