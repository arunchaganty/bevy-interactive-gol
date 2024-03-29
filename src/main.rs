use bevy::prelude::*;
use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::diagnostic::LogDiagnosticsPlugin;
// mod hellow;
// mod paddle;
mod conway;

fn setup(mut commands: Commands) -> () {
    commands.spawn(Camera2dBundle::default());
}
 
fn main() {
    App::new()
    .add_plugins(DefaultPlugins)
    .add_plugins(FrameTimeDiagnosticsPlugin::default())
    .add_plugins(LogDiagnosticsPlugin::default())
    .add_plugins(conway::ConwayPlugin)
    .add_systems(Startup, setup)
    .add_systems(Update, bevy::window::close_on_esc)
    .run();
}
