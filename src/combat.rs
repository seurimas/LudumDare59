use bevy::prelude::*;

use crate::rune_words::battle::{BattlePhase, BattleState, NpcType};
use crate::{GameAssets, GameState};

const BACKDROP_SIZE: f32 = 256.0;
const NPC_SIZE: f32 = 128.0;
const NPC_OFFSET: f32 = (BACKDROP_SIZE - NPC_SIZE) / 2.0;

#[derive(Component)]
struct CombatScene;

#[derive(Component)]
struct NpcSprite;

pub fn configure_combat(app: &mut App) {
    app.add_systems(OnEnter(GameState::Ready), spawn_combat_scene);
    app.add_systems(OnExit(GameState::Ready), despawn_combat_scene);
    app.add_systems(Update, sync_npc_sprite.run_if(in_state(GameState::Ready)));
}

fn spawn_combat_scene(mut commands: Commands, game_assets: Res<GameAssets>) {
    commands
        .spawn((
            CombatScene,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(0.0),
                right: Val::Px(0.0),
                width: Val::Px(BACKDROP_SIZE),
                height: Val::Px(BACKDROP_SIZE),
                ..default()
            },
        ))
        .with_children(|parent| {
            parent.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    top: Val::Px(0.0),
                    left: Val::Px(0.0),
                    width: Val::Px(BACKDROP_SIZE),
                    height: Val::Px(BACKDROP_SIZE),
                    ..default()
                },
                ImageNode::new(game_assets.backdrop.clone()),
                ZIndex(0),
            ));
        });
}

fn despawn_combat_scene(mut commands: Commands, query: Query<Entity, With<CombatScene>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

fn phase_to_sprite_index(phase: BattlePhase) -> usize {
    match phase {
        BattlePhase::Acting => 2,
        BattlePhase::Reacting => 1,
        BattlePhase::Binding => 3,
        BattlePhase::Idle => 0,
    }
}

fn npc_image(npc_type: NpcType, game_assets: &GameAssets) -> ImageNode {
    let (image, layout) = match npc_type {
        NpcType::Goblin => (
            game_assets.goblin.clone(),
            game_assets.goblin_layout.clone(),
        ),
        NpcType::Robed => (game_assets.robed.clone(), game_assets.robed_layout.clone()),
    };
    ImageNode::from_atlas_image(image, TextureAtlas { layout, index: 0 })
}

fn sync_npc_sprite(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    battle_state: Res<BattleState>,
    scene: Query<Entity, With<CombatScene>>,
    mut npc_query: Query<(Entity, &mut ImageNode), With<NpcSprite>>,
) {
    let should_show =
        battle_state.npc_type.is_some() && !matches!(battle_state.phase, BattlePhase::Idle);

    if !should_show {
        for (entity, _) in &npc_query {
            commands.entity(entity).despawn();
        }
        return;
    }

    let npc_type = battle_state.npc_type.unwrap();
    let sprite_index = phase_to_sprite_index(battle_state.phase);

    if npc_query.is_empty() {
        let Ok(scene_entity) = scene.single() else {
            return;
        };
        let mut image_node = npc_image(npc_type, &game_assets);
        if let Some(atlas) = &mut image_node.texture_atlas {
            atlas.index = sprite_index;
        }
        commands.entity(scene_entity).with_children(|parent| {
            parent.spawn((
                NpcSprite,
                Node {
                    position_type: PositionType::Absolute,
                    top: Val::Px(NPC_OFFSET),
                    left: Val::Px(NPC_OFFSET),
                    width: Val::Px(NPC_SIZE),
                    height: Val::Px(NPC_SIZE),
                    ..default()
                },
                image_node,
                ZIndex(1),
            ));
        });
    } else {
        for (_, mut image_node) in &mut npc_query {
            if let Some(atlas) = &mut image_node.texture_atlas {
                atlas.index = sprite_index;
            }
        }
    }
}
