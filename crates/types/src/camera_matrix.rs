use approx_derive::{AbsDiffEq, RelativeEq};
use coordinate_systems::{IntoTransform, Transform};
use nalgebra::{Isometry3, Matrix, Point2, Rotation3, Vector2};
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

use crate::{
    coordinate_systems::{Camera, Ground, Head, Pixel, Robot},
    horizon::Horizon,
    line::Line2,
};

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct CameraMatrices {
    pub top: CameraMatrix,
    pub bottom: CameraMatrix,
}

impl CameraMatrices {
    pub fn to_corrected(
        &self,
        correction_in_robot: Rotation3<f32>,
        correction_in_camera_top: Rotation3<f32>,
        correction_in_camera_bottom: Rotation3<f32>,
    ) -> Self {
        Self {
            top: self
                .top
                .to_corrected(correction_in_robot, correction_in_camera_top),
            bottom: self
                .bottom
                .to_corrected(correction_in_robot, correction_in_camera_bottom),
        }
    }
}

#[derive(
    Clone, Debug, Deserialize, PartialEq, Serialize, SerializeHierarchy, AbsDiffEq, RelativeEq,
)]
#[abs_diff_eq(epsilon = "f32")]
pub struct CameraMatrix {
    pub camera_to_head: Transform<Camera, Head, Isometry3<f32>>,
    pub camera_to_ground: Transform<Camera, Ground, Isometry3<f32>>,
    pub ground_to_camera: Transform<Ground, Camera, Isometry3<f32>>,
    pub camera_to_robot: Transform<Camera, Robot, Isometry3<f32>>,
    pub robot_to_camera: Transform<Robot, Camera, Isometry3<f32>>,
    pub focal_length: Vector2<f32>,
    pub optical_center: Point2<f32>,
    pub field_of_view: Vector2<f32>,
    pub horizon: Horizon,
}

impl Default for CameraMatrix {
    fn default() -> Self {
        Self {
            camera_to_head: Isometry3::identity().framed_transform(),
            camera_to_ground: Isometry3::identity().framed_transform(),
            ground_to_camera: Isometry3::identity().framed_transform(),
            camera_to_robot: Isometry3::identity().framed_transform(),
            robot_to_camera: Isometry3::identity().framed_transform(),
            focal_length: Default::default(),
            optical_center: Point2::origin(),
            field_of_view: Default::default(),
            horizon: Default::default(),
        }
    }
}

impl CameraMatrix {
    /// This takes [0, 1] range focal length & optical center values & actual image size to create camera matrix.
    pub fn from_normalized_focal_and_center(
        focal_length: Vector2<f32>,
        optical_center: Point2<f32>,
        image_size: Vector2<f32>,
        camera_to_head: Transform<Camera, Head, Isometry3<f32>>,
        head_to_robot: Transform<Head, Robot, Isometry3<f32>>,
        robot_to_ground: Transform<Robot, Ground, Isometry3<f32>>,
    ) -> Self {
        let camera_to_robot = head_to_robot * camera_to_head;
        let camera_to_ground = robot_to_ground * camera_to_robot;

        let image_size_diagonal = Matrix::from_diagonal(&image_size);
        let focal_length_scaled = image_size_diagonal * focal_length;
        let optical_center_scaled = image_size_diagonal * optical_center;

        let field_of_view = CameraMatrix::calculate_field_of_view(focal_length_scaled, image_size);

        let horizon = Horizon::from_parameters(
            camera_to_ground,
            focal_length_scaled,
            optical_center_scaled,
            image_size[0],
        );

        Self {
            camera_to_head,
            camera_to_ground,
            ground_to_camera: camera_to_ground.inverse(),
            camera_to_robot,
            robot_to_camera: camera_to_robot.inverse(),
            focal_length: focal_length_scaled,
            optical_center: optical_center_scaled,
            field_of_view,
            horizon,
        }
    }

