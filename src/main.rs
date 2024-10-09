use bevy::{
    input::mouse::{MouseMotion, MouseWheel},
    prelude::*,
    utils::HashMap,
    window::{PrimaryWindow, PresentMode},
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                move_camera_keyboard,
                move_camera_mouse,
                zoom_camera,
                toggle_simulating,
                detect_mouse_click,
                draw_cells,
                simulate,
            ),
        )
        .run();
}

#[derive(PartialEq)]
enum CellState {
    On,
    Off,
}

#[derive(Component)]
struct World {
    cells: HashMap<(i64, i64), CellState>,
    simulating: bool,
    time_since_tick: f32,
}

#[derive(Component)]
struct CameraController {
    move_speed: f32,
}

#[derive(Component)]
struct SimulatingText;

#[derive(Component)]
struct Cell;

fn setup(mut commands: Commands, mut window_query: Query<&mut Window, With<PrimaryWindow>>) {
    commands.spawn((
        Camera2dBundle::default(),
        CameraController { move_speed: 500.0 },
    ));

    commands.spawn(World {
        cells: HashMap::new(),
        simulating: false,
        time_since_tick: 0.0,
    });

    commands.spawn((
        TextBundle::from_section(
            "Paused",
            TextStyle {
                font_size: 50.0,
                ..default()
            },
        )
        .with_text_alignment(TextAlignment::Left)
        .with_style(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(5.0),
            left: Val::Px(5.0),
            ..default()
        }),
        SimulatingText,
    ));

    let mut window = window_query.single_mut();
    window.title = "Life".to_string();
}

fn move_camera_keyboard(
    keys: Res<Input<KeyCode>>,
    time: Res<Time>,
    mut query: Query<(&mut Transform, &CameraController), With<Camera2d>>,
) {
    let (mut transform, controller) = query.single_mut();

    let mut movement = Vec3::ZERO;

    if keys.pressed(KeyCode::W) {
        movement += Vec3::Y;
    }
    if keys.pressed(KeyCode::A) {
        movement -= Vec3::X;
    }
    if keys.pressed(KeyCode::S) {
        movement -= Vec3::Y;
    }
    if keys.pressed(KeyCode::D) {
        movement += Vec3::X;
    }

    *transform = transform.with_translation(
        transform.translation
            + movement.normalize_or_zero() * controller.move_speed * time.delta_seconds(),
    );
}

fn move_camera_mouse(
    mut cursor_event_reader: EventReader<MouseMotion>,
    buttons: Res<Input<MouseButton>>,
    keyboard_buttons: Res<Input<KeyCode>>,
    mut camera_query: Query<(&mut Transform, &OrthographicProjection), With<CameraController>>,
) {
    for ev in cursor_event_reader.iter() {
        if (buttons.pressed(MouseButton::Left) && keyboard_buttons.pressed(KeyCode::ShiftLeft))
            || buttons.pressed(MouseButton::Middle)
        {
            let (mut camera, projection) = camera_query.single_mut();
            *camera = camera.with_translation(
                camera.translation + projection.scale * Vec3::new(-ev.delta.x, ev.delta.y, 0.0),
            );
        }
    }
}

fn zoom_camera(
    mut scroll_event_reader: EventReader<MouseWheel>,
    mut projection_query: Query<&mut OrthographicProjection, With<CameraController>>,
) {
    for mut projection in projection_query.iter_mut() {
        for event in scroll_event_reader.iter() {
            projection.scale -= event.y;
            projection.scale = projection.scale.clamp(1.0, 5.0);
        }
    }
}

fn toggle_simulating(
    keys: Res<Input<KeyCode>>,
    mut world_query: Query<&mut World>,
    mut text_query: Query<&mut Text, With<SimulatingText>>,
) {
    if keys.just_pressed(KeyCode::Space) {
        world_query.single_mut().simulating = !world_query.single().simulating;
        text_query.single_mut().sections[0].value = if world_query.single().simulating {
            String::from("Simulating")
        } else {
            String::from("Paused")
        }
    }
}

