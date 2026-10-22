use super::context::{
    ModelStorage, RuleInputs, diagnostic_visual, push_model_template, set_model_visual,
};
use super::dispatcher::CompileRuleResult;
use {super::super::*, assets::BlockFace};

pub(in crate::compiler) fn compile_rule(
    record: &RegistryRecord,
    inputs: &RuleInputs<'_>,
    templates: &mut BTreeMap<[u32; 2], u32>,
    storage: &mut ModelStorage<'_>,
) -> Result<CompileRuleResult, AssetError> {
    if !is_fence(record) {
        return Ok(CompileRuleResult::NoMatch);
    }
    let mut visual = diagnostic_visual(record);
    if let Some(material) = inputs.material(record, BlockFace::South) {
        let flag = if assets::facts::block_render_fact(&record.name).kind
            == assets::facts::BlockRenderKind::NetherBrickFence
        {
            MODEL_TEMPLATE_FLAG_FENCE_NETHER
        } else {
            MODEL_TEMPLATE_FLAG_FENCE_WOOD
        };
        let key = [material, flag];
        let base = if let Some(&base) = templates.get(&key) {
            base
        } else {
            let base = u32::try_from(storage.templates.len()).map_err(|_| {
                AssetError::BlobSizeOverflow {
                    section: "model template",
                }
            })?;
            push_model_template(
                fence_post_quads(material),
                flag,
                storage.templates,
                storage.quads,
            )?;
            for mask in 0..16 {
                let arms = fence_arm_quads(material, mask);
                if arms.len() <= assets::MAX_MODEL_TEMPLATE_QUADS {
                    push_model_template(arms, flag, storage.templates, storage.quads)?;
                } else {
                    // Four connected sides have forty faces. Keep the first part marked as a
                    // compound so the mesher consumes the immediately following remainder.
                    let (first, second) = arms.split_at(assets::MAX_MODEL_TEMPLATE_QUADS);
                    push_model_template(
                        first.to_vec(),
                        assets::MODEL_TEMPLATE_FLAG_COMPOUND_NEXT,
                        storage.templates,
                        storage.quads,
                    )?;
                    push_model_template(second.to_vec(), 0, storage.templates, storage.quads)?;
                }
            }
            templates.insert(key, base);
            base
        };
        set_model_visual(&mut visual, [material; 6], base);
    }
    Ok(CompileRuleResult::Compiled(visual))
}

pub(in crate::compiler) fn fence_arm_quads(material: u32, mask: u32) -> Vec<ModelQuad> {
    debug_assert!(mask <= 15);
    let mut quads = Vec::with_capacity(mask.count_ones() as usize * 10);
    let directions = [
        // These bounds and UV rectangles are the authored Bedrock fence carrier.
        (1, [112, 0, 0], [144, 0, 96]),
        (2, [160, 0, 112], [256, 0, 144]),
        (4, [112, 0, 160], [144, 0, 256]),
        (8, [0, 0, 112], [96, 0, 144]),
    ];
    for (bit, mut min, mut max) in directions {
        if mask & bit == 0 {
            continue;
        }
        for (min_y, max_y, side_v) in [(96, 144, 7), (192, 240, 1)] {
            min[1] = min_y;
            max[1] = max_y;
            quads.extend(fence_cuboid_quads(
                material,
                min,
                max,
                fence_arm_faces(bit, side_v),
            ));
        }
    }
    quads
}

fn fence_arm_faces(bit: u32, side_v: u16) -> [Option<[u16; 4]>; 6] {
    match bit {
        1 => [
            Some([0, side_v, 6, side_v + 3]),
            Some([10, side_v, 16, side_v + 3]),
            Some([9, 16, 7, 10]),
            Some([9, 6, 7, 0]),
            Some([7, side_v, 9, side_v + 3]),
            None,
        ],
        2 => [
            Some([10, side_v, 16, side_v + 3]),
            Some([0, side_v, 6, side_v + 3]),
            Some([9, 6, 7, 0]),
            Some([9, 16, 7, 10]),
            None,
            Some([7, side_v, 9, side_v + 3]),
        ],
        4 => [
            Some([7, side_v, 9, side_v + 3]),
            None,
            Some([6, 9, 0, 7]),
            Some([6, 9, 0, 7]),
            Some([10, side_v, 16, side_v + 3]),
            Some([0, side_v, 6, side_v + 3]),
        ],
        8 => [
            None,
            Some([7, side_v, 9, side_v + 3]),
            Some([16, 9, 10, 7]),
            Some([16, 9, 10, 7]),
            Some([0, side_v, 6, side_v + 3]),
            Some([10, side_v, 16, side_v + 3]),
        ],
        _ => unreachable!("fence arm direction is one of four cardinal bits"),
    }
}

fn fence_post_quads(material: u32) -> Vec<ModelQuad> {
    fence_cuboid_quads(
        material,
        [96, 0, 96],
        [160, 256, 160],
        [
            Some([6, 0, 10, 16]),
            Some([6, 0, 10, 16]),
            Some([10, 10, 6, 6]),
            Some([10, 10, 6, 6]),
            Some([6, 0, 10, 16]),
            Some([6, 0, 10, 16]),
        ],
    )
}

fn fence_cuboid_quads(
    material: u32,
    min: [i16; 3],
    max: [i16; 3],
    faces: [Option<[u16; 4]>; 6],
) -> Vec<ModelQuad> {
    cuboid_quads([material; 6], min, max)
        .into_iter()
        .enumerate()
        .filter_map(|(face, mut quad)| {
            let uv = faces[face]?;
            quad.uvs = fence_uvs(face, uv);
            Some(quad)
        })
        .collect()
}

fn fence_uvs(face: usize, [u0, v0, u1, v1]: [u16; 4]) -> [[u16; 2]; 4] {
    let [u0, v0, u1, v1] = [u0, v0, u1, v1].map(|value| value * 256);
    match face {
        0 => [[u0, v1], [u1, v1], [u1, v0], [u0, v0]],
        1 => [[u1, v1], [u1, v0], [u0, v0], [u0, v1]],
        2 => [[u0, v1], [u1, v1], [u1, v0], [u0, v0]],
        3 => [[u0, v0], [u0, v1], [u1, v1], [u1, v0]],
        4 => [[u1, v1], [u1, v0], [u0, v0], [u0, v1]],
        _ => [[u0, v1], [u1, v1], [u1, v0], [u0, v0]],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fence_arms_keep_authored_rows_and_inner_caps() {
        let quads = fence_arm_quads(3, 1);
        assert_eq!(quads.len(), 10);
        assert_eq!(quads[0].uvs, fence_uvs(0, [0, 7, 6, 10]));
        assert_eq!(quads[5].uvs, fence_uvs(0, [0, 1, 6, 4]));
        assert_eq!(fence_arm_faces(1, 1)[5], None);
    }
}
