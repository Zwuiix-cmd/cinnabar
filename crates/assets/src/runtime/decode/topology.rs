use super::*;

/// Validates serialized neighbor-selected template groups before decoding.
pub(super) fn runtime_connected_bases(bytes: &[u8]) -> Result<Vec<(usize, u32)>, AssetError> {
    let records = bytes
        .as_chunks::<TEMPLATE_BYTES>()
        .0
        .iter()
        .map(|record| record.as_slice())
        .collect::<Vec<_>>();
    let mut bases = Vec::new();
    let mut index = 0;
    while index < records.len() {
        let flag = u32_at(records[index], 8);
        if flag == crate::MODEL_TEMPLATE_FLAG_DOOR {
            let Some(group) = records.get(index..index + crate::door::TEMPLATE_COUNT as usize)
            else {
                return Err(invalid("door template group is truncated"));
            };
            if group
                .iter()
                .any(|record| u32_at(record, 8) != flag || u32_at(record, 4) != 6)
            {
                return Err(invalid("door template group is noncanonical"));
            }
            bases.push((index, flag));
            index += crate::door::TEMPLATE_COUNT as usize;
        } else if flag == MODEL_TEMPLATE_FLAG_PANE {
            let Some(group) = records.get(index..index + 16) else {
                return Err(invalid("pane template group is truncated"));
            };
            if group.iter().enumerate().any(|(mask, record)| {
                u32_at(record, 8) != flag || u32_at(record, 4) != 6 + (mask as u32).count_ones() * 4
            }) {
                return Err(invalid("pane template group is noncanonical"));
            }
            bases.push((index, flag));
            index += 16;
        } else if matches!(
            flag,
            MODEL_TEMPLATE_FLAG_FENCE_WOOD | MODEL_TEMPLATE_FLAG_FENCE_NETHER
        ) {
            let compound = match records.get(index + 16) {
                Some(head) if u32_at(head, 8) == MODEL_TEMPLATE_FLAG_COMPOUND_NEXT => {
                    let Some(tail) = records.get(index + 17) else {
                        return Err(invalid("fence template group is truncated"));
                    };
                    if u32_at(tail, 8) != 0 {
                        return Err(invalid("fence template group is noncanonical"));
                    }
                    true
                }
                _ => false,
            };
            let group_len = if compound { 18 } else { 17 };
            let Some(group) = records.get(index..index + group_len) else {
                return Err(invalid("fence template group is truncated"));
            };
            let valid = u32_at(group[0], 8) == flag
                && u32_at(group[0], 4) == 6
                && (0..15).all(|mask| {
                    let record = group[mask + 1];
                    u32_at(record, 8) == flag
                        && u32_at(record, 4) == mask.count_ones() * if compound { 10 } else { 8 }
                })
                && if compound {
                    u32_at(group[16], 8) == MODEL_TEMPLATE_FLAG_COMPOUND_NEXT
                        && u32_at(group[16], 4) == 32
                        && u32_at(group[17], 8) == 0
                        && u32_at(group[17], 4) == 8
                } else {
                    u32_at(group[16], 8) == flag && u32_at(group[16], 4) == 15 * 8
                };
            if !valid {
                return Err(invalid("fence template group is noncanonical"));
            }
            bases.push((index, flag));
            index += group_len;
        } else if flag == MODEL_TEMPLATE_FLAG_BUBBLE_COLUMN {
            let Some(group) =
                records.get(index..index + crate::BUBBLE_COLUMN_TEMPLATE_COUNT as usize)
            else {
                return Err(invalid("bubble column template group is truncated"));
            };
            if group.iter().enumerate().any(|(offset, record)| {
                u32_at(record, 8) != flag
                    || u32_at(record, 4) != crate::bubble_column_quad_count(offset as u32)
            }) {
                return Err(invalid("bubble column template group is noncanonical"));
            }
            bases.push((index, flag));
            index += crate::BUBBLE_COLUMN_TEMPLATE_COUNT as usize;
        } else if flag == MODEL_TEMPLATE_FLAG_FIRE {
            let Some(group) = records.get(index..index + crate::fire::FIRE_TEMPLATE_COUNT as usize)
            else {
                return Err(invalid("fire template group is truncated"));
            };
            if group.iter().enumerate().any(|(offset, record)| {
                u32_at(record, 8) != flag
                    || u32_at(record, 4) != crate::fire::fire_template_quad_count(offset as u32)
            }) {
                return Err(invalid("fire template group is noncanonical"));
            }
            bases.push((index, flag));
            index += crate::fire::FIRE_TEMPLATE_COUNT as usize;
        } else {
            index += 1;
        }
    }
    Ok(bases)
}
