use bevy::color::Srgba;
use bevy::input::mouse::{MouseMotion, MouseScrollUnit, MouseWheel};
use bevy::pbr::wireframe::{WireframeConfig, WireframePlugin};
use bevy::prelude::*;
use bevy::render::{mesh::Indices, mesh::PrimitiveTopology, render_asset::RenderAssetUsages};
use bevy_egui::{EguiContexts, EguiPlugin, EguiPrimaryContextPass, egui};
use std::f32::consts::{FRAC_PI_2, PI, TAU};

/// A resource to hold the settings for our procedurally generated planet.
#[derive(Resource, Debug)]
struct PlanetSettings {
    resolution: u32,
    spherify: bool,
    wireframe: bool,
    color: Color,
}

impl Default for PlanetSettings {
    fn default() -> Self {
        Self {
            resolution: 10,
            spherify: true,
            wireframe: false,
            color: Color::srgb(0.5, 0.5, 0.6),
        }
    }
}

/// A resource to hold the handle to the planet's single material.
#[derive(Resource)]
struct PlanetMaterial(Handle<StandardMaterial>);

/// A component to identify a face of the planet and store its primary direction.
#[derive(Component)]
struct PlanetFace {
    normal: Vec3,
}

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            EguiPlugin::default(),
            WireframePlugin::default(),
        ))
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 2000.0,
            ..default()
        })
        .init_resource::<PlanetSettings>()
        .add_systems(Startup, (setup_camera, setup_planet, setup_lights))
        .add_systems(
            Update,
            (pan_orbit_camera, reset_camera, apply_planet_settings),
        )
        .add_systems(EguiPrimaryContextPass, ui_editor)
        .run();
}

fn setup_lights(mut commands: Commands) {
    commands.spawn((
        DirectionalLight {
            illuminance: 5000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -PI / 4.0, -PI / 4.0, 0.0)),
    ));
}

/// Creates the initial 6 faces of the planet and the shared material.
fn setup_planet(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    settings: Res<PlanetSettings>,
) {
    // Create the material and store its handle in a resource
    let material_handle = materials.add(StandardMaterial {
        base_color: settings.color,
        ..default()
    });
    commands.insert_resource(PlanetMaterial(material_handle.clone()));

    let directions = [
        Vec3::Y,
        Vec3::NEG_Y,
        Vec3::NEG_X,
        Vec3::X,
        Vec3::Z,
        Vec3::NEG_Z,
    ];

    for normal in directions {
        let mesh = create_face_mesh(settings.resolution, normal, settings.spherify);

        commands.spawn((
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(material_handle.clone()),
            Transform::default(),
            PlanetFace { normal },
        ));
    }
}

/// Regenerates meshes, updates wireframe, and updates material color if settings have changed.
fn apply_planet_settings(
    settings: Res<PlanetSettings>,
    planet_material: Res<PlanetMaterial>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut wireframe_config: ResMut<WireframeConfig>,
    mut query: Query<(&mut Mesh3d, &PlanetFace)>,
) {
    if settings.is_changed() {
        // Toggle wireframe
        wireframe_config.global = settings.wireframe;

        // Update color
        if let Some(material) = materials.get_mut(&planet_material.0) {
            material.base_color = settings.color;
        }

        // Regenerate meshes
        for (mut mesh_3d, face) in &mut query {
            let new_mesh = create_face_mesh(settings.resolution, face.normal, settings.spherify);
            *mesh_3d = Mesh3d(meshes.add(new_mesh));
        }
    }
}

