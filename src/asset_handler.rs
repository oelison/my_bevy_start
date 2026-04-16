use bevy::animation::graph::{AnimationGraph, AnimationNodeIndex};
use bevy::asset::Handle;
use bevy::ecs::resource::Resource;
use bevy::world_serialization::WorldAsset;

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
    pub scene: Handle<WorldAsset>,
    pub graph: Handle<AnimationGraph>,
    pub index: AnimationNodeIndex,
}

#[derive(Resource)]
pub struct AssetElementList {
    pub elements: Vec<AssetElement>,
}

// impl AssetElementList {
//     pub fn get_by_index(&self, index: usize) -> Option<&AssetElement> {
//         self.elements.get(index)
//     }
// }