fn detect_mouse_click(
    window_query: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Transform, &OrthographicProjection), With<CameraController>>,
    mut world_query: Query<&mut World>,
    buttons: Res<Input<MouseButton>>,
    keyboard_buttons: Res<Input<KeyCode>>,
) {
    if buttons.pressed(MouseButton::Left) && !keyboard_buttons.pressed(KeyCode::ShiftLeft) {
        let window = window_query.single();
        if let Some(mouse_position) = window.cursor_position() {
            let (camera_transform, projection) = camera_query.single();

            let cam_x = camera_transform.translation.x;
            let cam_y = camera_transform.translation.y;

            let cell_x = ((cam_x - 0.5 * window.width() * projection.scale
                + mouse_position.x * projection.scale)
                / 30.0)
                .floor() as i64;
            let cell_y = ((cam_y + 0.5 * window.height() * projection.scale
                - mouse_position.y * projection.scale)
                / 30.0)
                .floor() as i64;

            world_query
                .single_mut()
                .cells
                .insert((cell_x, cell_y), CellState::On);
        }
    }

    if buttons.pressed(MouseButton::Right) {
        let window = window_query.single();
        if let Some(mouse_position) = window.cursor_position() {
            let (camera_transform, projection) = camera_query.single();

            let cam_x = camera_transform.translation.x;
            let cam_y = camera_transform.translation.y;

            let cell_x = ((cam_x - 0.5 * window.width() * projection.scale
                + mouse_position.x * projection.scale)
                / 30.0)
                .floor() as i64;
            let cell_y = ((cam_y + 0.5 * window.height() * projection.scale
                - mouse_position.y * projection.scale)
                / 30.0)
                .floor() as i64;

            world_query.single_mut().cells.remove(&(cell_x, cell_y));
        }
    }
}

fn draw_cells(
    mut commands: Commands,
    window_query: Query<&Window, With<PrimaryWindow>>,
    world_query: Query<&World>,
    mut sprite_query: Query<(Entity, &Transform, &mut Sprite), With<Cell>>,
    camera_query: Query<(&Transform, &OrthographicProjection), With<CameraController>>,
) {
    let (camera_transform, projection) = camera_query.single();
    let world = world_query.single();

    let cam_x = camera_transform.translation.x;
    let cam_y = camera_transform.translation.y;

    let window = window_query.single();
    let win_width = window.width();
    let win_height = window.height();

    let mut already_drawn: HashMap<(i64, i64), bool> = HashMap::new();

    for (entity_id, transform, mut sprite) in sprite_query.iter_mut() {
        if (transform.translation.y - cam_y).abs() > window.height() * projection.scale + 30.0
            || (transform.translation.x - cam_x).abs() > window.width() * projection.scale + 30.0
        {
            commands.get_entity(entity_id).unwrap().despawn();
        } else {
            let x = (transform.translation.x as f32 / 30.0).floor() as i64;
            let y = (transform.translation.y as f32 / 30.0).floor() as i64;
            already_drawn.insert((x, y), true);
            let cell = world.cells.get(&(x, y));
            let color = if cell.is_none() || *cell.unwrap() == CellState::Off {
                Color::RED
            } else {
                Color::GREEN
            };
            sprite.color = color;
        }
    }

    let left_border = ((cam_x - 0.5 * win_width * projection.scale - 30.0) / 30.0).floor() as i64;
    let right_border = ((cam_x + 0.5 * win_width * projection.scale + 30.0) / 30.0).floor() as i64;
    let top_border = ((cam_y - 0.5 * win_height * projection.scale - 30.0) / 30.0).floor() as i64;
    let bottom_border =
        ((cam_y + 0.5 * win_height * projection.scale + 30.0) / 30.0).floor() as i64;

    for x in left_border..right_border {
        for y in top_border..bottom_border {
            let drawn = already_drawn.get_mut(&(x, y));
            let cell = world.cells.get(&(x, y));
            let color = if cell.is_none() || *cell.unwrap() == CellState::Off {
                Color::RED
            } else {
                Color::GREEN
            };
            if drawn.is_none() {
                commands.spawn((
                    SpriteBundle {
                        sprite: Sprite {
                            color,
                            custom_size: Some(Vec2::new(20.0, 20.0)),
                            ..default()
                        },
                        transform: Transform::from_xyz(
                            x as f32 * 30.0 + 15.0,
                            y as f32 * 30.0 + 15.0,
                            0.0,
                        ),
                        ..default()
                    },
                    Cell,
                ));
            }
        }
    }
}

