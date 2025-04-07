pub use self::angle::Angle;
pub use self::pin::NodePin;
pub use self::ray::Ray;

pub mod angle;
pub mod pin;
pub mod ray;

pub type Color = egui::Color32;
pub type Vector3 = nalgebra::Vector3<f64>;
pub type Vector3f32 = nalgebra::Vector3<f32>;
pub type Vector4f32 = nalgebra::Vector4<f32>;
pub type Point3 = Vector3;
pub type Matrix3 = nalgebra::Matrix3<f64>;
pub type Matrix4 = nalgebra::Matrix4<f64>;
pub type Matrix4f32 = nalgebra::Matrix4<f32>;

#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct Basis {
    pub u: Vector3,
    pub v: Vector3,
    pub w: Vector3,
}

pub fn from_vector3_to_vector3f32(v: &Vector3) -> Vector3f32 {
    Vector3f32::new(v.x as _, v.y as _, v.z as _)
}
