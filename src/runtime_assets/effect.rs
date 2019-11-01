use crate::asset::asset_loader::AssetLoader;
use crate::asset::database::AssetDatabase;
use crate::asset::str::StrFile;
use crate::common::measure_time;
use crate::effect::StrEffectType;
use crate::my_gl::Gl;
use crate::systems::render::opengl_render_sys::StrEffectCache;
use strum::IntoEnumIterator;

pub fn load_str_effects(
    gl: &Gl,
    asset_loader: &AssetLoader,
    mut asset_db: &mut AssetDatabase,
) -> (Vec<StrFile>, StrEffectCache) {
    let (str_effects, str_effect_cache) = {
        let mut str_effect_cache = StrEffectCache::new();
        let (elapsed, str_effects) =
            measure_time(|| load_effects(gl, &asset_loader, &mut asset_db, &mut str_effect_cache));
        log::info!("str loaded: {}ms", elapsed.as_millis());
        (str_effects, str_effect_cache)
    };
    (str_effects, str_effect_cache)
}

fn load_effects(
    gl: &Gl,
    asset_loader: &AssetLoader,
    asset_db: &mut AssetDatabase,
    effect_cache: &mut StrEffectCache,
) -> Vec<StrFile> {
    let mut str_effects: Vec<StrFile> = Vec::new();

    for effect_type in StrEffectType::iter() {
        load_and_prepare_effect(
            gl,
            effect_type.get_effect_filename(),
            effect_type,
            &mut str_effects,
            asset_loader,
            asset_db,
            effect_cache,
        )
    }

    str_effects
}

pub fn load_and_prepare_effect(
    gl: &Gl,
    name: &str,
    effect_id: StrEffectType,
    str_effects: &mut Vec<StrFile>,
    asset_loader: &AssetLoader,
    asset_db: &mut AssetDatabase,
    effect_cache: &mut StrEffectCache,
) {
    let str_file = asset_loader.load_effect(gl, name, asset_db).unwrap();
    effect_cache.precache_effect(gl, effect_id.into(), &str_file);
    str_effects.push(str_file);
}
