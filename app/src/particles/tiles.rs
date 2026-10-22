use std::sync::Arc;

use assets::{
    MATERIAL_FLAG_TINT_MASK, NetworkIdMode,
    facts::{BlockEntityFacts, BlockEntityRenderer},
};
use chunk_pipeline::WorldStream;
use client_interaction::movement::PhysicsCollisionRegistries;
use particles::{
    TileRequest,
    tiles::{material_stage_tint, material_tint},
};
use render_model::{BlockEntityAtlas, ChestVariant};

/// A block-textured particle tile with its gamma-space material tint.
pub(super) struct BlockTile {
    pub(super) tile: TileRequest,
    pub(super) tint: [f32; 4],
}

pub(super) fn block_tile(
    stream: &WorldStream,
    mode: NetworkIdMode,
    network_id: u32,
    block: [i32; 3],
    collisions: Option<&PhysicsCollisionRegistries>,
    block_entity_atlas: Option<&BlockEntityAtlas>,
) -> Option<BlockTile> {
    if !stream.runtime_assets().is_known(mode, network_id) {
        return None;
    }
    if let Some((tile, flags)) =
        particles::tiles::terrain_tile(stream.runtime_assets(), mode, network_id)
    {
        return Some(BlockTile {
            tile,
            tint: biome_tint(stream, flags, block),
        });
    }

    let collisions = collisions?;
    let identifier = collisions.block_identifier(mode, network_id)?;
    let renderer = BlockEntityFacts::pinned()
        .for_registry(network_id, mode, collisions.breg_sha256())?
        .entity
        .renderer;
    let texture = chest_texture_path(identifier, renderer)?;
    Some(BlockTile {
        tile: block_entity_texture_tile(block_entity_atlas?, &texture)?,
        tint: [1.0; 4],
    })
}

/// Selects the single-chest artwork for a block whose visible model is a block entity.
fn chest_texture_path(identifier: &str, renderer: BlockEntityRenderer) -> Option<String> {
    let variant = match renderer {
        BlockEntityRenderer::Chest => ChestVariant::from_block_name(identifier)?,
        BlockEntityRenderer::EnderChest => ChestVariant::Ender,
        _ => return None,
    };
    Some(variant.texture_path(false))
}

/// Copies one square static block-entity texture into the particle tile format.
fn block_entity_texture_tile(atlas: &BlockEntityAtlas, name: &str) -> Option<TileRequest> {
    let texture = atlas.texture(name, [64.0; 2])?;
    let side = texture.rect.width.round();
    if !side.is_finite()
        || side < 1.0
        || side > u32::MAX as f32
        || texture.rect.height.round() != side
    {
        return None;
    }
    let side = side as u32;
    let x = texture.rect.x.round();
    let y = texture.rect.y.round();
    if !x.is_finite() || !y.is_finite() || x < 0.0 || y < 0.0 {
        return None;
    }
    let x = x as u32;
    let y = y as u32;
    let [atlas_width, _] = atlas.size();
    let atlas_height = atlas.static_height();
    if x.checked_add(side)? > atlas_width || y.checked_add(side)? > atlas_height {
        return None;
    }
    let side = usize::try_from(side).ok()?;
    let atlas_width = usize::try_from(atlas_width).ok()?;
    let x = usize::try_from(x).ok()?;
    let y = usize::try_from(y).ok()?;
    let row_bytes = side.checked_mul(4)?;
    let mut pixels = vec![0; row_bytes.checked_mul(side)?];
    for row in 0..side {
        let source_start = (y
            .checked_add(row)?
            .checked_mul(atlas_width)?
            .checked_add(x)?)
        .checked_mul(4)?;
        let source = atlas
            .static_rgba8()
            .get(source_start..source_start.checked_add(row_bytes)?)?;
        let target_start = row.checked_mul(row_bytes)?;
        pixels[target_start..target_start + row_bytes].copy_from_slice(source);
    }
    Some(TileRequest {
        key: block_entity_tile_key(atlas.identity(), name),
        size: u32::try_from(side).ok()?,
        pixels: Arc::from(pixels),
    })
}

fn block_entity_tile_key(identity: [u8; 32], name: &str) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;
    const RESERVED_BITS: u64 = (1 << 63) | (1 << 62);
    let mut hash = FNV_OFFSET;
    for byte in identity.into_iter().chain(name.bytes()) {
        hash = (hash ^ u64::from(byte)).wrapping_mul(FNV_PRIME);
    }
    (1 << 62) | (hash & !RESERVED_BITS)
}

