use crate::asset::database::AssetDatabase;
use crate::asset::texture::TextureId;
use crate::asset::{AssetLoader, BinaryReader};
use crate::common::{v3, Mat3, Mat4, Vec3};
use crate::my_gl::{Gl, MyGlEnum};
use crate::runtime_assets::map::{
    DataForRenderingSingleNode, SameTextureNodeFaces, SameTextureNodeFacesRaw,
};
use crate::video::{VertexArray, VertexAttribDefinition};
use nalgebra::{Point3, Quaternion, Rotation3, Unit, UnitQuaternion, Vector4};
use std::collections::HashMap;

fn init_vec<T, F>(size: u32, def: T, mut init_func: F) -> Vec<T>
where
    T: Clone,
    F: FnMut(&mut T) -> (),
{
    let mut vec: Vec<T> = vec![def; size as usize];
    for i in 0..size as usize {
        init_func(&mut vec[i]);
    }
    vec
}

#[derive(Debug)]
pub struct Rsm {
    pub anim_len: i32,
    pub shade_type: i32,
    pub alpha: u8,
    pub version: f32,
    pub texture_names: Vec<String>,
    pub nodes: Vec<RsmNode>,
    pub main_node_index: usize,
    pub pos_key_frames: Vec<PosKeyFrame>,
    pub volume_boxes: Vec<VolumeBox>,
    pub bounding_box: BoundingBox,
}

#[derive(Debug)]
pub struct RsmNodeVertex {
    pub pos: [f32; 3],
    pub normal: [f32; 3],
    pub texcoord: [f32; 2],
}

#[derive(Debug, Clone)]
pub struct BoundingBox {
    pub min: Vec3,
    pub max: Vec3,
    pub range: Vec3,
    pub center: Vec3,
}

impl BoundingBox {
    pub fn new() -> BoundingBox {
        BoundingBox {
            min: v3(std::f32::INFINITY, std::f32::INFINITY, std::f32::INFINITY),
            max: v3(
                std::f32::NEG_INFINITY,
                std::f32::NEG_INFINITY,
                std::f32::NEG_INFINITY,
            ),
            range: v3(0.0, 0.0, 0.0),
            center: v3(0.0, 0.0, 0.0),
        }
    }
}

#[derive(Debug)]
pub struct RsmNode {
    pub name: String,
    pub parent_name: String,
    pub textures: Vec<u32>,
    pub mat3: Mat3,
    pub matrix: Mat4,
    pub offset: Vec3,
    pub pos: Vec3,
    pub rotangle: f32,
    pub rotaxis: Vec3,
    pub scale: Vec3,
    pub vertices: Vec<Vec3>,
    pub bounding_box: BoundingBox,
    pub mesh: Vec<RsmNodeVertex>,
    pub texture_vertices: Vec<f32>,
    pub faces: Vec<NodeFace>,
    pub pos_key_frames: Vec<PosKeyFrame>,
    pub rot_key_frames: Vec<RotKeyFrame>,
}

#[derive(Default, Clone, Debug)]
pub struct NodeFace {
    pub vertex_index: [u16; 3],
    pub texture_vertex_index: [u16; 3],
    pub texture_id: u16,
    pub padding: u16,
    pub two_side: i32,
    pub smooth_group: i32,
}

#[derive(Default, Clone, Debug)]
pub struct PosKeyFrame {
    frame: i32,
    px: f32,
    py: f32,
    pz: f32,
}

#[derive(Default, Clone, Debug)]
pub struct VolumeBox {
    size: [f32; 3],
    pos: [f32; 3],
    rot: [f32; 3],
    flag: i32,
}

#[derive(Default, Clone, Debug)]
pub struct RotKeyFrame {
    frame: i32,
    q: [f32; 4],
}

