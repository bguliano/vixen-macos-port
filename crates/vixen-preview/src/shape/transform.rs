//! Port of the rotation half of PreviewTools.cs and the base shape's `RotationAngle` setter.
//!
//! `TransformPreviewPoint` has a side effect in C#: while the rotate handle is the selected point it can
//! move the whole shape, so a shape resized since it was last rotated stays where it appears. That part
//! is `recenter_for_rotation`, and `transform_pure` is everything else, so a layout can run the side
//! effect once and then transform every pixel without a function call per pixel doing it again.

use super::{PointRef, Shape, ShapeKind};
use crate::point::{PointType, PreviewPoint};
use crate::tools::{get_angle, round, Matrix};

/// `PreviewTools.RotateTypes`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RotateTypes {
    Clockwise,
    Counterclockwise,
    Base0,
    Base180,
    None,
}

impl Shape {
    /// `shape._selectedPoint.PointType == RotateHandle`.
    #[inline]
    pub(crate) fn is_rotating(&self) -> bool {
        match self.selected_point {
            Some(PointRef::RotateHandle) => self.rotate_handle.is_some(),
            _ => false,
        }
    }

    fn rotation_for(&self, direction: RotateTypes) -> i32 {
        match direction {
            RotateTypes::Clockwise => self.rotation_angle,
            RotateTypes::Counterclockwise => self.rotation_angle.wrapping_neg(),
            RotateTypes::Base0 | RotateTypes::None => 0,
            RotateTypes::Base180 => 180,
        }
    }

    /// `PreviewTools.TransformPreviewPoint(shape, point, zoomLevel, direction)`.
    pub fn transform_preview_point(&mut self, point: PreviewPoint, zoom_level: f64, direction: RotateTypes) -> PreviewPoint {
        if self.is_rotating() {
            self.recenter_for_rotation();
        }
        self.transform_pure(point, zoom_level, direction)
    }

    /// The first half of the rotating branch: see where the center would land, and if the shape needs to
    /// move in the X-Y plane to keep it there, move it and put the rotation axis back on the center.
    fn recenter_for_rotation(&mut self) {
        let center = self.center();
        let new_center = self.rotate_about_axis_rounded(center);
        if new_center.x != center.x || new_center.y != center.y {
            let (left, top) = (self.left(), self.top());
            let width = self.right().wrapping_sub(left);
            let height = self.bottom().wrapping_sub(top);
            let dx = new_center.x.wrapping_sub(width / 2).wrapping_sub(left);
            let dy = new_center.y.wrapping_sub(height / 2).wrapping_sub(top);
            // Adjust all points to match the new center. `_selectPoints` is null here if the shape was
            // deselected since the last rotation, which throws in C#.
            if let Some(list) = self.select_points.clone() {
                for r in list {
                    self.translate_ref(r, dx, dy);
                }
            }
            //Move the rotation axis to the current center
            self.reset_rotation_axis();
        }
    }

    /// The inner `TransformPreviewPoint(shape, new PreviewPoint(shape.Center))`.
    fn rotate_about_axis_rounded(&self, p: PreviewPoint) -> PreviewPoint {
        if self.rotation_angle == 0 {
            return PreviewPoint::new(p.x, p.y);
        }
        match self.rotation_axis {
            Some(axis) => {
                let m = Matrix::rotate_at(self.rotation_angle as f64, axis.x as f64, axis.y as f64);
                let (x, y) = m.apply(p.x as f64, p.y as f64);
                PreviewPoint::new(round(x), round(y))
            }
            None => PreviewPoint::new(p.x, p.y),
        }
    }

    /// The rest of `TransformPreviewPoint`, with no side effect.
    pub(crate) fn transform_pure(&self, point: PreviewPoint, zoom_level: f64, direction: RotateTypes) -> PreviewPoint {
        // If zoomLevel < 0, then we really want to do the converse of the current zoom level
        let (zoom_to_physical, zoom_to_relative) = if zoom_level < 0.0 { (-1.0 / zoom_level, 1.0) } else { (1.0, zoom_level) };
        let rotation = self.rotation_for(direction);
        let (x, y) = if self.is_rotating() {
            let axis = self.rotation_axis.unwrap_or_default();
            let m = Matrix::rotate_at(rotation as f64, axis.x as f64, axis.y as f64);
            let (sx, sy) = m.apply(point.x as f64, point.y as f64);
            let f = if zoom_to_physical == 1.0 { zoom_to_relative } else { zoom_to_physical };
            (round(sx * f), round(sy * f))
        } else if rotation == 0 {
            let f = if zoom_to_physical == 1.0 { zoom_to_relative } else { zoom_to_physical };
            (round(point.x as f64 * f), round(point.y as f64 * f))
        } else if let Some(axis) = self.rotation_axis {
            // Convert the point's physical coordinates to its relative coordinates within the rotated system
            let m = Matrix::rotate_at(rotation as f64, axis.x as f64, axis.y as f64);
            let (sx, sy) = m.apply(point.x as f64 * zoom_to_physical, point.y as f64 * zoom_to_physical);
            (round(sx * zoom_to_relative), round(sy * zoom_to_relative))
        } else {
            // ArgumentNullException in C#: no rotation axis defined.
            (point.x, point.y)
        };
        PreviewPoint { x, y, point_type: point.point_type }
    }

