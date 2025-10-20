use bevy::asset::Handle;
use bevy::ecs::resource::Resource;
use bevy::scene::Scene;

const SIMPLE_HUMAN_RIG: &str = "simpleHumanRig.glb";
const SIMPLE_WALL: &str = "simpleWall.glb";

pub const MAX_ASSET_ELEMENTS: usize = 2;

pub struct AssetElementFile {
    pub file_name: &'static str,
}

pub static ASSET_ELEMENTS: &[AssetElementFile] = &[
    AssetElementFile {
        file_name: SIMPLE_HUMAN_RIG,
    },
    AssetElementFile {
        file_name: SIMPLE_WALL,
    },
];

#[derive(Clone)]
pub struct AssetElement {
    pub asset: Handle<Scene>,
}

#[derive(Resource)]
pub struct AssetElementList {
    pub elements: Vec<AssetElement>,
}

impl AssetElementList {
    pub fn get_by_index(&self, index: usize) -> Option<&Handle<Scene>> {
        self.elements.get(index).map(|e| &e.asset)
    }
}