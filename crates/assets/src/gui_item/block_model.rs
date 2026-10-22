//! Shared quads and texels for block thumbnails and native GUI meshes.
//! Connected world shapes keep their authored connectivity; carried inventory shapes are explicit.

use crate::{
    BlockFace, MATERIAL_FLAG_ALPHA_BLEND, MATERIAL_FLAG_ALPHA_CUTOUT, MATERIAL_FLAG_DISABLE_AO,
    MATERIAL_FLAG_DISABLE_FACE_DIMMING, MATERIAL_FLAG_ISOTROPIC, MODEL_TEMPLATE_FLAG_FENCE_NETHER,
    MODEL_TEMPLATE_FLAG_FENCE_WOOD, MODEL_TEMPLATE_FLAG_WALL, Material, ModelQuad, ModelTemplate,
    TextureArray, VisualKind,
};

use super::CUBE_FACES;

/// Largest material tile side drawn; session overlays resample to at most 128.
pub const MAX_GUI_TILE_SIDE: usize = 128;

/// Flags whose material texels can be sampled without unresolved tint or UV transforms.
/// Lighting flags are retained for the renderer and do not alter the stored pixels.
pub const GUI_BLOCK_MATERIAL_FLAGS: u32 = MATERIAL_FLAG_ALPHA_BLEND
    | MATERIAL_FLAG_ALPHA_CUTOUT
    | MATERIAL_FLAG_ISOTROPIC
    | MATERIAL_FLAG_DISABLE_AO
    | MATERIAL_FLAG_DISABLE_FACE_DIMMING;

/// One textured quad of a block item, in block units with tile-relative UVs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GuiBlockQuad {
    pub corners: [[f32; 3]; 4],
    pub uvs: [[f32; 2]; 4],
    pub material: u32,
}

/// Why a block item's geometry or texels cannot be drawn.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GuiBlockReject {
    Geometry,
    Material,
    Texture,
}

/// The quads a block item draws: a cube's camera-facing GUI surfaces, or its template's quads.
pub fn block_item_quads(
    kind: VisualKind,
    faces: [u32; 6],
    template: Option<u32>,
    templates: &[ModelTemplate],
    quads: &[ModelQuad],
) -> Result<Vec<GuiBlockQuad>, GuiBlockReject> {
    let mut out = Vec::new();
    match (kind, template) {
        (VisualKind::Cube, _) => {
            for (face, corners, uvs, _) in CUBE_FACES {
                out.push(GuiBlockQuad {
                    corners,
                    uvs,
                    material: faces[face as usize],
                });
            }
        }
        (VisualKind::Model, Some(template)) => {
            let first = templates
                .get(template as usize)
                .ok_or(GuiBlockReject::Geometry)?;
            if first.flags & (MODEL_TEMPLATE_FLAG_FENCE_WOOD | MODEL_TEMPLATE_FLAG_FENCE_NETHER)
                != 0
            {
                // Bedrock's carried fence is not the connected-world template: it has two
                // end posts and two continuous rails. Use that authored inventory shape for
                // vanilla fences so it matches custom fence item visuals.
                out.extend(carried_fence_quads(faces[BlockFace::South as usize]));
            } else if first.flags & MODEL_TEMPLATE_FLAG_WALL != 0 {
                out.extend(carried_wall_quads(faces[BlockFace::South as usize]));
            } else {
                let parts = crate::model_template_parts(templates, template)
                    .ok_or(GuiBlockReject::Geometry)?;
                for part in parts {
                    let start = part.quad_start as usize;
                    let part_quads = quads
                        .get(start..start + part.quad_count as usize)
                        .ok_or(GuiBlockReject::Geometry)?;
                    out.extend(part_quads.iter().map(|quad| {
                        GuiBlockQuad {
                            corners: quad
                                .positions
                                .map(|point| point.map(|component| f32::from(component) / 256.0)),
                            uvs: quad
                                .uvs
                                .map(|uv| uv.map(|component| f32::from(component) / 4096.0)),
                            material: quad.material,
                        }
                    }));
                }
            }
        }
        _ => return Err(GuiBlockReject::Geometry),
    }
    if out.is_empty() {
        return Err(GuiBlockReject::Geometry);
    }
    Ok(out)
}

