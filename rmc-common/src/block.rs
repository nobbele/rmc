use enum_assoc::Assoc;

use crate::DiscreteBlend;

#[derive(Debug, Default, PartialEq, Eq, Copy, Clone, Assoc)]
#[func(pub fn light_emission(&self) -> Option<u8>)]
#[func(pub fn light_passing(&self) -> bool { false })]
pub enum BlockType {
    #[default]
    #[assoc(light_passing = true)]
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
    pub fn not_air(self) -> Option<Block> {
        if self.ty == BlockType::Air {
            return None;
        }

        Some(self)
    }
}

impl Block {
    pub const AIR: Block = Block {
        ty: BlockType::Air,
        light: 0,
        open_to_sky: false,
    };
    pub const TEST: Block = Block {
        ty: BlockType::Test,
        light: 0,
        open_to_sky: false,
    };
    pub const GRASS: Block = Block {
        ty: BlockType::Grass,
        light: 0,
        open_to_sky: false,
    };
    pub const LANTERN: Block = Block {
        ty: BlockType::Lantern,
        light: 0,
        open_to_sky: false,
    };

    // Transparent rendering is hard :(
    pub const MESH: Block = Block {
        ty: BlockType::Mesh,
        light: 0,
        open_to_sky: false,
    };

    pub const WOOD: Block = Block {
        ty: BlockType::Wood,
        light: 0,
        open_to_sky: false,
    };
}

impl DiscreteBlend for Block {}