    pub fn calculate_field_of_view(
        focal_lengths: Vector2<f32>,
        image_size: Vector2<f32>,
    ) -> Vector2<f32> {
        // Ref:  https://www.edmundoptics.eu/knowledge-center/application-notes/imaging/understanding-focal-length-and-field-of-view/
        image_size.zip_map(&focal_lengths, |image_dim, focal_length| -> f32 {
            2.0 * (image_dim * 0.5 / focal_length).atan()
        })
    }

    pub fn to_corrected(
        &self,
        correction_in_robot: Rotation3<f32>,
        correction_in_camera: Rotation3<f32>,
    ) -> Self {
        let camera_to_head = self.camera_to_head;
        let robot_to_head = self.camera_to_head * self.robot_to_camera;
        let head_to_robot = robot_to_head.inverse();
        let ground_to_robot = self.camera_to_robot * self.ground_to_camera;
        let robot_to_ground = ground_to_robot.inverse();

        let corrected_camera_to_head = camera_to_head
            * Isometry3::from_parts(Default::default(), correction_in_camera.inverse().into())
                .framed_transform();
        let head_to_corrected_camera = corrected_camera_to_head.inverse();
        let head_to_corrected_robot =
            Isometry3::from_parts(Default::default(), correction_in_robot.inverse().into())
                .framed_transform()
                * head_to_robot;
        let corrected_robot_to_head = head_to_corrected_robot.inverse();

        let camera_to_robot = head_to_corrected_robot * corrected_camera_to_head;
        let robot_to_camera = head_to_corrected_camera * corrected_robot_to_head;

        CameraMatrix {
            camera_to_head: corrected_camera_to_head,
            camera_to_ground: robot_to_ground * camera_to_robot,
            ground_to_camera: robot_to_camera * ground_to_robot,
            camera_to_robot,
            robot_to_camera,
            focal_length: self.focal_length,
            optical_center: self.optical_center,
            field_of_view: self.field_of_view,
            horizon: self.horizon,
        }
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;
    use nalgebra::{point, vector, Translation3};

    use super::*;

    #[test]
    fn check_field_of_view_calculation() {
        // Old implementation, assumes normalized values
        fn old_fov(focal_lengths: Vector2<f32>) -> Vector2<f32> {
            focal_lengths.map(|f| 2.0 * (0.5 / f).atan())
        }

        let focals = vector![0.63, 1.34];
        let image_size = vector![1.0, 1.0];

        let image_size_abs = vector![640.0, 480.0];
        let focals_scaled = image_size_abs.zip_map(&focals, |dim, focal| dim * focal);

        assert_relative_eq!(
            old_fov(focals),
            CameraMatrix::calculate_field_of_view(focals, image_size)
        );

        assert_relative_eq!(
            old_fov(focals),
            CameraMatrix::calculate_field_of_view(focals_scaled, image_size_abs)
        );
    }

    #[test]
    fn zero_corrections_result_in_identity() {
        let original = CameraMatrix::from_normalized_focal_and_center(
            vector![0.42, 0.1337],
            point![0.42, 0.1337],
            vector![640.0, 480.0],
            Isometry3::from_parts(
                Translation3::new(0.42, 0.1337, 0.17),
                Rotation3::from_euler_angles(0.42, 0.1337, 0.17).into(),
            )
            .framed_transform(),
            Isometry3::from_parts(
                Translation3::new(0.42, 0.1337, 0.17),
                Rotation3::from_euler_angles(0.42, 0.1337, 0.17).into(),
            )
            .framed_transform(),
            Isometry3::from_parts(
                Translation3::new(0.42, 0.1337, 0.17),
                Rotation3::from_euler_angles(0.42, 0.1337, 0.17).into(),
            )
            .framed_transform(),
        );

        let corrected = original.to_corrected(Rotation3::default(), Rotation3::default());

        assert_relative_eq!(original, corrected, epsilon = 0.001);
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct ProjectedFieldLines {
    pub top: Vec<Line2<Pixel>>,
    pub bottom: Vec<Line2<Pixel>>,
}
