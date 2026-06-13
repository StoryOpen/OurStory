pub mod common;
pub mod map;
pub mod mob;
pub mod npc;
pub mod character;
pub mod equip;
pub mod skill;
pub mod quest;
pub mod physics;

use std::sync::{OnceLock, RwLock};
use std::sync::Arc;
use lru::LruCache;
use std::num::NonZeroUsize;

use crate::error::WzError;
use crate::node::Node;
use crate::vector2d::Vector2D;

use self::map::MapData;
use self::mob::MobData;
use self::npc::NpcData;
use self::character::CharacterData;
use self::equip::EquipData;
use self::skill::SkillDatabase;
use self::quest::QuestRegistry;
use self::physics::PhysicsConstants;

pub struct WzData {
    base: Node,

    // Caches
    map_cache: RwLock<LruCache<i32, Arc<MapData>>>,
    mob_cache: RwLock<LruCache<i32, Arc<MobData>>>,
    npc_cache: RwLock<LruCache<i32, Arc<NpcData>>>,
    equip_cache: RwLock<LruCache<i32, Arc<EquipData>>>,
    char_cache: RwLock<LruCache<(u32, u32, u32), Arc<CharacterData>>>,
    skill_cache: OnceLock<Arc<SkillDatabase>>,
    quest_cache: OnceLock<Arc<QuestRegistry>>,
    physics_cache: OnceLock<Arc<PhysicsConstants>>,

    #[cfg(feature = "image-data")]
    image_cache: RwLock<LruCache<String, Arc<image::DynamicImage>>>,
}

impl WzData {
    pub fn open(path: &str) -> Result<Self, WzError> {
        let wz_node = wz_reader::util::resolve_base(path, None)?;
        let base = Node::from(wz_node);
        crate::set_base_node(base.clone());
        Ok(WzData {
            base,
            map_cache: RwLock::new(LruCache::new(NonZeroUsize::new(5).unwrap())),
            mob_cache: RwLock::new(LruCache::new(NonZeroUsize::new(50).unwrap())),
            npc_cache: RwLock::new(LruCache::new(NonZeroUsize::new(50).unwrap())),
            equip_cache: RwLock::new(LruCache::new(NonZeroUsize::new(50).unwrap())),
            char_cache: RwLock::new(LruCache::new(NonZeroUsize::new(10).unwrap())),
            skill_cache: OnceLock::new(),
            quest_cache: OnceLock::new(),
            physics_cache: OnceLock::new(),
            #[cfg(feature = "image-data")]
            image_cache: RwLock::new(LruCache::new(NonZeroUsize::new(200).unwrap())),
        })
    }

