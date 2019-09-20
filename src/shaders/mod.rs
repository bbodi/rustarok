use crate::my_gl::{Gl, MyGlEnum};
use crate::video::{
    Shader, ShaderParam1f, ShaderParam1i, ShaderParam2fv, ShaderParam2i, ShaderParam3fv,
    ShaderParam3x3fv, ShaderParam4ubv, ShaderParam4x4fv, ShaderProgram,
};
use std::os::raw::c_uint;

pub struct Shaders {
    pub ground_shader: ShaderProgram<GroundShaderParameters>,
    pub model_shader: ShaderProgram<ModelShaderParameters>,
    pub sprite_shader: ShaderProgram<Sprite3dShaderParameters>,
    pub horiz_texture_shader: ShaderProgram<HorizTexture3dShaderParameters>,
    pub str_effect_shader: ShaderProgram<StrEffect3dShaderParameters>,
    pub sprite2d_shader: ShaderProgram<Texture2dShaderParameters>,
    pub trimesh_shader: ShaderProgram<Trimesh3dShaderParameters>,
    pub trimesh2d_shader: ShaderProgram<Trimesh2dShaderParameters>,
    pub point2d_shader: ShaderProgram<Point2dShaderParameters>,
}

pub fn load_shaders(gl: &Gl) -> Shaders {
    Shaders {
        ground_shader: ShaderProgram::from_shaders(
            gl,
            &[
                Shader::from_source(gl, include_str!("ground.vert"), MyGlEnum::VERTEX_SHADER)
                    .unwrap(),
                Shader::from_source(gl, include_str!("ground.frag"), MyGlEnum::FRAGMENT_SHADER)
                    .unwrap(),
            ],
            |program_id| GroundShaderParameters::new(gl, program_id),
        )
        .unwrap(),
        model_shader: ShaderProgram::from_shaders(
            gl,
            &[
                Shader::from_source(gl, include_str!("model.vert"), MyGlEnum::VERTEX_SHADER)
                    .unwrap(),
                Shader::from_source(gl, include_str!("model.frag"), MyGlEnum::FRAGMENT_SHADER)
                    .unwrap(),
            ],
            |program_id| ModelShaderParameters::new(gl, program_id),
        )
        .unwrap(),
        horiz_texture_shader: ShaderProgram::from_shaders(
            gl,
            &[
                Shader::from_source(
                    gl,
                    include_str!("horiz_texture.vert"),
                    MyGlEnum::VERTEX_SHADER,
                )
                .unwrap(),
                Shader::from_source(
                    gl,
                    include_str!("horiz_texture.frag"),
                    MyGlEnum::FRAGMENT_SHADER,
                )
                .unwrap(),
            ],
            |program_id| HorizTexture3dShaderParameters::new(gl, program_id),
        )
        .unwrap(),
        sprite_shader: ShaderProgram::from_shaders(
            gl,
            &[
                Shader::from_source(gl, include_str!("sprite.vert"), MyGlEnum::VERTEX_SHADER)
                    .unwrap(),
                Shader::from_source(gl, include_str!("sprite.frag"), MyGlEnum::FRAGMENT_SHADER)
                    .unwrap(),
            ],
            |program_id| Sprite3dShaderParameters::new(gl, program_id),
        )
        .unwrap(),
        str_effect_shader: ShaderProgram::from_shaders(
            gl,
            &[
                Shader::from_source(gl, include_str!("str_effect.vert"), MyGlEnum::VERTEX_SHADER)
                    .unwrap(),
                Shader::from_source(
                    gl,
                    include_str!("str_effect.frag"),
                    MyGlEnum::FRAGMENT_SHADER,
                )
                .unwrap(),
            ],
            |program_id| StrEffect3dShaderParameters::new(gl, program_id),
        )
        .unwrap(),
        sprite2d_shader: ShaderProgram::from_shaders(
            gl,
            &[
                Shader::from_source(gl, include_str!("sprite2d.vert"), MyGlEnum::VERTEX_SHADER)
                    .unwrap(),
                Shader::from_source(gl, include_str!("sprite2d.frag"), MyGlEnum::FRAGMENT_SHADER)
                    .unwrap(),
            ],
            |program_id| Texture2dShaderParameters::new(gl, program_id),
        )
        .unwrap(),
        trimesh_shader: ShaderProgram::from_shaders(
            gl,
            &[
                Shader::from_source(gl, include_str!("trimesh.vert"), MyGlEnum::VERTEX_SHADER)
                    .unwrap(),
                Shader::from_source(gl, include_str!("trimesh.frag"), MyGlEnum::FRAGMENT_SHADER)
                    .unwrap(),
            ],
            |program_id| Trimesh3dShaderParameters::new(gl, program_id),
        )
        .unwrap(),
        trimesh2d_shader: ShaderProgram::from_shaders(
            gl,
            &[
                Shader::from_source(gl, include_str!("trimesh2d.vert"), MyGlEnum::VERTEX_SHADER)
                    .unwrap(),
                Shader::from_source(
                    gl,
                    include_str!("trimesh2d.frag"),
                    MyGlEnum::FRAGMENT_SHADER,
                )
                .unwrap(),
            ],
            |program_id| Trimesh2dShaderParameters::new(gl, program_id),
        )
        .unwrap(),
        point2d_shader: ShaderProgram::from_shaders(
            gl,
            &[
                Shader::from_source(gl, include_str!("point2d.vert"), MyGlEnum::VERTEX_SHADER)
                    .unwrap(),
                Shader::from_source(gl, include_str!("point2d.frag"), MyGlEnum::FRAGMENT_SHADER)
                    .unwrap(),
            ],
            |program_id| Point2dShaderParameters::new(gl, program_id),
        )
        .unwrap(),
    }
}

