use crate::common::{BinaryReader, init_vec};
use nalgebra::{Matrix3, Vector, Vector3, Matrix4, Translation3, Rotation3, Unit, Quaternion, Vector4, UnitQuaternion, Point3};
use ncollide3d::procedural::{TriMesh as ProceduralTriMesh, IndexBuffer};
use ncollide3d::shape::TriMesh;
use ncollide3d::bounding_volume::bounding_volume::HasBoundingVolume;
use crate::opengl::{GlTexture, VertexArray, VertexAttribDefinition};
use sdl2::pixels::{PixelFormatEnum, Color};
use std::collections::HashMap;
use crate::RenderableRsmNode;


pub struct Rsm {
    pub anim_len: i32,
    pub shade_type: i32,
    pub alpha: f32,
    pub version: f32,
    pub texture_names: Vec<String>,
    pub nodes: Vec<RsmNode>,
    pub main_node_index: usize,
    pub pos_key_frames: Vec<PosKeyFrame>,
    pub volume_boxes: Vec<VolumeBox>,
}

#[derive(Debug)]
pub struct RsmNodeVertex {
    pub pos: [f32; 3],
    //    pub normal: [f32; 3],
    pub texcoord: [f32; 2],
}

pub struct RsmNode {
    pub name: String,
    pub parent_name: String,
    pub textures: Vec<u32>,
    pub mat3: Matrix3<f32>,
    pub matrix: Matrix4<f32>,
    pub offset: Vector3<f32>,
    pub pos: Vector3<f32>,
    pub rotangle: f32,
    pub rotaxis: Vector3<f32>,
    pub scale: Vector3<f32>,
    pub vertices: Vec<Point3<f32>>,
    pub trimesh: TriMesh<f32>,
    pub mesh: Vec<RsmNodeVertex>,
    pub texture_vertices: Vec<f32>,
    pub faces: Vec<NodeFace>,
    pub pos_key_frames: Vec<PosKeyFrame>,
    pub rot_key_frames: Vec<RotKeyFrame>,
}

#[derive(Default, Clone)]
pub struct NodeFace {
    pub vertex_index: [u16; 3],
    pub texture_vertex_index: [u16; 3],
    pub texture_id: u16,
    pub padding: u16,
    pub two_side: i32,
    pub smooth_group: i32,
}

#[derive(Default, Clone)]
pub struct PosKeyFrame {
    frame: i32,
    px: f32,
    py: f32,
    pz: f32,
}

#[derive(Default, Clone)]
pub struct VolumeBox {
    size: [f32; 3],
    pos: [f32; 3],
    rot: [f32; 3],
    flag: i32,
}

#[derive(Default, Clone)]
pub struct RotKeyFrame {
    frame: i32,
    q: [f32; 4],
}