impl RsmNode {
    fn load(buf: &mut BinaryReader, rsm_version: f32) -> Self {
        let name = buf.string(40);
        let parent_name = buf.string(40);

        let textures: Vec<u32> = init_vec(buf.next_u32(), 0, |item| {
            *item = buf.next_u32();
        });

        let mat3 = Mat3::new(
            buf.next_f32(),
            buf.next_f32(),
            buf.next_f32(),
            buf.next_f32(),
            buf.next_f32(),
            buf.next_f32(),
            buf.next_f32(),
            buf.next_f32(),
            buf.next_f32(),
        )
        .transpose();
        let offset = v3(buf.next_f32(), buf.next_f32(), buf.next_f32());
        let pos = v3(buf.next_f32(), buf.next_f32(), buf.next_f32());
        let rotangle = buf.next_f32();
        let rotaxis = v3(buf.next_f32(), buf.next_f32(), buf.next_f32());
        let scale = v3(buf.next_f32(), buf.next_f32(), buf.next_f32());

        let vertices: Vec<Vec3> = init_vec(buf.next_u32(), v3(0.0, 0.0, 0.0), |item| {
            *item = v3(buf.next_f32(), buf.next_f32(), buf.next_f32());
        });

        let texture_vertices: Vec<f32> = {
            let mut texture_vertices: Vec<f32> = vec![0.0f32; buf.next_u32() as usize * 6];
            for i in (0..texture_vertices.capacity()).step_by(6) {
                if rsm_version >= 1.2 {
                    texture_vertices[i + 0] = buf.next_u8() as f32 / 255.0;
                    texture_vertices[i + 1] = buf.next_u8() as f32 / 255.0;
                    texture_vertices[i + 2] = buf.next_u8() as f32 / 255.0;
                    texture_vertices[i + 3] = buf.next_u8() as f32 / 255.0;
                }
                texture_vertices[i + 4] = buf.next_f32() * 0.98 + 0.01;
                texture_vertices[i + 5] = buf.next_f32() * 0.98 + 0.01;
            }
            texture_vertices
        };

        let faces: Vec<NodeFace> = init_vec(buf.next_u32(), NodeFace::default(), |item| {
            *item = NodeFace {
                vertex_index: [buf.next_u16(), buf.next_u16(), buf.next_u16()],
                texture_vertex_index: [buf.next_u16(), buf.next_u16(), buf.next_u16()],
                texture_id: buf.next_u16(),
                padding: buf.next_u16(),
                two_side: buf.next_i32(),
                smooth_group: if rsm_version >= 1.2 {
                    buf.next_i32()
                } else {
                    0
                },
            };
        });

        let pos_key_frames: Vec<PosKeyFrame> = if rsm_version >= 1.5 {
            init_vec(buf.next_u32(), PosKeyFrame::default(), |item| {
                *item = PosKeyFrame {
                    frame: buf.next_i32(),
                    px: buf.next_f32(),
                    py: buf.next_f32(),
                    pz: buf.next_f32(),
                };
            })
        } else {
            Vec::new()
        };

        let rot_key_frames: Vec<RotKeyFrame> =
            init_vec(buf.next_u32(), RotKeyFrame::default(), |item| {
                *item = RotKeyFrame {
                    frame: buf.next_i32(),
                    q: [
                        buf.next_f32(),
                        buf.next_f32(),
                        buf.next_f32(),
                        buf.next_f32(),
                    ],
                };
            });

        RsmNode {
            name,
            parent_name,
            textures,
            mat3,
            offset,
            pos,
            rotangle,
            rotaxis,
            scale,
            vertices,
            texture_vertices,
            faces,
            pos_key_frames,
            rot_key_frames,
            matrix: Mat4::identity(),
            mesh: Vec::new(), // dummy
            bounding_box: BoundingBox::new(),
        }
    }
}

