//! Continuous spring motion coordinator for fluid notch transitions.
//!
//! Provides analytical/semi-implicit spring physics for smooth geometry transitions.
//! Target changes preserve instantaneous position and velocity (no abrupt resets).
//! When settled within tolerance, values snap cleanly to their targets and the driver
//! stops completely (0 wake at idle).

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpringScalar {
    pub current: f32,
    pub target: f32,
    pub velocity: f32,
    pub stiffness: f32, // k
    pub damping: f32,   // c
}

impl SpringScalar {
    pub const SETTLE_DISTANCE: f32 = 0.25; // DIP
    pub const SETTLE_VELOCITY: f32 = 2.0;  // DIP/s
    pub const MAX_SUB_STEP: f32 = 1.0 / 240.0;

    pub fn new(initial: f32, stiffness: f32, damping: f32) -> Self {
        Self {
            current: initial,
            target: initial,
            velocity: 0.0,
            stiffness,
            damping,
        }
    }

    #[inline]
    pub fn current(&self) -> f32 {
        self.current
    }

    #[inline]
    pub fn target(&self) -> f32 {
        self.target
    }

    #[inline]
    pub fn velocity(&self) -> f32 {
        self.velocity
    }

    #[inline]
    pub fn stiffness(&self) -> f32 {
        self.stiffness
    }

    #[inline]
    pub fn damping(&self) -> f32 {
        self.damping
    }

    /// Preserves current position and velocity!
    pub fn set_target(&mut self, new_target: f32) {
        self.target = new_target;
    }

    /// Snaps current and target to value, zeroing velocity.
    pub fn snap(&mut self, val: f32) {
        self.current = val;
        self.target = val;
        self.velocity = 0.0;
    }

    /// True when distance < 0.25 DIP and abs(velocity) < 2.0 DIP/s.
    #[inline]
    pub fn is_settled(&self) -> bool {
        (self.current - self.target).abs() < Self::SETTLE_DISTANCE
            && self.velocity.abs() < Self::SETTLE_VELOCITY
    }

    /// Advances the spring simulation by dt seconds using semi-implicit Euler
    /// with sub-stepping (maximum sub-step of 1/240 s).
    /// If settled, snaps to target with zero velocity.
    /// Returns true if still moving, false if settled.
    pub fn step(&mut self, dt: f32) -> bool {
        if dt <= 0.0 {
            return !self.is_settled();
        }

        // Cap total elapsed to 1.0s to avoid lag spikes after suspend/freeze
        let mut remaining = dt.min(1.0);
        let max_chunk = Self::MAX_SUB_STEP * 10.0; // ~41.7 ms chunk
        while remaining > 0.0 {
            let chunk = remaining.min(max_chunk);
            remaining -= chunk;

            let sub_steps = (chunk / Self::MAX_SUB_STEP).ceil().max(1.0) as usize;
            let sub_dt = chunk / (sub_steps as f32);

            for _ in 0..sub_steps {
                let displacement = self.current - self.target;
                let force = -self.stiffness * displacement - self.damping * self.velocity;
                self.velocity += force * sub_dt;
                self.current += self.velocity * sub_dt;
            }

            if self.is_settled() {
                self.current = self.target;
                self.velocity = 0.0;
                return false;
            }
        }

        !self.is_settled()
    }
}

/// Spring parameters according to specification:
/// Width & Height: k = 170.0, c = 22.0 (slightly underdamped for responsive fluid feel)
/// Radius & Ear:   k = 170.0, c = 26.1 (critically damped for stable contour)
pub const K_GEOMETRY: f32 = 170.0;
pub const C_GEOMETRY: f32 = 22.0;

pub const K_CRITICAL: f32 = 170.0;
pub const C_CRITICAL: f32 = 26.1;

/// Coordinates 4 independent spring channels: width, height, radius, and ear fillets.
#[derive(Debug, Clone, PartialEq)]
pub struct MotionCoordinator {
    pub width: SpringScalar,
    pub height: SpringScalar,
    pub radius: SpringScalar,
    pub ear: SpringScalar,
    reduced_motion: bool,
}

impl MotionCoordinator {
    pub fn new(w: f32, h: f32, r: f32, ear: f32) -> Self {
        Self {
            width: SpringScalar::new(w, K_GEOMETRY, C_GEOMETRY),
            height: SpringScalar::new(h, K_GEOMETRY, C_GEOMETRY),
            radius: SpringScalar::new(r, K_CRITICAL, C_CRITICAL),
            ear: SpringScalar::new(ear, K_CRITICAL, C_CRITICAL),
            reduced_motion: false,
        }
    }

    /// Retargets all 4 spring channels.
    /// If reduced motion is active, immediately snaps to targets.
    /// Otherwise, preserves instantaneous positions and velocities.
    pub fn retarget(&mut self, w: f32, h: f32, r: f32, ear: f32) {
        if self.reduced_motion {
            self.width.snap(w);
            self.height.snap(h);
            self.radius.snap(r);
            self.ear.snap(ear);
        } else {
            self.width.set_target(w);
            self.height.set_target(h);
            self.radius.set_target(r);
            self.ear.set_target(ear);
        }
    }