impl RsmNode {
    pub fn load(buf: &mut BinaryReader, rsm_version: f32) -> RsmNode {
        let name = buf.string(40);
        let parent_name = buf.string(40);


        let textures: Vec<u32> = init_vec(buf.next_u32(), 0, |item| {
            *item = buf.next_u32();
        });

        let mat3 = Matrix3::<f32>::new(
            buf.next_f32(), buf.next_f32(), buf.next_f32(),
            buf.next_f32(), buf.next_f32(), buf.next_f32(),
            buf.next_f32(), buf.next_f32(), buf.next_f32(),
        );
        let offset = Vector3::<f32>::new(buf.next_f32(), buf.next_f32(), buf.next_f32());
        let pos = Vector3::<f32>::new(buf.next_f32(), buf.next_f32(), buf.next_f32());
        let rotangle = buf.next_f32();
        let rotaxis = Vector3::<f32>::new(buf.next_f32(), buf.next_f32(), buf.next_f32());
        let scale = Vector3::<f32>::new(buf.next_f32(), buf.next_f32(), buf.next_f32());

        let vertices: Vec<Point3<f32>> = init_vec(buf.next_u32(), Point3::<f32>::new(0.0, 0.0, 0.0), |item| {
            *item = Point3::<f32>::new(buf.next_f32(), buf.next_f32(), buf.next_f32());
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
                smooth_group: if rsm_version >= 1.2 { buf.next_i32() } else { 0 },
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

        let rot_key_frames: Vec<RotKeyFrame> = init_vec(buf.next_u32(), RotKeyFrame::default(), |item| {
            *item = RotKeyFrame {
                frame: buf.next_i32(),
                q: [buf.next_f32(), buf.next_f32(), buf.next_f32(), buf.next_f32()],
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
            matrix: Matrix4::identity(),
            trimesh: TriMesh::from(ProceduralTriMesh::new(vec![], None, None, None)), // dummy,
            mesh: Vec::new(), // dummy
        }
    }
}


impl Rsm {
    pub fn load(buf: &mut BinaryReader) -> Rsm {
        let header = buf.string(4);
        if header != "GRSM" {
            panic!("Invalig RSM header: {}", header);
        }

        let version = buf.next_u8() as f32 + buf.next_u8() as f32 / 10f32;
        let anim_len = buf.next_i32();
        let shade_type = buf.next_i32();
        let alpha: f32 = if version >= 1.4 { buf.next_u8() as f32 / 255.0 } else { 1.0 };
        println!("version: {}, anim_len: {}, shade_type: {}, alpha: {}", version, anim_len, shade_type, alpha);

        let _ = buf.string(16); // skip, reserved

        let texture_names: Vec<String> = (0..buf.next_u32()).map(|i| {
            buf.string(40)
        }).collect();

        let main_node_name = buf.string(40);
        let (mut nodes, main_node_index) = {
            let mut nodes = Vec::<RsmNode>::with_capacity(buf.next_u32() as usize);
            let mut main_node_index = None;
            for i in 0..nodes.capacity() {
                let node = RsmNode::load(buf, version);
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
        Rsm::transform_children_recursively(main_node_index, &mut nodes, is_only);

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
        }
    }

    pub fn generate_meshes_by_texture_id(nodes: &Vec<RsmNode>, textures: &Vec<GlTexture>) -> Vec<Vec<RenderableRsmNode>> {
        let mut shit: Vec<Vec<RenderableRsmNode>> = Vec::new();
        for node in nodes {
            let faces_by_texture_id = {
                let mut faces_by_texture_id: HashMap<u16, Vec<&NodeFace>> = HashMap::new();
                for face in &node.faces {
                    faces_by_texture_id.entry(face.texture_id)
                        .or_insert(Vec::new())
                        .push(&face);
                }
                faces_by_texture_id
            };
            let vertices_per_texture_per_node: Vec<RenderableRsmNode> = faces_by_texture_id
                .iter()
                .map(|(&texture_index, faces)| {
                    // a node összes olyan face-e, akinek texture_index a texturája
                    let mesh = Rsm::generate_trimesh(node, faces.as_slice());
                    let gl_tex = textures[node.textures[texture_index as usize] as usize].clone();
                    let renderable = RenderableRsmNode {
                        vao: VertexArray::new(&mesh, &[
                            VertexAttribDefinition {
                                number_of_components: 3,
                                offset_of_first_element: 0,
                            },
                            VertexAttribDefinition { // uv
                                number_of_components: 2,
                                offset_of_first_element: 3,
                            }
                        ]),
                        vertex_count: mesh.len(),
                        texture: gl_tex,
                    };
                    renderable
                }).collect();
            shit.push(vertices_per_texture_per_node);
        }
        return shit;
    }

    pub fn load_textures(texture_names: &Vec<String>) -> Vec<GlTexture> {
        texture_names.iter().map(|texture_name| {
            let path = format!("d:\\Games\\TalonRO\\grf\\data\\texture\\{}", texture_name);
            GlTexture::from_file(path)
        }).collect()
    }

    fn transform_children_recursively(parent_node_index: usize, nodes: &mut Vec<RsmNode>, is_only: bool) {
        {
            let mut parent_node = &mut nodes[parent_node_index];
            Rsm::transform_vertices(parent_node, Matrix4::identity(), is_only);
        }

        let parent_node_name = nodes[parent_node_index].name.clone();
        let parent_node_name_of_parent = nodes[parent_node_index].parent_name.clone();
        let children_indices = nodes.iter_mut().enumerate().filter(|(i, n)| {
            parent_node_name == n.parent_name && parent_node_name != parent_node_name_of_parent
        }).map(|(i, n)| { i }).collect::<Vec<usize>>();
        for i in children_indices {
            Rsm::transform_children_recursively(i, nodes, is_only);
        }
    }

    fn transform_vertices(node: &mut RsmNode, parent_matrix: Matrix4<f32>, is_only: bool) {
        node.matrix = {
            let mut node_matrix = parent_matrix;

            node_matrix.append_translation_mut(&node.pos);
            // Dynamic or static model
            if node.rot_key_frames.is_empty() {
                let rotation = Rotation3::from_axis_angle(&Unit::new_normalize(node.rotaxis), node.rotangle).to_homogeneous();
                node_matrix = node_matrix * rotation;
            } else {
                let quat = Quaternion::from(Vector4::from(node.rot_key_frames[0].q));
                let rotation = UnitQuaternion::from_quaternion(quat);
                node_matrix = rotation.to_homogeneous() * node_matrix;
//            mat4.rotateQuat( this.matrix, this.matrix, this.rotKeyframes[0].q );
            }

            node_matrix.prepend_nonuniform_scaling_mut(&node.scale);
            node_matrix
        };

        let mut node_local_matrix = node.matrix;

        if !is_only {
            node_local_matrix.append_translation_mut(&node.offset);
        }
        node_local_matrix *= node.mat3.to_homogeneous();

        for vert in node.vertices.iter_mut() {
            *vert = node_local_matrix.transform_point(&vert);
        }
    }

    fn generate_trimesh(node: &RsmNode, faces: &[&NodeFace]) -> Vec<RsmNodeVertex> {
        let mut mesh: Vec<RsmNodeVertex> = Vec::with_capacity(faces.len() * 3);
        let verts = &node.vertices;
        let tverts = &node.texture_vertices;
        for face in faces {
            for i in 0..3 {
                let v = verts[face.vertex_index[i] as usize];
                let tid = face.texture_vertex_index[i] as usize * 6;
                mesh.push(RsmNodeVertex {
                    pos: [v.x, v.y, v.z],
                    texcoord: [tverts[tid + 4], tverts[tid + 5]],
                });
            }
        }

//        let mut trimesh = ProceduralTriMesh::new(
//            std::mem::replace(&mut node.vertices, vec![]),
//            None,
//            None,
//            Some(IndexBuffer::Unified(indices)),
//        );
//        trimesh.recompute_normals();
//        trimesh.replicate_vertices();
//        let trimesh = ncollide3d::shape::TriMesh::from(trimesh);
//        node.trimesh = trimesh;
        return mesh;
    }
}
