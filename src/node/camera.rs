use egui::{InputState, Key, Pos2, Ui, Vec2};
use egui_snarl::ui::PinInfo;
use egui_snarl::{InPin, NodeId, OutPin, Snarl};
use serde::{Deserialize, Serialize};

use crate::node::viewer::{
    as_number_input_view, empty_input_view, number_input_remote_value, number_input_view, vector_input_remote_value,
    vector_input_view,
};
use crate::node::{Node, NodeFlags};
use crate::types::{Angle, Matrix3, NodePin, Point3, Vector3};

#[derive(Clone, Serialize, Deserialize)]
pub struct CameraNode {
    pub position: NodePin<Point3>,
    pub yaw: NodePin<Angle>,
    pub pitch: NodePin<Angle>,
    /// vfov angle must be between 0..=90 degrees.
    pub vfov: NodePin<Angle>,
    /// Aperture must be between 0..=1.
    pub aperture: NodePin<f64>,
    /// Focus distance must be a positive number.
    pub focus_distance: NodePin<f64>,
    pub scene: NodePin<Vec<NodeId>>,

    previous_mouse_pos: Option<Pos2>,
}

impl Default for CameraNode {
    fn default() -> Self {
        let look_from = Vector3::new(-10.0, 2.0, -4.0);
        let look_at = Vector3::new(0.0, 1.0, 0.0);
        let focus_distance = (look_at - look_from).magnitude();

        Self {
            position: NodePin::new(look_from),
            yaw: NodePin::new(Angle::degrees(25.0)),
            pitch: NodePin::new(Angle::degrees(-10.0)),
            vfov: NodePin::new(Angle::degrees(30.0)),
            aperture: NodePin::new(0.8),
            focus_distance: NodePin::new(focus_distance),
            scene: NodePin::default(),

            previous_mouse_pos: None,
        }
    }
}

impl CameraNode {
    pub const NAME: &str = "Camera";
    pub const INPUTS: [u64; 7] = [
        NodeFlags::TYPICAL_VECTOR_INPUT.bits(),
        NodeFlags::TYPICAL_NUMBER_INPUT.bits(),
        NodeFlags::TYPICAL_NUMBER_INPUT.bits(),
        NodeFlags::TYPICAL_NUMBER_INPUT.bits(),
        NodeFlags::TYPICAL_NUMBER_INPUT.bits(),
        NodeFlags::TYPICAL_NUMBER_INPUT.bits(),
        NodeFlags::PRIMITIVES.bits() | NodeFlags::COLLECTION.bits(),
    ];
    pub const OUTPUTS: [u64; 1] = [NodeFlags::CAMERA.bits()];

    pub fn inputs(&self) -> &[u64] {
        &Self::INPUTS
    }

    pub fn outputs(&self) -> &[u64] {
        &Self::OUTPUTS
    }

    pub fn show_input(pin: &InPin, ui: &mut Ui, snarl: &mut Snarl<Node>) -> PinInfo {
        match pin.id.input {
            0 => {
                const LABEL: &str = "Position";

                let remote_value = vector_input_remote_value(pin, snarl, LABEL);
                let node = snarl[pin.id.node].as_camera_node_mut();
                vector_input_view(ui, LABEL, &mut node.position, remote_value)
            },
            1 => {
                const LABEL: &str = "Yaw";

                let remote_value = number_input_remote_value(pin, snarl, LABEL);
                let node = snarl[pin.id.node].as_camera_node_mut();
                as_number_input_view(ui, LABEL, &mut node.yaw, remote_value)
            },
            2 => {
                const LABEL: &str = "Pitch";

                let remote_value = number_input_remote_value(pin, snarl, LABEL);
                let node = snarl[pin.id.node].as_camera_node_mut();
                as_number_input_view(ui, LABEL, &mut node.pitch, remote_value)
            },
            3 => {
                const LABEL: &str = "VFOV";

                let remote_value = number_input_remote_value(pin, snarl, LABEL);
                let node = snarl[pin.id.node].as_camera_node_mut();
                as_number_input_view(ui, LABEL, &mut node.vfov, remote_value)
            },
            4 => {
                const LABEL: &str = "Aperture";

                let remote_value = number_input_remote_value(pin, snarl, LABEL);
                let node = snarl[pin.id.node].as_camera_node_mut();
                number_input_view(ui, LABEL, &mut node.aperture, remote_value)
            },
            5 => {
                const LABEL: &str = "Focus Distance";

                let remote_value = number_input_remote_value(pin, snarl, LABEL);
                let node = snarl[pin.id.node].as_camera_node_mut();
                number_input_view(ui, LABEL, &mut node.focus_distance, remote_value)
            },
            6 => {
                const LABEL: &str = "Scene";

                let remote_value = match &*pin.remotes {
                    [] => None,
                    [remote] => Some(match &snarl[remote.node] {
                        Node::Primitive(_) => vec![remote.node],
                        Node::Collection(collection) => collection.cloned_nodes(),
                        node => unreachable!("{LABEL} input not suppor connection with `{}`", node.name()),
                    }),
                    _ => None,
                };

                if let Some(value) = remote_value {
                    let Node::Camera(node) = &mut snarl[pin.id.node] else {
                        panic!()
                    };
                    node.scene.set(value);
                }

                empty_input_view(ui, LABEL)
            },
            _ => unreachable!(),
        }
    }

