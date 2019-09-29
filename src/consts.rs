use encoding;
use encoding::types::Encoding;
use encoding::DecoderTrap;
use std::collections::HashMap;

use strum_macros::Display;
use strum_macros::EnumIter;
use strum_macros::EnumString;

#[derive(EnumIter, EnumString, Display, Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum MonsterId {
    Baphomet,
    Poring,
    Barricade,
    GEFFEN_MAGE_6,
    GEFFEN_MAGE_12, // red
    GEFFEN_MAGE_9,  // blue
    Dimik,
}

#[derive(EnumIter, EnumString, Display, Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum JobId {
    CRUSADER,
    SWORDMAN,
    ARCHER,
    HUNTER,
    ASSASSIN,
    ROGUE,
    KNIGHT,
    WIZARD,
    SAGE,
    ALCHEMIST,
    BLACKSMITH,
    PRIEST,
    MONK,
    GUNSLINGER,

    TargetDummy,
    HealingDummy,
    MeleeMinion,
    RangedMinion,
    Turret,
    Guard,
}

pub const PLAYABLE_CHAR_SPRITES: [JobSpriteId; 14] = [
    JobSpriteId::CRUSADER,
    JobSpriteId::SWORDMAN,
    JobSpriteId::ARCHER,
    JobSpriteId::ASSASSIN,
    JobSpriteId::ROGUE,
    JobSpriteId::KNIGHT,
    JobSpriteId::WIZARD,
    JobSpriteId::SAGE,
    JobSpriteId::ALCHEMIST,
    JobSpriteId::BLACKSMITH,
    JobSpriteId::PRIEST,
    JobSpriteId::MONK,
    JobSpriteId::GUNSLINGER,
    JobSpriteId::HUNTER,
];

#[derive(EnumIter, EnumString, Debug, Display, Clone, Copy, Eq, PartialEq, Hash)]
pub enum JobSpriteId {
    NOVICE = 0,
    SWORDMAN = 1,
    MAGICIAN = 2,
    ARCHER = 3,
    ACOLYTE = 4,
    MERCHANT = 5,
    THIEF = 6,
    KNIGHT = 7,
    PRIEST = 8,
    WIZARD = 9,
    BLACKSMITH = 10,
    HUNTER = 11,
    ASSASSIN = 12,
    KNIGHT2 = 13,
    CRUSADER = 14,
    MONK = 15,
    SAGE = 16,
    ROGUE = 17,
    ALCHEMIST = 18,
    BARD = 19,
    DANCER = 20,
    CRUSADER2 = 21,
    MARRIED = 22,
    SUPERNOVICE = 23,
    GUNSLINGER = 24,
    NINJA = 25,
    XMAS = 26,
    SUMMER = 27,
    NoviceH = 4001,
    SwordmanH = 4002,
    MagicianH = 4003,
    ArcherH = 4004,
    AcolyteH = 4005,
    MerchantH = 4006,
    ThiefH = 4007,
    KnightH = 4008,
    PriestH = 4009,
    WizardH = 4010,
    BlacksmithH = 4011,
    HunterH = 4012,
    AssassinH = 4013,
    Knight2H = 4014,
    CrusaderH = 4015,
    MonkH = 4016,
    SageH = 4017,
    RogueH = 4018,
    AlchemistH = 4019,
    BardH = 4020,
    DancerH = 4021,
    Crusader2H = 4022,
    NoviceB = 4023,
    SwordmanB = 4024,
    MagicianB = 4025,
    ArcherB = 4026,
    AcolyteB = 4027,
    MerchantB = 4028,
    ThiefB = 4029,
    KnightB = 4030,
    PriestB = 4031,
    WizardB = 4032,
    BlacksmithB = 4033,
    HunterB = 4034,
    AssassinB = 4035,
    Knight2B = 4036,
    CrusaderB = 4037,
    MonkB = 4038,
    SageB = 4039,
    RogueB = 4040,
    AlchemistB = 4041,
    BardB = 4042,
    DancerB = 4043,
    Crusader2B = 4044,
    SupernoviceB = 4045,
    TAEKWON = 4046,
    STAR = 4047,
    STAR2 = 4048,
    LINKER = 4049,
    /*
    not used yet=
    Job_Gangsi	4050
    Job_Death_Knight	4051
    Job_Dark_Collector	4052
    */
    RuneKnight = 4054,
    WARLOCK = 4055,
    RANGER = 4056,
    ARCHBISHOP = 4057,
    MECHANIC = 4058,
    GuillotineCross = 4059,
    RuneKnightH = 4060,
    WarlockH = 4061,
    RangerH = 4062,
    ArchbishopH = 4063,
    MechanicH = 4064,
    GuillotineCrossH = 4065,
    RoyalGuard = 4066,
    SORCERER = 4067,
    MINSTREL = 4068,
    WANDERER = 4069,
    SURA = 4070,
    GENETIC = 4071,
    ShadowChaser = 4072,
    RoyalGuardH = 4073,
    SorcererH = 4074,
    MinstrelH = 4075,
    WandererH = 4076,
    SuraH = 4077,
    GeneticH = 4078,
    ShadowChaserH = 4079,
    RuneKnight2 = 4080,
    RuneKnight2H = 4081,
    RoyalGuard2 = 4082,
    RoyalGuard2H = 4083,
    RANGER2 = 4084,
    Ranger2H = 4085,
    MECHANIC2 = 4086,
    Mechanic2H = 4087,