pub struct Trimesh3dShaderParameters {
    pub projection_mat: ShaderParam4x4fv,
    pub model_mat: ShaderParam4x4fv,
    pub view_mat: ShaderParam4x4fv,
    pub color: ShaderParam4ubv,
    pub scale: ShaderParam2fv,
}

impl Trimesh3dShaderParameters {
    pub fn new(gl: &Gl, program_id: c_uint) -> Trimesh3dShaderParameters {
        Trimesh3dShaderParameters {
            projection_mat: ShaderParam4x4fv(Shader::get_location(gl, program_id, "projection")),
            model_mat: ShaderParam4x4fv(Shader::get_location(gl, program_id, "model")),
            view_mat: ShaderParam4x4fv(Shader::get_location(gl, program_id, "view")),
            color: ShaderParam4ubv(Shader::get_location(gl, program_id, "color")),
            scale: ShaderParam2fv(Shader::get_location(gl, program_id, "size")),
        }
    }
}

pub struct Texture2dShaderParameters {
    pub projection_mat: ShaderParam4x4fv,
    pub model_mat: ShaderParam4x4fv,
    pub color: ShaderParam4ubv,
    pub z: ShaderParam1f,
    pub offset: ShaderParam2i,
    pub size: ShaderParam2fv,
    pub texture: ShaderParam1i,
}

impl Texture2dShaderParameters {
    pub fn new(gl: &Gl, program_id: c_uint) -> Texture2dShaderParameters {
        Texture2dShaderParameters {
            projection_mat: ShaderParam4x4fv(Shader::get_location(gl, program_id, "projection")),
            model_mat: ShaderParam4x4fv(Shader::get_location(gl, program_id, "model")),
            color: ShaderParam4ubv(Shader::get_location(gl, program_id, "color")),
            z: ShaderParam1f(Shader::get_location(gl, program_id, "z")),
            offset: ShaderParam2i(Shader::get_location(gl, program_id, "offset")),
            size: ShaderParam2fv(Shader::get_location(gl, program_id, "size")),
            texture: ShaderParam1i(Shader::get_location(gl, program_id, "model_texture")),
        }
    }
}

pub struct HorizTexture3dShaderParameters {
    pub projection_mat: ShaderParam4x4fv,
    pub model_mat: ShaderParam4x4fv,
    pub view_mat: ShaderParam4x4fv,
    pub color: ShaderParam4ubv,
    pub size: ShaderParam2fv,
    pub texture: ShaderParam1i,
}

