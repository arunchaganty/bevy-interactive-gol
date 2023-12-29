use bevy::prelude::*;
use bevy::sprite::{
    Material2d,
    Material2dPlugin,
    MaterialMesh2dBundle,
    collide_aabb::{Collision, collide},
};
use bevy::reflect::TypePath;
use bevy::render::render_resource::*;

pub struct PaddlePlugin;

#[derive(Component)]
struct Paddle;

#[derive(Component)]
struct Ball;

#[derive(Component)]
struct Velocity(Vec3);

#[derive(Event, Default)]
struct CollisionEvent;

const PADDLE_COLOR: Color = Color::rgb(0.3, 0.3, 0.7);
const PADDLE_SIZE: Vec3 = Vec3::new(120.0, 20.0, 0.0);
const BALL_SIZE: Vec3 = Vec3::new(20.0, 20.0, 0.0);


#[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
struct CustomMaterial {
    #[uniform(0)]
    color: Vec4,
}

impl Material2d for CustomMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/circle.wgsl".into()
    }
}

fn setup(
    mut commands: Commands,
    mut materials: ResMut<Assets<CustomMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    // Create a paddle and ball.
    commands.spawn(
        (
            Paddle,
            Velocity(Vec3::new(0.0, 0.0, 0.0)),
            SpriteBundle {
                transform: Transform::
                    from_translation(Vec3::new(0.0, -250.0, 0.0))
                    .with_scale(PADDLE_SIZE),
                sprite: Sprite {
                    color: PADDLE_COLOR,
                    ..default()
                },
                ..default()
            },
        )
    );
    commands.spawn(
        (
            Ball,
            Velocity(Vec3::new(0.0, -200.0, 0.0)),
            MaterialMesh2dBundle {
                mesh: meshes.add(shape::Circle::default().into()).into(),
                material: materials.add(CustomMaterial {color: Color::WHITE.into()}),
                transform: Transform {
                    translation: Vec3::new(0.0, 0.0, 0.0),
                    scale: BALL_SIZE,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
    );
}

fn control_paddle(
    keyboard_input: Res<Input<KeyCode>>,
    mut query: Query<&mut Velocity, With<Paddle>>,
) {
    for mut velocity in query.iter_mut() {
        if keyboard_input.pressed(KeyCode::Left) {
            velocity.0.x -= 10.0;
        }
        if keyboard_input.pressed(KeyCode::Right) {
            velocity.0.x += 10.0;
        }
    }
}

fn check_collisions(
    mut ball_query: Query<(&mut Velocity, &Transform), With<Ball>>,
    mut collision_query: Query<&Transform, With<Paddle>>,
    mut collision_events: EventWriter<CollisionEvent>,
) {
    let (mut ball_velocity, ball_transform) = ball_query.single_mut();
    let ball_size = ball_transform.scale.truncate();

    for paddle_transform in collision_query.iter_mut() {
        let collision = collide(
            ball_transform.translation,
            ball_size,
            paddle_transform.translation,
            paddle_transform.scale.truncate(),
        );
        if let Some(collision) = collision {
            collision_events.send_default();
            match collision {
                Collision::Left | Collision::Right => {
                    ball_velocity.0.x = -ball_velocity.0.x;
                }
                Collision::Top | Collision::Bottom => {
                    ball_velocity.0.y = -ball_velocity.0.y;
                }
                Collision::Inside => {}
            }
        }

    }
}

fn move_objects(
    mut query: Query<(&mut Transform, &Velocity)>,
    time: Res<Time>,
) {
    for (mut transform, velocity) in query.iter_mut() {
        transform.translation += velocity.0 * time.delta_seconds();
    }
}

fn damp_paddle(
    mut query: Query<&mut Velocity, With<Paddle>>,
    time: Res<Time>,
) {
    for mut velocity in query.iter_mut() {
        let velocity_ = velocity.0;
        // TODO: better damping.
        velocity.0 -= velocity_ * 0.9 * time.delta_seconds();
    }
}

fn change_ball_color(time: Res<Time>, mut materials: ResMut<Assets<CustomMaterial>>) {
    for (_, material) in materials.iter_mut() {
        // rainbow color effect
        let new_color = Color::hsl((time.elapsed_seconds() * 60.0) % 360.0, 1., 0.5);
        material.color = new_color.into();
    }
}

impl Plugin for PaddlePlugin {
    fn build(&self, app: &mut App) {
        app
        .add_plugins(Material2dPlugin::<CustomMaterial>::default())
        .add_event::<CollisionEvent>()
        .add_systems(Startup, setup)
        .add_systems(FixedUpdate, (
            move_objects,
            damp_paddle,
            control_paddle,
            check_collisions,
        ).chain())
        .add_systems(Update, (
            change_ball_color, bevy::window::close_on_esc
        )
        )
        ;
    }
}
