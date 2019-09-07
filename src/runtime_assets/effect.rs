use crate::asset::database::AssetDatabase;
use crate::asset::str::StrFile;
use crate::asset::AssetLoader;
use crate::common::measure_time;
use crate::effect::StrEffectType;
use crate::my_gl::Gl;
use crate::systems::render::opengl_render_sys::StrEffectCache;

pub fn load_str_effects(
    gl: &Gl,
    asset_loader: &AssetLoader,
    mut asset_database: &mut AssetDatabase,
) -> (Vec<StrFile>, StrEffectCache) {
    let (str_effects, str_effect_cache) = {
        let mut str_effect_cache = StrEffectCache::new();
        let (elapsed, str_effects) = measure_time(|| {
            load_effects(
                gl,
                &asset_loader,
                &mut asset_database,
                &mut str_effect_cache,
            )
        });
        log::info!("str loaded: {}ms", elapsed.as_millis());
        (str_effects, str_effect_cache)
    };
    (str_effects, str_effect_cache)
}

fn load_effects(
    gl: &Gl,
    asset_loader: &AssetLoader,
    asset_database: &mut AssetDatabase,
    effect_cache: &mut StrEffectCache,
) -> Vec<StrFile> {
    let mut str_effects: Vec<StrFile> = Vec::new();

    load_and_prepare_effect(
        gl,
        "firewall",
        StrEffectType::FireWall,
        &mut str_effects,
        asset_loader,
        asset_database,
        effect_cache,
    );
    load_and_prepare_effect(
        gl,
        "stormgust",
        StrEffectType::StormGust,
        &mut str_effects,
        asset_loader,
        asset_database,
        effect_cache,
    );
    load_and_prepare_effect(
        gl,
        "lord",
        StrEffectType::LordOfVermilion,
        &mut str_effects,
        asset_loader,
        asset_database,
        effect_cache,
    );

    load_and_prepare_effect(
        gl,
        "lightning",
        StrEffectType::Lightning,
        &mut str_effects,
        asset_loader,
        asset_database,
        effect_cache,
    );
    load_and_prepare_effect(
        gl,
        "concentration",
        StrEffectType::Concentration,
        &mut str_effects,
        asset_loader,
        asset_database,
        effect_cache,
    );
    load_and_prepare_effect(
        gl,
        "moonstar",
        StrEffectType::Moonstar,
        &mut str_effects,
        asset_loader,
        asset_database,
        effect_cache,
    );
    load_and_prepare_effect(
        gl,
        "hunter_poison",
        StrEffectType::Poison,
        &mut str_effects,
        asset_loader,
        asset_database,
        effect_cache,
    );
    load_and_prepare_effect(
        gl,
        "quagmire",
        StrEffectType::Quagmire,
        &mut str_effects,
        asset_loader,
        asset_database,
        effect_cache,
    );
    load_and_prepare_effect(
        gl,
        "firewall_blue",
        StrEffectType::FireWallBlue,
        &mut str_effects,
        asset_loader,
        asset_database,
        effect_cache,
    );

    load_and_prepare_effect(
        gl,
        "firepillarbomb",
        StrEffectType::FirePillarBomb,
        &mut str_effects,
        asset_loader,
        asset_database,
        effect_cache,
    );
    load_and_prepare_effect(
        gl,
        "ramadan",
        StrEffectType::Ramadan,
        &mut str_effects,
        asset_loader,
        asset_database,
        effect_cache,
    );

    str_effects
}

pub fn load_and_prepare_effect(
    gl: &Gl,
    name: &str,
    effect_id: StrEffectType,
    str_effects: &mut Vec<StrFile>,
    asset_loader: &AssetLoader,
    asset_database: &mut AssetDatabase,
    effect_cache: &mut StrEffectCache,
) {
    let str_file = asset_loader.load_effect(gl, name, asset_database).unwrap();
    effect_cache.precache_effect(gl, effect_id.into(), &str_file);
    str_effects.push(str_file);
}
