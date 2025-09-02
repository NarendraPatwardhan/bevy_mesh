use bevy::input::mouse::{MouseMotion, MouseScrollUnit, MouseWheel};
use bevy::prelude::*;
use bevy::render::{mesh::Indices, mesh::PrimitiveTopology, render_asset::RenderAssetUsages};
use std::f32::consts::{FRAC_PI_2, PI, TAU};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 2000.0,
            affects_lightmapped_meshes: true,
        })
        .add_systems(Startup, (setup_camera, setup_cube, setup_lights))
        .add_systems(Update, (pan_orbit_camera, reset_camera))
        .run();
}

fn setup_lights(mut commands: Commands) {
    // Directional light for better overall illumination
    commands.spawn((
        DirectionalLight {
            illuminance: 5000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -PI / 4.0, -PI / 4.0, 0.0)),
    ));
}

// Components for custom camera (no bundle needed)
#[derive(Component)]
struct PanOrbitState {
    center: Vec3,
    radius: f32,
    upside_down: bool,
    pitch: f32,
    yaw: f32,
}

impl Default for PanOrbitState {
    fn default() -> Self {
        PanOrbitState {
            center: Vec3::ZERO,
            radius: 1.0,
            upside_down: false,
            pitch: 0.0,
            yaw: 0.0,
        }
    }
}

impl PanOrbitState {
    fn default_position() -> Self {
        Self {
            center: Vec3::ZERO,
            radius: 6.0,
            pitch: 0.0,
            yaw: 0.0,
            upside_down: false,
        }
    }
}

