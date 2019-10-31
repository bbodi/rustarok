use crate::asset::database::AssetDatabase;
use crate::asset::gat::Gat;
use crate::asset::rsm::{BoundingBox, RsmNodeVertex};
use crate::asset::rsw::LightData;
use crate::asset::texture::{TextureId, DUMMY_TEXTURE_ID_FOR_TEST};
use crate::asset::AssetLoader;
use crate::common::{measure_time, Mat4};
use crate::common::{v2, Vec2};
use crate::my_gl::{Gl, MyGlEnum};
use crate::video::{VertexArray, VertexAttribDefinition};
use nalgebra::{Rotation3, Vector2, Vector3};
use ncollide2d::pipeline::CollisionGroups;
use ncollide2d::shape::ShapeHandle;
use nphysics2d::force_generator::DefaultForceGeneratorSet;
use nphysics2d::joint::DefaultJointConstraintSet;
use nphysics2d::object::{
    BodyPartHandle, ColliderDesc, DefaultBodyHandle, DefaultBodySet, DefaultColliderHandle,
    DefaultColliderSet, RigidBodyDesc,
};
use nphysics2d::solver::SignoriniModel;
use nphysics2d::world::{DefaultGeometricalWorld, DefaultMechanicalWorld};

#[derive(Clone, Copy)]
pub enum CollisionGroup {
    StaticModel,
    LeftPlayer,
    RightPlayer,
    LeftBarricade,
    RightBarricade,
    NeutralPlayerPlayer,
    NonCollidablePlayer,
    Minion,
    Turret,
    Guard,
    SkillArea,
}

pub struct ModelInstance {
    pub asset_db_model_index: usize,
    pub matrix: Mat4,
    pub bottom_left_front: Vector3<f32>,
    pub top_right_back: Vector3<f32>,
}

pub struct MapRenderData {
    pub gat: Gat,
    pub ground_width: u32,
    pub ground_height: u32,
    pub light: LightData,
    pub use_tile_colors: bool,
    pub use_lightmaps: bool,
    pub use_lighting: bool,
    pub ground_vertex_array: VertexArray,
    pub centered_sprite_vertex_array: VertexArray,
    pub bottom_left_sprite_vertex_array: VertexArray,
    pub rectangle_vertex_array: VertexArray,
    pub texture_atlas: TextureId,
    pub tile_color_texture: TextureId,
    pub lightmap_texture: TextureId,
    pub model_instances: Vec<ModelInstance>,
    pub draw_models: bool,
    pub draw_ground: bool,
    pub ground_walkability_mesh: VertexArray,
    pub ground_walkability_mesh2: VertexArray,
    pub ground_walkability_mesh3: VertexArray,
    pub minimap_texture_id: TextureId,
}

pub struct ModelRenderData {
    pub bounding_box: BoundingBox,
    pub alpha: u8,
    pub model: Vec<DataForRenderingSingleNode>,
}

pub type DataForRenderingSingleNode = Vec<SameTextureNodeFaces>;

pub struct SameTextureNodeFacesRaw {
    pub mesh: Vec<RsmNodeVertex>,
    pub texture: TextureId,
    pub texture_name: String, // todo: why does it store texture name?
}

#[derive(Clone)]
pub struct SameTextureNodeFaces {
    pub vao: VertexArray,
    pub texture: TextureId,
    pub texture_name: String, // todo: why does it store texture name?
}

struct GroundLoadResult {
    ground_vertex_array: VertexArray,
    ground_walkability_mesh: VertexArray,
    ground_walkability_mesh2: VertexArray,
    ground_walkability_mesh3: VertexArray,
    ground_width: u32,
    ground_height: u32,
    texture_atlas: TextureId,
    tile_color_texture: TextureId,
    lightmap_texture: TextureId,
}