impl HorizTexture3dShaderParameters {
    pub fn new(gl: &Gl, program_id: c_uint) -> HorizTexture3dShaderParameters {
        HorizTexture3dShaderParameters {
            projection_mat: ShaderParam4x4fv(Shader::get_location(gl, program_id, "projection")),
            model_mat: ShaderParam4x4fv(Shader::get_location(gl, program_id, "model")),
            view_mat: ShaderParam4x4fv(Shader::get_location(gl, program_id, "view")),
            color: ShaderParam4ubv(Shader::get_location(gl, program_id, "color")),
            size: ShaderParam2fv(Shader::get_location(gl, program_id, "size")),
            texture: ShaderParam1i(Shader::get_location(gl, program_id, "model_texture")),
        }
    }
}

pub struct Sprite3dShaderParameters {
    pub projection_mat: ShaderParam4x4fv,
    pub model_mat: ShaderParam4x4fv,
    pub rot_mat: ShaderParam4x4fv,
    pub view_mat: ShaderParam4x4fv,
    pub color: ShaderParam4ubv,
    pub size: ShaderParam2fv,
    pub offset: ShaderParam2fv,
    pub texture: ShaderParam1i,
}

impl Sprite3dShaderParameters {
    pub fn new(gl: &Gl, program_id: c_uint) -> Sprite3dShaderParameters {
        Sprite3dShaderParameters {
            projection_mat: ShaderParam4x4fv(Shader::get_location(gl, program_id, "projection")),
            model_mat: ShaderParam4x4fv(Shader::get_location(gl, program_id, "model")),
            rot_mat: ShaderParam4x4fv(Shader::get_location(gl, program_id, "rot_mat")),
            view_mat: ShaderParam4x4fv(Shader::get_location(gl, program_id, "view")),
            color: ShaderParam4ubv(Shader::get_location(gl, program_id, "color")),
            size: ShaderParam2fv(Shader::get_location(gl, program_id, "size")),
            offset: ShaderParam2fv(Shader::get_location(gl, program_id, "offset")),
            texture: ShaderParam1i(Shader::get_location(gl, program_id, "model_texture")),
        }
    }
}

pub struct GroundShaderParameters {
    pub projection_mat: ShaderParam4x4fv,
    pub model_view_mat: ShaderParam4x4fv,
    pub normal_mat: ShaderParam3x3fv,
    pub light_dir: ShaderParam3fv,
    pub light_ambient: ShaderParam3fv,
    pub light_diffuse: ShaderParam3fv,
    pub light_opacity: ShaderParam1f,
    pub gnd_texture_atlas: ShaderParam1i,
    pub tile_color_texture: ShaderParam1i,
    pub lightmap_texture: ShaderParam1i,

    pub use_tile_color: ShaderParam1i,
    pub use_lightmap: ShaderParam1i,
    pub use_lighting: ShaderParam1i,
}

impl GroundShaderParameters {
    pub fn new(gl: &Gl, program_id: c_uint) -> GroundShaderParameters {
        GroundShaderParameters {
            projection_mat: ShaderParam4x4fv(Shader::get_location(gl, program_id, "projection")),
            model_view_mat: ShaderParam4x4fv(Shader::get_location(gl, program_id, "model_view")),
            normal_mat: ShaderParam3x3fv(Shader::get_location(gl, program_id, "normal_matrix")),
            light_dir: ShaderParam3fv(Shader::get_location(gl, program_id, "light_dir")),
            light_ambient: ShaderParam3fv(Shader::get_location(gl, program_id, "light_ambient")),
            light_diffuse: ShaderParam3fv(Shader::get_location(gl, program_id, "light_diffuse")),
            light_opacity: ShaderParam1f(Shader::get_location(gl, program_id, "light_opacity")),
            gnd_texture_atlas: ShaderParam1i(Shader::get_location(
                gl,
                program_id,
                "gnd_texture_atlas",
            )),
            tile_color_texture: ShaderParam1i(Shader::get_location(
                gl,
                program_id,
                "tile_color_texture",
            )),
            lightmap_texture: ShaderParam1i(Shader::get_location(
                gl,
                program_id,
                "lightmap_texture",
            )),
            use_tile_color: ShaderParam1i(Shader::get_location(gl, program_id, "use_tile_color")),
            use_lightmap: ShaderParam1i(Shader::get_location(gl, program_id, "use_lightmap")),
            use_lighting: ShaderParam1i(Shader::get_location(gl, program_id, "use_lighting")),
        }
    }
}