#[derive(Component)]
struct PanOrbitSettings {
    pan_sensitivity: f32,
    orbit_sensitivity: f32,
    zoom_sensitivity: f32,
    pan_button: Option<MouseButton>,
    orbit_button: Option<MouseButton>,
    zoom_button: Option<MouseButton>,
    scroll_action: Option<PanOrbitAction>,
    scroll_line_sensitivity: f32,
    scroll_pixel_sensitivity: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum PanOrbitAction {
    Pan,
    Orbit,
    Zoom,
}

impl Default for PanOrbitSettings {
    fn default() -> Self {
        PanOrbitSettings {
            pan_sensitivity: 0.001,
            orbit_sensitivity: 0.1f32.to_radians(),
            zoom_sensitivity: 0.01,
            pan_button: Some(MouseButton::Middle),
            orbit_button: Some(MouseButton::Right),
            zoom_button: None,
            scroll_action: Some(PanOrbitAction::Zoom),
            scroll_line_sensitivity: 16.0,
            scroll_pixel_sensitivity: 1.0,
        }
    }
}

fn setup_camera(mut commands: Commands) {
    let transform = Transform::from_xyz(0.0, 2.0, 6.0).looking_at(Vec3::ZERO, Vec3::Y);
    let (yaw, pitch, _) = transform.rotation.to_euler(EulerRot::YXZ);
    let radius = transform.translation.length();

    // Spawn with Camera3d component (auto-inserts required camera deps)
    commands.spawn((
        Camera3d::default(),
        transform,
        PanOrbitState {
            center: Vec3::ZERO,
            radius,
            upside_down: false,
            pitch,
            yaw,
        },
        PanOrbitSettings::default(),
    ));
}

fn pan_orbit_camera(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut evr_motion: EventReader<MouseMotion>,
    mut evr_scroll: EventReader<MouseWheel>,
    mut q_camera: Query<(&PanOrbitSettings, &mut PanOrbitState, &mut Transform)>,
) {
    let mut total_motion: Vec2 = evr_motion.read().map(|ev| ev.delta).sum();
    total_motion.y = -total_motion.y;

    let mut total_scroll_lines = Vec2::ZERO;
    let mut total_scroll_pixels = Vec2::ZERO;
    for ev in evr_scroll.read() {
        match ev.unit {
            MouseScrollUnit::Line => {
                total_scroll_lines.x += ev.x;
                total_scroll_lines.y -= ev.y;
            }
            MouseScrollUnit::Pixel => {
                total_scroll_pixels.x += ev.x;
                total_scroll_pixels.y -= ev.y;
            }
        }
    }

    for (settings, mut state, mut transform) in &mut q_camera {
        let mut total_pan = Vec2::ZERO;
        if settings
            .pan_button
            .map(|btn| mouse_buttons.pressed(btn))
            .unwrap_or(false)
        {
            total_pan -= total_motion * settings.pan_sensitivity;
        }
        if settings.scroll_action == Some(PanOrbitAction::Pan) {
            total_pan -=
                total_scroll_lines * settings.scroll_line_sensitivity * settings.pan_sensitivity;
            total_pan -=
                total_scroll_pixels * settings.scroll_pixel_sensitivity * settings.pan_sensitivity;
        }

        let mut total_orbit = Vec2::ZERO;
        if settings
            .orbit_button
            .map(|btn| mouse_buttons.pressed(btn))
            .unwrap_or(false)
        {
            total_orbit -= total_motion * settings.orbit_sensitivity;
        }
        if settings.scroll_action == Some(PanOrbitAction::Orbit) {
            total_orbit -=
                total_scroll_lines * settings.scroll_line_sensitivity * settings.orbit_sensitivity;
            total_orbit -= total_scroll_pixels
                * settings.scroll_pixel_sensitivity
                * settings.orbit_sensitivity;
        }

        let mut total_zoom = Vec2::ZERO;
        if settings
            .zoom_button
            .map(|btn| mouse_buttons.pressed(btn))
            .unwrap_or(false)
        {
            total_zoom -= total_motion * settings.zoom_sensitivity;
        }
        if settings.scroll_action == Some(PanOrbitAction::Zoom) {
            total_zoom -=
                total_scroll_lines * settings.scroll_line_sensitivity * settings.zoom_sensitivity;
            total_zoom -=
                total_scroll_pixels * settings.scroll_pixel_sensitivity * settings.zoom_sensitivity;
        }

        let mut any = false;

        if total_zoom != Vec2::ZERO {
            any = true;
            state.radius *= (-total_zoom.y).exp();
        }

        if total_orbit != Vec2::ZERO {
            any = true;
            if settings
                .orbit_button
                .map(|btn| mouse_buttons.just_pressed(btn))
                .unwrap_or(false)
            {
                state.upside_down = state.pitch < -FRAC_PI_2 || state.pitch > FRAC_PI_2;
            }
            if state.upside_down {
                total_orbit.x = -total_orbit.x;
            }
            state.yaw += total_orbit.x;
            state.pitch += total_orbit.y;
            if state.yaw > PI {
                state.yaw -= TAU;
            }
            if state.yaw < -PI {
                state.yaw += TAU;
            }
        }

        if total_pan != Vec2::ZERO {
            any = true;
            let radius = state.radius; // Cache to avoid borrow conflict
            let right = transform.rotation * Vec3::X;
            let up = transform.rotation * Vec3::Y;
            state.center += right * (total_pan.x * radius);
            state.center += up * (total_pan.y * radius);
        }

        if any {
            let rot = Quat::from_euler(EulerRot::YXZ, state.yaw, state.pitch, 0.0);
            transform.rotation = rot;
            transform.translation = state.center + rot * Vec3::Z * state.radius;
        }
    }
}

fn reset_camera(
    keys: Res<ButtonInput<KeyCode>>,
    mut q_camera: Query<(&mut PanOrbitState, &mut Transform)>,
) {
    if keys.just_pressed(KeyCode::KeyR) {
        for (mut state, mut transform) in &mut q_camera {
            *state = PanOrbitState::default_position();

            let rot = Quat::from_euler(EulerRot::YXZ, state.yaw, state.pitch, 0.0);
            transform.rotation = rot;
            transform.translation = state.center + rot * Vec3::Z * state.radius;
        }
    }
}

fn setup_cube(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.5, 0.5, 0.6),
        ..default()
    });

    let indices = Indices::U32(vec![0, 1, 2, 0, 2, 3]);

    let half_size = 0.5;

    for axis in 0..3 {
        for side in [-1.0f32, 1.0] {
            let mut normal = Vec3::ZERO;
            normal[axis] = side;

            let u_axis = (axis + 1) % 3;
            let v_axis = (axis + 2) % 3;

            let u_mult = side;
            let v_mult = 1.0;

            let mut vertices = vec![];
            for i in 0..4 {
                let su = if i == 0 || i == 3 {
                    -half_size
                } else {
                    half_size
                } * u_mult;
                let sv = if i == 0 || i == 1 {
                    -half_size
                } else {
                    half_size
                } * v_mult;

                let mut pos = Vec3::ZERO;
                pos[axis] = half_size * side;
                pos[u_axis] = su;
                pos[v_axis] = sv;
                vertices.push(pos);
            }

            commands.spawn((
                Mesh3d(meshes.add(create_plane_mesh(vertices, normal, indices.clone()))),
                MeshMaterial3d(material.clone()),
                Transform::default(),
            ));
        }
    }
}

fn create_plane_mesh(vertices: Vec<Vec3>, normal: Vec3, indices: Indices) -> Mesh {
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_POSITION,
        vertices.iter().map(|v| [v.x, v.y, v.z]).collect::<Vec<_>>(),
    );
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_NORMAL,
        vec![[normal.x, normal.y, normal.z]; 4],
    );
    mesh.insert_indices(indices);
    mesh
}