impl Rsm {
    pub(super) fn load(mut buf: BinaryReader) -> Self {
        let header = buf.string(4);
        if header != "GRSM" {
            panic!("Invalid RSM header: {}", header);
        }

        let version = buf.next_u8() as f32 + buf.next_u8() as f32 / 10f32;
        let anim_len = buf.next_i32();
        let shade_type = buf.next_i32();
        let alpha: u8 = if version >= 1.4 { buf.next_u8() } else { 255 };

        let _ = buf.string(16); // skip, reserved

        let texture_names: Vec<String> = (0..buf.next_u32()).map(|_i| buf.string(40)).collect();

        let main_node_name = buf.string(40);
        let (mut nodes, main_node_index) = {
            let mut nodes = Vec::<RsmNode>::with_capacity(buf.next_u32() as usize);
            let mut main_node_index = None;
            for i in 0..nodes.capacity() {
                let node = RsmNode::load(&mut buf, version);
                if node.name == main_node_name {
                    main_node_index = Some(i);
                }
                nodes.push(node);
            }
            // In some custom models, the default name don't match nodes name.
            // So by default, assume the main node is the first one.
            let main_node_index = main_node_index.unwrap_or(0);
            (nodes, main_node_index)
        };

        let pos_key_frames: Vec<PosKeyFrame> = if version < 1.5 {
            init_vec(buf.next_u32(), PosKeyFrame::default(), |item| {
                *item = PosKeyFrame {
                    frame: buf.next_i32(),
                    px: buf.next_f32(),
                    py: buf.next_f32(),
                    pz: buf.next_f32(),
                };
            })
        } else {
            Vec::new()
        };

        let volume_boxes: Vec<VolumeBox> = init_vec(buf.next_u32(), VolumeBox::default(), |item| {
            *item = VolumeBox {
                size: [buf.next_f32(), buf.next_f32(), buf.next_f32()],
                pos: [buf.next_f32(), buf.next_f32(), buf.next_f32()],
                rot: [buf.next_f32(), buf.next_f32(), buf.next_f32()],
                flag: buf.next_i32(),
            };
        });

        let is_only = nodes.len() == 1;
        Rsm::calc_matrix_and_bounding_box_recursively(
            main_node_index,
            &mut nodes,
            is_only,
            &Mat4::identity(),
        );

        let mut bbox = BoundingBox::new();
        for i in 0..3 {
            for node in &nodes {
                bbox.min[i] = node.bounding_box.min[i].min(bbox.min[i]);
                bbox.max[i] = node.bounding_box.max[i].max(bbox.max[i]);
            }
            bbox.range[i] = (bbox.max[i] - bbox.min[i]) / 2.0;
            bbox.center[i] = bbox.min[i] + bbox.range[i];
        }

        Rsm {
            anim_len,
            shade_type,
            alpha,
            version,
            texture_names,
            nodes,
            main_node_index,
            pos_key_frames,
            volume_boxes,
            bounding_box: bbox,
        }
    }

    pub fn generate_meshes_by_texture_id(
        model_bbox: &BoundingBox,
        shade_type: i32,
        is_only: bool,
        nodes: &Vec<RsmNode>,
        textures: &Vec<(String, TextureId)>,
    ) -> (Vec<Vec<SameTextureNodeFacesRaw>>, BoundingBox) {
        let mut real_bounding_box = BoundingBox::new();
        let mut full_model_rendering_data: Vec<Vec<SameTextureNodeFacesRaw>> = Vec::new();
        for node in nodes {
            let faces_by_texture_id = {
                let mut faces_by_texture_id: HashMap<u16, Vec<&NodeFace>> = HashMap::new();
                for face in &node.faces {
                    faces_by_texture_id
                        .entry(face.texture_id)
                        .or_insert(Vec::new())
                        .push(&face);
                }
                faces_by_texture_id
            };
            let vertices_per_texture_per_node: Vec<SameTextureNodeFacesRaw> = faces_by_texture_id
                .iter()
                .map(|(&texture_index, faces)| {
                    // a node összes olyan face-e, akinek texture_index a texturája
                    let mesh = Rsm::generate_trimesh(
                        model_bbox,
                        node,
                        faces.as_slice(),
                        shade_type,
                        is_only,
                    );
                    for v in mesh.iter() {
                        for i in 0..3 {
                            real_bounding_box.min[i] = v.pos[i].min(real_bounding_box.min[i]);
                            real_bounding_box.max[i] = v.pos[i].max(real_bounding_box.max[i]);
                        }
                    }

                    let (name, gl_tex) = &textures[node.textures[texture_index as usize] as usize];
                    let renderable = SameTextureNodeFacesRaw {
                        mesh,
                        texture: gl_tex.clone(),
                        texture_name: name.to_owned(),
                    };
                    //                    SameTextureNodeFaces {
                    //                        vao: VertexArray::new_static(
                    //                            gl,
                    //                            MyGlEnum::TRIANGLES,
                    //                            mesh,
                    //                            vec![
                    //                                VertexAttribDefinition {
                    //                                    number_of_components: 3,
                    //                                    offset_of_first_element: 0,
                    //                                },
                    //                                VertexAttribDefinition {
                    //                                    // normal
                    //                                    number_of_components: 3,
                    //                                    offset_of_first_element: 3,
                    //                                },
                    //                                VertexAttribDefinition {
                    //                                    // uv
                    //                                    number_of_components: 2,
                    //                                    offset_of_first_element: 6,
                    //                                },
                    //                            ],
                    //                        ),
                    //                        texture: gl_tex.clone(),
                    //                        texture_name: name.to_owned(),
                    //                    }
                    renderable
                })
                .collect();
            full_model_rendering_data.push(vertices_per_texture_per_node);
        }
        for i in 0..3 {
            real_bounding_box.range[i] =
                (real_bounding_box.max[i] - real_bounding_box.min[i]) / 2.0;
            real_bounding_box.center[i] = real_bounding_box.min[i] + real_bounding_box.range[i];
        }
        return (full_model_rendering_data, real_bounding_box);
    }

