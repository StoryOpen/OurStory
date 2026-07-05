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
#[cfg(feature = "image-data")]
use std::io::Cursor;
use std::sync::Arc;
use lru::LruCache;
use std::num::NonZeroUsize;

use crate::error::WzError;
use crate::node::Node;
use crate::vector2d::Vector2D;

use self::map::MapData;
use self::mob::MobData;
use self::npc::NpcData;
use self::character::{CharacterBody, HairBody, FaceExpression};
use self::equip::{EquipData, EquipAction};
use self::skill::SkillDatabase;
use self::quest::QuestRegistry;
use self::physics::PhysicsConstants;

/// Shared static that both global() and init_wasm() use.
static GLOBAL_WZ_DATA: OnceLock<WzData> = OnceLock::new();

pub struct WzData {
    base: Node,

    // Caches
    map_cache: RwLock<LruCache<i32, Arc<MapData>>>,
    mob_cache: RwLock<LruCache<i32, Arc<MobData>>>,
    npc_cache: RwLock<LruCache<i32, Arc<NpcData>>>,
    equip_cache: RwLock<LruCache<i32, Arc<EquipData>>>,
    equip_action_cache: RwLock<LruCache<(i32, String), Arc<EquipAction>>>,
    body_cache: RwLock<LruCache<(u32, String), Arc<CharacterBody>>>,
    hair_cache: RwLock<LruCache<(u32, String), Arc<HairBody>>>,
    face_cache: RwLock<LruCache<(u32, String), Arc<FaceExpression>>>,
    skill_cache: OnceLock<Arc<SkillDatabase>>,
    quest_cache: OnceLock<Arc<QuestRegistry>>,
    physics_cache: OnceLock<Arc<PhysicsConstants>>,

    #[cfg(feature = "image-data")]
    image_cache: RwLock<LruCache<String, Arc<image::DynamicImage>>>,

    #[cfg(target_arch = "wasm32")]
    api_base_url: String,
}

#[cfg(target_arch = "wasm32")]
impl WzData {
    pub fn api_base_url(&self) -> &str {
        &self.api_base_url
    }
}