pub fn load_map(
    physics_world: &mut PhysicEngine,
    gl: &Gl,
    map_name: &str,
    asset_loader: &AssetLoader,
    asset_db: &mut AssetDatabase,
) -> (MapRenderData) {
    let (elapsed, world) = measure_time(|| asset_loader.load_map(&map_name).unwrap());
    log::info!("rsw loaded: {}ms", elapsed.as_millis());
    let (elapsed, (gat, rectangles)) = measure_time(|| asset_loader.load_gat(map_name).unwrap());
    log::info!("gat loaded: {}ms", elapsed.as_millis());

    log::info!("coliders");
    let colliders: Vec<(Vec2, Vec2)> = rectangles
        .iter()
        .map(|cell| {
            let rot = Rotation3::<f32>::new(Vector3::new(180f32.to_radians(), 0.0, 0.0));
            let half_w = cell.width as f32 / 2.0;
            let x = cell.start_x as f32 + half_w;
            let half_h = cell.height as f32 / 2.0;
            let y = (cell.bottom - cell.height) as f32 + 1.0 + half_h;
            let half_extents = v2(half_w, half_h);

            let cuboid = ShapeHandle::new(ncollide2d::shape::Cuboid::new(half_extents));
            let v = rot * Vector3::new(x, 0.0, y);
            let v2 = v2(v.x, v.z);
            let parent_rigid_body = RigidBodyDesc::new()
                .translation(v2)
                .gravity_enabled(false)
                .status(nphysics2d::object::BodyStatus::Static)
                .build();
            let parent_handle = physics_world.bodies.insert(parent_rigid_body);
            let cuboid = ColliderDesc::new(cuboid)
                .density(10.0)
                .collision_groups(
                    CollisionGroups::new()
                        .with_membership(&[CollisionGroup::StaticModel as usize])
                        .with_blacklist(&[
                            CollisionGroup::StaticModel as usize,
                            CollisionGroup::NonCollidablePlayer as usize,
                        ]),
                )
                .build(BodyPartHandle(parent_handle, 0));
            let cuboid_pos = cuboid.position_wrt_body().translation.vector;
            physics_world.colliders.insert(cuboid);
            (half_extents, cuboid_pos)
        })
        .collect();

    asset_loader.start_loading_ground(
        gl,
        asset_db,
        map_name,
        rectangles,
        gat.clone(),
        world.water.clone(),
        colliders.clone(),
    );

    let dummy_vbo = VertexArray::new_static(
        gl,
        MyGlEnum::TRIANGLES,
        vec![0.0, 0.0, 0.0],
        vec![VertexAttribDefinition {
            number_of_components: 3,
            offset_of_first_element: 0,
        }],
    );
    let ground_data = GroundLoadResult {
        ground_vertex_array: dummy_vbo.clone(),
        ground_walkability_mesh: dummy_vbo.clone(),
        ground_walkability_mesh2: dummy_vbo.clone(),
        ground_walkability_mesh3: dummy_vbo,
        ground_width: 0,
        ground_height: 0,
        texture_atlas: DUMMY_TEXTURE_ID_FOR_TEST,
        tile_color_texture: DUMMY_TEXTURE_ID_FOR_TEST,
        lightmap_texture: DUMMY_TEXTURE_ID_FOR_TEST,
    };

    ////////////////////////////
    //// MODELS
    ////////////////////////////
    asset_loader.start_loading_models(gl, world.models, asset_db, gat.width / 2, gat.height / 2);

    let mut model_instances: Vec<ModelInstance> = vec![];

    let s: Vec<[f32; 4]> = vec![
        [-0.5, 0.5, 0.0, 0.0],
        [0.5, 0.5, 1.0, 0.0],
        [-0.5, -0.5, 0.0, 1.0],
        [0.5, -0.5, 1.0, 1.0],
    ];
    let centered_sprite_vertex_array = VertexArray::new_static(
        gl,
        MyGlEnum::TRIANGLE_STRIP,
        s,
        vec![
            VertexAttribDefinition {
                number_of_components: 2,
                offset_of_first_element: 0,
            },
            VertexAttribDefinition {
                // uv
                number_of_components: 2,
                offset_of_first_element: 2,
            },
        ],
    );
    let s: Vec<[f32; 4]> = vec![
        [0.0, 0.0, 0.0, 0.0],
        [1.0, 0.0, 1.0, 0.0],
        [0.0, 1.0, 0.0, 1.0],
        [1.0, 1.0, 1.0, 1.0],
    ];
    let sprite_vertex_array = VertexArray::new_static(
        gl,
        MyGlEnum::TRIANGLE_STRIP,
        s,
        vec![
            VertexAttribDefinition {
                number_of_components: 2,
                offset_of_first_element: 0,
            },
            VertexAttribDefinition {
                // uv
                number_of_components: 2,
                offset_of_first_element: 2,
            },
        ],
    );
    let s: Vec<[f32; 2]> = vec![[0.0, 1.0], [1.0, 1.0], [0.0, 0.0], [1.0, 0.0]];
    let rectangle_vertex_array = VertexArray::new_static(
        gl,
        MyGlEnum::TRIANGLE_STRIP,
        s,
        vec![VertexAttribDefinition {
            number_of_components: 2,
            offset_of_first_element: 0,
        }],
    );

    physics_world
        .mechanical_world
        .solver
        .set_contact_model(Box::new(SignoriniModel::new()));

    let path = format!("data\\texture\\À¯ÀúÀÎÅÍÆäÀÌ½º\\map\\{}.bmp", map_name);
    let minimap_texture = asset_db.get_texture_id(&path).unwrap_or_else(|| {
        let surface = asset_loader.load_sdl_surface(&path);
        log::trace!("Surface loaded: {}", path);
        let mut surface = surface.unwrap_or_else(|e| {
            log::warn!("Missing texture: {}, {}", path, e);
            asset_loader.backup_surface()
        });
        // make it grey
        let w = surface.width() as usize;
        let h = surface.height() as usize;
        let fmt = surface.pixel_format();
        surface.with_lock_mut(|pixels| unsafe {
            let pixels: &mut [u32] =
                std::slice::from_raw_parts_mut(std::mem::transmute(pixels.as_mut_ptr()), w * h);
            for pixel in pixels {
                let color = sdl2::pixels::Color::from_u32(&fmt, *pixel);
                if color.r != 255 && color.b != 255 {
                    let new_color = (color.r as f32 * 0.212671
                        + color.g as f32 * 0.715160
                        + color.b as f32 * 0.072169) as u8;
                    *pixel = sdl2::pixels::Color::RGBA(new_color, new_color, new_color, color.a)
                        .to_u32(&fmt)
                }
            }
        });
        AssetLoader::create_texture_from_surface(gl, &path, surface, MyGlEnum::NEAREST, asset_db)
    });

    // TODO
    // remove the the upper half of lamps on which Guards are standing
    if false && map_name == "prontera" {
        let lamp_name = "ÇÁ·ÐÅ×¶ó\\ÈÖÀå°¡·Îµî.rsm";
        let model_index = asset_db.get_model_index(lamp_name);
        let model = asset_db.get_model(model_index);
        let new_model = ModelRenderData {
            bounding_box: model.bounding_box.clone(),
            alpha: 255,
            model: model
                .model
                .iter()
                .map(|m| {
                    m.iter()
                        .filter(|m| {
                            m.texture_name.ends_with("stone-down.bmp")
                                || m.texture_name.ends_with("STONE-UP.BMP")
                        })
                        .map(|m| m.clone())
                        .collect()
                })
                .collect(),
        };
        asset_db.register_model("half_lamp", new_model);
        let new_model_index = asset_db.get_model_index("half_lamp");
        // RIGHT TEAM GUARDS
        // middle final 4 guards on lamps
        model_instances[453].asset_db_model_index = new_model_index;
        model_instances[454].asset_db_model_index = new_model_index;
        model_instances[455].asset_db_model_index = new_model_index;
        model_instances[456].asset_db_model_index = new_model_index;
        // top, guard alone on lamp
        model_instances[695].asset_db_model_index = new_model_index;
        // top, two guards on lamps
        model_instances[549].asset_db_model_index = new_model_index;
        model_instances[550].asset_db_model_index = new_model_index;
        // LEFT TEAM GUARDS
        // middle final 4 guards on lamps
        model_instances[457].asset_db_model_index = new_model_index;
        model_instances[458].asset_db_model_index = new_model_index;
        model_instances[459].asset_db_model_index = new_model_index;
        model_instances[460].asset_db_model_index = new_model_index;
        // top, guard alone on lamp
        model_instances[712].asset_db_model_index = new_model_index;
        // top, two guards on lamps
        model_instances[536].asset_db_model_index = new_model_index;
        model_instances[537].asset_db_model_index = new_model_index;
    }

    MapRenderData {
        gat,
        ground_width: ground_data.ground_width,
        ground_height: ground_data.ground_height,
        light: world.light,
        ground_vertex_array: ground_data.ground_vertex_array,
        texture_atlas: ground_data.texture_atlas,
        tile_color_texture: ground_data.tile_color_texture,
        lightmap_texture: ground_data.lightmap_texture,
        model_instances,
        centered_sprite_vertex_array,
        bottom_left_sprite_vertex_array: sprite_vertex_array,
        rectangle_vertex_array,
        use_tile_colors: true,
        use_lightmaps: true,
        use_lighting: true,
        draw_models: true,
        draw_ground: true,
        ground_walkability_mesh: ground_data.ground_walkability_mesh,
        ground_walkability_mesh2: ground_data.ground_walkability_mesh2,
        ground_walkability_mesh3: ground_data.ground_walkability_mesh3,
        minimap_texture_id: minimap_texture,
    }
}

