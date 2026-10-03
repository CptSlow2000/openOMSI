//! Physical screen layout and off-axis projections for a spanned multimonitor display.

use glam::{DVec3, Mat4, Vec3, Vec4};

#[derive(Clone, Debug, PartialEq)]
pub struct Layout {
    pub count: u8,
    pub width_mm: f32,
    pub height_mm: f32,
    pub distance_mm: f32,
    pub bezel_mm: f32,
    pub left_angle_deg: f32,
    pub right_angle_deg: f32,
    /// Zero uses the physically measured screen and eye geometry.
    pub fov_deg: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct View {
    pub center: DVec3,
    pub yaw_deg: f32,
    /// Off-axis projection; the renderer combines it with the screen's view transform.
    pub projection: Mat4,
}

impl Layout {
    pub fn validate(&self) -> Result<(), &'static str> {
        if !(1..=9).contains(&self.count) {
            return Err("monitor count must be 1..=9");
        }
        if !self.width_mm.is_finite() || !(100.0..=3000.0).contains(&self.width_mm) {
            return Err("monitor width must be 100..=3000 mm");
        }
        if !self.height_mm.is_finite() || !(100.0..=2000.0).contains(&self.height_mm) {
            return Err("monitor height must be 100..=2000 mm");
        }
        if !self.distance_mm.is_finite() || !(100.0..=3000.0).contains(&self.distance_mm) {
            return Err("eye-to-screen distance must be 100..=3000 mm");
        }
        if !self.bezel_mm.is_finite() || !(0.0..=200.0).contains(&self.bezel_mm) {
            return Err("bezel correction must be 0..=200 mm");
        }
        if !self.left_angle_deg.is_finite()
            || !self.right_angle_deg.is_finite()
            || !(-80.0..=0.0).contains(&self.left_angle_deg)
            || !(0.0..=80.0).contains(&self.right_angle_deg)
        {
            return Err("screen angles must be within -80..=80 degrees and straddle zero");
        }
        if !self.fov_deg.is_finite()
            || (self.fov_deg != 0.0 && !(20.0..=120.0).contains(&self.fov_deg))
        {
            return Err("multimonitor FOV must be 0 (physical) or 20..=120 degrees");
        }
        Ok(())
    }

    /// The screens, ordered left-to-right. Angles interpolate between the configured outer
    /// screens; with an odd count, the middle screen is straight ahead.
    pub fn views(&self, eye: DVec3, base_yaw_deg: f32) -> Result<Vec<View>, &'static str> {
        self.validate()?;
        if !eye.is_finite() || !base_yaw_deg.is_finite() {
            return Err("eye position and heading must be finite");
        }

        let count = self.count as usize;
        let base_yaw = base_yaw_deg.to_radians();
        let angle_at = |index: usize| {
            if count == 1 {
                0.0
            } else {
                self.left_angle_deg
                    + (self.right_angle_deg - self.left_angle_deg) * index as f32
                        / (count - 1) as f32
            }
        };
        let right_at = |index: usize| {
            let yaw = base_yaw + angle_at(index).to_radians();
            Vec3::new(yaw.cos(), -yaw.sin(), 0.0)
        };
        let forward = Vec3::new(base_yaw.sin(), base_yaw.cos(), 0.0);
        let mut centers = vec![DVec3::ZERO; count];
        let gap = f64::from(self.bezel_mm) / 1000.0;
        let width = f64::from(self.width_mm) / 1000.0;
        let distance = f64::from(self.distance_mm) / 1000.0;
        let center_index = count / 2;

