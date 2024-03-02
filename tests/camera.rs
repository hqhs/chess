use glam::{self, Mat4, Vec3};

#[test]
fn test_camera() {
    let eye = Vec3::new(1.5, -5.0, 3.0);
    let looking_at = Vec3::ZERO;
    let top = Vec3::Z;
    let view = Mat4::look_at_rh(eye, looking_at, top);

    let (scale, rotation, translation) = view.to_scale_rotation_translation();

    let euler_rotation = glam::EulerRot::YXZ;
    let (yaw, pitch, roll) = rotation.to_euler(euler_rotation);

    println!("{yaw} {pitch} {roll}");
    println!("{scale} {translation}");
}