/// The two-post, two-rail fence carried by the Bedrock creative inventory.
fn carried_fence_quads(material: u32) -> Vec<GuiBlockQuad> {
    let mut out = Vec::with_capacity(40);
    let elements = [
        (
            [96, 0, 0],
            [160, 256, 64],
            [
                Some([0, 0, 4, 16]),
                Some([12, 0, 16, 16]),
                Some([6, 12, 10, 16]),
                Some([6, 0, 10, 4]),
                Some([6, 0, 10, 16]),
                Some([6, 0, 10, 16]),
            ],
        ),
        (
            [96, 0, 192],
            [160, 256, 256],
            [
                Some([12, 0, 16, 16]),
                Some([0, 0, 4, 16]),
                Some([6, 0, 10, 4]),
                Some([6, 12, 10, 16]),
                Some([6, 0, 10, 16]),
                Some([6, 0, 10, 16]),
            ],
        ),
        (
            [112, 208, -32],
            [144, 240, 0],
            [
                Some([0, 1, 2, 4]),
                Some([0, 1, 2, 4]),
                Some([7, 14, 9, 16]),
                Some([7, 14, 9, 16]),
                Some([0, 1, 2, 4]),
                None,
            ],
        ),
        (
            [112, 208, 64],
            [144, 240, 192],
            [
                Some([4, 1, 12, 4]),
                Some([4, 1, 12, 4]),
                Some([7, 4, 9, 12]),
                Some([7, 4, 9, 12]),
                None,
                None,
            ],
        ),
        (
            [112, 208, 256],
            [144, 240, 288],
            [
                Some([14, 1, 16, 4]),
                Some([14, 1, 16, 4]),
                Some([7, 0, 9, 2]),
                Some([7, 0, 9, 2]),
                None,
                Some([14, 1, 16, 4]),
            ],
        ),
        (
            [112, 80, -32],
            [144, 112, 0],
            [
                Some([0, 7, 2, 10]),
                Some([0, 7, 2, 10]),
                Some([7, 14, 9, 16]),
                Some([7, 14, 9, 16]),
                Some([0, 7, 2, 10]),
                None,
            ],
        ),
        (
            [112, 80, 64],
            [144, 112, 192],
            [
                Some([4, 7, 12, 10]),
                Some([4, 7, 12, 10]),
                Some([7, 4, 9, 12]),
                Some([7, 4, 9, 12]),
                None,
                None,
            ],
        ),
        (
            [112, 80, 256],
            [144, 112, 288],
            [
                Some([14, 7, 16, 10]),
                Some([14, 7, 16, 10]),
                Some([7, 0, 9, 2]),
                Some([7, 0, 9, 2]),
                None,
                Some([14, 7, 16, 10]),
            ],
        ),
    ];
    for (min, max, uvs) in elements {
        for (face, (corners, uv)) in carried_cuboid_faces(min, max)
            .into_iter()
            .zip(uvs)
            .enumerate()
        {
            let Some([u0, v0, u1, v1]) = uv else {
                continue;
            };
            out.push(GuiBlockQuad {
                corners,
                uvs: carried_face_uvs(face, [u0, v0, u1, v1]),
                material,
            });
        }
    }
    out
}

/// Two end pillars and a lower span, matching the authored `geometry.wall_render` item.
fn carried_wall_quads(material: u32) -> Vec<GuiBlockQuad> {
    let post_uvs = [
        [0, 0, 8, 16],
        [0, 0, 8, 16],
        [8, 8, 0, 0],
        [8, 8, 0, 0],
        [0, 0, 8, 16],
        [0, 0, 8, 16],
    ];
    let elements = [
        ([64, 0, 192], [192, 256, 320], post_uvs),
        ([64, 0, -64], [192, 256, 64], post_uvs),
        (
            [80, 0, 16],
            [176, 208, 240],
            [
                [0, 0, 4, 13],
                [0, 0, 4, 13],
                [6, 4, 0, 0],
                [6, 4, 0, 0],
                [0, 0, 6, 13],
                [0, 0, 6, 13],
            ],
        ),
    ];
    let mut out = Vec::with_capacity(elements.len() * 6);
    for (min, max, uvs) in elements {
        for (face, (corners, uv)) in carried_cuboid_faces(min, max)
            .into_iter()
            .zip(uvs)
            .enumerate()
        {
            out.push(GuiBlockQuad {
                // The item's root turns the wall across X, independently of world connections.
                corners: corners.map(|[x, y, z]| [z, y, 1.0 - x]),
                uvs: carried_face_uvs(face, uv),
                material,
            });
        }
    }
    out
}