    RuneKnightB = 4096,
    WarlockB = 4097,
    RangerB = 4098,
    ArchbishopB = 4099,
    MechanicB = 4100,
    GuillotineCrossB = 4101,
    RoyalGuardB = 4102,
    SorcererB = 4103,
    MinstrelB = 4104,
    WandererB = 4105,
    SuraB = 4106,
    GeneticB = 4107,
    ShadowChaserB = 4108,
    RuneKnight2B = 4109,
    RoyalGuard2B = 4110,
    Ranger2B = 4111,
    Mechanic2B = 4112,
    // 4113 ?
    FrogNinja = 4114,
    PecoGunner = 4115,
    PecoSword = 4116,
    // 4117 ?
    PigWhitesmith = 4118,
    PigMerchant = 4119,
    PigGenetic = 4120,
    PigCreator = 4121,
    OstrichArcher = 4122,
    PoringStar = 4123,
    PoringNovice = 4124,
    SheepMonk = 4125,
    SheepAco = 4126,
    SheepSura = 4127,
    PoringSnovice = 4128,
    SheepArcb = 4129,
    FoxMagician = 4130,
    FoxSage = 4131,
    FoxSorcerer = 4132,
    FoxWarlock = 4133,
    FoxWiz = 4134,
    // 4135 ?
    FoxHwiz = 4136,
    PigAlche = 4137,
    PigBlacksmith = 4138,
    SheepChamp = 4139,
    DogGCross = 4140,
    DogThief = 4141,
    DogRogue = 4142,
    DogChaser = 4143,
    DogStalker = 4144,
    DogAssassin = 4145,
    DogAssaX = 4146,
    OstrichDancer = 4147,
    OstrichMinstrel = 4148,
    OstrichBard = 4149,
    OstrichSniper = 4150,
    OstrichWander = 4151,
    OstrichZipsi = 4152,
    OstrichCrown = 4153,
    OstrichHunter = 4154,
    PoringTaekwon = 4155,
    SheepPriest = 4156,
    SheepHpriest = 4157,
    PoringNoviceB = 4158,
    // 4159 ?
    FoxMagicianB = 4160,
    OstrichArcherB = 4161,
    SheepAcoB = 4162,
    PigMerchantB = 4163,
    OstrichHunterB = 4164,
    DogAssassinB = 4165,
    SheepMonkB = 4166,
    FoxSageB = 4167,
    DogRogueB = 4168,
    PigAlcheB = 4169,
    OstrichBardB = 4170,
    OstrichDancerB = 4171,
    PoringSnoviceB = 4172,
    FoxWarlockB = 4173,
    SheepArcbB = 4174,
    DogGCrossB = 4175,
    FoxSorcererB = 4176,
    OstrichMinstrelB = 4177,
    OstrichWanderB = 4178,
    SheepSuraB = 4179,
    PigGeneticB = 4180,
    DogThiefB = 4181,
    DogChaserB = 4182,
    PoringNoviceH = 4183,
    // 4184 ?
    FoxMagicianH = 4185,
    OstrichArcherH = 4186,
    SheepAcoH = 4187,
    PigMerchantH = 4188,
    DogThiefH = 4189,
    SUPERNOVICE2 = 4190,
    Supernovice2B = 4191,
    PoringSnovice2 = 4192,
    PoringSnovice2B = 4193,
    SheepPriestB = 4194,
    FoxWizB = 4195,
    PigBlacksmithB = 4196,

