use crate::DiscreteBlend;
use enum_assoc::Assoc;
use std::fmt::{Display, Formatter};

#[derive(Debug, Default, PartialEq, Eq, Copy, Clone, Assoc)]
#[func(pub fn light_emission(&self) -> Option<u8>)]
#[func(pub fn light_passing(&self) -> bool { false })]
#[func(pub fn is_air(&self) -> bool { false })]
#[func(pub fn name(&self) -> &'static str { "??" })]
#[repr(u8)]
pub enum BlockType {
    #[default]
    #[assoc(light_passing = true)]
    #[assoc(is_air = true)]
    Air,

    #[assoc(name = "Test")]
    Test,

    #[assoc(name = "Grass")]
    Grass,

    #[assoc(light_emission = 224)]
    #[assoc(name = "Lantern")]
    Lantern,

    #[assoc(light_passing = true)]
    #[assoc(name = "Mesh")]
    Mesh,

    #[assoc(name = "Wood")]
    Wood,

    #[assoc(name = "Stone")]
    Stone,
}

impl Display for BlockType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

#[derive(Debug, Default, PartialEq, Eq, Copy, Clone)]
pub struct Block {
    pub ty: BlockType,
    pub light: u8,
    pub open_to_sky: bool,

    /// Whether a block is fully occluded from view or not, used for rendering optimization.
    pub occluded: bool,
}

impl Block {
    pub const fn new(ty: BlockType) -> Self {
        Block {
            ty,
            light: 0,
            open_to_sky: false,
            occluded: false,
        }
    }

    pub const fn with_light(mut self, light: u8) -> Block {
        self.light = light;
        self
    }

    pub const AIR: Block = Block::new(BlockType::Air);
    pub const TEST: Block = Block::new(BlockType::Test);
    pub const GRASS: Block = Block::new(BlockType::Grass);
    pub const LANTERN: Block = Block::new(BlockType::Lantern);
    // Transparent rendering is hard :(
    pub const MESH: Block = Block::new(BlockType::Mesh);
    pub const WOOD: Block = Block::new(BlockType::Wood);
    pub const STONE: Block = Block::new(BlockType::Stone);
}

impl DiscreteBlend for Block {}