    pub fn connect_input(&mut self, _from: &OutPin, _to: &InPin) {}

    pub fn disconnect_input(&mut self, input_pin: &InPin) {
        match input_pin.id.input {
            0 => self.position.reset(),
            1 => self.yaw.reset(),
            2 => self.pitch.reset(),
            3 => self.vfov.reset(),
            4 => self.aperture.reset(),
            5 => self.focus_distance.reset(),
            6 => self.scene.reset(),
            _ => unreachable!(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Orientation {
    pub forward: Vector3,
    pub right: Vector3,
    pub up: Vector3,
}

impl CameraNode {
    pub fn orientation(&self) -> Orientation {
        let forward = Vector3::new(
            self.yaw.as_ref().as_radians().cos() * self.pitch.as_ref().as_radians().cos(),
            self.pitch.as_ref().as_radians().sin(),
            self.yaw.as_ref().as_radians().sin() * self.pitch.as_ref().as_radians().cos(),
        )
        .normalize();

        let world_up = Vector3::new(0.0, 1.0, 0.0);
        let right = forward.cross(&world_up);
        let up = right.cross(&forward);

        Orientation { forward, right, up }
    }

    pub fn after_events(&mut self, input_state: &InputState) {
        let translation_scale = 2.0 * input_state.stable_dt as f64;
        let look_pressed = input_state.pointer.secondary_down();
        let forward_pressed = input_state.key_pressed(Key::W);
        let backward_pressed = input_state.key_pressed(Key::S);
        let left_pressed = input_state.key_pressed(Key::A);
        let right_pressed = input_state.key_pressed(Key::D);
        let down_pressed = input_state.key_pressed(Key::Q);
        let up_pressed = input_state.key_pressed(Key::E);
        let mouse_pos = input_state.pointer.latest_pos().unwrap_or_default();
        let viewport_size = input_state
            .viewport()
            .inner_rect
            .map(|rect| rect.size())
            .unwrap_or_default();

        if look_pressed {
            if let Some(prev_mouse_pos) = self.previous_mouse_pos {
                let orientation = self.orientation();
                let c1 = orientation.right;
                let c2 = orientation.forward;
                let c3 = c1.cross(&c2).normalize();
                let from_local = Matrix3::new(c1.x, c2.x, c3.x, c1.y, c2.y, c3.y, c1.z, c2.z, c3.z);
                let to_local = from_local.try_inverse().expect("Could not invert matrix");

                // Perform cartesian to spherical coordinate conversion in camera-local space,
                // where z points straight into the screen. That way there is no need to worry
                // about which quadrant of the sphere we are in for the conversion.
                let current_dir = to_local * self.generate_ray_dir(mouse_pos, viewport_size);
                let previous_dir = to_local * self.generate_ray_dir(prev_mouse_pos, viewport_size);

                let x1 = current_dir.x;
                let y1 = current_dir.y;
                let z1 = current_dir.z;

                let x2 = previous_dir.x;
                let y2 = previous_dir.y;
                let z2 = previous_dir.z;

                let p1 = z1.acos();
                let p2 = z2.acos();

                let a1 = y1.signum() * (x1 / (x1 * x1 + y1 * y1).sqrt()).acos();
                let a2 = y2.signum() * (x2 / (x2 * x2 + y2 * y2).sqrt()).acos();

                *self.yaw.as_mut() = self.yaw.get() + Angle::radians(a1 - a2);
                *self.pitch.as_mut() =
                    (self.pitch.get() + Angle::radians(p1 - p2)).clamp(Angle::degrees(-89.0), Angle::degrees(89.0));
            }
        }

        {
            let v = |b| if b { 1.0 } else { 0.0 };
            let translation = Vector3::new(
                translation_scale * (v(right_pressed) - v(left_pressed)),
                translation_scale * (v(up_pressed) - v(down_pressed)),
                translation_scale * (v(forward_pressed) - v(backward_pressed)),
            );

            let orientation = self.orientation();
            *self.position.as_mut() = self.position.get()
                + orientation.right * translation.x
                + orientation.up * translation.y
                + orientation.forward * translation.z;
        }

        self.previous_mouse_pos = Some(mouse_pos);
    }

    fn generate_ray_dir(&self, mouse_pos: Pos2, viewport_size: Vec2) -> Vector3 {
        let position = self.position.get();
        let focus_distance = self.focus_distance.get();
        let aspect_ratio = viewport_size.x as f64 / viewport_size.y as f64;
        let half_height = focus_distance * (0.5 * self.vfov.get().as_radians()).tan();
        let half_width = aspect_ratio * half_height;

        let x = mouse_pos.x as f64 / (viewport_size.x as f64);
        let y = mouse_pos.y as f64 / (viewport_size.y as f64);

        let orientation = self.orientation();

        let point_on_plane = position
            + focus_distance * orientation.forward
            + (2.0 * x - 1.0) * half_width * orientation.right
            + (1.0 - 2.0 * y) * half_height * orientation.up;

        (point_on_plane - position).normalize()
    }
}

pub fn camera_node_by_id(camera_id: NodeId, snarl: &Snarl<Node>) -> Option<&CameraNode> {
    snarl.get_node(camera_id).and_then(Node::camera_node_ref)
}