/// Generates the vertices and indices for a single face of the cube/sphere.
fn create_face_mesh(resolution: u32, normal: Vec3, spherify: bool) -> Mesh {
    let axis_a = Vec3::new(normal.y, normal.z, normal.x);
    let axis_b = normal.cross(axis_a);

    let num_vertices = (resolution * resolution) as usize;
    let num_indices = ((resolution.saturating_sub(1)).pow(2) * 6) as usize;

    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(num_vertices);
    let mut normals: Vec<[f32; 3]> = Vec::with_capacity(num_vertices);
    let mut indices = Vec::with_capacity(num_indices);

    for y in 0..resolution {
        for x in 0..resolution {
            let i = x + y * resolution;
            let percent = Vec2::new(x as f32, y as f32) / (resolution - 1) as f32;

            let point_on_unit_cube =
                normal + (percent.x - 0.5) * 2.0 * axis_a + (percent.y - 0.5) * 2.0 * axis_b;

            if spherify {
                let point_on_unit_sphere = point_on_unit_cube.normalize();
                positions.push(point_on_unit_sphere.into());
                normals.push(point_on_unit_sphere.into());
            } else {
                positions.push(point_on_unit_cube.into());
                normals.push(normal.into());
            }

            if x != resolution - 1 && y != resolution - 1 {
                indices.push(i);
                indices.push(i + resolution + 1);
                indices.push(i + resolution);

                indices.push(i);
                indices.push(i + 1);
                indices.push(i + resolution + 1);
            }
        }
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

/// UI for controlling planet settings and camera reset.
fn ui_editor(
    mut contexts: EguiContexts,
    mut settings: ResMut<PlanetSettings>,
    mut q_camera: Query<(&mut PanOrbitState, &mut Transform)>,
) {
    let Ok(ctx) = contexts.ctx_mut() else { return };
    egui::Window::new("Controls").show(ctx, |ui| {
        ui.label("Planet Settings");
        ui.add(egui::Slider::new(&mut settings.resolution, 2..=256).text("Resolution"));
        ui.checkbox(&mut settings.spherify, "Spherify");
        ui.checkbox(&mut settings.wireframe, "Wireframe");

        ui.label("Base Color:");
        color_picker_widget(ui, &mut settings.color);

        ui.separator();

        ui.label("Press 'R' to reset camera.");
        if ui.button("Reset Camera Now").clicked() {
            for (mut state, mut transform) in &mut q_camera {
                *state = PanOrbitState::default_position();
                let rot = Quat::from_euler(EulerRot::YXZ, state.yaw, state.pitch, 0.0);
                transform.rotation = rot;
                transform.translation = state.center + rot * Vec3::Z * state.radius;
            }
        }
    });
}

/// A helper function to create a color picker widget.
fn color_picker_widget(ui: &mut egui::Ui, color: &mut Color) -> egui::Response {
    let [r, g, b, a] = Srgba::from(*color).to_f32_array();
    let mut egui_color: egui::Rgba = egui::Rgba::from_srgba_unmultiplied(
        (r * 255.0) as u8,
        (g * 255.0) as u8,
        (b * 255.0) as u8,
        (a * 255.0) as u8,
    );
    let res = egui::widgets::color_picker::color_edit_button_rgba(
        ui,
        &mut egui_color,
        egui::color_picker::Alpha::Opaque,
    );
    let [r, g, b, a] = egui_color.to_srgba_unmultiplied();
    *color = Color::srgba(
        r as f32 / 255.0,
        g as f32 / 255.0,
        b as f32 / 255.0,
        a as f32 / 255.0,
    );
    res
}

// --- Camera Controller Code (Unchanged from your original) ---

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
    mut contexts: EguiContexts,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut evr_motion: EventReader<MouseMotion>,
    mut evr_scroll: EventReader<MouseWheel>,
    mut q_camera: Query<(&PanOrbitSettings, &mut PanOrbitState, &mut Transform)>,
) {
    if let Ok(ctx) = contexts.ctx_mut() {
        if ctx.wants_pointer_input() {
            return;
        }
    }
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
            let radius = state.radius;
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
    mut contexts: EguiContexts,
    keys: Res<ButtonInput<KeyCode>>,
    mut q_camera: Query<(&mut PanOrbitState, &mut Transform)>,
) {
    if let Ok(ctx) = contexts.ctx_mut() {
        if ctx.wants_keyboard_input() {
            return;
        }
    }
    if keys.just_pressed(KeyCode::KeyR) {
        for (mut state, mut transform) in &mut q_camera {
            *state = PanOrbitState::default_position();
            let rot = Quat::from_euler(EulerRot::YXZ, state.yaw, state.pitch, 0.0);
            transform.rotation = rot;
            transform.translation = state.center + rot * Vec3::Z * state.radius;
        }
    }
}
