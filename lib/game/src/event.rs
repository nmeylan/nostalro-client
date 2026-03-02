use packets::packets::{CharacterInfoNeoUnion, ServerAddr};

#[derive(Debug)]
pub enum GameEvent {
    // Network → Game (server responses)
    LoginAccepted {
        account_id: u32,
        login_id1: i32,
        login_id2: u32,
        sex: u8,
        servers: Vec<ServerInfo>,
    },
    LoginRefused {
        error_code: u8,
    },
    CharacterListReceived {
        characters: Vec<CharacterInfo>,
    },
    ZoneServerConnectInfo {
        char_id: u32,
        map_name: String,
        ip: u32,
        port: i16,
    },
    MapEntered {
        x: u16,
        y: u16,
        dir: u8,
        tick: u32,
    },
    MapChanged {
        map_name: String,
        x: i16,
        y: i16,
    },
    ServerTick {
        tick: u32,
    },
    Disconnected(String),

    // Game → Network (user-initiated requests)
    RequestLogin {
        username: String,
        password: String,
    },
    RequestSelectServer {
        index: usize,
    },
    RequestSelectCharacter {
        slot: u8,
    },
    RequestMove {
        x: u16,
        y: u16,
    },
    PlayerMoved {
        start_x: u16,
        start_y: u16,
        dest_x: u16,
        dest_y: u16,
        start_time: u32,
    },
    BackToLogin,
    BackToServerSelect,
}

#[derive(Debug, Clone)]
pub struct ServerInfo {
    pub ip: u32,
    pub port: i16,
    pub name: String,
    pub user_count: u16,
}

impl From<&ServerAddr> for ServerInfo {
    fn from(addr: &ServerAddr) -> Self {
        Self {
            ip: addr.ip,
            port: addr.port,
            name: addr.name.iter().take_while(|c| **c != '\0').collect(),
            user_count: addr.user_count,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CharacterInfo {
    pub gid: u32,
    pub name: String,
    pub class: u16,
    pub base_level: u16,
    pub job_level: u32,
    pub map: String,
    pub slot: i8,
    pub head: u16,
    pub hair_color: u16,
    pub sex: u8,
    pub hp: u32,
    pub max_hp: u32,
    pub sp: u16,
    pub max_sp: u16,
    pub str: u8,
    pub agi: u8,
    pub vit: u8,
    pub int: u8,
    pub dex: u8,
    pub luk: u8,
}

impl CharacterInfo {
    pub fn from_neo_union(info: &CharacterInfoNeoUnion, packetver: u32) -> Self {
        tracing::debug!(
            "CharInfo raw: gid={} class={} level={} joblevel={} name_raw={:?} slot={} hp={}/{} sp={}/{} speed={}",
            info.gid, info.class, info.level, info.joblevel,
            &info.name_raw[..16], info.char_num, info.hp, info.maxhp, info.sp, info.maxsp, info.speed,
        );
        let name: String = info.name.iter().take_while(|c| **c != '\0').collect();
        let map: String = if packetver >= 20100720 {
            info.last_map.iter().take_while(|c| **c != '\0').collect()
        } else {
            String::new()
        };
        let (hp, max_hp) = if packetver > 20081217 {
            (info.hp, info.maxhp)
        } else {
            (info.hp_16 as u32, info.maxhp_16 as u32)
        };
        let sex = if packetver >= 20141016 {
            info.sex
        } else {
            0
        };

        Self {
            gid: info.gid,
            name,
            class: info.class,
            base_level: info.level,
            job_level: info.joblevel,
            map,
            slot: info.char_num,
            head: info.head,
            hair_color: info.hair_color,
            sex,
            hp,
            max_hp,
            sp: info.sp,
            max_sp: info.maxsp,
            str: info.str,
            agi: info.agi,
            vit: info.vit,
            int: info.int,
            dex: info.dex,
            luk: info.luk,
        }
    }
}
