// so we can load effects dynamically
#[derive(Hash, Eq, PartialEq, Clone, Copy)]
pub struct StrEffectId(pub usize);

#[derive(Eq, Hash, PartialEq, Debug, Copy, Clone)]
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