    /// Advances all spring channels by dt seconds.
    /// Returns true if any channel is still moving, false if all settled.
    pub fn step(&mut self, dt: f32) -> bool {
        if self.reduced_motion {
            return false;
        }
        let w_moving = self.width.step(dt);
        let h_moving = self.height.step(dt);
        let r_moving = self.radius.step(dt);
        let e_moving = self.ear.step(dt);
        w_moving || h_moving || r_moving || e_moving
    }

    /// True if all 4 channels are settled.
    #[inline]
    pub fn is_settled(&self) -> bool {
        self.width.is_settled()
            && self.height.is_settled()
            && self.radius.is_settled()
            && self.ear.is_settled()
    }

    pub fn set_reduced_motion(&mut self, reduced: bool) {
        self.reduced_motion = reduced;
        if reduced {
            self.width.snap(self.width.target);
            self.height.snap(self.height.target);
            self.radius.snap(self.radius.target);
            self.ear.snap(self.ear.target);
        }
    }

    #[inline]
    pub fn is_reduced_motion(&self) -> bool {
        self.reduced_motion
    }

    #[inline]
    pub fn current_values(&self) -> (f32, f32, f32, f32) {
        (
            self.width.current,
            self.height.current,
            self.radius.current,
            self.ear.current,
        )
    }

    #[inline]
    pub fn target_values(&self) -> (f32, f32, f32, f32) {
        (
            self.width.target,
            self.height.target,
            self.radius.target,
            self.ear.target,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spring_retarget_preserves_position_and_velocity() {
        let mut spring = SpringScalar::new(100.0, 170.0, 22.0);
        spring.set_target(200.0);
        spring.step(0.016);
        let p1 = spring.current();
        let v1 = spring.velocity();
        assert!(v1 > 0.0, "spring should have picked up positive velocity");
        assert!(p1 > 100.0, "spring should have moved forward");

        // Retargeting to 150.0 must preserve p1 and v1 exactly!
        spring.set_target(150.0);
        assert_eq!(spring.current(), p1, "position must be preserved across retarget");
        assert_eq!(spring.velocity(), v1, "velocity must be preserved across retarget");
    }

    #[test]
    fn test_spring_convergence_and_settle_timing() {
        // Test a large transition typical of Stage 0 (50 DIP) -> Stage 2 (460 DIP)
        let mut coordinator = MotionCoordinator::new(50.0, 10.0, 5.0, 6.0);
        coordinator.retarget(460.0, 180.0, 28.0, 12.0);

        let dt = 0.008; // 8ms ticks matching Windows multimedia timer
        let mut elapsed = 0.0;
        let mut settled = false;

        // Run simulation for up to 1.0 second
        while elapsed < 1.0 {
            let moving = coordinator.step(dt);
            elapsed += dt;
            if !moving && coordinator.is_settled() {
                settled = true;
                break;
            }
        }

        assert!(settled, "coordinator must settle within 1 second");
        // Spec target: settles and snaps within 500-700ms for large transitions
        assert!(
            elapsed >= 0.450 && elapsed <= 0.750,
            "expected settle time between 450ms and 750ms, got {:.3}s",
            elapsed
        );

        // Verify values have snapped exactly to targets
        let (w, h, r, ear) = coordinator.current_values();
        assert_eq!(w, 460.0);
        assert_eq!(h, 180.0);
        assert_eq!(r, 28.0);
        assert_eq!(ear, 12.0);
        assert_eq!(coordinator.width.velocity, 0.0);
        assert_eq!(coordinator.height.velocity, 0.0);
        assert_eq!(coordinator.radius.velocity, 0.0);
        assert_eq!(coordinator.ear.velocity, 0.0);
    }

    #[test]
    fn test_reduced_motion_immediate_snap() {
        let mut coordinator = MotionCoordinator::new(50.0, 10.0, 5.0, 6.0);
        coordinator.set_reduced_motion(true);
        assert!(coordinator.is_reduced_motion());

        // Retarget while reduced motion is enabled should snap immediately
        coordinator.retarget(300.0, 32.0, 16.0, 12.0);
        assert!(coordinator.is_settled());
        assert_eq!(coordinator.current_values(), (300.0, 32.0, 16.0, 12.0));
        assert_eq!(coordinator.width.velocity, 0.0);

        // step should return false immediately
        assert!(!coordinator.step(0.016));

        // When switching reduced motion ON mid-flight, it should snap immediately
        coordinator.set_reduced_motion(false);
        coordinator.retarget(500.0, 200.0, 24.0, 12.0);
        coordinator.step(0.016);
        assert!(!coordinator.is_settled());

        coordinator.set_reduced_motion(true);
        assert!(coordinator.is_settled());
        assert_eq!(coordinator.width.current, 500.0);
        assert_eq!(coordinator.width.velocity, 0.0);
    }

    #[test]
    fn test_zero_wake_and_is_settled_contract() {
        let mut coordinator = MotionCoordinator::new(100.0, 30.0, 15.0, 8.0);
        assert!(coordinator.is_settled());

        // Stepping a settled coordinator must return false (no wake needed)
        for _ in 0..10 {
            let moving = coordinator.step(0.016);
            assert!(!moving);
            assert!(coordinator.is_settled());
            assert_eq!(coordinator.width.velocity, 0.0);
        }
    }
}
