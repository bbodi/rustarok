use std::collections::HashMap;
use std::string::ToString;

use encoding::types::Encoding;
use encoding::DecoderTrap;
use sdl2::ttf::Sdl2TtfContext;
use strum::IntoEnumIterator;

use crate::asset::database::AssetDatabase;
use crate::asset::texture::{GlTexture, TextureId, DUMMY_TEXTURE_ID_FOR_TEST};
use crate::asset::{AssetLoader, SpriteResource};
use crate::common::measure_time;
use crate::components::char::CharActionIndex;
use crate::components::controller::SkillKey;
use crate::components::skills::skills::Skills;
use crate::consts::{job_name_table, JobId, JobSpriteId, MonsterId, PLAYABLE_CHAR_SPRITES};
use crate::my_gl::{Gl, MyGlEnum};
use crate::systems::console_commands::STATUS_NAMES;
use crate::systems::{EffectSprites, Sprites};
use crate::video::Video;

pub struct Texts {
    pub skill_name_texts: HashMap<Skills, TextureId>,
    pub skill_key_texts: HashMap<SkillKey, TextureId>,
    pub custom_texts: HashMap<String, TextureId>,
    pub attack_absorbed: TextureId,
    pub attack_blocked: TextureId,
    pub minus: TextureId,
    pub plus: TextureId,
}

impl Texts {
    pub fn new_for_test() -> Texts {
        Texts {
            skill_name_texts: Default::default(),
            skill_key_texts: Default::default(),
            custom_texts: Default::default(),
            attack_absorbed: DUMMY_TEXTURE_ID_FOR_TEST,
            attack_blocked: DUMMY_TEXTURE_ID_FOR_TEST,
            minus: DUMMY_TEXTURE_ID_FOR_TEST,
            plus: DUMMY_TEXTURE_ID_FOR_TEST,
        }
    }
}

