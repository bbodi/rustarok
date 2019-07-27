use nalgebra::{Vector2};
use crate::systems::SystemVariables;
use crate::video::draw_circle_inefficiently;

pub struct LightningSkill;

impl LightningSkill {
    pub fn render_target_selection(
        skill_pos: &Vector2<f32>,
        char_to_skill_dir: &Vector2<f32>,
        system_vars: &SystemVariables,
    ) {
        for i in 0..3 {
            draw_circle_inefficiently(&system_vars.shaders.trimesh_shader,
                                      &system_vars.matrices.projection,
                                      &system_vars.matrices.view,
                                      &(skill_pos + char_to_skill_dir * i as f32 * 2.2),
                                      0.0,
                                      1.0,
                                      &[0.0, 1.0, 0.0, 1.0]);
        }
    }
}