    /// What `DrawSelectPoints` changes before a frame, so that reading the overlay changes nothing.
    pub(crate) fn prepare_overlay(&mut self) {
        self.sync_rotate_handle();
        if self.is_rotating() {
            self.recenter_for_rotation();
        }
    }

    /// `PreviewTools.TransformPreviewPoint(shape)`: reset the rotation axis.
    #[inline]
    pub(crate) fn reset_rotation_axis(&mut self) {
        if self.rotation_axis.is_some() {
            let c = self.center();
            if let Some(axis) = &mut self.rotation_axis {
                axis.x = c.x;
                axis.y = c.y;
            }
        }
    }

    /// `PreviewTools.CalculateRotation`.
    pub(crate) fn calculate_rotation(&mut self, base_point: (i32, i32), zoom_level: f64, use_detents: bool) -> i32 {
        // If the shape is inverted, all the calculations need to be rotated by 180 degrees.
        let top_left = self
            .select_points
            .as_deref()
            .unwrap_or(&[])
            .iter()
            .map(|r| self.point_of(*r))
            .find(|p| p.point_type == PointType::SizeTopLeft);
        let center = self.center();
        let target = match top_left {
            Some(tl) if tl.y > center.y => RotateTypes::Base180,
            _ => RotateTypes::Base0,
        };

        // Reset the rotational axis to the center of the object
        match &mut self.rotation_axis {
            Some(axis) => {
                axis.x = center.x;
                axis.y = center.y;
            }
            None => self.rotation_axis = Some(center.with_type(PointType::RotationAxis)),
        }

        // Calculate the rotational geometry
        let point = self.transform_preview_point(PreviewPoint::new(base_point.0, base_point.1), -zoom_level, target);
        let axis = self.rotation_axis.unwrap_or_default();
        let mut angle = get_angle((axis.x, axis.y), (point.x, point.y)) as f64;

        // Use detents of 0, 45, 90, 135, 180, 225, 270 and 315 when holding the Ctrl modifier key down.
        if use_detents {
            angle = detent(angle);
        }
        // Math.Round(angle, MidpointRounding.AwayFromZero)
        angle.round() as i32
    }

    /// `RotationAngle` setter.
    pub fn set_rotation_angle(&mut self, value: i32) {
        if !(-360..=360).contains(&value) {
            return;
        }
        // Check for a special case since Custom Props do their own rotation
        if matches!(self.kind, ShapeKind::CustomProp(_)) {
            self.rotation_angle = value;
            self.layout();
            return;
        }
        // Save off the currently selected point, if any
        let hold = self.selected_point;
        // We need to force the rotation axis to reset to the center of the object
        if self.rotation_axis.is_some() {
            self.selected_point = self.rotate_handle.map(|_| PointRef::RotateHandle);
            if self.is_rotating() {
                self.recenter_for_rotation();
            }
        }
        // Set the actual desired rotation angle
        self.rotation_angle = value;
        self.layout();
        // Reset the selected point back to the original, if temporarily changed above
        self.selected_point = hold;
    }

    /// `SetPixelZoomRotate`, over `_pixels`. The C# calls `TransformPreviewPoint` per pixel, with a rotation
    /// matrix built each time. This runs the side effect once and shares one matrix.
    pub(crate) fn set_pixel_zoom_rotate(&mut self) {
        if self.pixels.is_empty() {
            return;
        }
        let zoom = self.zoom();
        if self.is_rotating() {
            self.recenter_for_rotation();
        }
        let rotation = self.rotation_angle;
        if rotation == 0 {
            if zoom == 1.0 && !self.pixels[0].high_precision {
                return; // Math.Round of an integer times one.
            }
            for p in &mut self.pixels {
                let q = p.point();
                p.x = round(q.x as f64 * zoom);
                p.y = round(q.y as f64 * zoom);
            }
            return;
        }
        let Some(axis) = self.rotation_axis else {
            return; // ArgumentNullException in C#.
        };
        let m = Matrix::rotate_at(rotation as f64, axis.x as f64, axis.y as f64);
        for p in &mut self.pixels {
            let q = p.point();
            let (sx, sy) = m.apply(q.x as f64, q.y as f64);
            p.x = round(sx * zoom);
            p.y = round(sy * zoom);
        }
    }
}

/// The detents of `CalculateRotation`, also used by the Custom Prop.
pub(crate) fn detent(angle: f64) -> f64 {
    if (22.5..67.5).contains(&angle) {
        45.0
    } else if (67.5..112.5).contains(&angle) {
        90.0
    } else if (112.5..157.5).contains(&angle) {
        135.0
    } else if (157.5..202.5).contains(&angle) {
        180.0
    } else if (202.5..247.5).contains(&angle) {
        225.0
    } else if (247.5..292.5).contains(&angle) {
        270.0
    } else if (292.5..337.5).contains(&angle) {
        315.0
    } else if angle >= 337.5 || angle < 22.5 {
        0.0
    } else {
        angle
    }
}
