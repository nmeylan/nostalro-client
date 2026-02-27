#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    Disconnected,
    ConnectingToAccount,
    LoggingIn,
    SelectingServer,
    ConnectingToChar,
    SelectingCharacter,
    ConnectingToZone,
    InGame,
}

pub struct Session {
    pub state: SessionState,
    pub packetver: u32,
    pub account_id: u32,
    pub login_id1: i32,
    pub login_id2: u32,
    pub sex: u8,
    pub char_id: u32,
    pub map_name: String,
}

impl Session {
    pub fn new(packetver: u32) -> Self {
        Self {
            state: SessionState::Disconnected,
            packetver,
            account_id: 0,
            login_id1: 0,
            login_id2: 0,
            sex: 0,
            char_id: 0,
            map_name: String::new(),
        }
    }

    pub fn store_login(&mut self, account_id: u32, login_id1: i32, login_id2: u32, sex: u8) {
        self.account_id = account_id;
        self.login_id1 = login_id1;
        self.login_id2 = login_id2;
        self.sex = sex;
    }

    pub fn store_zone_info(&mut self, char_id: u32, map_name: String) {
        self.char_id = char_id;
        self.map_name = map_name;
    }
}
