use strum_macros::Display;
use strum_macros::EnumIter;
use strum_macros::EnumString;

// so we can load effects dynamically
#[derive(Hash, Eq, PartialEq, Clone, Copy)]
pub struct StrEffectId(pub usize);

#[derive(Eq, Hash, PartialEq, Debug, Copy, Clone, EnumIter, EnumString, Display)]
pub enum StrEffectType {
    FireWall,
    StormGust,
    LordOfVermilion,
    Lightning,
    Concentration,
    Moonstar,
    Poison,
    Quagmire,
    FireWallBlue,
    FirePillarBomb,
    Ramadan,
}

impl From<StrEffectType> for StrEffectId {
    fn from(typ: StrEffectType) -> Self {
        StrEffectId(typ as usize)
    }
}

impl StrEffectType {
    pub fn get_effect_filename(&self) -> &'static str {
        match self {
            StrEffectType::FireWall => "firewall",
            StrEffectType::StormGust => "stormgust",
            StrEffectType::LordOfVermilion => "lord",
            StrEffectType::Lightning => "lightning",
            StrEffectType::Concentration => "concentration",
            StrEffectType::Moonstar => "moonstar",
            StrEffectType::Poison => "hunter_poison",
            StrEffectType::Quagmire => "quagmire",
            StrEffectType::FireWallBlue => "firewall_blue",
            StrEffectType::FirePillarBomb => "firepillarbomb",
            StrEffectType::Ramadan => "ramadan",
        }
    }
}