pub fn load_sprites(gl: &Gl, asset_loader: &AssetLoader, asset_db: &mut AssetDatabase) -> Sprites {
    let (elapsed, sprites) = measure_time(|| {
        let job_sprite_name_table = job_name_table();
        let mut exoskeleton = asset_loader
            .load_spr_and_act(gl, "data\\sprite\\ÀÎ°£Á·\\¸öÅë\\³²\\¸¶µµ±â¾î_³²", asset_db)
            .unwrap();
        // for Idle action, character sprites contains head rotating animations, we don't need them
        exoskeleton
            .action
            .remove_frames_in_every_direction(CharActionIndex::Idle as usize, 1..);
        Sprites {
            cursors: asset_loader
                .load_spr_and_act(gl, "data\\sprite\\cursors", asset_db)
                .unwrap(),
            exoskeleton,
            ginseng_bullet: asset_loader
                .load_spr_and_act(gl, "data\\sprite\\¸ó½ºÅÍ\\ginseng_bullet", asset_db)
                .unwrap(),
            arrow: asset_loader
                .load_spr_and_act(gl, "data\\sprite\\npc\\skel_archer_arrow", asset_db)
                .unwrap(),
            falcon: asset_loader
                .load_spr_and_act(gl, "data\\sprite\\ÀÌÆÑÆ®\\¸Å", asset_db)
                .unwrap(),
            stun: asset_loader
                .load_spr_and_act(gl, "data\\sprite\\ÀÌÆÑÆ®\\status-stun", asset_db)
                .unwrap(),
            timefont: asset_loader
                .load_spr_and_act(gl, "data\\sprite\\ÀÌÆÑÆ®\\timefont", asset_db)
                .unwrap(),
            numbers: GlTexture::from_file(gl, "assets/damage.bmp", asset_db),
            magic_target: asset_loader
                .load_texture(
                    gl,
                    "data\\texture\\effect\\magic_target.tga",
                    MyGlEnum::NEAREST,
                    asset_db,
                )
                .unwrap(),
            fire_particle: asset_loader
                .load_texture(
                    gl,
                    "data\\texture\\effect\\fireparticle.tga",
                    MyGlEnum::NEAREST,
                    asset_db,
                )
                .unwrap(),
            clock: asset_loader
                .load_texture(
                    gl,
                    "data\\texture\\effect\\blast_mine##clock.bmp",
                    MyGlEnum::NEAREST,
                    asset_db,
                )
                .unwrap(),
            mounted_character_sprites: {
                let mut mounted_sprites = HashMap::new();
                let mounted_file_name = &job_sprite_name_table[&JobSpriteId::CRUSADER2];
                let folder1 = encoding::all::WINDOWS_1252
                    .decode(&[0xC0, 0xCE, 0xB0, 0xA3, 0xC1, 0xB7], DecoderTrap::Strict)
                    .unwrap();
                let folder2 = encoding::all::WINDOWS_1252
                    .decode(&[0xB8, 0xF6, 0xC5, 0xEB], DecoderTrap::Strict)
                    .unwrap();
                let male_file_name = format!(
                    "data\\sprite\\{}\\{}\\³²\\{}_³²",
                    folder1, folder2, mounted_file_name
                );
                let mut male = asset_loader
                    .load_spr_and_act(gl, &male_file_name, asset_db)
                    .expect(&format!("Failed loading {:?}", JobSpriteId::CRUSADER2));
                // for Idle action, character sprites contains head rotating animations, we don't need them
                male.action
                    .remove_frames_in_every_direction(CharActionIndex::Idle as usize, 1..);
                let female = male.clone();
                mounted_sprites.insert(JobId::CRUSADER, [male, female]);
                mounted_sprites
            },
            character_sprites: load_char_sprites(
                gl,
                asset_loader,
                asset_db,
                &job_sprite_name_table,
            ),
            head_sprites: [
                (1..=25)
                    .map(|i| {
                        let male_file_name =
                            format!("data\\sprite\\ÀÎ°£Á·\\¸Ó¸®Åë\\³²\\{}_³²", i.to_string());
                        let male = if asset_loader.exists(&(male_file_name.clone() + ".act")) {
                            let mut head = asset_loader
                                .load_spr_and_act(gl, &male_file_name, asset_db)
                                .expect(&format!("Failed loading head({})", i));
                            // for Idle action, character sprites contains head rotating animations, we don't need them
                            head.action.remove_frames_in_every_direction(
                                CharActionIndex::Idle as usize,
                                1..,
                            );
                            Some(head)
                        } else {
                            None
                        };
                        male
                    })
                    .filter_map(|it| it)
                    .collect::<Vec<SpriteResource>>(),
                (1..=25)
                    .map(|i| {
                        let female_file_name =
                            format!("data\\sprite\\ÀÎ°£Á·\\¸Ó¸®Åë\\¿©\\{}_¿©", i.to_string());
                        let female = if asset_loader.exists(&(female_file_name.clone() + ".act")) {
                            let mut head = asset_loader
                                .load_spr_and_act(gl, &female_file_name, asset_db)
                                .expect(&format!("Failed loading head({})", i));
                            // for Idle action, character sprites contains head rotating animations, we don't need them
                            head.action.remove_frames_in_every_direction(
                                CharActionIndex::Idle as usize,
                                1..,
                            );
                            Some(head)
                        } else {
                            None
                        };
                        female
                    })
                    .filter_map(|it| it)
                    .collect::<Vec<SpriteResource>>(),
            ],
            monster_sprites: MonsterId::iter()
                .map(|monster_id| {
                    let file_name = format!(
                        "data\\sprite\\npc\\{}",
                        monster_id.to_string().to_lowercase()
                    );
                    (
                        monster_id,
                        asset_loader
                            .load_spr_and_act(gl, &file_name, asset_db)
                            .or_else(|_e| {
                                let file_name = format!(
                                    "data\\sprite\\¸ó½ºÅÍ\\{}",
                                    monster_id.to_string().to_lowercase()
                                );
                                asset_loader.load_spr_and_act(gl, &file_name, asset_db)
                            })
                            .unwrap(),
                    )
                })
                .collect::<HashMap<MonsterId, SpriteResource>>(),
            effect_sprites: EffectSprites {
                torch: asset_loader
                    .load_spr_and_act(gl, "data\\sprite\\ÀÌÆÑÆ®\\torch_01", asset_db)
                    .unwrap(),
                fire_wall: asset_loader
                    .load_spr_and_act(gl, "data\\sprite\\ÀÌÆÑÆ®\\firewall", asset_db)
                    .unwrap(),
                fire_ball: asset_loader
                    .load_spr_and_act(gl, "data\\sprite\\ÀÌÆÑÆ®\\fireball", asset_db)
                    .unwrap(),
                plasma: asset_loader
                    .load_spr_and_act(gl, "data\\sprite\\¸ó½ºÅÍ\\plasma_r", asset_db)
                    .unwrap(),
            },
        }
    });

    log::info!(
        "act and spr files loaded[{}]: {}ms",
        (sprites.character_sprites.len() * 2)
            + sprites.head_sprites[0].len()
            + sprites.head_sprites[1].len()
            + sprites.monster_sprites.len(),
        elapsed.as_millis()
    );
    return sprites;
}