/// Resolves stage colour before consulting the biome at `block`; white when untinted.
pub(super) fn biome_tint(stream: &WorldStream, flags: u32, block: [i32; 3]) -> [f32; 4] {
    if let Some(tint) = material_stage_tint(flags) {
        return tint;
    }
    if flags & MATERIAL_FLAG_TINT_MASK == 0 {
        return [1.0; 4];
    }
    let center = block.map(|c| c as f32 + 0.5);
    let Some(raw) = stream.camera_biome_id(center) else {
        return [1.0; 4];
    };
    let tints = stream.resolved_biome_tints_snapshot();
    let Some(record) = tints.records.get(tints.dense_index(raw) as usize) else {
        return [1.0; 4];
    };
    material_tint(record, flags)
}

#[cfg(test)]
mod tests {
    use assets::{BlockFace, MATERIAL_FLAG_TINT_MASK, NetworkIdMode, RuntimeAssets, TextureRef};
    use particles::tiles::terrain_tile;

    #[test]
    fn stem_colour_survives_particle_routing_without_a_loaded_biome() {
        let stream = chunk_pipeline::WorldStream::new(protocol::WorldBootstrap {
            dimension: 0,
            local_player_runtime_id: 1,
            local_player_unique_id: 1,
            player_position: [0.0; 3],
            world_spawn_position: [0; 3],
            air_network_id: protocol_ids::SEQUENTIAL_AIR_NETWORK_ID,
            block_network_ids_are_hashes: false,
        });
        for growth in [0, 7] {
            let flags = assets::stem::material_flags(growth).unwrap();
            let rgb = assets::stem::colour(growth).unwrap();
            assert_eq!(
                super::biome_tint(&stream, flags, [0; 3]),
                [
                    rgb[0] as f32 / 255.0,
                    rgb[1] as f32 / 255.0,
                    rgb[2] as f32 / 255.0,
                    1.0
                ]
            );
        }
        assert_eq!(super::biome_tint(&stream, 0, [0; 3]), [1.0; 4]);
    }

    #[test]
    fn block_entity_texture_tile_copies_a_chest_texture_from_the_static_atlas() {
        let pixels = (0..16u8)
            .flat_map(|value| [value, 0, 0, 255])
            .collect::<Vec<_>>();
        let bytes = assets::encode_block_entity_catalog(
            b"fixture",
            4,
            4,
            &pixels,
            &[assets::BlockEntityPlacement {
                name: "textures/entity/chest/normal".into(),
                x: 0,
                y: 0,
                width: 4,
                height: 4,
            }],
        )
        .unwrap();
        let assets = assets::RuntimeBlockEntityAssets::decode(&bytes).unwrap();
        let atlas = render_model::BlockEntityAtlas::from_assets(&assets);
        let tile =
            super::block_entity_texture_tile(&atlas, "textures/entity/chest/normal").unwrap();
        assert_eq!(tile.size, 4);
        assert_eq!(&tile.pixels[..4], &[0, 0, 0, 255]);
        assert_eq!(&tile.pixels[12..16], &[1, 0, 0, 255]);
    }

    #[test]
    #[ignore = "requires the local compiled vanilla world carrier"]
    fn real_carrier_terrain_tiles_resolve_stone_deepslate_and_grass() {
        let bytes = std::fs::read(assets::carriers::startup::DEFAULT_ASSET_PATH).unwrap();
        let assets = RuntimeAssets::decode(&bytes).unwrap();
        let records = assets::read_registry_for_protocol(
            assets::pinned_block_registry_bytes(),
            assets::active_content_registry_protocol(),
        )
        .unwrap();
        for name in [
            "minecraft:stone",
            "minecraft:deepslate",
            "minecraft:grass_block",
        ] {
            let states = records
                .iter()
                .filter(|record| &*record.name == name)
                .collect::<Vec<_>>();
            assert!(!states.is_empty(), "pinned registry is missing {name}");
            for record in states {
                let (tile, flags) =
                    terrain_tile(&assets, NetworkIdMode::Sequential, record.sequential_id).unwrap();
                let down = assets.material(
                    assets
                        .resolve(NetworkIdMode::Sequential, record.sequential_id)
                        .face(BlockFace::Down)
                        .material_id(),
                );
                assert_eq!(
                    tile.key,
                    (u64::from(down.texture.page()) << 32) | u64::from(down.texture.layer())
                );
                assert_ne!(down.texture, TextureRef::DIAGNOSTIC);
                assert!(!tile.pixels.as_chunks::<4>().0.contains(&[255, 0, 255, 255]));
                if name == "minecraft:grass_block" {
                    assert_eq!(flags & MATERIAL_FLAG_TINT_MASK, 0);
                }
            }
        }
    }
}