/// Outward-facing cuboid corners from fixed-point block coordinates.
fn carried_cuboid_faces(
    [min_x, min_y, min_z]: [i16; 3],
    [max_x, max_y, max_z]: [i16; 3],
) -> [[[f32; 3]; 4]; 6] {
    let convert = |[x, y, z]: [i16; 3]| {
        [
            f32::from(x) / 256.0,
            f32::from(y) / 256.0,
            f32::from(z) / 256.0,
        ]
    };
    [
        [
            [min_x, min_y, min_z],
            [min_x, min_y, max_z],
            [min_x, max_y, max_z],
            [min_x, max_y, min_z],
        ],
        [
            [max_x, min_y, min_z],
            [max_x, max_y, min_z],
            [max_x, max_y, max_z],
            [max_x, min_y, max_z],
        ],
        [
            [min_x, min_y, min_z],
            [max_x, min_y, min_z],
            [max_x, min_y, max_z],
            [min_x, min_y, max_z],
        ],
        [
            [min_x, max_y, min_z],
            [min_x, max_y, max_z],
            [max_x, max_y, max_z],
            [max_x, max_y, min_z],
        ],
        [
            [min_x, min_y, min_z],
            [min_x, max_y, min_z],
            [max_x, max_y, min_z],
            [max_x, min_y, min_z],
        ],
        [
            [min_x, min_y, max_z],
            [max_x, min_y, max_z],
            [max_x, max_y, max_z],
            [min_x, max_y, max_z],
        ],
    ]
    .map(|face| face.map(convert))
}

/// Maps a pixel UV rectangle to the corresponding outward-facing cuboid corners.
fn carried_face_uvs(face: usize, [u0, v0, u1, v1]: [u16; 4]) -> [[f32; 2]; 4] {
    let [u0, v0, u1, v1] = [u0, v0, u1, v1].map(|value| f32::from(value) / 16.0);
    match face {
        0 => [[u0, v1], [u1, v1], [u1, v0], [u0, v0]],
        1 => [[u1, v1], [u1, v0], [u0, v0], [u0, v1]],
        2 => [[u0, v1], [u1, v1], [u1, v0], [u0, v0]],
        3 => [[u0, v0], [u0, v1], [u1, v1], [u1, v0]],
        4 => [[u1, v1], [u1, v0], [u0, v0], [u0, v1]],
        _ => [[u0, v1], [u1, v1], [u1, v0], [u0, v0]],
    }
}

/// Reads a material's first-mip tile, side and alpha mode from its texture page.
/// World isotropy and disabled ambient occlusion preserve isolated tiles; tints and rotated UVs do not.
pub fn material_tile<'a>(
    materials: &[Material],
    id: u32,
    array: impl FnOnce(u32) -> Option<&'a TextureArray>,
) -> Result<(&'a [u8], usize, bool), GuiBlockReject> {
    if id == crate::DIAGNOSTIC_MATERIAL {
        return Err(GuiBlockReject::Material);
    }
    let material = materials.get(id as usize).ok_or(GuiBlockReject::Material)?;
    if material.flags & !GUI_BLOCK_MATERIAL_FLAGS != 0 {
        return Err(GuiBlockReject::Material);
    }
    let array = array(material.texture.page()).ok_or(GuiBlockReject::Texture)?;
    let mip = array.mips.first().ok_or(GuiBlockReject::Texture)?;
    let side = mip.size as usize;
    if side == 0 || side > MAX_GUI_TILE_SIDE || material.texture.layer() >= array.layers {
        return Err(GuiBlockReject::Texture);
    }
    let bytes = side * side * 4;
    let start = material.texture.layer() as usize * bytes;
    let tile = mip
        .rgba8
        .get(start..start + bytes)
        .ok_or(GuiBlockReject::Texture)?;
    Ok((tile, side, material.flags & MATERIAL_FLAG_ALPHA_BLEND != 0))
}

