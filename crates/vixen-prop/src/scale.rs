//! BackgroundImageScaleViewModel and calculator: dialog-only state until OK.
use crate::PropError;

#[derive(Clone, Debug)]
pub struct BackgroundScale {
    pub source: (i32, i32),
    pub current: (i32, i32),
    pub target: (i32, i32),
    pub percent: bool,
    pub locked: bool,
    pub has_lights: bool,
    pub scale_lights: bool,
    pub width: String,
    pub height: String,
    lock_base: (i32, i32),
    last_width: bool,
}
fn dimension(value: f64) -> Option<i32> {
    let rounded = value.round(); // Upstream explicitly requests AwayFromZero.
    (rounded.is_finite() && (1.0..=100_000.0).contains(&rounded)).then_some(rounded as i32)
}
impl BackgroundScale {
    pub fn new(source: (i32, i32), current: (i32, i32), has_lights: bool) -> Result<Self, PropError> {
        if [source.0,source.1,current.0,current.1].iter().any(|n| !(1..=100_000).contains(n)) { return Err(PropError::new("Scale Background Image", "Image and canvas dimensions must be 1 through 100,000 pixels.")) }
        Ok(Self { source, current, target: current, percent: false, locked: true, has_lights, scale_lights: has_lights, width: current.0.to_string(), height: current.1.to_string(), lock_base: current, last_width: true })
    }
    fn pixels(&self, width: bool) -> Option<i32> {
        let text = if width { &self.width } else { &self.height };
        let value = text.trim().parse::<f64>().ok()?;
        dimension(if self.percent { value * if width { self.source.0 } else { self.source.1 } as f64 / 100.0 } else { value })
    }
    fn display(&self, width: bool) -> String {
        let target = if width { self.target.0 } else { self.target.1 };
        if self.percent { format!("{}", target as f64 * 100.0 / if width { self.source.0 } else { self.source.1 } as f64) } else { target.to_string() }
    }
    pub fn set_percent(&mut self, percent: bool) {
        if self.percent == percent { return }
        self.percent = percent; self.width = self.display(true); self.height = self.display(false);
    }
    pub fn set_locked(&mut self, locked: bool) {
        if locked && !self.locked { self.lock_base = self.target; self.locked = true; self.update_target(); } else { self.locked = locked; }
    }
    pub fn set_input(&mut self, width: bool, text: String) {
        self.last_width = width;
        if width { self.width = text; } else { self.height = text; }
        self.update_target();
    }
    fn update_target(&mut self) {
        let Some(value) = self.pixels(self.last_width) else { return };
        if self.locked {
            let ratio = if self.last_width { self.lock_base.1 as f64 / self.lock_base.0 as f64 } else { self.lock_base.0 as f64 / self.lock_base.1 as f64 };
            let Some(other) = dimension(value as f64 * ratio) else { return };
            self.target = if self.last_width { (value, other) } else { (other,value) };
            if self.last_width { self.height = self.display(false); } else { self.width = self.display(true); }
        } else if self.last_width { self.target.0 = value; } else { self.target.1 = value; }
    }
    pub fn errors(&self) -> Vec<String> {
        let width = self.pixels(true); let height = self.pixels(false); let mut errors = Vec::new();
        if width.is_none() { errors.push("Enter a finite width that results in 1 through 100,000 pixels.".into()); }
        if height.is_none() { errors.push("Enter a finite height that results in 1 through 100,000 pixels.".into()); }
        if let (Some(w),Some(h)) = (width,height) {
            if self.locked {
                if self.last_width && dimension(w as f64 * self.lock_base.1 as f64 / self.lock_base.0 as f64).is_none() { errors.push("The aspect-ratio height must result in 1 through 100,000 pixels.".into()); }
                if !self.last_width && dimension(h as f64 * self.lock_base.0 as f64 / self.lock_base.1 as f64).is_none() { errors.push("The aspect-ratio width must result in 1 through 100,000 pixels.".into()); }
            }
        }
        errors
    }
}
#[cfg(test)] mod tests {
    use super::*;
    #[test] fn percentage_uses_bitmap_and_rounds_away_from_zero() {
        let mut s = BackgroundScale::new((200,100),(100,50),true).unwrap(); s.set_percent(true);
        assert_eq!((&*s.width,&*s.height), ("50","50"));
        s.set_input(true,"50.25".into()); assert_eq!(s.target,(101,51)); assert_eq!(s.height,"51");
        s.set_percent(false); assert_eq!((&*s.width,&*s.height), ("101","51"));
    }
    #[test] fn re_enabling_lock_captures_current_ratio_and_bad_values_disable_ok() {
        let mut s = BackgroundScale::new((200,100),(100,50),false).unwrap(); assert!(!s.scale_lights);
        s.set_locked(false); s.set_input(false,"100".into()); s.set_locked(true);
        s.set_input(true,"60".into()); assert_eq!(s.target,(60,60));
        s.set_input(true,"NaN".into()); assert_eq!(s.target,(60,60)); assert_eq!(s.errors().len(),1);
        s.set_percent(true); assert!(s.errors().is_empty()); // Unit change restores last valid target.
        s.set_locked(false); s.set_input(true,"0".into()); assert_eq!(s.errors().len(),1);
    }
    #[test] fn excessive_locked_dimension_has_its_own_message() {
        let mut s = BackgroundScale::new((1,100_000),(1,100_000),true).unwrap(); s.set_input(true,"2".into());
        assert_eq!(s.target,(1,100_000)); assert!(s.errors()[0].starts_with("The aspect-ratio height"));
    }
}