    pub fn load_textures(
        gl: &Gl,
        asset_loader: &AssetLoader,
        asset_db: &mut AssetDatabase,
        texture_names: &Vec<String>,
    ) -> Vec<(String, TextureId)> {
        texture_names
            .iter()
            .map(|texture_name| {
                let path = format!("data\\texture\\{}", texture_name);
                let ret = asset_db.get_texture_id(&path).unwrap_or_else(|| {
                    asset_loader
                        .load_texture(gl, &path, MyGlEnum::NEAREST, asset_db)
                        .unwrap()
                });
                return (AssetDatabase::replace_non_ascii_chars(&path), ret);
            })
            .collect()
    }

    fn calc_matrix_and_bounding_box_recursively(
        parent_node_index: usize,
        nodes: &mut Vec<RsmNode>,
        is_only: bool,
        parent_matrix: &Mat4,
    ) {
        let parent_node_name_of_parent = nodes[parent_node_index].parent_name.clone();
        {
            let mut parent_node = &mut nodes[parent_node_index];
            parent_node.matrix = Rsm::calc_matrix(&parent_node, parent_matrix);
            parent_node.bounding_box = Rsm::calc_bounding_box(parent_node, is_only);
        }

        let parent_node_name = nodes[parent_node_index].name.clone();
        let node_matrix = nodes[parent_node_index].matrix;
        let children_indices = nodes
            .iter_mut()
            .enumerate()
            .filter(|(_i, n)| {
                parent_node_name == n.parent_name && parent_node_name != parent_node_name_of_parent
            })
            .map(|(i, _n)| i)
            .collect::<Vec<usize>>();
        for i in children_indices {
            Rsm::calc_matrix_and_bounding_box_recursively(i, nodes, is_only, &node_matrix);
        }
    }

    fn calc_matrix(node: &RsmNode, parent_matrix: &Mat4) -> Mat4 {
        let mut node_matrix = parent_matrix.clone();

        node_matrix.prepend_translation_mut(&node.pos);

        // Dynamic or static model
        if node.rot_key_frames.is_empty() {
            let rotation =
                Rotation3::from_axis_angle(&Unit::new_normalize(node.rotaxis), node.rotangle)
                    .to_homogeneous();
            node_matrix = node_matrix * rotation;
        } else {
            let quat = Quaternion::from(Vector4::from(node.rot_key_frames[0].q));
            let rotation = UnitQuaternion::from_quaternion(quat);
            node_matrix = node_matrix * rotation.to_homogeneous();
        }
        node_matrix.prepend_nonuniform_scaling_mut(&node.scale);
        node_matrix
    }

    fn calc_bounding_box(node: &RsmNode, is_only: bool) -> BoundingBox {
        let mut node_local_matrix = node.matrix.clone();

        if !is_only {
            node_local_matrix.prepend_translation_mut(&-node.offset);
        }
        node_local_matrix = node_local_matrix * node.mat3.to_homogeneous();

        let mut bbox = BoundingBox::new();

        for vert in node.vertices.iter() {
            let v = node_local_matrix.transform_point(&Point3::new(vert.x, vert.y, vert.z));
            for i in 0..3 {
                bbox.min[i] = v[i].min(bbox.min[i]);
                bbox.max[i] = v[i].max(bbox.max[i]);
            }
        }
        for i in 0..3 {
            bbox.range[i] = (bbox.max[i] - bbox.min[i]) / 2.0;
            bbox.center[i] = bbox.min[i] + bbox.range[i];
        }
        return bbox;
    }

