use std::collections::HashMap;
use std::string::ToString;

use sdl2::ttf::Sdl2TtfContext;
use strum::IntoEnumIterator;

use crate::asset::database::AssetDatabase;
use crate::asset::texture::{TextureId, DUMMY_TEXTURE_ID_FOR_TEST};
use crate::asset::AssetLoader;
use crate::components::controller::SkillKey;
use crate::components::skills::skills::Skills;
use crate::my_gl::{Gl, MyGlEnum};
use crate::systems::console_commands::STATUS_NAMES;
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