#[cfg(target_arch = "wasm32")]
fn dummy_base_node() -> Node {
    use std::sync::Arc;
    use wz_reader::property::WzSubProperty;
    let node = wz_reader::WzNode {
        name: wz_reader::WzNodeName::from(""),
        object_type: wz_reader::WzObjectType::Property(WzSubProperty::Property),
        parent: std::sync::Weak::new(),
        children: hashbrown::HashMap::new(),
    };
    Node::from(Arc::new(std::sync::RwLock::new(node)))
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
            equip_action_cache: RwLock::new(LruCache::new(NonZeroUsize::new(200).unwrap())),
            body_cache: RwLock::new(LruCache::new(NonZeroUsize::new(200).unwrap())),
            hair_cache: RwLock::new(LruCache::new(NonZeroUsize::new(100).unwrap())),
            face_cache: RwLock::new(LruCache::new(NonZeroUsize::new(100).unwrap())),
            skill_cache: OnceLock::new(),
            quest_cache: OnceLock::new(),
            physics_cache: OnceLock::new(),
            #[cfg(feature = "image-data")]
            image_cache: RwLock::new(LruCache::new(NonZeroUsize::new(200).unwrap())),
            #[cfg(target_arch = "wasm32")]
            api_base_url: String::new(),
        })
    }

    pub fn global() -> &'static Self {
        GLOBAL_WZ_DATA.get_or_init(|| {
            #[cfg(not(target_arch = "wasm32"))]
            {
                let path = std::env::var("WZ_PATH").unwrap_or_else(|_| "./wz/Base.wz".to_string());
                WzData::open(&path).expect("WzData::global() failed to open Base.wz")
            }
            #[cfg(target_arch = "wasm32")]
            {
                panic!("WzData::global() not available on wasm32; use WzData::init_wasm() instead")
            }
        })
    }

    #[cfg(target_arch = "wasm32")]
    pub fn init_wasm(api_base_url: String) -> &'static Self {
        GLOBAL_WZ_DATA.get_or_init(|| {
            // WzData for wasm — base is unused, data fetched via API
            let base = dummy_base_node();
            WzData {
                base,
                map_cache: RwLock::new(LruCache::new(NonZeroUsize::new(5).unwrap())),
                mob_cache: RwLock::new(LruCache::new(NonZeroUsize::new(50).unwrap())),
                npc_cache: RwLock::new(LruCache::new(NonZeroUsize::new(50).unwrap())),
                equip_cache: RwLock::new(LruCache::new(NonZeroUsize::new(50).unwrap())),
                equip_action_cache: RwLock::new(LruCache::new(NonZeroUsize::new(200).unwrap())),
                body_cache: RwLock::new(LruCache::new(NonZeroUsize::new(200).unwrap())),
                hair_cache: RwLock::new(LruCache::new(NonZeroUsize::new(100).unwrap())),
                face_cache: RwLock::new(LruCache::new(NonZeroUsize::new(100).unwrap())),
                skill_cache: OnceLock::new(),
                quest_cache: OnceLock::new(),
                physics_cache: OnceLock::new(),
                #[cfg(feature = "image-data")]
                image_cache: RwLock::new(LruCache::new(NonZeroUsize::new(200).unwrap())),
                api_base_url,
            }
        })
    }

    #[cfg(feature = "image-data")]
    pub fn load_image(&self, path: &str) -> Result<Arc<image::DynamicImage>, WzError> {
        #[cfg(not(target_arch = "wasm32"))]
        {
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
            return Ok(img);
        }
        #[cfg(target_arch = "wasm32")]
        {
            // Check cache first
            {
                let cache = self.image_cache.read().map_err(|_| WzError::LockPoisoned)?;
                if let Some(img) = cache.peek(path) {
                    return Ok(img.clone());
                }
            }
            // Kick off async fetch; if not cached yet, return error
            let url = format!("{}/wz/image/{}", self.api_base_url, path);
            let cache = &self.image_cache as *const RwLock<LruCache<String, Arc<image::DynamicImage>>>;
            let path_owned = path.to_string();
            wasm_bindgen_futures::spawn_local(async move {
                match crate::source::wasm_fetch(&url).await {
                    Ok(bytes) => {
                        if let Ok(img) = image::load_from_memory(&bytes) {
                            if let Ok(mut cache) = unsafe { &*cache }.write() {
                                cache.put(path_owned, Arc::new(img));
                            }
                        }
                    }
                    Err(e) => {
                        log::warn!("failed to fetch image {}: {:?}", path_owned, e.as_string());
                    }
                }
            });
            Err(WzError::ValueError(format!("image not loaded yet: {}", path)))
        }
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

    pub fn load_character_body(&self, skin_suffix: u32, action: &str) -> Result<Arc<CharacterBody>, WzError> {
        let key = (skin_suffix, action.to_string());
        {
            let mut cache = self.body_cache.write().map_err(|_| WzError::LockPoisoned)?;
            if let Some(c) = cache.get(&key) {
                return Ok(c.clone());
            }
        }
        let data = CharacterBody::load(&self.base, skin_suffix, action)?;
        let data = Arc::new(data);
        let mut cache = self.body_cache.write().map_err(|_| WzError::LockPoisoned)?;
        cache.put(key, data.clone());
        Ok(data)
    }

    pub fn load_hair_body(&self, hair_id: u32, action: &str) -> Result<Arc<HairBody>, WzError> {
        let key = (hair_id, action.to_string());
        {
            let mut cache = self.hair_cache.write().map_err(|_| WzError::LockPoisoned)?;
            if let Some(h) = cache.get(&key) {
                return Ok(h.clone());
            }
        }
        let data = HairBody::load(&self.base, hair_id, action)?;
        let data = Arc::new(data);
        let mut cache = self.hair_cache.write().map_err(|_| WzError::LockPoisoned)?;
        cache.put(key, data.clone());
        Ok(data)
    }

    pub fn load_face_expression(&self, face_id: u32, expression: &str) -> Result<Arc<FaceExpression>, WzError> {
        let key = (face_id, expression.to_string());
        {
            let mut cache = self.face_cache.write().map_err(|_| WzError::LockPoisoned)?;
            if let Some(f) = cache.get(&key) {
                return Ok(f.clone());
            }
        }
        let data = FaceExpression::load(&self.base, face_id, expression)?;
        let data = Arc::new(data);
        let mut cache = self.face_cache.write().map_err(|_| WzError::LockPoisoned)?;
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

    pub fn load_equip_action(&self, item_id: i32, action: &str) -> Result<Arc<EquipAction>, WzError> {
        let key = (item_id, action.to_string());
        {
            let mut cache = self.equip_action_cache.write().map_err(|_| WzError::LockPoisoned)?;
            if let Some(e) = cache.get(&key) {
                return Ok(e.clone());
            }
        }
        let data = EquipAction::load(&self.base, item_id, action)?;
        let data = Arc::new(data);
        let mut cache = self.equip_action_cache.write().map_err(|_| WzError::LockPoisoned)?;
        cache.put(key, data.clone());
        Ok(data)
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

    #[cfg(feature = "image-data")]
    pub fn load_portal_frames(&self) -> Result<Vec<PortalFrameData>, WzError> {
        let pv_root = self.base.at_path("Map/MapHelper.img/portal/game/pv")?;
        let mut children = pv_root.children();
        children.sort_by(|a, _, b, _| {
            let ai = a.as_str().parse::<i32>().unwrap_or(0);
            let bi = b.as_str().parse::<i32>().unwrap_or(0);
            ai.cmp(&bi)
        });
        let mut frames = Vec::new();
        for (_name, child) in &children {
            let origin = child
                .try_get("origin")
                .and_then(|n| n.read_origin(child).ok())
                .unwrap_or(Vector2D::ZERO);
            let png_data = child.extract_image()
                .map(|img| {
                    let mut cursor = Cursor::new(Vec::new());
                    let _ = img.write_to(&mut cursor, image::ImageFormat::Png);
                    cursor.into_inner()
                })
                .unwrap_or_default();
            frames.push(PortalFrameData {
                origin,
                delay: 100,
                png_data,
            });
        }
        Ok(frames)
    }

    /// Read the origin Vector2D from a WZ sprite node
    pub fn load_origin(&self, path: &str) -> Result<Vector2D, WzError> {
        let node = self.base.at_path(path)?;
        let origin_node = node.try_get("origin").ok_or_else(|| WzError::NodeNotFound(format!("{path}/origin")))?;
        origin_node.read_origin(&node)
    }

    #[cfg(target_arch = "wasm32")]
    async fn fetch_json<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T, WzError> {
        let url = format!("{}{}", self.api_base_url, path);
        let bytes = crate::source::wasm_fetch(&url).await
            .map_err(|e| WzError::ValueError(e.as_string().unwrap_or_else(|| "unknown error".into())))?;
        serde_json::from_slice(&bytes)
            .map_err(|e| WzError::ValueError(e.to_string()))
    }

    #[cfg(target_arch = "wasm32")]
    async fn fetch_binary<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T, WzError> {
        let url = format!("{}{}", self.api_base_url, path);
        let bytes = crate::source::wasm_fetch(&url).await
            .map_err(|e| WzError::ValueError(e.as_string().unwrap_or_else(|| "unknown error".into())))?;
        bincode::serde::decode_from_slice(&bytes, bincode::config::standard())
            .map(|(v, _)| v)
            .map_err(|e| WzError::ValueError(e.to_string()))
    }
}

#[cfg(target_arch = "wasm32")]
impl WzData {
    pub async fn load_map_wasm(&self, id: i32) -> Result<Arc<MapData>, WzError> {
        {
            let mut cache = self.map_cache.write().map_err(|_| WzError::LockPoisoned)?;
            if let Some(map) = cache.get(&id) {
                return Ok(map.clone());
            }
        }
        let map: MapData = self.fetch_json(&format!("/wz/data/map/{id}")).await?;
        let map = Arc::new(map);
        let mut cache = self.map_cache.write().map_err(|_| WzError::LockPoisoned)?;
        cache.put(id, map.clone());
        Ok(map)
    }

    pub async fn load_mob_wasm(&self, id: i32) -> Result<Arc<MobData>, WzError> {
        {
            let mut cache = self.mob_cache.write().map_err(|_| WzError::LockPoisoned)?;
            if let Some(mob) = cache.get(&id) {
                return Ok(mob.clone());
            }
        }
        let mob: MobData = self.fetch_json(&format!("/wz/data/mob/{id}")).await?;
        let mob = Arc::new(mob);
        let mut cache = self.mob_cache.write().map_err(|_| WzError::LockPoisoned)?;
        cache.put(id, mob.clone());
        Ok(mob)
    }

    pub async fn load_npc_wasm(&self, id: i32) -> Result<Arc<NpcData>, WzError> {
        {
            let mut cache = self.npc_cache.write().map_err(|_| WzError::LockPoisoned)?;
            if let Some(npc) = cache.get(&id) {
                return Ok(npc.clone());
            }
        }
        let npc: NpcData = self.fetch_json(&format!("/wz/data/npc/{id}")).await?;
        let npc = Arc::new(npc);
        let mut cache = self.npc_cache.write().map_err(|_| WzError::LockPoisoned)?;
        cache.put(id, npc.clone());
        Ok(npc)
    }

    pub async fn load_equip_wasm(&self, item_id: i32) -> Result<Arc<EquipData>, WzError> {
        {
            let mut cache = self.equip_cache.write().map_err(|_| WzError::LockPoisoned)?;
            if let Some(e) = cache.get(&item_id) {
                return Ok(e.clone());
            }
        }
        let equip: EquipData = self.fetch_json(&format!("/wz/data/equip/{item_id}")).await?;
        let equip = Arc::new(equip);
        let mut cache = self.equip_cache.write().map_err(|_| WzError::LockPoisoned)?;
        cache.put(item_id, equip.clone());
        Ok(equip)
    }

    pub async fn load_physics_wasm(&self) -> Result<Arc<PhysicsConstants>, WzError> {
        if let Some(phys) = self.physics_cache.get() {
            return Ok(phys.clone());
        }
        let phys: PhysicsConstants = self.fetch_json("/wz/data/physics").await?;
        let phys = Arc::new(phys);
        if self.physics_cache.set(phys.clone()).is_err() {
            Ok(self.physics_cache.get().unwrap().clone())
        } else {
            Ok(phys)
        }
    }

    pub async fn load_zmap_wasm(&self) -> Result<Vec<(String, usize)>, WzError> {
        let data: Vec<serde_json::Value> = self.fetch_json("/wz/data/zmap").await?;
        Ok(data.into_iter().filter_map(|pair| {
            let arr = pair.as_array()?;
            let name = arr.get(0)?.as_str()?.to_string();
            let idx = arr.get(1)?.as_i64()? as usize;
            Some((name, idx))
        }).collect())
    }

    pub async fn load_image_wasm(&self, path: &str) -> Result<Arc<image::DynamicImage>, WzError> {
        {
            let cache = self.image_cache.read().map_err(|_| WzError::LockPoisoned)?;
            if let Some(img) = cache.peek(path) {
                return Ok(img.clone());
            }
        }
        let url = format!("{}/wz/image/{}", self.api_base_url, path);
        let bytes = crate::source::wasm_fetch(&url).await
            .map_err(|e| WzError::ValueError(e.as_string().unwrap_or_else(|| "unknown error".into())))?;
        let img = Arc::new(image::load_from_memory(&bytes)
            .map_err(|e| WzError::ValueError(e.to_string()))?);
        let mut cache = self.image_cache.write().map_err(|_| WzError::LockPoisoned)?;
        cache.put(path.to_string(), img.clone());
        Ok(img)
    }

    // ═══════════════════════════════════════════════════════════
    //  Binary (bincode) wasm methods
    //  These fetch from /wz/bdata/... endpoints (binary, not JSON)
    //  Used by the unified loaders in asset_loaders.rs
    // ═══════════════════════════════════════════════════════════

    pub async fn load_map_bincode_wasm(&self, id: i32) -> Result<Arc<MapData>, WzError> {
        {
            let mut cache = self.map_cache.write().map_err(|_| WzError::LockPoisoned)?;
            if let Some(map) = cache.get(&id) {
                return Ok(map.clone());
            }
        }
        let map: MapData = self.fetch_binary(&format!("/wz/bdata/map/{id}")).await?;
        let map = Arc::new(map);
        let mut cache = self.map_cache.write().map_err(|_| WzError::LockPoisoned)?;
        cache.put(id, map.clone());
        Ok(map)
    }

    pub async fn load_mob_bincode_wasm(&self, id: i32) -> Result<Arc<MobData>, WzError> {
        {
            let mut cache = self.mob_cache.write().map_err(|_| WzError::LockPoisoned)?;
            if let Some(mob) = cache.get(&id) {
                return Ok(mob.clone());
            }
        }
        let mob: MobData = self.fetch_binary(&format!("/wz/bdata/mob/{id}")).await?;
        let mob = Arc::new(mob);
        let mut cache = self.mob_cache.write().map_err(|_| WzError::LockPoisoned)?;
        cache.put(id, mob.clone());
        Ok(mob)
    }

    pub async fn load_npc_bincode_wasm(&self, id: i32) -> Result<Arc<NpcData>, WzError> {
        {
            let mut cache = self.npc_cache.write().map_err(|_| WzError::LockPoisoned)?;
            if let Some(npc) = cache.get(&id) {
                return Ok(npc.clone());
            }
        }
        let npc: NpcData = self.fetch_binary(&format!("/wz/bdata/npc/{id}")).await?;
        let npc = Arc::new(npc);
        let mut cache = self.npc_cache.write().map_err(|_| WzError::LockPoisoned)?;
        cache.put(id, npc.clone());
        Ok(npc)
    }

    pub async fn load_equip_bincode_wasm(&self, item_id: i32) -> Result<Arc<EquipData>, WzError> {
        {
            let mut cache = self.equip_cache.write().map_err(|_| WzError::LockPoisoned)?;
            if let Some(e) = cache.get(&item_id) {
                return Ok(e.clone());
            }
        }
        let equip: EquipData = self.fetch_binary(&format!("/wz/bdata/equip/{item_id}")).await?;
        let equip = Arc::new(equip);
        let mut cache = self.equip_cache.write().map_err(|_| WzError::LockPoisoned)?;
        cache.put(item_id, equip.clone());
        Ok(equip)
    }

    pub async fn load_physics_bincode_wasm(&self) -> Result<Arc<PhysicsConstants>, WzError> {
        if let Some(phys) = self.physics_cache.get() {
            return Ok(phys.clone());
        }
        let phys: PhysicsConstants = self.fetch_binary("/wz/bdata/physics").await?;
        let phys = Arc::new(phys);
        if self.physics_cache.set(phys.clone()).is_err() {
            Ok(self.physics_cache.get().unwrap().clone())
        } else {
            Ok(phys)
        }
    }

    pub async fn load_zmap_bincode_wasm(&self) -> Result<Vec<(String, usize)>, WzError> {
        let data: Vec<(String, usize)> = self.fetch_binary("/wz/bdata/zmap").await?;
        Ok(data)
    }

    pub async fn load_skill_database_bincode_wasm(&self) -> Result<Arc<SkillDatabase>, WzError> {
        if let Some(db) = self.skill_cache.get() {
            return Ok(db.clone());
        }
        let db: SkillDatabase = self.fetch_binary("/wz/bdata/skill-database").await?;
        let db = Arc::new(db);
        if self.skill_cache.set(db.clone()).is_err() {
            Ok(self.skill_cache.get().unwrap().clone())
        } else {
            Ok(db)
        }
    }

    pub async fn load_character_body_bincode_wasm(&self, skin: u32, action: &str) -> Result<Arc<CharacterBody>, WzError> {
        let key = (skin, action.to_string());
        {
            let mut cache = self.body_cache.write().map_err(|_| WzError::LockPoisoned)?;
            if let Some(c) = cache.get(&key) {
                return Ok(c.clone());
            }
        }
        let data: CharacterBody = self.fetch_binary(&format!("/wz/bdata/character-body/{}/{}", skin, action)).await?;
        let data = Arc::new(data);
        let mut cache = self.body_cache.write().map_err(|_| WzError::LockPoisoned)?;
        cache.put(key, data.clone());
        Ok(data)
    }

    pub async fn load_hair_body_bincode_wasm(&self, hair_id: u32, action: &str) -> Result<Arc<HairBody>, WzError> {
        let key = (hair_id, action.to_string());
        {
            let mut cache = self.hair_cache.write().map_err(|_| WzError::LockPoisoned)?;
            if let Some(h) = cache.get(&key) {
                return Ok(h.clone());
            }
        }
        let data: HairBody = self.fetch_binary(&format!("/wz/bdata/hair-body/{}/{}", hair_id, action)).await?;
        let data = Arc::new(data);
        let mut cache = self.hair_cache.write().map_err(|_| WzError::LockPoisoned)?;
        cache.put(key, data.clone());
        Ok(data)
    }

    pub async fn load_equip_action_bincode_wasm(&self, item_id: i32, action: &str) -> Result<Arc<EquipAction>, WzError> {
        let key = (item_id, action.to_string());
        {
            let mut cache = self.equip_action_cache.write().map_err(|_| WzError::LockPoisoned)?;
            if let Some(e) = cache.get(&key) {
                return Ok(e.clone());
            }
        }
        let data: EquipAction = self.fetch_binary(&format!("/wz/bdata/equip-action/{}/{}", item_id, action)).await?;
        let data = Arc::new(data);
        let mut cache = self.equip_action_cache.write().map_err(|_| WzError::LockPoisoned)?;
        cache.put(key, data.clone());
        Ok(data)
    }

    pub async fn load_face_expression_bincode_wasm(&self, face_id: u32, expression: &str) -> Result<Arc<FaceExpression>, WzError> {
        let key = (face_id, expression.to_string());
        {
            let mut cache = self.face_cache.write().map_err(|_| WzError::LockPoisoned)?;
            if let Some(f) = cache.get(&key) {
                return Ok(f.clone());
            }
        }
        let data: FaceExpression = self.fetch_binary(&format!("/wz/bdata/face-expression/{}/{}", face_id, expression)).await?;
        let data = Arc::new(data);
        let mut cache = self.face_cache.write().map_err(|_| WzError::LockPoisoned)?;
        cache.put(key, data.clone());
        Ok(data)
    }

    pub async fn load_origin_bincode_wasm(&self, path: &str) -> Result<Vector2D, WzError> {
        let data: (f32, f32) = self.fetch_binary(&format!("/wz/bdata/origin/{}", path)).await?;
        Ok(Vector2D(data.0, data.1))
    }

    pub async fn load_action_lists_bincode_wasm(&self) -> Result<(Vec<String>, Vec<String>), WzError> {
        #[derive(::serde::Deserialize)]
        struct ActionListsResponse {
            basic: Vec<String>,
            composite: Vec<String>,
        }
        let resp: ActionListsResponse = self.fetch_binary("/wz/bdata/action-lists").await?;
        Ok((resp.basic, resp.composite))
    }

    pub async fn load_job_catalog_bincode_wasm(&self) -> Result<Vec<(u32, String)>, WzError> {
        self.fetch_binary("/wz/bdata/job-catalog").await
    }

    pub async fn load_portal_frames_bincode_wasm(&self) -> Result<Vec<PortalFrameData>, WzError> {
        self.fetch_binary("/wz/bdata/portal-frames").await
    }

    pub async fn load_node_children_wasm(&self, path: &str) -> Result<Vec<String>, WzError> {
        let value: serde_json::Value = self.fetch_json(&format!("/wz/node/{path}")).await?;
        let children = value
            .get("children")
            .and_then(|c| c.as_array())
            .ok_or_else(|| WzError::ValueError(format!("node {path} has no children")))?;
        let mut names: Vec<String> = children
            .iter()
            .filter_map(|c| c.get("name").and_then(|n| n.as_str().map(|s| s.to_string())))
            .collect();
        names.sort_by(|a, b| {
            let an: usize = a.parse().unwrap_or(0);
            let bn: usize = b.parse().unwrap_or(0);
            an.cmp(&bn)
        });
        Ok(names)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PortalFrameData {
    pub origin: Vector2D,
    pub delay: u32,
    pub png_data: Vec<u8>,
}

// ═══════════════════════════════════════════════════════════
//  Bundle types — returned by server GET /wz/bundle/*
//  Contains both data and all referenced images, so the
//  client gets everything in one cacheable request.
// ═══════════════════════════════════════════════════════════

/// Map data + all images referenced by its tiles, objs, backgrounds.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MapBundle {
    pub data: MapData,
    pub images: std::collections::HashMap<String, Vec<u8>>,
}

/// Mob data + all images referenced by its action frames.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MobBundle {
    pub data: MobData,
    pub images: std::collections::HashMap<String, Vec<u8>>,
}

/// NPC data + all images referenced by its action frames.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NpcBundle {
    pub data: NpcData,
    pub images: std::collections::HashMap<String, Vec<u8>>,
}

/// Ad-hoc set of images requested by path. Used by UI screens.
/// Includes origins so the caller gets everything in one cacheable GET.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ImageBundle {
    pub images: std::collections::HashMap<String, Vec<u8>>,
    pub origins: std::collections::HashMap<String, (f32, f32)>,
}