    fn generate_trimesh(
        model_bbox: &BoundingBox,
        node: &RsmNode,
        faces: &[&NodeFace],
        shade_type: i32,
        is_only: bool,
    ) -> Vec<RsmNodeVertex> {
        let verts = &node.vertices;
        let tverts = &node.texture_vertices;

        let mut matrix = Mat4::identity();
        matrix.prepend_translation_mut(&v3(
            -model_bbox.center[0],
            -model_bbox.max[1],
            -model_bbox.center[2],
        ));
        matrix = matrix * node.matrix;
        if !is_only {
            matrix.prepend_translation_mut(&node.offset);
        }
        matrix *= node.mat3.to_homogeneous();

        let mesh = match shade_type {
            1/*FLAT*/ => {
                let (normals, _group_used) = Rsm::calc_flat_normals(node);
                Rsm::generate_mesh_flat(&matrix, faces, &verts, &tverts, normals)
            }
            2/*SMOOTH*/ => {
                let (normals, group_used) = Rsm::calc_flat_normals(node);
                let normal_groups = Rsm::calc_smooth_normals(node, normals, group_used);
                Rsm::generate_mesh_smooth(&matrix, faces, &verts, &tverts, normal_groups)
            }
            _/*NONE*/ => {
                let normals = node.faces.iter().map(|_face| {
                    v3(-1.0f32, -1.0f32, -1.0f32)
                }).collect();
                Rsm::generate_mesh_flat(&matrix, faces, &verts, &tverts, normals)
            }
        };
        return mesh;
    }

    fn generate_mesh_flat(
        matrix: &Mat4,
        faces: &[&NodeFace],
        verts: &Vec<Vec3>,
        tverts: &Vec<f32>,
        normals: Vec<Vec3>,
    ) -> Vec<RsmNodeVertex> {
        let mut mesh: Vec<RsmNodeVertex> = Vec::with_capacity(faces.len() * 3);
        for (face, normal) in faces.iter().zip(normals) {
            for i in 0..3 {
                let v = &verts[face.vertex_index[i] as usize];
                let v = matrix.transform_point(&Point3::new(v.x, v.y, v.z));
                let tid = face.texture_vertex_index[i] as usize * 6;
                mesh.push(RsmNodeVertex {
                    pos: [v[0], v[1], v[2]],
                    normal: [normal[0], normal[1], normal[2]],
                    texcoord: [tverts[tid + 4], tverts[tid + 5]],
                });
            }
        }
        return mesh;
    }

    fn generate_mesh_smooth(
        matrix: &Mat4,
        faces: &[&NodeFace],
        verts: &Vec<Vec3>,
        tverts: &Vec<f32>,
        normal_groups: [Vec<Vec3>; 32],
    ) -> Vec<RsmNodeVertex> {
        let mut mesh: Vec<RsmNodeVertex> = Vec::with_capacity(faces.len() * 3);
        for face in faces {
            let normals = &normal_groups[face.smooth_group as usize];
            for i in 0..3 {
                let v = &verts[face.vertex_index[i] as usize];
                let v = matrix.transform_point(&Point3::new(v.x, v.y, v.z));
                let normal = &normals[face.vertex_index[i] as usize];
                let tid = face.texture_vertex_index[i] as usize * 6;
                mesh.push(RsmNodeVertex {
                    pos: [v[0], v[1], v[2]],
                    normal: [normal[0], normal[1], normal[2]],
                    texcoord: [tverts[tid + 4], tverts[tid + 5]],
                });
            }
        }
        return mesh;
    }

    fn calc_flat_normals(node: &RsmNode) -> (Vec<Vec3>, [bool; 32]) {
        pub fn triangle_normal(p1: &Vec3, p2: &Vec3, p3: &Vec3) -> Vec3 {
            (p2 - p1).cross(&(p3 - p1)).normalize()
        }
        let mut group_used = [false; 32];
        let normals = node
            .faces
            .iter()
            .map(|face| {
                group_used[face.smooth_group as usize] = true;
                triangle_normal(
                    &node.vertices[face.vertex_index[0] as usize],
                    &node.vertices[face.vertex_index[1] as usize],
                    &node.vertices[face.vertex_index[2] as usize],
                )
            })
            .collect();
        return (normals, group_used);
    }

    fn calc_smooth_normals(
        node: &RsmNode,
        normals: Vec<Vec3>,
        group_used: [bool; 32],
    ) -> [Vec<Vec3>; 32] {
        let mut group: [Vec<Vec3>; 32] = Default::default();
        for group_index in 0..32 {
            if !group_used[group_index] {
                continue;
            }
            group[group_index].reserve(node.vertices.len());
            for vertex_index in 0..node.vertices.len() {
                let mut grouped_normal = v3(0.0f32, 0.0f32, 0.0f32);
                for (face_index, face) in node.faces.iter().enumerate() {
                    if face.smooth_group as usize == group_index
                        && (face.vertex_index[0] == vertex_index as u16
                            || face.vertex_index[1] == vertex_index as u16
                            || face.vertex_index[2] == vertex_index as u16)
                    {
                        grouped_normal += normals[face_index];
                    }
                }
                group[group_index].push(grouped_normal.normalize());
            }
        }

        return group;
    }
}