/// A full-block face with the cube thumbnail's UV orientation.
#[must_use]
pub fn cube_face(face: BlockFace) -> ([[f32; 3]; 4], [[f32; 2]; 4]) {
    let side_uv = [[0., 1.], [1., 1.], [1., 0.], [0., 0.]];
    match face {
        BlockFace::Up => (
            [[0., 1., 0.], [0., 1., 1.], [1., 1., 1.], [1., 1., 0.]],
            [[0., 0.], [0., 1.], [1., 1.], [1., 0.]],
        ),
        BlockFace::Down => (
            [[0., 0., 1.], [0., 0., 0.], [1., 0., 0.], [1., 0., 1.]],
            [[0., 0.], [0., 1.], [1., 1.], [1., 0.]],
        ),
        BlockFace::South => (
            [[0., 0., 1.], [1., 0., 1.], [1., 1., 1.], [0., 1., 1.]],
            side_uv,
        ),
        BlockFace::North => (
            [[1., 0., 0.], [0., 0., 0.], [0., 1., 0.], [1., 1., 0.]],
            side_uv,
        ),
        BlockFace::West => (
            [[0., 0., 0.], [0., 0., 1.], [0., 1., 1.], [0., 1., 0.]],
            side_uv,
        ),
        BlockFace::East => (
            [[1., 0., 1.], [1., 0., 0.], [1., 1., 0.], [1., 1., 1.]],
            side_uv,
        ),
    }
}