        if count % 2 == 1 {
            centers[center_index] = eye + forward.as_dvec3() * distance;
            for index in (0..center_index).rev() {
                let joint = centers[index + 1] - right_at(index + 1).as_dvec3() * (width * 0.5);
                let gap_dir = (right_at(index) + right_at(index + 1)).normalize_or_zero();
                centers[index] =
                    joint - gap_dir.as_dvec3() * gap - right_at(index).as_dvec3() * (width * 0.5);
            }
            for index in center_index + 1..count {
                let joint = centers[index - 1] + right_at(index - 1).as_dvec3() * (width * 0.5);
                let gap_dir = (right_at(index - 1) + right_at(index)).normalize_or_zero();
                centers[index] =
                    joint + gap_dir.as_dvec3() * gap + right_at(index).as_dvec3() * (width * 0.5);
            }
        } else {
            let seam = eye + forward.as_dvec3() * distance;
            centers[center_index - 1] =
                seam - right_at(center_index - 1).as_dvec3() * (width * 0.5);
            centers[center_index] = seam + right_at(center_index).as_dvec3() * (width * 0.5);
            for index in (0..center_index - 1).rev() {
                let joint = centers[index + 1] - right_at(index + 1).as_dvec3() * (width * 0.5);
                let gap_dir = (right_at(index) + right_at(index + 1)).normalize_or_zero();
                centers[index] =
                    joint - gap_dir.as_dvec3() * gap - right_at(index).as_dvec3() * (width * 0.5);
            }
            for index in center_index + 1..count {
                let joint = centers[index - 1] + right_at(index - 1).as_dvec3() * (width * 0.5);
                let gap_dir = (right_at(index - 1) + right_at(index)).normalize_or_zero();
                centers[index] =
                    joint + gap_dir.as_dvec3() * gap + right_at(index).as_dvec3() * (width * 0.5);
            }
        }

        let height = f64::from(self.height_mm) / 1000.0;
        let near = 0.1;
        let far = 6_000.0;
        let fov_scale = if self.fov_deg == 0.0 {
            1.0
        } else {
            (self.fov_deg.to_radians() * 0.5).tan()
                / (height as f32 * 0.5 / (self.distance_mm / 1000.0))
        };
        (0..count)
            .map(|index| {
                let yaw_deg = base_yaw_deg + angle_at(index);
                let yaw = yaw_deg.to_radians();
                let right = Vec3::new(yaw.cos(), -yaw.sin(), 0.0);
                let normal = Vec3::new(yaw.sin(), yaw.cos(), 0.0);
                let center = centers[index];
                let pa =
                    (center - eye - right.as_dvec3() * (width * 0.5) - DVec3::Z * (height * 0.5))
                        .as_vec3();
                let pb = (center - eye + right.as_dvec3() * (width * 0.5)
                    - DVec3::Z * (height * 0.5))
                    .as_vec3();
                let pc = (center - eye - right.as_dvec3() * (width * 0.5)
                    + DVec3::Z * (height * 0.5))
                    .as_vec3();
                let distance = pa.dot(normal);
                if !distance.is_finite() || distance <= 0.0 {
                    return Err("screen plane must be in front of the eye");
                }
                let left = pa.dot(right) * near / distance as f32 * fov_scale;
                let right_edge = pb.dot(right) * near / distance as f32 * fov_scale;
                let bottom = pa.z * near / distance as f32 * fov_scale;
                let top = pc.z * near / distance as f32 * fov_scale;
                let projection = off_axis_frustum(left, right_edge, bottom, top, near, far)?;
                Ok(View {
                    center,
                    yaw_deg,
                    projection,
                })
            })
            .collect()
    }
}

