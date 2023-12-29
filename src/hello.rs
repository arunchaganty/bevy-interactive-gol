use bevy::prelude::*;
use rand::{SeedableRng, Rng};

pub struct HelloPlugin;

#[derive(Component)]
struct Name(String);

#[derive(Component)]
struct Person;


#[derive(Resource)]
struct EventTimer(Timer);


#[derive(Resource)]
struct RNG {
    rng: rand::rngs::StdRng,
}

#[derive(Resource)]
struct PeopleList {
    names: Vec<String>,
}

const TEXT_FONT_SIZE: f32 = 40.0;

const PEOPLE_NAMES: &'static [&'static str] = &["Elaina Proctor", "Renzo Hume", "Zayna Nieves"];

fn add_people(
    mut commands: Commands,
    time: Res<Time>,
    mut timer: ResMut<EventTimer>,
    mut people: ResMut<PeopleList>,
    mut rng: ResMut<RNG>,
) {
    if timer.0.tick(time.delta()).just_finished() {
        let i = rng.rng.gen_range(0..PEOPLE_NAMES.len());
        people.names.push(PEOPLE_NAMES[i].to_string());
    }
}

fn setup(mut commands: Commands) {
    commands.spawn(
    TextBundle::from_section(
        "",
        TextStyle {
            font_size: TEXT_FONT_SIZE,
            color: Color::WHITE,
            ..Default::default()
        }
    ));
}

fn render_names(mut query: Query<&mut Text>, people: Res<PeopleList>) {
    let mut text = query.single_mut();
    text.sections[0].value = people.names.join("\n");
}

impl Plugin for HelloPlugin {
    fn build(&self, app: &mut App) {
        app
        .insert_resource(PeopleList { names: Vec::new() })
        .insert_resource(RNG { rng: rand::rngs::StdRng::seed_from_u64(42) })
        .insert_resource(EventTimer(Timer::from_seconds(1.0, TimerMode::Repeating)))
        .add_systems(Startup, setup)
        .add_systems(FixedUpdate, render_names)
        .add_systems(Update, add_people)
        ;
    }
}