fn load_char_sprites(
    gl: &Gl,
    asset_loader: &AssetLoader,
    asset_db: &mut AssetDatabase,
    job_sprite_name_table: &HashMap<JobSpriteId, String>,
) -> HashMap<JobSpriteId, [[SpriteResource; 2]; 2]> {
    PLAYABLE_CHAR_SPRITES
        .iter()
        .map(|job_sprite_id| {
            let job_file_name = &job_sprite_name_table[&job_sprite_id];
            let folder1 = encoding::all::WINDOWS_1252
                .decode(&[0xC0, 0xCE, 0xB0, 0xA3, 0xC1, 0xB7], DecoderTrap::Strict)
                .unwrap();
            let folder2 = encoding::all::WINDOWS_1252
                .decode(&[0xB8, 0xF6, 0xC5, 0xEB], DecoderTrap::Strict)
                .unwrap();
            let male_file_path = format!(
                "data\\sprite\\{}\\{}\\³²\\{}_³²",
                folder1, folder2, job_file_name
            );
            let female_file_path = format!(
                "data\\sprite\\{}\\{}\\¿©\\{}_¿©",
                folder1, folder2, job_file_name
            );

            // order is red, blue
            let (male_palette_ids, female_palette_ids) = match job_sprite_id {
                JobSpriteId::CRUSADER => ([153, 152], [153, 152]),
                JobSpriteId::SWORDMAN => ([153, 152], [153, 152]),
                JobSpriteId::ARCHER => ([153, 152], [153, 152]),
                JobSpriteId::ASSASSIN => ([153, 152], [153, 152]),
                JobSpriteId::ROGUE => ([153, 152], [153, 152]),
                JobSpriteId::KNIGHT => ([153, 152], [153, 152]),
                JobSpriteId::WIZARD => ([153, 152], [153, 152]),
                JobSpriteId::SAGE => ([153, 152], [153, 152]),
                JobSpriteId::ALCHEMIST => ([153, 152], [153, 152]),
                JobSpriteId::BLACKSMITH => ([153, 152], [153, 152]),
                JobSpriteId::PRIEST => ([153, 152], [153, 152]),
                JobSpriteId::MONK => ([153, 152], [153, 152]),
                JobSpriteId::GUNSLINGER => ([153, 152], [153, 152]),
                JobSpriteId::HUNTER => ([153, 152], [153, 152]),
                _ => panic!(),
            };

            let (male_red, male_blue, female_red, female_blue) =
                if !asset_loader.exists(&format!("{}.act", female_file_path)) {
                    let mut male = asset_loader
                        .load_spr_and_act(gl, &male_file_path, asset_db)
                        .expect(&format!("Failed loading {:?}", job_sprite_id));
                    // for Idle action, character sprites contains head rotating animations, we don't need them
                    male.action
                        .remove_frames_in_every_direction(CharActionIndex::Idle as usize, 1..);
                    let female = male.clone();
                    (male.clone(), female.clone(), male, female)
                } else if !asset_loader.exists(&format!("{}.act", male_file_path)) {
                    let mut female = asset_loader
                        .load_spr_and_act(gl, &female_file_path, asset_db)
                        .expect(&format!("Failed loading {:?}", job_sprite_id));
                    // for Idle action, character sprites contains head rotating animations, we don't need them
                    female
                        .action
                        .remove_frames_in_every_direction(CharActionIndex::Idle as usize, 1..);
                    let male = female.clone();
                    (male.clone(), female.clone(), male, female)
                } else {
                    let male_red = load_sprite(
                        gl,
                        &asset_loader,
                        asset_db,
                        &job_sprite_id,
                        &job_file_name,
                        &male_file_path,
                        male_palette_ids[0],
                    );
                    let male_blue = load_sprite(
                        gl,
                        &asset_loader,
                        asset_db,
                        &job_sprite_id,
                        &job_file_name,
                        &male_file_path,
                        male_palette_ids[1],
                    );
                    let female_red = load_sprite(
                        gl,
                        &asset_loader,
                        asset_db,
                        &job_sprite_id,
                        &job_file_name,
                        &female_file_path,
                        female_palette_ids[0],
                    );
                    let female_blue = load_sprite(
                        gl,
                        &asset_loader,
                        asset_db,
                        &job_sprite_id,
                        &job_file_name,
                        &female_file_path,
                        female_palette_ids[1],
                    );
                    (male_red, male_blue, female_red, female_blue)
                };
            (
                *job_sprite_id,
                [[male_red, female_red], [male_blue, female_blue]],
            )
        })
        .collect::<HashMap<JobSpriteId, [[SpriteResource; 2]; 2]>>()
}