/// A right-handed, zero-to-one depth frustum with reversed Z (near maps to 1, far to 0).
fn off_axis_frustum(
    left: f32,
    right: f32,
    bottom: f32,
    top: f32,
    near: f32,
    far: f32,
) -> Result<Mat4, &'static str> {
    if ![left, right, bottom, top, near, far]
        .into_iter()
        .all(f32::is_finite)
        || right <= left
        || top <= bottom
        || near <= 0.0
        || far <= near
    {
        return Err("invalid off-axis frustum");
    }
    let depth = far - near;
    Ok(Mat4::from_cols(
        Vec4::new(2.0 * near / (right - left), 0.0, 0.0, 0.0),
        Vec4::new(0.0, 2.0 * near / (top - bottom), 0.0, 0.0),
        Vec4::new(
            (right + left) / (right - left),
            (top + bottom) / (top - bottom),
            near / depth,
            -1.0,
        ),
        Vec4::new(0.0, 0.0, near * far / depth, 0.0),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn triple() -> Layout {
        Layout {
            count: 3,
            width_mm: 600.0,
            height_mm: 340.0,
            distance_mm: 650.0,
            bezel_mm: 0.0,
            left_angle_deg: -45.0,
            right_angle_deg: 45.0,
            fov_deg: 0.0,
        }
    }

    #[test]
    fn three_monitor_layout_is_centered_and_oriented() {
        let views = triple().views(DVec3::ZERO, 0.0).unwrap();
        assert_eq!(views.len(), 3);
        assert_eq!(views[1].yaw_deg, 0.0);
        assert!((views[0].yaw_deg + 45.0).abs() < 1e-5);
        assert!((views[2].yaw_deg - 45.0).abs() < 1e-5);
        assert!((views[1].center.y - 0.65).abs() < 1e-5);
        assert!((views[0].center.x + views[2].center.x).abs() < 1e-5);
        assert!((views[0].projection.x_axis.x - views[2].projection.x_axis.x).abs() < 1e-5);
        assert!((views[0].projection.y_axis.y - views[2].projection.y_axis.y).abs() < 1e-5);
        assert!((views[0].projection.z_axis.x + views[2].projection.z_axis.x).abs() < 1e-5);
    }

    #[test]
    fn single_monitor_is_a_centered_off_axis_projection() {
        let mut layout = triple();
        layout.count = 1;
        let view = layout.views(DVec3::ZERO, 0.0).unwrap()[0];
        assert_eq!(view.yaw_deg, 0.0);
        assert!(view.projection.x_axis.y.abs() < 1e-6);
        assert!(view.projection.y_axis.x.abs() < 1e-6);
        assert!(view.projection.x_axis.x > 0.0);
        assert!(view.projection.y_axis.y > 0.0);
        let center = view.projection * glam::Vec4::new(0.0, 0.0, -0.65, 1.0);
        assert!(center.x.abs() < 1e-5);
        assert!(center.y.abs() < 1e-5);
    }

    #[test]
    fn physical_screen_corners_map_to_viewport_edges() {
        let layout = triple();
        let eye = DVec3::new(3.0, -2.0, 1.5);
        for view in layout.views(eye, 17.0).unwrap() {
            let yaw = view.yaw_deg.to_radians();
            let right = Vec3::new(yaw.cos(), -yaw.sin(), 0.0);
            let normal = Vec3::new(yaw.sin(), yaw.cos(), 0.0);
            let width = layout.width_mm / 1000.0;
            let height = layout.height_mm / 1000.0;
            let corners = [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)];
            for (x_sign, y_sign) in corners {
                let corner = view.center - eye
                    + right.as_dvec3() * (f64::from(width) * 0.5 * f64::from(x_sign))
                    + DVec3::Z * (f64::from(height) * 0.5 * f64::from(y_sign));
                let local = Vec3::new(
                    corner.dot(right.as_dvec3()) as f32,
                    corner.z as f32,
                    -corner.dot(normal.as_dvec3()) as f32,
                );
                let ndc = view.projection.project_point3(local);
                assert!((ndc.x - x_sign).abs() < 1e-5);
                assert!((ndc.y - y_sign).abs() < 1e-5);
            }
        }
    }

    #[test]
    fn wider_bezel_gap_moves_outer_panels_apart() {
        let mut layout = triple();
        let no_gap = layout.views(DVec3::ZERO, 0.0).unwrap();
        layout.bezel_mm = 40.0;
        let gap = layout.views(DVec3::ZERO, 0.0).unwrap();
        assert!(gap[0].center.x < no_gap[0].center.x);
        assert!(gap[2].center.x > no_gap[2].center.x);
    }

    #[test]
    fn all_supported_monitor_counts_produce_valid_views() {
        let mut layout = triple();
        for count in 1..=9 {
            layout.count = count;
            assert_eq!(
                layout.views(DVec3::ZERO, 0.0).unwrap().len(),
                count as usize
            );
        }
    }

    #[test]
    fn rejects_invalid_physical_dimensions_and_angles() {
        let mut layout = triple();
        layout.width_mm = f32::NAN;
        assert!(layout.validate().is_err());
        layout.width_mm = 600.0;
        layout.left_angle_deg = 10.0;
        assert!(layout.validate().is_err());
    }
}
