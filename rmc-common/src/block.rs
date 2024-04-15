use enum_assoc::Assoc;

use crate::DiscreteBlend;

#[derive(Debug, Default, PartialEq, Eq, Copy, Clone, Assoc)]
#[func(pub fn light_emission(&self) -> Option<u8>)]
#[func(pub fn light_passing(&self) -> bool { false })]
#[func(pub fn is_air(&self) -> bool { false })]
pub enum BlockType {
    #[default]
    #[assoc(light_passing = true)]
    #[assoc(is_air = true)]
    Air,
    Test,
    Grass,
    #[assoc(light_emission = 224)]
    Lantern,
    #[assoc(light_passing = true)]
    Mesh,
    Wood,
}

#[derive(Debug, Default, PartialEq, Eq, Copy, Clone)]
pub struct Block {
    pub ty: BlockType,
    pub light: u8,
    pub open_to_sky: bool,
}

impl Block {
    pub const fn new(ty: BlockType) -> Self {
        Block {
            ty,
            light: 0,
            open_to_sky: false,
        }
    }

    pub const AIR: Block = Block::new(BlockType::Air);
    pub const TEST: Block = Block::new(BlockType::Test);
    pub const GRASS: Block = Block::new(BlockType::Grass);
    pub const LANTERN: Block = Block::new(BlockType::Lantern);
    // Transparent rendering is hard :(
    // pub const MESH: Block = Block::new(BlockType::Mesh);
    pub const WOOD: Block = Block::new(BlockType::Wood);
}

impl DiscreteBlend for Block {}