/// The cube thumbnail's side shading, chosen by the face normal's dominant axis.
#[must_use]
pub fn face_brightness(corners: [[f32; 3]; 4]) -> f32 {
    let [a, b, c] = [corners[0], corners[1], corners[2]];
    let (u, v) = (
        [b[0] - a[0], b[1] - a[1], b[2] - a[2]],
        [c[0] - a[0], c[1] - a[1], c[2] - a[2]],
    );
    let normal = [
        (u[1] * v[2] - u[2] * v[1]).abs(),
        u[2] * v[0] - u[0] * v[2],
        (u[0] * v[1] - u[1] * v[0]).abs(),
    ];
    if normal[1].abs() >= normal[0] && normal[1].abs() >= normal[2] {
        CUBE_FACES[0].3
    } else if normal[0] >= normal[2] {
        CUBE_FACES[2].3
    } else {
        CUBE_FACES[1].3
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MATERIAL_FLAG_ISOTROPIC, MATERIAL_FLAG_ROTATE_UV, TextureMip, TextureRef};

    #[test]
    fn cube_gui_surfaces_face_the_inventory_camera() {
        let faces = BlockFace::ALL.map(|face| face as u32 + 1);
        let quads = block_item_quads(VisualKind::Cube, faces, None, &[], &[]).unwrap();
        for quad in &quads {
            let [a, b, c, _] = quad.corners.map(super::super::project_cube);
            let area = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
            assert!(
                area < 0.0,
                "rear material {} is visible through the icon",
                quad.material
            );
        }
        assert_eq!(quads.len(), CUBE_FACES.len());
    }

    #[test]
    fn gui_materials_preserve_tiles_and_alpha_modes() {
        let array = TextureArray {
            layers: 1,
            mips: vec![TextureMip {
                size: 1,
                rgba8: vec![20, 40, 60, 128].into(),
            }]
            .into(),
        };
        for alpha in [0, MATERIAL_FLAG_ALPHA_BLEND, MATERIAL_FLAG_ALPHA_CUTOUT] {
            for flags in [
                MATERIAL_FLAG_ISOTROPIC,
                MATERIAL_FLAG_ISOTROPIC | MATERIAL_FLAG_DISABLE_AO,
            ] {
                let materials = [
                    Material::unvaried(),
                    Material {
                        texture: TextureRef::new(0, 0).unwrap(),
                        flags: alpha | flags,
                        ..Material::unvaried()
                    },
                ];
                let (tile, side, blend) = material_tile(&materials, 1, |_| Some(&array))
                    .expect("ambient occlusion does not affect isolated GUI tiles");
                assert_eq!(tile, [20, 40, 60, 128]);
                assert_eq!(side, 1);
                assert_eq!(blend, alpha == MATERIAL_FLAG_ALPHA_BLEND);
                let mut unsupported = materials;
                unsupported[1].flags |= MATERIAL_FLAG_ROTATE_UV;
                assert_eq!(
                    material_tile(&unsupported, 1, |_| Some(&array)),
                    Err(GuiBlockReject::Material)
                );
            }
        }
    }

    #[test]
    fn fences_use_the_two_post_carried_inventory_shape() {
        let quads = block_item_quads(
            VisualKind::Model,
            [7; 6],
            Some(0),
            &[ModelTemplate {
                quad_start: 0,
                quad_count: 0,
                flags: MODEL_TEMPLATE_FLAG_FENCE_WOOD,
            }],
            &[],
        )
        .unwrap();
        assert_eq!(quads.len(), 40);
        assert!(quads.iter().all(|quad| {
            quad.uvs
                .iter()
                .flatten()
                .all(|value| (0.0..=1.0).contains(value))
        }));
        assert!(
            quads
                .iter()
                .any(|quad| { quad.corners.iter().all(|[_, _, z]| *z < 0.0) })
        );
        assert!(
            quads
                .iter()
                .any(|quad| { quad.corners.iter().all(|[_, _, z]| *z > 1.0) })
        );
    }

    #[test]
    fn walls_use_two_carried_posts_with_a_lower_connecting_span() {
        let world_post =
            carried_cuboid_faces([64, 0, 64], [192, 256, 192]).map(|corners| ModelQuad {
                positions: corners.map(|point| point.map(|value| (value * 256.0) as i16)),
                uvs: [[0, 0], [4096, 0], [4096, 4096], [0, 4096]],
                material: 7,
                flags: 0,
            });
        let quads = block_item_quads(
            VisualKind::Model,
            [7; 6],
            Some(0),
            &[ModelTemplate {
                quad_start: 0,
                quad_count: world_post.len() as u32,
                flags: crate::MODEL_TEMPLATE_FLAG_WALL,
            }],
            &world_post,
        )
        .unwrap();
        let mut bounds = [[f32::INFINITY; 3], [f32::NEG_INFINITY; 3]];
        for quad in &quads {
            assert_eq!(quad.material, 7);
            assert!(quad.uvs.iter().flatten().all(|uv| (0.0..=1.0).contains(uv)));
            for point in quad.corners {
                for axis in 0..3 {
                    bounds[0][axis] = bounds[0][axis].min(point[axis]);
                    bounds[1][axis] = bounds[1][axis].max(point[axis]);
                }
            }
        }
        assert_eq!(bounds, [[-0.25, 0.0, 0.25], [1.25, 1.0, 0.75]]);
        let top_faces = quads
            .iter()
            .filter(|quad| quad.corners.iter().all(|point| point[1] == 1.0))
            .collect::<Vec<_>>();
        assert_eq!(top_faces.len(), 2);
        assert!(
            top_faces
                .iter()
                .any(|quad| quad.corners.iter().all(|p| p[0] <= 0.25))
        );
        assert!(
            top_faces
                .iter()
                .any(|quad| quad.corners.iter().all(|p| p[0] >= 0.75))
        );
        assert!(quads.iter().any(|quad| {
            quad.corners.iter().all(|p| p[1] == 13.0 / 16.0)
                && quad.corners.iter().any(|p| p[0] < 0.5)
                && quad.corners.iter().any(|p| p[0] > 0.5)
        }));
    }

    #[test]
    fn lighting_flags_preserve_material_tiles_without_admitting_unresolved_tint() {
        let array = TextureArray {
            layers: 1,
            mips: vec![crate::TextureMip {
                size: 1,
                rgba8: vec![20, 40, 60, 255].into(),
            }]
            .into(),
        };
        for flags in [
            MATERIAL_FLAG_ALPHA_CUTOUT | crate::MATERIAL_FLAG_DISABLE_FACE_DIMMING,
            MATERIAL_FLAG_DISABLE_AO | crate::MATERIAL_FLAG_DISABLE_FACE_DIMMING,
        ] {
            let materials = [
                Material::unvaried(),
                Material {
                    flags,
                    ..Material::unvaried()
                },
            ];
            let (pixels, side, blend) = material_tile(&materials, 1, |_| Some(&array)).unwrap();
            assert_eq!(pixels, &[20, 40, 60, 255]);
            assert_eq!(side, 1);
            assert!(!blend);
        }
        let materials = [
            Material::unvaried(),
            Material {
                flags: crate::MATERIAL_FLAG_GRASS_TINT,
                ..Material::unvaried()
            },
        ];
        assert_eq!(
            material_tile(&materials, 1, |_| Some(&array)),
            Err(GuiBlockReject::Material)
        );
    }
}