fn simulate(mut query: Query<&mut World>, time: Res<Time>) {
    let mut world = query.single_mut();
    world.time_since_tick += time.delta_seconds();

    if world.time_since_tick > 0.1 && world.simulating {
        world.time_since_tick = 0.0;
        let mut neighbourhood_score: HashMap<(i64, i64), i8> = HashMap::new();

        for (position, cell) in world.cells.iter() {
            if *cell == CellState::On {
                let score = neighbourhood_score.get_mut(&(position.0 - 1, position.1 - 1));
                if score.is_none() {
                    neighbourhood_score.insert((position.0 - 1, position.1 - 1), 1);
                } else {
                    *score.unwrap() += 1;
                }

                let score = neighbourhood_score.get_mut(&(position.0, position.1 - 1));
                if score.is_none() {
                    neighbourhood_score.insert((position.0, position.1 - 1), 1);
                } else {
                    *score.unwrap() += 1;
                }

                let score = neighbourhood_score.get_mut(&(position.0 + 1, position.1 - 1));
                if score.is_none() {
                    neighbourhood_score.insert((position.0 + 1, position.1 - 1), 1);
                } else {
                    *score.unwrap() += 1;
                }

                let score = neighbourhood_score.get_mut(&(position.0 - 1, position.1));
                if score.is_none() {
                    neighbourhood_score.insert((position.0 - 1, position.1), 1);
                } else {
                    *score.unwrap() += 1;
                }

                let score = neighbourhood_score.get_mut(&(position.0 + 1, position.1));
                if score.is_none() {
                    neighbourhood_score.insert((position.0 + 1, position.1), 1);
                } else {
                    *score.unwrap() += 1;
                }

                let score = neighbourhood_score.get_mut(&(position.0 - 1, position.1 + 1));
                if score.is_none() {
                    neighbourhood_score.insert((position.0 - 1, position.1 + 1), 1);
                } else {
                    *score.unwrap() += 1;
                }

                let score = neighbourhood_score.get_mut(&(position.0, position.1 + 1));
                if score.is_none() {
                    neighbourhood_score.insert((position.0, position.1 + 1), 1);
                } else {
                    *score.unwrap() += 1;
                }

                let score = neighbourhood_score.get_mut(&(position.0 + 1, position.1 + 1));
                if score.is_none() {
                    neighbourhood_score.insert((position.0 + 1, position.1 + 1), 1);
                } else {
                    *score.unwrap() += 1;
                }

                if neighbourhood_score.get(position).is_none() {
                    neighbourhood_score.insert(*position, 0);
                }
            }
        }

        for (position, score) in neighbourhood_score {
            let cell = world.cells.get(&position);
            if (cell.is_none() || *cell.unwrap() == CellState::Off) && score == 3 {
                world.cells.insert(position, CellState::On);
            } else if (cell.is_some() && *cell.unwrap() == CellState::On)
                && (score == 2 || score == 3)
            {
                world.cells.insert(position, CellState::On);
            } else if cell.is_some() {
                world.cells.remove(&position);
            }
        }
    }
}