pub struct ModelShaderParameters {
    pub projection_mat: ShaderParam4x4fv,
    pub model_mat: ShaderParam4x4fv,
    pub view_mat: ShaderParam4x4fv,
    pub normal_mat: ShaderParam3x3fv,
    pub alpha: ShaderParam1f,
    pub light_dir: ShaderParam3fv,
    pub texture: ShaderParam1i,
    pub light_ambient: ShaderParam3fv,
    pub light_diffuse: ShaderParam3fv,
    pub light_opacity: ShaderParam1f,
    pub use_lighting: ShaderParam1i,
}

impl ModelShaderParameters {
    pub fn new(gl: &Gl, program_id: c_uint) -> ModelShaderParameters {
        ModelShaderParameters {
            projection_mat: ShaderParam4x4fv(Shader::get_location(gl, program_id, "projection")),
            model_mat: ShaderParam4x4fv(Shader::get_location(gl, program_id, "model")),
            view_mat: ShaderParam4x4fv(Shader::get_location(gl, program_id, "view")),
            normal_mat: ShaderParam3x3fv(Shader::get_location(gl, program_id, "normal_matrix")),
            alpha: ShaderParam1f(Shader::get_location(gl, program_id, "alpha")),
            light_dir: ShaderParam3fv(Shader::get_location(gl, program_id, "light_dir")),
            texture: ShaderParam1i(Shader::get_location(gl, program_id, "model_texture")),
            light_ambient: ShaderParam3fv(Shader::get_location(gl, program_id, "light_ambient")),
            light_diffuse: ShaderParam3fv(Shader::get_location(gl, program_id, "light_diffuse")),
            light_opacity: ShaderParam1f(Shader::get_location(gl, program_id, "light_opacity")),
            use_lighting: ShaderParam1i(Shader::get_location(gl, program_id, "use_lighting")),
        }
    }
}

pub struct StrEffect3dShaderParameters {
    pub projection_mat: ShaderParam4x4fv,
    pub model_mat: ShaderParam4x4fv,
    pub view_mat: ShaderParam4x4fv,
    pub color: ShaderParam4ubv,
    pub offset: ShaderParam2fv,
    pub texture: ShaderParam1i,
}

impl StrEffect3dShaderParameters {
    pub fn new(gl: &Gl, program_id: c_uint) -> StrEffect3dShaderParameters {
        StrEffect3dShaderParameters {
            projection_mat: ShaderParam4x4fv(Shader::get_location(gl, program_id, "projection")),
            view_mat: ShaderParam4x4fv(Shader::get_location(gl, program_id, "view")),
            texture: ShaderParam1i(Shader::get_location(gl, program_id, "model_texture")),
            model_mat: ShaderParam4x4fv(Shader::get_location(gl, program_id, "model")),
            color: ShaderParam4ubv(Shader::get_location(gl, program_id, "color")),
            offset: ShaderParam2fv(Shader::get_location(gl, program_id, "offset")),
        }
    }
}

pub struct Trimesh2dShaderParameters {
    pub projection_mat: ShaderParam4x4fv,
    pub model_mat: ShaderParam4x4fv,
    pub z: ShaderParam1f,
    pub color: ShaderParam4ubv,
    pub size: ShaderParam2fv,
}

impl Trimesh2dShaderParameters {
    pub fn new(gl: &Gl, program_id: c_uint) -> Trimesh2dShaderParameters {
        Trimesh2dShaderParameters {
            projection_mat: ShaderParam4x4fv(Shader::get_location(gl, program_id, "projection")),
            model_mat: ShaderParam4x4fv(Shader::get_location(gl, program_id, "model")),
            color: ShaderParam4ubv(Shader::get_location(gl, program_id, "color")),
            size: ShaderParam2fv(Shader::get_location(gl, program_id, "size")),
            z: ShaderParam1f(Shader::get_location(gl, program_id, "z")),
        }
    }
}

pub struct Point2dShaderParameters {
    pub projection_mat: ShaderParam4x4fv,
}

impl Point2dShaderParameters {
    pub fn new(gl: &Gl, program_id: c_uint) -> Point2dShaderParameters {
        Point2dShaderParameters {
            projection_mat: ShaderParam4x4fv(Shader::get_location(gl, program_id, "projection")),
        }
    }
}