fn load_sprite(
    gl: &Gl,
    asset_loader: &AssetLoader,
    asset_db: &mut AssetDatabase,
    job_sprite_id: &JobSpriteId,
    job_file_name: &str,
    file_path: &str,
    palette_id: usize,
) -> SpriteResource {
    let mut sprite_res = asset_loader
        .load_spr_and_act_with_palette(
            gl,
            &file_path,
            asset_db,
            palette_id,
            &load_palette(asset_loader, &job_sprite_id, job_file_name, palette_id),
        )
        .expect(&format!("Failed loading {:?}", job_sprite_id));
    // for Idle action, character sprites contains head rotating animations, we don't need them
    sprite_res
        .action
        .remove_frames_in_every_direction(CharActionIndex::Idle as usize, 1..);
    sprite_res
}

fn load_palette(
    asset_loader: &AssetLoader,
    job_sprite_id: &JobSpriteId,
    job_file_name: &str,
    palette_id: usize,
) -> Vec<u8> {
    let palette = {
        // for some jobs, the palette file name is truncated, so this
        // code tries names one by one removing the last char in each
        // iteration
        let mut tmp_name: String = job_file_name.to_owned();
        loop {
            if tmp_name.is_empty() {
                break Err("".to_owned());
            }
            let pal = asset_loader.get_content(&format!(
                "data\\palette\\¸ö\\{}_³²_{}.pal",
                tmp_name, palette_id
            ));
            if pal.is_ok() {
                break pal;
            }
            tmp_name.pop();
        }
    }
    .expect(&format!(
        "Couldn't load palette file for {}, id: {}",
        job_sprite_id, palette_id
    ));
    palette
}

pub fn load_status_icons(
    gl: &Gl,
    asset_loader: &AssetLoader,
    asset_db: &mut AssetDatabase,
) -> HashMap<&'static str, TextureId> {
    let mut status_icons = HashMap::new();
    status_icons.insert(
        "shield",
        asset_loader
            .load_texture(
                gl,
                "data\\texture\\À¯ÀúÀÎÅÍÆäÀÌ½º\\item\\pa_shieldchain.bmp",
                MyGlEnum::NEAREST,
                asset_db,
            )
            .unwrap(),
    );
    return status_icons;
}

pub fn load_skill_icons(
    gl: &Gl,
    asset_loader: &AssetLoader,
    asset_db: &mut AssetDatabase,
) -> HashMap<Skills, TextureId> {
    let mut skill_icons = HashMap::new();
    for skill in Skills::iter() {
        let def = skill.get_definition();
        if def.get_icon_path().is_empty() {
            continue;
        }
        let skill_icon = asset_db
            .get_texture_id(&def.get_icon_path())
            .unwrap_or_else(|| {
                asset_loader
                    .load_texture(gl, def.get_icon_path(), MyGlEnum::NEAREST, asset_db)
                    .unwrap()
            });
        skill_icons.insert(skill, skill_icon);
    }
    return skill_icons;
}