    KAGEROU = 4211,
    OBORO = 4212,
    FrogKagerou = 4213,
    FrogOboro = 4214,
}

pub fn job_name_table() -> HashMap<JobSpriteId, String> {
    let mut table = HashMap::new();

    table.insert(
        JobSpriteId::NOVICE,
        encoding::all::WINDOWS_1252
            .decode(&[0xC3, 0xCA, 0xBA, 0xB8, 0xC0, 0xDA], DecoderTrap::Strict)
            .unwrap(),
    );

    table.insert(
        JobSpriteId::SWORDMAN,
        encoding::all::WINDOWS_1252
            .decode(&[0xB0, 0xCB, 0xBB, 0xE7], DecoderTrap::Strict)
            .unwrap(),
    );
    table.insert(
        JobSpriteId::MAGICIAN,
        encoding::all::WINDOWS_1252
            .decode(&[0xB8, 0xB6, 0xB9, 0xFD, 0xBB, 0xE7], DecoderTrap::Strict)
            .unwrap(),
    );
    table.insert(
        JobSpriteId::ARCHER,
        encoding::all::WINDOWS_1252
            .decode(&[0xB1, 0xC3, 0xBC, 0xF6], DecoderTrap::Strict)
            .unwrap(),
    );
    table.insert(
        JobSpriteId::ACOLYTE,
        encoding::all::WINDOWS_1252
            .decode(&[0xBC, 0xBA, 0xC1, 0xF7, 0xC0, 0xDA], DecoderTrap::Strict)
            .unwrap(),
    );
    table.insert(
        JobSpriteId::MERCHANT,
        encoding::all::WINDOWS_1252
            .decode(&[0xBB, 0xF3, 0xC0, 0xCE], DecoderTrap::Strict)
            .unwrap(),
    );
    table.insert(
        JobSpriteId::THIEF,
        encoding::all::WINDOWS_1252
            .decode(&[0xB5, 0xB5, 0xB5, 0xCF], DecoderTrap::Strict)
            .unwrap(),
    );

    table.insert(
        JobSpriteId::KNIGHT,
        encoding::all::WINDOWS_1252
            .decode(&[0xB1, 0xE2, 0xBB, 0xE7], DecoderTrap::Strict)
            .unwrap(),
    );
    table.insert(
        JobSpriteId::PRIEST,
        encoding::all::WINDOWS_1252
            .decode(
                &[0xC7, 0xC1, 0xB8, 0xAE, 0xBD, 0xBA, 0xC6, 0xAE],
                DecoderTrap::Strict,
            )
            .unwrap(),
    );
    table.insert(
        JobSpriteId::WIZARD,
        encoding::all::WINDOWS_1252
            .decode(&[0xC0, 0xA7, 0xC0, 0xFA, 0xB5, 0xE5], DecoderTrap::Strict)
            .unwrap(),
    );
    table.insert(
        JobSpriteId::BLACKSMITH,
        encoding::all::WINDOWS_1252
            .decode(&[0xC1, 0xA6, 0xC3, 0xB6, 0xB0, 0xF8], DecoderTrap::Strict)
            .unwrap(),
    );
    table.insert(
        JobSpriteId::HUNTER,
        encoding::all::WINDOWS_1252
            .decode(&[0xC7, 0xE5, 0xC5, 0xCD], DecoderTrap::Strict)
            .unwrap(),
    );
    table.insert(
        JobSpriteId::ASSASSIN,
        encoding::all::WINDOWS_1252
            .decode(&[0xBE, 0xEE, 0xBC, 0xBC, 0xBD, 0xC5], DecoderTrap::Strict)
            .unwrap(),
    );
    table.insert(
        JobSpriteId::KNIGHT2,
        encoding::all::WINDOWS_1252
            .decode(
                &[
                    0xC6, 0xE4, 0xC4, 0xDA, 0xC6, 0xE4, 0xC4, 0xDA, 0x5f, 0xB1, 0xE2, 0xBB, 0xE7,
                ],
                DecoderTrap::Strict,
            )
            .unwrap(),
    );

    table.insert(
        JobSpriteId::CRUSADER,
        encoding::all::WINDOWS_1252
            .decode(
                &[0xC5, 0xA9, 0xB7, 0xE7, 0xBC, 0xBC, 0xC0, 0xCC, 0xB4, 0xF5],
                DecoderTrap::Strict,
            )
            .unwrap(),
    );
    table.insert(
        JobSpriteId::MONK,
        encoding::all::WINDOWS_1252
            .decode(&[0xB8, 0xF9, 0xC5, 0xA9], DecoderTrap::Strict)
            .unwrap(),
    );
    table.insert(
        JobSpriteId::SAGE,
        encoding::all::WINDOWS_1252
            .decode(&[0xBC, 0xBC, 0xC0, 0xCC, 0xC1, 0xF6], DecoderTrap::Strict)
            .unwrap(),
    );
    table.insert(
        JobSpriteId::ROGUE,
        encoding::all::WINDOWS_1252
            .decode(&[0xB7, 0xCE, 0xB1, 0xD7], DecoderTrap::Strict)
            .unwrap(),
    );
    table.insert(
        JobSpriteId::ALCHEMIST,
        encoding::all::WINDOWS_1252
            .decode(
                &[0xBF, 0xAC, 0xB1, 0xDD, 0xBC, 0xFA, 0xBB, 0xE7],
                DecoderTrap::Strict,
            )
            .unwrap(),
    );
    table.insert(
        JobSpriteId::BARD,
        encoding::all::WINDOWS_1252
            .decode(&[0xB9, 0xD9, 0xB5, 0xE5], DecoderTrap::Strict)
            .unwrap(),
    );
    table.insert(
        JobSpriteId::DANCER,
        encoding::all::WINDOWS_1252
            .decode(&[0xB9, 0xAB, 0xC8, 0xF1], DecoderTrap::Strict)
            .unwrap(),
    );
    table.insert(
        JobSpriteId::CRUSADER2,
        encoding::all::WINDOWS_1252
            .decode(
                &[
                    0xBD, 0xC5, 0xC6, 0xE4, 0xC4, 0xDA, 0xC5, 0xA9, 0xB7, 0xE7, 0xBC, 0xBC, 0xC0,
                    0xCC, 0xB4, 0xF5,
                ],
                DecoderTrap::Strict,
            )
            .unwrap(),
    );

    table.insert(
        JobSpriteId::SUPERNOVICE,
        encoding::all::WINDOWS_1252
            .decode(
                &[0xBD, 0xB4, 0xC6, 0xDB, 0xB3, 0xEB, 0xBA, 0xF1, 0xBD, 0xBA],
                DecoderTrap::Strict,
            )
            .unwrap(),
    );
    table.insert(
        JobSpriteId::GUNSLINGER,
        encoding::all::WINDOWS_1252
            .decode(&[0xB0, 0xC7, 0xB3, 0xCA], DecoderTrap::Strict)
            .unwrap(),
    );
    table.insert(
        JobSpriteId::NINJA,
        encoding::all::WINDOWS_1252
            .decode(&[0xB4, 0xD1, 0xC0, 0xDA], DecoderTrap::Strict)
            .unwrap(),
    );
    table.insert(
        JobSpriteId::TAEKWON,
        encoding::all::WINDOWS_1252
            .decode(
                &[0xc5, 0xc2, 0xb1, 0xc7, 0xbc, 0xd2, 0xb3, 0xe2],
                DecoderTrap::Strict,
            )
            .unwrap(),
    );
    table.insert(
        JobSpriteId::STAR,
        encoding::all::WINDOWS_1252
            .decode(&[0xb1, 0xc7, 0xbc, 0xba], DecoderTrap::Strict)
            .unwrap(),
    );
    table.insert(
        JobSpriteId::STAR2,
        encoding::all::WINDOWS_1252
            .decode(
                &[0xb1, 0xc7, 0xbc, 0xba, 0xc0, 0xb6, 0xc7, 0xd5],
                DecoderTrap::Strict,
            )
            .unwrap(),
    );
    table.insert(
        JobSpriteId::LINKER,
        encoding::all::WINDOWS_1252
            .decode(
                &[0xbc, 0xd2, 0xbf, 0xef, 0xb8, 0xb5, 0xc4, 0xbf],
                DecoderTrap::Strict,
            )
            .unwrap(),
    );

    table.insert(
        JobSpriteId::MARRIED,
        encoding::all::WINDOWS_1252
            .decode(&[0xB0, 0xE1, 0xC8, 0xA5], DecoderTrap::Strict)
            .unwrap(),
    );
    table.insert(
        JobSpriteId::XMAS,
        encoding::all::WINDOWS_1252
            .decode(&[0xBB, 0xEA, 0xC5, 0xB8], DecoderTrap::Strict)
            .unwrap(),
    );
    table.insert(
        JobSpriteId::SUMMER,
        encoding::all::WINDOWS_1252
            .decode(&[0xBF, 0xA9, 0xB8, 0xA7], DecoderTrap::Strict)
            .unwrap(),
    );

    table.insert(
        JobSpriteId::KnightH,
        encoding::all::WINDOWS_1252
            .decode(
                &[0xB7, 0xCE, 0xB5, 0xE5, 0xB3, 0xAA, 0xC0, 0xCC, 0xC6, 0xAE],
                DecoderTrap::Strict,
            )
            .unwrap(),
    );
    table.insert(
        JobSpriteId::PriestH,
        encoding::all::WINDOWS_1252
            .decode(
                &[0xC7, 0xCF, 0xC0, 0xCC, 0xC7, 0xC1, 0xB8, 0xAE],
                DecoderTrap::Strict,
            )
            .unwrap(),
    );
    table.insert(
        JobSpriteId::WizardH,
        encoding::all::WINDOWS_1252
            .decode(
                &[0xC7, 0xCF, 0xC0, 0xCC, 0xC0, 0xA7, 0xC0, 0xFA, 0xB5, 0xE5],
                DecoderTrap::Strict,
            )
            .unwrap(),
    );
    table.insert(
        JobSpriteId::BlacksmithH,
        encoding::all::WINDOWS_1252
            .decode(
                &[
                    0xC8, 0xAD, 0xC0, 0xCC, 0xC6, 0xAE, 0xBD, 0xBA, 0xB9, 0xCC, 0xBD, 0xBA,
                ],
                DecoderTrap::Strict,
            )
            .unwrap(),
    );
    table.insert(
        JobSpriteId::HunterH,
        encoding::all::WINDOWS_1252
            .decode(
                &[0xBD, 0xBA, 0xB3, 0xAA, 0xC0, 0xCC, 0xC6, 0xDB],
                DecoderTrap::Strict,
            )
            .unwrap(),
    );
    table.insert(
        JobSpriteId::AssassinH,
        encoding::all::WINDOWS_1252
            .decode(
                &[
                    0xBE, 0xEE, 0xBD, 0xD8, 0xBD, 0xC5, 0xC5, 0xA9, 0xB7, 0xCE, 0xBD, 0xBA,
                ],
                DecoderTrap::Strict,
            )
            .unwrap(),
    );
    table.insert(
        JobSpriteId::Knight2H,
        encoding::all::WINDOWS_1252
            .decode(
                &[0xB7, 0xCE, 0xB5, 0xE5, 0xC6, 0xE4, 0xC4, 0xDA],
                DecoderTrap::Strict,
            )
            .unwrap(),
    );
    table.insert(
        JobSpriteId::CrusaderH,
        encoding::all::WINDOWS_1252
            .decode(&[0xC6, 0xC8, 0xB6, 0xF3, 0xB5, 0xF2], DecoderTrap::Strict)
            .unwrap(),
    );
    table.insert(
        JobSpriteId::MonkH,
        encoding::all::WINDOWS_1252
            .decode(&[0xC3, 0xA8, 0xC7, 0xC7, 0xBF, 0xC2], DecoderTrap::Strict)
            .unwrap(),
    );
    table.insert(
        JobSpriteId::SageH,
        encoding::all::WINDOWS_1252
            .decode(
                &[0xC7, 0xC1, 0xB7, 0xCE, 0xC6, 0xE4, 0xBC, 0xAD],
                DecoderTrap::Strict,
            )
            .unwrap(),
    );
    table.insert(
        JobSpriteId::RogueH,
        encoding::all::WINDOWS_1252
            .decode(&[0xBD, 0xBA, 0xC5, 0xE4, 0xC4, 0xBF], DecoderTrap::Strict)
            .unwrap(),
    );
    table.insert(
        JobSpriteId::AlchemistH,
        encoding::all::WINDOWS_1252
            .decode(
                &[0xC5, 0xA9, 0xB8, 0xAE, 0xBF, 0xA1, 0xC0, 0xCC, 0xC5, 0xCD],
                DecoderTrap::Strict,
            )
            .unwrap(),
    );
    table.insert(
        JobSpriteId::BardH,
        encoding::all::WINDOWS_1252
            .decode(&[0xC5, 0xAC, 0xB6, 0xF3, 0xBF, 0xEE], DecoderTrap::Strict)
            .unwrap(),
    );
    table.insert(
        JobSpriteId::DancerH,
        encoding::all::WINDOWS_1252
            .decode(&[0xC1, 0xFD, 0xBD, 0xC3], DecoderTrap::Strict)
            .unwrap(),
    );
    table.insert(
        JobSpriteId::Crusader2H,
        encoding::all::WINDOWS_1252
            .decode(
                &[0xC6, 0xE4, 0xC4, 0xDA, 0xC6, 0xC8, 0xB6, 0xF3, 0xB5, 0xF2],
                DecoderTrap::Strict,
            )
            .unwrap(),
    );

    table.insert(
        JobSpriteId::RuneKnight,
        encoding::all::WINDOWS_1252
            .decode(
                &[0xB7, 0xE9, 0xB3, 0xAA, 0xC0, 0xCC, 0xC6, 0xAE],
                DecoderTrap::Strict,
            )
            .unwrap(),
    );
    table.insert(
        JobSpriteId::WARLOCK,
        encoding::all::WINDOWS_1252
            .decode(&[0xBF, 0xF6, 0xB7, 0xCF], DecoderTrap::Strict)
            .unwrap(),
    );
    table.insert(
        JobSpriteId::RANGER,
        encoding::all::WINDOWS_1252
            .decode(&[0xB7, 0xB9, 0xC0, 0xCE, 0xC1, 0xAE], DecoderTrap::Strict)
            .unwrap(),
    );
    table.insert(
        JobSpriteId::ARCHBISHOP,
        encoding::all::WINDOWS_1252
            .decode(
                &[0xBE, 0xC6, 0xC5, 0xA9, 0xBA, 0xF1, 0xBC, 0xF3],
                DecoderTrap::Strict,
            )
            .unwrap(),
    );
    table.insert(
        JobSpriteId::MECHANIC,
        encoding::all::WINDOWS_1252
            .decode(&[0xB9, 0xCC, 0xC4, 0xC9, 0xB4, 0xD0], DecoderTrap::Strict)
            .unwrap(),
    );
    table.insert(
        JobSpriteId::GuillotineCross,
        encoding::all::WINDOWS_1252
            .decode(
                &[
                    0xB1, 0xE6, 0xB7, 0xCE, 0xC6, 0xBE, 0xC5, 0xA9, 0xB7, 0xCE, 0xBD, 0xBA,
                ],
                DecoderTrap::Strict,
            )
            .unwrap(),
    );

    table.insert(
        JobSpriteId::RoyalGuard,
        encoding::all::WINDOWS_1252
            .decode(&[0xB0, 0xA1, 0xB5, 0xE5], DecoderTrap::Strict)
            .unwrap(),
    );
    table.insert(
        JobSpriteId::SORCERER,
        encoding::all::WINDOWS_1252
            .decode(&[0xBC, 0xD2, 0xBC, 0xAD, 0xB7, 0xAF], DecoderTrap::Strict)
            .unwrap(),
    );
    table.insert(
        JobSpriteId::MINSTREL,
        encoding::all::WINDOWS_1252
            .decode(
                &[0xB9, 0xCE, 0xBD, 0xBA, 0xC6, 0xAE, 0xB7, 0xB2],
                DecoderTrap::Strict,
            )
            .unwrap(),
    );
    table.insert(
        JobSpriteId::WANDERER,
        encoding::all::WINDOWS_1252
            .decode(&[0xBF, 0xF8, 0xB4, 0xF5, 0xB7, 0xAF], DecoderTrap::Strict)
            .unwrap(),
    );
    table.insert(
        JobSpriteId::SURA,
        encoding::all::WINDOWS_1252
            .decode(&[0xBD, 0xB4, 0xB6, 0xF3], DecoderTrap::Strict)
            .unwrap(),
    );
    table.insert(
        JobSpriteId::GENETIC,
        encoding::all::WINDOWS_1252
            .decode(&[0xC1, 0xA6, 0xB3, 0xD7, 0xB8, 0xAF], DecoderTrap::Strict)
            .unwrap(),
    );
    table.insert(
        JobSpriteId::ShadowChaser,
        encoding::all::WINDOWS_1252
            .decode(
                &[
                    0xBD, 0xA6, 0xB5, 0xB5, 0xBF, 0xEC, 0xC3, 0xBC, 0xC0, 0xCC, 0xBC, 0xAD,
                ],
                DecoderTrap::Strict,
            )
            .unwrap(),
    );

    table.insert(
        JobSpriteId::RuneKnight2,
        encoding::all::WINDOWS_1252
            .decode(
                &[
                    0xB7, 0xE9, 0xB3, 0xAA, 0xC0, 0xCC, 0xC6, 0xAE, 0xBB, 0xDA, 0xB6, 0xEC,
                ],
                DecoderTrap::Strict,
            )
            .unwrap(),
    );
    table.insert(
        JobSpriteId::RoyalGuard2,
        encoding::all::WINDOWS_1252
            .decode(
                &[0xB1, 0xD7, 0xB8, 0xAE, 0xC6, 0xF9, 0xB0, 0xA1, 0xB5, 0xE5],
                DecoderTrap::Strict,
            )
            .unwrap(),
    );
    table.insert(
        JobSpriteId::RANGER2,
        encoding::all::WINDOWS_1252
            .decode(
                &[0xB7, 0xB9, 0xC0, 0xCE, 0xC1, 0xAE, 0xB4, 0xC1, 0xB4, 0xEB],
                DecoderTrap::Strict,
            )
            .unwrap(),
    );
    table.insert(
        JobSpriteId::MECHANIC2,
        encoding::all::WINDOWS_1252
            .decode(
                &[0xB8, 0xB6, 0xB5, 0xB5, 0xB1, 0xE2, 0xBE, 0xEE],
                DecoderTrap::Strict,
            )
            .unwrap(),
    );

    return table;
}
