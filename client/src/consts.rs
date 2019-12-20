use encoding;
use encoding::types::Encoding;
use encoding::DecoderTrap;
use std::collections::HashMap;

use rustarok_common::components::job_ids::JobSpriteId;
use serde::Deserialize;
use serde::Serialize;
use strum_macros::Display;
use strum_macros::EnumIter;
use strum_macros::EnumString;

pub const PLAYABLE_CHAR_SPRITES: [JobSpriteId; 13] = [
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
    //    JobSpriteId::RANGER,
];

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
    table.insert(JobSpriteId::MAGICIAN, "¸¶¹Ý»Ç".to_owned());
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