pub const FONT_SIZE_SKILL_KEY: i32 = 20;

pub fn load_texts(gl: &Gl, ttf_context: &Sdl2TtfContext, asset_db: &mut AssetDatabase) -> Texts {
    let skill_name_font =
        Video::load_font(ttf_context, "assets/fonts/UbuntuMono-B.ttf", 32).unwrap();
    let mut skill_name_font_outline =
        Video::load_font(ttf_context, "assets/fonts/UbuntuMono-B.ttf", 32).unwrap();
    skill_name_font_outline.set_outline_width(2);

    let skill_key_font = Video::load_font(
        ttf_context,
        "assets/fonts/UbuntuMono-B.ttf",
        FONT_SIZE_SKILL_KEY as u16,
    )
    .unwrap();
    let mut skill_key_font_bold_outline = Video::load_font(
        ttf_context,
        "assets/fonts/UbuntuMono-B.ttf",
        FONT_SIZE_SKILL_KEY as u16,
    )
    .unwrap();
    skill_key_font_bold_outline.set_outline_width(2);

    let mut skill_key_font_outline = Video::load_font(
        ttf_context,
        "assets/fonts/UbuntuMono-B.ttf",
        FONT_SIZE_SKILL_KEY as u16,
    )
    .unwrap();
    skill_key_font_outline.set_outline_width(1);

    let small_font = Video::load_font(ttf_context, "assets/fonts/UbuntuMono-B.ttf", 14).unwrap();
    let mut small_font_outline =
        Video::load_font(ttf_context, "assets/fonts/UbuntuMono-B.ttf", 14).unwrap();
    small_font_outline.set_outline_width(1);

    let mut texts = Texts {
        skill_name_texts: HashMap::new(),
        skill_key_texts: HashMap::new(),
        custom_texts: HashMap::new(),
        attack_absorbed: Video::create_outline_text_texture(
            gl,
            &skill_key_font,
            &skill_key_font_bold_outline,
            "absorb",
            asset_db,
        ),
        attack_blocked: Video::create_outline_text_texture(
            gl,
            &skill_key_font,
            &skill_key_font_bold_outline,
            "block",
            asset_db,
        ),
        minus: Video::create_outline_text_texture(
            gl,
            &small_font,
            &small_font_outline,
            "-",
            asset_db,
        ),
        plus: Video::create_outline_text_texture(
            gl,
            &small_font,
            &small_font_outline,
            "+",
            asset_db,
        ),
    };

    for name in &[
        "Poison",
        "AbsorbShield",
        "FireBomb",
        "ArmorUp",
        "ArmorDown",
        "Heal",
        "Damage",
    ] {
        texts.custom_texts.insert(
            name.to_string(),
            Video::create_outline_text_texture(
                gl,
                &skill_key_font,
                &skill_key_font_outline,
                name,
                asset_db,
            ),
        );
    }
    STATUS_NAMES.iter().for_each(|name| {
        texts.custom_texts.insert(
            name.to_string(),
            Video::create_outline_text_texture(
                gl,
                &skill_key_font,
                &skill_key_font_outline,
                name,
                asset_db,
            ),
        );
    });

    for skill in Skills::iter() {
        let texture = Video::create_outline_text_texture(
            gl,
            &skill_name_font,
            &skill_name_font_outline,
            &format!("{:?}", skill),
            asset_db,
        );
        texts.skill_name_texts.insert(skill, texture);
    }

    for skill_key in SkillKey::iter() {
        let texture = Video::create_outline_text_texture(
            gl,
            &skill_key_font,
            &skill_key_font_bold_outline,
            &skill_key.to_string(),
            asset_db,
        );
        texts.skill_key_texts.insert(skill_key, texture);
    }

    for i in -200..=200 {
        texts.custom_texts.insert(
            i.to_string(),
            Video::create_outline_text_texture(
                gl,
                &small_font,
                &small_font_outline,
                &format!("{:+}", i),
                asset_db,
            ),
        );
    }
    return texts;
}
