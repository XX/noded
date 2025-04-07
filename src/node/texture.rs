use egui_snarl::{InPin, OutPin};
use serde::{Deserialize, Serialize};

use super::NodeFlags;

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct TextureNode {
    pub path: String,
}

impl TextureNode {
    pub const NAME: &str = "Texture";
    pub const INPUTS: [u64; 0] = [];
    pub const OUTPUTS: [u64; 1] = [NodeFlags::TEXTURE.bits() | NodeFlags::STRING.bits()];

    pub fn inputs(&self) -> &[u64] {
        &Self::INPUTS
    }

    pub fn outputs(&self) -> &[u64] {
        &Self::OUTPUTS
    }

    pub fn connect_input(&mut self, _from: &OutPin, _to: &InPin) {}

    pub fn disconnect_input(&mut self, _input_pin: &InPin) {}
}
