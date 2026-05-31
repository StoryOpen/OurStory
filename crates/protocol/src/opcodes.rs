#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecvOpcode {
    LoginPassword,
    ServerList,
    CharacterList,
    CharacterSelect,
    PlayerLoggedin,
    ChangeMap,
    ChangeChannel,
    PlayerMove,
    Chat,
    UseSkill,
    Attack,
    NpcTalk,
    ShopAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendOpcode {
    LoginStatus,
    ServerList,
    CharacterList,
    CharacterSelect,
    ServerIp,
    WarpToMap,
    ChangeChannel,
    AddPlayer,
    RemovePlayer,
    MovePlayer,
    SpawnMob,
    MoveMob,
    MobHp,
    DropItem,
    RemoveItem,
    Chat,
    Buff,
    SkillEffect,
}
