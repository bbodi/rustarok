use crate::asset::database::AssetDatabase;
use crate::asset::{AssetLoader, SpriteResource};
use crate::common::measure_time;
use crate::components::char::CharActionIndex;
use crate::components::controller::SkillKey;
use crate::components::skills::skills::Skills;
use crate::consts::{job_name_table, JobId, JobSpriteId, MonsterId, PLAYABLE_CHAR_SPRITES};
use crate::my_gl::{Gl, MyGlEnum};
use crate::systems::console_commands::STATUS_NAMES;
use crate::systems::{EffectSprites, Sprites};
use crate::video::{GlTexture, Video};
use encoding::types::Encoding;
use encoding::DecoderTrap;
use sdl2::ttf::Sdl2TtfContext;
use std::collections::HashMap;
use std::string::ToString;
use strum::IntoEnumIterator;

pub struct Texts {
    // TODO: texture id instead?
    pub skill_name_texts: HashMap<Skills, GlTexture>,
    pub skill_key_texts: HashMap<SkillKey, GlTexture>,
    pub custom_texts: HashMap<String, GlTexture>,
    pub attack_absorbed: GlTexture,
    pub attack_blocked: GlTexture,
    pub minus: GlTexture,
    pub plus: GlTexture,
}

pub fn load_sprites(
    gl: &Gl,
    asset_loader: &AssetLoader,
    asset_database: &mut AssetDatabase,
) -> Sprites {
    let (elapsed, sprites) = measure_time(|| {
        let job_sprite_name_table = job_name_table();
        Sprites {
            cursors: asset_loader
                .load_spr_and_act(gl, "data\\sprite\\cursors", asset_database)
                .unwrap(),
            ginseng_bullet: asset_loader
                .load_spr_and_act(
                    gl,
                    "data\\sprite\\¸ó½ºÅÍ\\ginseng_bullet",
                    asset_database,
                )
                .unwrap(),
            falcon: asset_loader
                .load_spr_and_act(gl, "data\\sprite\\ÀÌÆÑÆ®\\¸Å", asset_database)
                .unwrap(),
            stun: asset_loader
                .load_spr_and_act(
                    gl,
                    "data\\sprite\\ÀÌÆÑÆ®\\status-stun",
                    asset_database,
                )
                .unwrap(),
            timefont: asset_loader
                .load_spr_and_act(gl, "data\\sprite\\ÀÌÆÑÆ®\\timefont", asset_database)
                .unwrap(),
            numbers: GlTexture::from_file(gl, "assets\\damage.bmp", asset_database),
            magic_target: asset_loader
                .load_texture(
                    gl,
                    "data\\texture\\effect\\magic_target.tga",
                    MyGlEnum::NEAREST,
                    asset_database,
                )
                .unwrap(),
            fire_particle: asset_loader
                .load_texture(
                    gl,
                    "data\\texture\\effect\\fireparticle.tga",
                    MyGlEnum::NEAREST,
                    asset_database,
                )
                .unwrap(),
            clock: asset_loader
                .load_texture(
                    gl,
                    "data\\texture\\effect\\blast_mine##clock.bmp",
                    MyGlEnum::NEAREST,
                    asset_database,
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
                    .load_spr_and_act(gl, &male_file_name, asset_database)
                    .expect(&format!("Failed loading {:?}", JobSpriteId::CRUSADER2));
                // for Idle action, character sprites contains head rotating animations, we don't need them
                male.action
                    .remove_frames_in_every_direction(CharActionIndex::Idle as usize, 1..);
                let female = male.clone();
                mounted_sprites.insert(JobId::CRUSADER, [male, female]);
                mounted_sprites
            },
            character_sprites: PLAYABLE_CHAR_SPRITES
                .iter()
                .map(|job_sprite_id| {
                    let job_file_name = &job_sprite_name_table[&job_sprite_id];
                    let folder1 = encoding::all::WINDOWS_1252
                        .decode(&[0xC0, 0xCE, 0xB0, 0xA3, 0xC1, 0xB7], DecoderTrap::Strict)
                        .unwrap();
                    let folder2 = encoding::all::WINDOWS_1252
                        .decode(&[0xB8, 0xF6, 0xC5, 0xEB], DecoderTrap::Strict)
                        .unwrap();
                    let male_file_name = format!(
                        "data\\sprite\\{}\\{}\\³²\\{}_³²",
                        folder1, folder2, job_file_name
                    );
                    let female_file_name = format!(
                        "data\\sprite\\{}\\{}\\¿©\\{}_¿©",
                        folder1, folder2, job_file_name
                    );
                    let (male, female) = if !asset_loader
                        .exists(&format!("{}.act", female_file_name))
                    {
                        let mut male = asset_loader
                            .load_spr_and_act(gl, &male_file_name, asset_database)
                            .expect(&format!("Failed loading {:?}", job_sprite_id));
                        // for Idle action, character sprites contains head rotating animations, we don't need them
                        male.action
                            .remove_frames_in_every_direction(CharActionIndex::Idle as usize, 1..);
                        let female = male.clone();
                        (male, female)
                    } else if !asset_loader.exists(&format!("{}.act", male_file_name)) {
                        let mut female = asset_loader
                            .load_spr_and_act(gl, &female_file_name, asset_database)
                            .expect(&format!("Failed loading {:?}", job_sprite_id));
                        // for Idle action, character sprites contains head rotating animations, we don't need them
                        female
                            .action
                            .remove_frames_in_every_direction(CharActionIndex::Idle as usize, 1..);
                        let male = female.clone();
                        (male, female)
                    } else {
                        let mut male = asset_loader
                            .load_spr_and_act(gl, &male_file_name, asset_database)
                            .expect(&format!("Failed loading {:?}", job_sprite_id));
                        // for Idle action, character sprites contains head rotating animations, we don't need them
                        male.action
                            .remove_frames_in_every_direction(CharActionIndex::Idle as usize, 1..);
                        let mut female = asset_loader
                            .load_spr_and_act(gl, &female_file_name, asset_database)
                            .expect(&format!("Failed loading {:?}", job_sprite_id));
                        // for Idle action, character sprites contains head rotating animations, we don't need them
                        female
                            .action
                            .remove_frames_in_every_direction(CharActionIndex::Idle as usize, 1..);
                        (male, female)
                    };
                    (*job_sprite_id, [male, female])
                })
                .collect::<HashMap<JobSpriteId, [SpriteResource; 2]>>(),
            head_sprites: [
                (1..=25)
                    .map(|i| {
                        let male_file_name = format!(
                            "data\\sprite\\ÀÎ°£Á·\\¸Ó¸®Åë\\³²\\{}_³²",
                            i.to_string()
                        );
                        let male = if asset_loader.exists(&(male_file_name.clone() + ".act")) {
                            let mut head = asset_loader
                                .load_spr_and_act(gl, &male_file_name, asset_database)
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
                        let female_file_name = format!(
                            "data\\sprite\\ÀÎ°£Á·\\¸Ó¸®Åë\\¿©\\{}_¿©",
                            i.to_string()
                        );
                        let female = if asset_loader.exists(&(female_file_name.clone() + ".act")) {
                            let mut head = asset_loader
                                .load_spr_and_act(gl, &female_file_name, asset_database)
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
                            .load_spr_and_act(gl, &file_name, asset_database)
                            .or_else(|e| {
                                let file_name = format!(
                                    "data\\sprite\\¸ó½ºÅÍ\\{}",
                                    monster_id.to_string().to_lowercase()
                                );
                                asset_loader.load_spr_and_act(gl, &file_name, asset_database)
                            })
                            .unwrap(),
                    )
                })
                .collect::<HashMap<MonsterId, SpriteResource>>(),
            effect_sprites: EffectSprites {
                torch: asset_loader
                    .load_spr_and_act(gl, "data\\sprite\\ÀÌÆÑÆ®\\torch_01", asset_database)
                    .unwrap(),
                fire_wall: asset_loader
                    .load_spr_and_act(gl, "data\\sprite\\ÀÌÆÑÆ®\\firewall", asset_database)
                    .unwrap(),
                fire_ball: asset_loader
                    .load_spr_and_act(gl, "data\\sprite\\ÀÌÆÑÆ®\\fireball", asset_database)
                    .unwrap(),
                plasma: asset_loader
                    .load_spr_and_act(gl, "data\\sprite\\¸ó½ºÅÍ\\plasma_r", asset_database)
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

pub fn load_status_icons(
    gl: &Gl,
    asset_loader: &AssetLoader,
    asset_database: &mut AssetDatabase,
) -> HashMap<&'static str, GlTexture> {
    let mut status_icons = HashMap::new();
    status_icons.insert(
        "shield",
        asset_loader
            .load_texture(
                gl,
                "data\\texture\\À¯ÀúÀÎÅÍÆäÀÌ½º\\item\\pa_shieldchain.bmp",
                MyGlEnum::NEAREST,
                asset_database,
            )
            .unwrap(),
    );
    return status_icons;
}

pub fn load_skill_icons(
    gl: &Gl,
    asset_loader: &AssetLoader,
    asset_database: &mut AssetDatabase,
) -> HashMap<Skills, GlTexture> {
    let mut skill_icons = HashMap::new();
    for skill in Skills::iter() {
        let def = skill.get_definition();
        if def.get_icon_path().is_empty() {
            continue;
        }
        let skill_icon = asset_database
            .get_texture(gl, &def.get_icon_path())
            .unwrap_or_else(|| {
                asset_loader
                    .load_texture(gl, def.get_icon_path(), MyGlEnum::NEAREST, asset_database)
                    .unwrap()
            });
        skill_icons.insert(skill, skill_icon);
    }
    return skill_icons;
}

pub fn load_texts(
    gl: &Gl,
    ttf_context: &Sdl2TtfContext,
    asset_database: &mut AssetDatabase,
) -> Texts {
    let skill_name_font =
        Video::load_font(ttf_context, "assets/fonts/UbuntuMono-B.ttf", 32).unwrap();
    let mut skill_name_font_outline =
        Video::load_font(ttf_context, "assets/fonts/UbuntuMono-B.ttf", 32).unwrap();
    skill_name_font_outline.set_outline_width(2);

    let skill_key_font =
        Video::load_font(ttf_context, "assets/fonts/UbuntuMono-B.ttf", 20).unwrap();
    let mut skill_key_font_bold_outline =
        Video::load_font(ttf_context, "assets/fonts/UbuntuMono-B.ttf", 20).unwrap();
    skill_key_font_bold_outline.set_outline_width(2);

    let mut skill_key_font_outline =
        Video::load_font(ttf_context, "assets/fonts/UbuntuMono-B.ttf", 20).unwrap();
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
            asset_database,
        ),
        attack_blocked: Video::create_outline_text_texture(
            gl,
            &skill_key_font,
            &skill_key_font_bold_outline,
            "block",
            asset_database,
        ),
        minus: Video::create_outline_text_texture(
            gl,
            &small_font,
            &small_font_outline,
            "-",
            asset_database,
        ),
        plus: Video::create_outline_text_texture(
            gl,
            &small_font,
            &small_font_outline,
            "+",
            asset_database,
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
                asset_database,
            ),
        );
    }
    STATUS_NAMES.iter().for_each(|name| {
        let key = format!("outlinetext_{}", name);
        texts.custom_texts.insert(
            name.to_string(),
            Video::create_outline_text_texture(
                gl,
                &skill_key_font,
                &skill_key_font_outline,
                name,
                asset_database,
            ),
        );
    });

    for skill in Skills::iter() {
        let key = format!("outlinetext_{:?}", skill);
        let texture = Video::create_outline_text_texture(
            gl,
            &skill_name_font,
            &skill_name_font_outline,
            &format!("{:?}", skill),
            asset_database,
        );
        texts.skill_name_texts.insert(skill, texture);
    }

    for skill_key in SkillKey::iter() {
        let texture = Video::create_outline_text_texture(
            gl,
            &skill_key_font,
            &skill_key_font_bold_outline,
            &skill_key.to_string(),
            asset_database,
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
                asset_database,
            ),
        );
    }
    return texts;
}
