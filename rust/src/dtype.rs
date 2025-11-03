/// Data types supported by Resonant Protocol
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DType {
    F16 = 0x01,
    I8 = 0x02,
    Q4 = 0x03,
    SparseCoo = 0x10,
}

impl DType {
    pub fn from_u8(val: u8) -> Option<Self> {
        match val {
            0x01 => Some(DType::F16),
            0x02 => Some(DType::I8),
            0x03 => Some(DType::Q4),
            0x10 => Some(DType::SparseCoo),
            _ => None,
        }
    }
}

/// Modality of the data
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Modality {
    Text = 0,
    Image = 1,
    Audio = 2,
    Graph = 3,
    Mixed = 4,
}

impl Modality {
    pub fn from_u8(val: u8) -> Option<Self> {
        match val {
            0 => Some(Modality::Text),
            1 => Some(Modality::Image),
            2 => Some(Modality::Audio),
            3 => Some(Modality::Graph),
            4 => Some(Modality::Mixed),
            _ => None,
        }
    }
}