    pub fn global() -> &'static Self {
        static WZ_DATA: OnceLock<WzData> = OnceLock::new();
        WZ_DATA.get_or_init(|| {
            let path = std::env::var("WZ_PATH").unwrap_or_else(|_| "./wz/Base.wz".to_string());
            WzData::open(&path).expect("WzData::global() failed to open Base.wz")
        })
    }

    #[cfg(feature = "image-data")]
    pub fn load_image(&self, path: &str) -> Result<Arc<image::DynamicImage>, WzError> {
        {
            let cache = self.image_cache.read().map_err(|_| WzError::LockPoisoned)?;
            if let Some(img) = cache.peek(path) {
                return Ok(img.clone());
            }
        }
        let node = self.base.at_path(path)?;
        let img = Arc::new(node.extract_image()?);
        let mut cache = self.image_cache.write().map_err(|_| WzError::LockPoisoned)?;
        cache.put(path.to_string(), img.clone());
        Ok(img)
    }

    pub fn load_map(&self, id: i32) -> Result<Arc<MapData>, WzError> {
        {
            let mut cache = self.map_cache.write().map_err(|_| WzError::LockPoisoned)?;
            if let Some(map) = cache.get(&id) {
                return Ok(map.clone());
            }
        }
        let map = MapData::load(&self.base, id)?;
        let map = Arc::new(map);
        let mut cache = self.map_cache.write().map_err(|_| WzError::LockPoisoned)?;
        cache.put(id, map.clone());
        Ok(map)
    }

    pub fn load_mob(&self, id: i32) -> Result<Arc<MobData>, WzError> {
        {
            let mut cache = self.mob_cache.write().map_err(|_| WzError::LockPoisoned)?;
            if let Some(mob) = cache.get(&id) {
                return Ok(mob.clone());
            }
        }
        let mob = MobData::load(&self.base, id)?;
        let mob = Arc::new(mob);
        let mut cache = self.mob_cache.write().map_err(|_| WzError::LockPoisoned)?;
        cache.put(id, mob.clone());
        Ok(mob)
    }

    pub fn load_npc(&self, id: i32) -> Result<Arc<NpcData>, WzError> {
        {
            let mut cache = self.npc_cache.write().map_err(|_| WzError::LockPoisoned)?;
            if let Some(npc) = cache.get(&id) {
                return Ok(npc.clone());
            }
        }
        let npc = NpcData::load(&self.base, id)?;
        let npc = Arc::new(npc);
        let mut cache = self.npc_cache.write().map_err(|_| WzError::LockPoisoned)?;
        cache.put(id, npc.clone());
        Ok(npc)
    }

    pub fn load_character(&self, skin_suffix: u32, hair_id: u32, face_id: u32) -> Result<Arc<CharacterData>, WzError> {
        let key = (skin_suffix, hair_id, face_id);
        {
            let mut cache = self.char_cache.write().map_err(|_| WzError::LockPoisoned)?;
            if let Some(c) = cache.get(&key) {
                return Ok(c.clone());
            }
        }
        let data = CharacterData::load(&self.base, skin_suffix, hair_id, face_id)?;
        let data = Arc::new(data);
        let mut cache = self.char_cache.write().map_err(|_| WzError::LockPoisoned)?;
        cache.put(key, data.clone());
        Ok(data)
    }

    pub fn load_equip(&self, item_id: i32) -> Result<Arc<EquipData>, WzError> {
        {
            let mut cache = self.equip_cache.write().map_err(|_| WzError::LockPoisoned)?;
            if let Some(e) = cache.get(&item_id) {
                return Ok(e.clone());
            }
        }
        let equip = EquipData::load(&self.base, item_id)?;
        let equip = Arc::new(equip);
        let mut cache = self.equip_cache.write().map_err(|_| WzError::LockPoisoned)?;
        cache.put(item_id, equip.clone());
        Ok(equip)
    }

    pub fn load_skill_database(&self) -> Result<Arc<SkillDatabase>, WzError> {
        if let Some(db) = self.skill_cache.get() {
            return Ok(db.clone());
        }
        let db = Arc::new(SkillDatabase::load(&self.base)?);
        if self.skill_cache.set(db.clone()).is_err() {
            Ok(self.skill_cache.get().unwrap().clone())
        } else {
            Ok(db)
        }
    }

    pub fn load_quest_registry(&self) -> Result<Arc<QuestRegistry>, WzError> {
        if let Some(reg) = self.quest_cache.get() {
            return Ok(reg.clone());
        }
        let reg = Arc::new(QuestRegistry::load(&self.base)?);
        if self.quest_cache.set(reg.clone()).is_err() {
            Ok(self.quest_cache.get().unwrap().clone())
        } else {
            Ok(reg)
        }
    }

    pub fn load_physics(&self) -> Result<Arc<PhysicsConstants>, WzError> {
        if let Some(phys) = self.physics_cache.get() {
            return Ok(phys.clone());
        }
        let phys = Arc::new(PhysicsConstants::load(&self.base)?);
        if self.physics_cache.set(phys.clone()).is_err() {
            Ok(self.physics_cache.get().unwrap().clone())
        } else {
            Ok(phys)
        }
    }

    pub fn load_zmap(&self) -> Result<Vec<(String, usize)>, WzError> {
        let zmap_node = self.base.at_path("zmap.img")?;
        let children = zmap_node.ordered_children()?;
        Ok(children.into_iter().enumerate().map(|(i, (name, _))| (name, i)).collect())
    }

    pub fn load_smap(&self) -> Result<Vec<(String, String)>, WzError> {
        let smap_node = self.base.at_path("smap.img")?;
        let children = smap_node.ordered_children()?;
        Ok(children.into_iter()
            .filter_map(|(name, child)| {
                let s: Result<String, _> = child.try_into();
                s.ok().map(|s| (name, s))
            })
            .collect())
    }

    /// List child names at a WZ path (e.g. "Skill" → class names)
    pub fn list_children(&self, path: &str) -> Result<Vec<String>, WzError> {
        let node = self.base.at_path(path)?;
        Ok(node.children().into_iter().map(|(n, _)| n.to_string()).collect())
    }

    /// Read a string value from a WZ path
    pub fn read_string(&self, path: &str) -> Option<String> {
        self.base.at_path(path).ok().and_then(|n| n.try_into().ok())
    }

    /// Read an i32 value from a WZ path
    pub fn read_i32(&self, path: &str) -> Option<i32> {
        self.base.at_path(path).ok().and_then(|n| n.try_into().ok())
    }

    /// Read the origin Vector2D from a WZ sprite node
    pub fn load_origin(&self, path: &str) -> Result<Vector2D, WzError> {
        let node = self.base.at_path(path)?;
        let origin_node = node.try_get("origin").ok_or_else(|| WzError::NodeNotFound(format!("{path}/origin")))?;
        origin_node.read_origin(&node)
    }
}
