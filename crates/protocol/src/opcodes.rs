#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecvOpcode {
    LoginPassword = 0x0001,
    ServerList = 0x0002,
    CharacterList = 0x0003,
    CharacterSelect = 0x0004,
    PlayerLoggedin = 0x0005,
    ChangeMap = 0x0006,
    ChangeChannel = 0x0007,
    PlayerMove = 0x0008,
    Chat = 0x0009,
    UseSkill = 0x000A,
    Attack = 0x000B,
    NpcTalk = 0x000C,
    ShopAction = 0x000D,
}

impl RecvOpcode {
    pub fn from_u16(value: u16) -> Option<Self> {
        Some(match value {
            0x0001 => Self::LoginPassword,
            0x0002 => Self::ServerList,
            0x0003 => Self::CharacterList,
            0x0004 => Self::CharacterSelect,
            0x0005 => Self::PlayerLoggedin,
            0x0006 => Self::ChangeMap,
            0x0007 => Self::ChangeChannel,
            0x0008 => Self::PlayerMove,
            0x0009 => Self::Chat,
            0x000A => Self::UseSkill,
            0x000B => Self::Attack,
            0x000C => Self::NpcTalk,
            0x000D => Self::ShopAction,
            _ => return None,
        })
    }
}

#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendOpcode {
    LoginStatus = 0x1001,
    ServerList = 0x1002,
    CharacterList = 0x1003,
    CharacterSelect = 0x1004,
    ServerIp = 0x1005,
    WarpToMap = 0x1006,
    ChangeChannel = 0x1007,
    AddPlayer = 0x1008,
    RemovePlayer = 0x1009,
    MovePlayer = 0x100A,
    SpawnMob = 0x100B,
    MoveMob = 0x100C,
    MobHp = 0x100D,
    DropItem = 0x100E,
    RemoveItem = 0x100F,
    Chat = 0x1010,
    Buff = 0x1011,
    SkillEffect = 0x1012,
}