pub struct PhysicEngine {
    pub mechanical_world: DefaultMechanicalWorld<f32>,
    pub geometrical_world: DefaultGeometricalWorld<f32>,

    pub bodies: DefaultBodySet<f32>,
    pub colliders: DefaultColliderSet<f32>,
    pub joint_constraints: DefaultJointConstraintSet<f32>,
    pub force_generators: DefaultForceGeneratorSet<f32>,
}

impl PhysicEngine {
    pub fn new() -> PhysicEngine {
        PhysicEngine {
            mechanical_world: DefaultMechanicalWorld::new(Vector2::zeros()),
            geometrical_world: DefaultGeometricalWorld::new(),

            bodies: DefaultBodySet::new(),
            colliders: DefaultColliderSet::new(),
            joint_constraints: DefaultJointConstraintSet::new(),
            force_generators: DefaultForceGeneratorSet::new(),
        }
    }

    pub fn step(&mut self, dt: f32) {
        self.mechanical_world.set_timestep(dt);
        self.mechanical_world.step(
            &mut self.geometrical_world,
            &mut self.bodies,
            &mut self.colliders,
            &mut self.joint_constraints,
            &mut self.force_generators,
        );
    }

    pub fn add_cuboid_skill_area(
        &mut self,
        pos: Vec2,
        rot_angle_in_rad: f32,
        extent: Vec2,
    ) -> (DefaultColliderHandle, DefaultBodyHandle) {
        let cuboid = ShapeHandle::new(ncollide2d::shape::Cuboid::new(extent / 2.0));
        let body_handle = self.bodies.insert(
            RigidBodyDesc::new()
                .status(nphysics2d::object::BodyStatus::Static)
                .gravity_enabled(false)
                .build(),
        );
        let coll_handle = self.colliders.insert(
            ColliderDesc::new(cuboid)
                .translation(pos)
                .rotation(rot_angle_in_rad.to_degrees())
                .collision_groups(
                    CollisionGroups::new()
                        .with_membership(&[CollisionGroup::SkillArea as usize])
                        .with_blacklist(&[
                            CollisionGroup::StaticModel as usize,
                            CollisionGroup::SkillArea as usize,
                        ]),
                )
                .sensor(true)
                .build(BodyPartHandle(body_handle, 0)),
        );
        return (coll_handle, body_handle);
    }
}
