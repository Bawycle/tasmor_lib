// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Routine builder for executing multiple actions atomically.
//!
//! This module provides a fluent builder for creating routines - sequences of
//! device actions that execute without inter-action delays using Tasmota's
//! `Backlog0` functionality.
//!
//! # Overview
//!
//! Routines allow you to execute multiple Tasmota actions in a single network
//! request. This is useful for:
//!
//! - Atomic state changes (power + dimmer + color together)
//! - Timed sequences with explicit delays
//! - Reducing network round-trips for related actions
//!
//! # Limitations
//!
//! - Maximum 30 steps per routine (Tasmota hardware limit)
//! - Each delay counts as one step toward this limit
//! - Actions execute sequentially without inter-action delays unless
//!   explicitly added via [`RoutineBuilder::delay`]
//!
//! # Examples
//!
//! ## Basic routine
//!
//! ```
//! use tasmor_lib::command::Routine;
//! use tasmor_lib::types::{PowerIndex, Dimmer};
//!
//! let routine = Routine::builder()
//!     .power_on(PowerIndex::one())
//!     .set_dimmer(Dimmer::new(75).unwrap())
//!     .build()
//!     .unwrap();
//!
//! assert_eq!(routine.len(), 2);
//! ```
//!
//! ## Wake-up routine with delays
//!
//! ```
//! use tasmor_lib::command::Routine;
//! use tasmor_lib::types::{PowerIndex, Dimmer};
//! use std::time::Duration;
//!
//! let routine = Routine::builder()
//!     .power_on(PowerIndex::one())
//!     .set_dimmer(Dimmer::new(10).unwrap())
//!     .delay(Duration::from_secs(2))
//!     .set_dimmer(Dimmer::new(50).unwrap())
//!     .delay(Duration::from_secs(2))
//!     .set_dimmer(Dimmer::new(100).unwrap())
//!     .build()
//!     .unwrap();
//!
//! assert_eq!(routine.len(), 6); // 4 actions + 2 delays
//! ```

use std::time::Duration;

use crate::command::{
    ColorTemperatureCommand, Command, DimmerCommand, FadeCommand, FadeSpeedCommand,
    HsbColorCommand, PowerCommand, SchemeCommand, StartupFadeCommand, WakeupDurationCommand,
};
use crate::error::{DeviceError, Error};
use crate::types::{
    ColorTemperature, Dimmer, FadeSpeed, HsbColor, PowerIndex, PowerState, RgbColor, Scheme,
    WakeupDuration,
};

/// Maximum number of steps allowed in a routine.
///
/// This is a Tasmota limitation for the Backlog command.
pub const MAX_ROUTINE_STEPS: usize = 30;

/// A validated routine of actions to execute atomically.
///
/// Routines are constructed using [`RoutineBuilder`] and executed via
/// [`Device::run`](crate::Device::run).
///
/// The routine is serialized to Tasmota's `Backlog0` format, which executes
/// actions sequentially without inter-action delays (unless explicit delays
/// are added).
///
/// # Examples
///
/// ```
/// use tasmor_lib::command::Routine;
/// use tasmor_lib::types::PowerIndex;
///
/// let routine = Routine::builder()
///     .power_on(PowerIndex::one())
///     .build()
///     .unwrap();
///
/// assert!(!routine.is_empty());
/// ```
#[derive(Debug, Clone)]
pub struct Routine {
    /// Raw command strings that will be joined with semicolons.
    steps: Vec<String>,
}

impl Routine {
    /// Creates a new routine builder.
    ///
    /// # Examples
    ///
    /// ```
    /// use tasmor_lib::command::Routine;
    ///
    /// let builder = Routine::builder();
    /// assert!(builder.is_empty());
    /// ```
    #[must_use]
    pub fn builder() -> RoutineBuilder {
        RoutineBuilder::new()
    }

    /// Returns the number of steps in the routine.
    ///
    /// This includes both actions and delays.
    #[must_use]
    pub fn len(&self) -> usize {
        self.steps.len()
    }

    /// Returns `true` if the routine contains no steps.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// Serializes the routine to Tasmota's `Backlog0` format.
    ///
    /// Format: `Backlog0 <action1>; <action2>; <action3>`
    #[must_use]
    pub(crate) fn to_backlog_command(&self) -> String {
        format!("Backlog0 {}", self.steps.join("; "))
    }
}

/// Builder for constructing routines.
///
/// Provides a fluent API for adding actions and delays to a routine.
/// The routine is validated when [`build`](Self::build) is called.
///
/// # Examples
///
/// ```
/// use tasmor_lib::command::Routine;
/// use tasmor_lib::types::{PowerIndex, Dimmer, ColorTemperature};
/// use std::time::Duration;
///
/// let routine = Routine::builder()
///     .power_on(PowerIndex::one())
///     .set_dimmer(Dimmer::new(75).unwrap())
///     .set_color_temperature(ColorTemperature::WARM)
///     .delay(Duration::from_millis(500))
///     .enable_fade()
///     .build()
///     .unwrap();
/// ```
#[derive(Debug, Clone, Default)]
pub struct RoutineBuilder {
    steps: Vec<String>,
}

impl RoutineBuilder {
    /// Creates a new empty builder.
    #[must_use]
    pub fn new() -> Self {
        Self { steps: Vec::new() }
    }

    // ========== Power Control ==========

    /// Turns on the relay at the specified index.
    ///
    /// # Examples
    ///
    /// ```
    /// use tasmor_lib::command::Routine;
    /// use tasmor_lib::types::PowerIndex;
    ///
    /// let routine = Routine::builder()
    ///     .power_on(PowerIndex::one())
    ///     .build()
    ///     .unwrap();
    /// ```
    #[must_use]
    pub fn power_on(self, index: PowerIndex) -> Self {
        self.add_command(&PowerCommand::on(index))
    }

    /// Turns off the relay at the specified index.
    ///
    /// # Examples
    ///
    /// ```
    /// use tasmor_lib::command::Routine;
    /// use tasmor_lib::types::PowerIndex;
    ///
    /// let routine = Routine::builder()
    ///     .power_off(PowerIndex::one())
    ///     .build()
    ///     .unwrap();
    /// ```
    #[must_use]
    pub fn power_off(self, index: PowerIndex) -> Self {
        self.add_command(&PowerCommand::off(index))
    }

    /// Toggles the relay at the specified index.
    ///
    /// # Examples
    ///
    /// ```
    /// use tasmor_lib::command::Routine;
    /// use tasmor_lib::types::PowerIndex;
    ///
    /// let routine = Routine::builder()
    ///     .power_toggle(PowerIndex::one())
    ///     .build()
    ///     .unwrap();
    /// ```
    #[must_use]
    pub fn power_toggle(self, index: PowerIndex) -> Self {
        self.add_command(&PowerCommand::toggle(index))
    }

    /// Sets the relay at the specified index to the given state.
    ///
    /// # Examples
    ///
    /// ```
    /// use tasmor_lib::command::Routine;
    /// use tasmor_lib::types::{PowerIndex, PowerState};
    ///
    /// let routine = Routine::builder()
    ///     .set_power(PowerIndex::one(), PowerState::On)
    ///     .build()
    ///     .unwrap();
    /// ```
    #[must_use]
    pub fn set_power(self, index: PowerIndex, state: PowerState) -> Self {
        self.add_command(&PowerCommand::Set { index, state })
    }

    // ========== Dimmer Control ==========

    /// Sets the dimmer brightness level.
    ///
    /// # Examples
    ///
    /// ```
    /// use tasmor_lib::command::Routine;
    /// use tasmor_lib::types::Dimmer;
    ///
    /// let routine = Routine::builder()
    ///     .set_dimmer(Dimmer::new(75).unwrap())
    ///     .build()
    ///     .unwrap();
    /// ```
    #[must_use]
    pub fn set_dimmer(self, value: Dimmer) -> Self {
        self.add_command(&DimmerCommand::Set(value))
    }

    // ========== Color Temperature Control ==========

    /// Sets the color temperature for white/CCT lights.
    ///
    /// # Examples
    ///
    /// ```
    /// use tasmor_lib::command::Routine;
    /// use tasmor_lib::types::ColorTemperature;
    ///
    /// let routine = Routine::builder()
    ///     .set_color_temperature(ColorTemperature::WARM)
    ///     .build()
    ///     .unwrap();
    /// ```
    #[must_use]
    pub fn set_color_temperature(self, ct: ColorTemperature) -> Self {
        self.add_command(&ColorTemperatureCommand::Set(ct))
    }

    // ========== Color Control ==========

    /// Sets the light color using HSB (Hue, Saturation, Brightness) values.
    ///
    /// # Examples
    ///
    /// ```
    /// use tasmor_lib::command::Routine;
    /// use tasmor_lib::types::HsbColor;
    ///
    /// let routine = Routine::builder()
    ///     .set_hsb_color(HsbColor::red())
    ///     .build()
    ///     .unwrap();
    /// ```
    #[must_use]
    pub fn set_hsb_color(self, color: HsbColor) -> Self {
        self.add_command(&HsbColorCommand::Set(color))
    }

    /// Sets the light color using RGB values.
    ///
    /// The RGB color is converted to HSB format for Tasmota.
    ///
    /// # Examples
    ///
    /// ```
    /// use tasmor_lib::command::Routine;
    /// use tasmor_lib::types::RgbColor;
    ///
    /// let routine = Routine::builder()
    ///     .set_rgb_color(RgbColor::new(255, 128, 0))
    ///     .build()
    ///     .unwrap();
    /// ```
    #[must_use]
    pub fn set_rgb_color(self, color: RgbColor) -> Self {
        self.add_command(&HsbColorCommand::Set(color.to_hsb()))
    }

    // ========== Scheme Control ==========

    /// Sets the light effect scheme.
    ///
    /// # Examples
    ///
    /// ```
    /// use tasmor_lib::command::Routine;
    /// use tasmor_lib::types::Scheme;
    ///
    /// let routine = Routine::builder()
    ///     .set_scheme(Scheme::CYCLE_UP)
    ///     .build()
    ///     .unwrap();
    /// ```
    #[must_use]
    pub fn set_scheme(self, scheme: Scheme) -> Self {
        self.add_command(&SchemeCommand::Set(scheme))
    }

    /// Sets the wakeup duration for the wakeup scheme.
    ///
    /// # Examples
    ///
    /// ```
    /// use tasmor_lib::command::Routine;
    /// use tasmor_lib::types::WakeupDuration;
    ///
    /// let routine = Routine::builder()
    ///     .set_wakeup_duration(WakeupDuration::from_minutes(5).unwrap())
    ///     .build()
    ///     .unwrap();
    /// ```
    #[must_use]
    pub fn set_wakeup_duration(self, duration: WakeupDuration) -> Self {
        self.add_command(&WakeupDurationCommand::Set(duration))
    }

    // ========== Fade Control ==========

    /// Enables fade transitions between brightness/color changes.
    ///
    /// # Examples
    ///
    /// ```
    /// use tasmor_lib::command::Routine;
    ///
    /// let routine = Routine::builder()
    ///     .enable_fade()
    ///     .build()
    ///     .unwrap();
    /// ```
    #[must_use]
    pub fn enable_fade(self) -> Self {
        self.add_command(&FadeCommand::Enable)
    }

    /// Disables fade transitions.
    ///
    /// # Examples
    ///
    /// ```
    /// use tasmor_lib::command::Routine;
    ///
    /// let routine = Routine::builder()
    ///     .disable_fade()
    ///     .build()
    ///     .unwrap();
    /// ```
    #[must_use]
    pub fn disable_fade(self) -> Self {
        self.add_command(&FadeCommand::Disable)
    }

    /// Sets the fade transition speed.
    ///
    /// # Examples
    ///
    /// ```
    /// use tasmor_lib::command::Routine;
    /// use tasmor_lib::types::FadeSpeed;
    ///
    /// let routine = Routine::builder()
    ///     .set_fade_speed(FadeSpeed::SLOW)
    ///     .build()
    ///     .unwrap();
    /// ```
    #[must_use]
    pub fn set_fade_speed(self, speed: FadeSpeed) -> Self {
        self.add_command(&FadeSpeedCommand::Set(speed))
    }

    /// Enables fade effect at device startup.
    ///
    /// # Examples
    ///
    /// ```
    /// use tasmor_lib::command::Routine;
    ///
    /// let routine = Routine::builder()
    ///     .enable_fade_at_startup()
    ///     .build()
    ///     .unwrap();
    /// ```
    #[must_use]
    pub fn enable_fade_at_startup(self) -> Self {
        self.add_command(&StartupFadeCommand::Enable)
    }

    /// Disables fade effect at device startup.
    ///
    /// # Examples
    ///
    /// ```
    /// use tasmor_lib::command::Routine;
    ///
    /// let routine = Routine::builder()
    ///     .disable_fade_at_startup()
    ///     .build()
    ///     .unwrap();
    /// ```
    #[must_use]
    pub fn disable_fade_at_startup(self) -> Self {
        self.add_command(&StartupFadeCommand::Disable)
    }

    // ========== Timing ==========

    /// Adds a delay to the routine.
    ///
    /// Delays are implemented using Tasmota's `Delay` command and count
    /// toward the 30-step limit. The duration is converted to deciseconds
    /// (1 decisecond = 100ms).
    ///
    /// # Arguments
    ///
    /// * `duration` - The delay duration. Converted to deciseconds and
    ///   clamped to the valid range of 1-65535 deciseconds
    ///   (100ms to ~109 minutes).
    ///
    /// # Examples
    ///
    /// ```
    /// use tasmor_lib::command::Routine;
    /// use tasmor_lib::types::PowerIndex;
    /// use std::time::Duration;
    ///
    /// let routine = Routine::builder()
    ///     .power_on(PowerIndex::one())
    ///     .delay(Duration::from_secs(2))
    ///     .power_off(PowerIndex::one())
    ///     .build()
    ///     .unwrap();
    /// ```
    #[must_use]
    pub fn delay(mut self, duration: Duration) -> Self {
        // Tasmota Delay command uses deciseconds (0.1s = 100ms)
        // Range: 1-65535 deciseconds
        let deciseconds = (duration.as_millis() / 100).clamp(1, 65535);
        self.steps.push(format!("Delay {deciseconds}"));
        self
    }

    // ========== Build ==========

    /// Builds the routine.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The routine is empty
    /// - The routine exceeds the maximum of 30 steps
    ///
    /// # Examples
    ///
    /// ```
    /// use tasmor_lib::command::Routine;
    /// use tasmor_lib::types::PowerIndex;
    ///
    /// // Valid routine
    /// let result = Routine::builder()
    ///     .power_on(PowerIndex::one())
    ///     .build();
    /// assert!(result.is_ok());
    ///
    /// // Empty routine fails
    /// let result = Routine::builder().build();
    /// assert!(result.is_err());
    /// ```
    pub fn build(self) -> Result<Routine, Error> {
        if self.steps.is_empty() {
            return Err(Error::Device(DeviceError::InvalidConfiguration(
                "routine cannot be empty".to_string(),
            )));
        }

        if self.steps.len() > MAX_ROUTINE_STEPS {
            return Err(Error::Device(DeviceError::InvalidConfiguration(format!(
                "routine exceeds maximum of {MAX_ROUTINE_STEPS} steps (got {})",
                self.steps.len()
            ))));
        }

        Ok(Routine { steps: self.steps })
    }

    /// Returns the current number of steps in the builder.
    #[must_use]
    pub fn len(&self) -> usize {
        self.steps.len()
    }

    /// Returns `true` if no steps have been added.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// Returns the remaining capacity before hitting the 30-step limit.
    #[must_use]
    pub fn remaining_capacity(&self) -> usize {
        MAX_ROUTINE_STEPS.saturating_sub(self.steps.len())
    }

    // ========== Internal ==========

    /// Adds a command to the routine (internal helper).
    fn add_command<C: Command>(mut self, cmd: &C) -> Self {
        self.steps.push(cmd.to_http_command());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_routine_fails() {
        let result = Routine::builder().build();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            Error::Device(DeviceError::InvalidConfiguration(_))
        ));
    }

    #[test]
    fn single_action_routine() {
        let routine = Routine::builder()
            .power_on(PowerIndex::one())
            .build()
            .unwrap();

        assert_eq!(routine.len(), 1);
        assert!(!routine.is_empty());
        assert_eq!(routine.to_backlog_command(), "Backlog0 Power1 ON");
    }

    #[test]
    fn multiple_actions_routine() {
        let routine = Routine::builder()
            .power_on(PowerIndex::one())
            .set_dimmer(Dimmer::new(75).unwrap())
            .build()
            .unwrap();

        assert_eq!(routine.len(), 2);
        assert_eq!(
            routine.to_backlog_command(),
            "Backlog0 Power1 ON; Dimmer 75"
        );
    }

    #[test]
    fn routine_with_delay() {
        let routine = Routine::builder()
            .power_on(PowerIndex::one())
            .delay(Duration::from_millis(500))
            .power_off(PowerIndex::one())
            .build()
            .unwrap();

        assert_eq!(routine.len(), 3);
        assert_eq!(
            routine.to_backlog_command(),
            "Backlog0 Power1 ON; Delay 5; Power1 OFF"
        );
    }

    #[test]
    fn delay_clamped_to_minimum() {
        let routine = Routine::builder()
            .delay(Duration::from_millis(50))
            .build()
            .unwrap();

        assert!(routine.to_backlog_command().contains("Delay 1"));
    }

    #[test]
    fn delay_clamped_to_maximum() {
        let routine = Routine::builder()
            .delay(Duration::from_secs(7000))
            .build()
            .unwrap();

        assert!(routine.to_backlog_command().contains("Delay 65535"));
    }

    #[test]
    fn power_control_methods() {
        let routine = Routine::builder()
            .power_on(PowerIndex::one())
            .power_off(PowerIndex::new(2).unwrap())
            .power_toggle(PowerIndex::new(3).unwrap())
            .set_power(PowerIndex::new(4).unwrap(), PowerState::On)
            .build()
            .unwrap();

        assert_eq!(routine.len(), 4);
        let cmd = routine.to_backlog_command();
        assert!(cmd.contains("Power1 ON"));
        assert!(cmd.contains("Power2 OFF"));
        assert!(cmd.contains("Power3 TOGGLE"));
        assert!(cmd.contains("Power4 ON"));
    }

    #[test]
    fn dimmer_control() {
        let routine = Routine::builder()
            .set_dimmer(Dimmer::new(50).unwrap())
            .build()
            .unwrap();

        assert!(routine.to_backlog_command().contains("Dimmer 50"));
    }

    #[test]
    fn color_temperature_control() {
        let routine = Routine::builder()
            .set_color_temperature(ColorTemperature::WARM)
            .build()
            .unwrap();

        assert!(routine.to_backlog_command().contains("CT 370"));
    }

    #[test]
    fn hsb_color_control() {
        let routine = Routine::builder()
            .set_hsb_color(HsbColor::red())
            .build()
            .unwrap();

        assert!(routine.to_backlog_command().contains("HSBColor 0,100,100"));
    }

    #[test]
    fn rgb_color_control() {
        let routine = Routine::builder()
            .set_rgb_color(RgbColor::new(255, 0, 0))
            .build()
            .unwrap();

        // RGB is converted to HSB
        assert!(routine.to_backlog_command().contains("HSBColor"));
    }

    #[test]
    fn scheme_control() {
        let routine = Routine::builder()
            .set_scheme(Scheme::CYCLE_UP)
            .build()
            .unwrap();

        assert!(routine.to_backlog_command().contains("Scheme 2"));
    }

    #[test]
    fn wakeup_duration_control() {
        let routine = Routine::builder()
            .set_wakeup_duration(WakeupDuration::new(300).unwrap())
            .build()
            .unwrap();

        assert!(routine.to_backlog_command().contains("WakeupDuration 300"));
    }

    #[test]
    fn fade_control() {
        let routine = Routine::builder()
            .enable_fade()
            .set_fade_speed(FadeSpeed::SLOW)
            .build()
            .unwrap();

        let cmd = routine.to_backlog_command();
        assert!(cmd.contains("Fade 1"));
        assert!(cmd.contains("Speed 40"));
    }

    #[test]
    fn fade_at_startup_control() {
        let routine = Routine::builder().enable_fade_at_startup().build().unwrap();

        assert!(routine.to_backlog_command().contains("SetOption91 1"));
    }

    #[test]
    fn routine_at_max_capacity() {
        let mut builder = Routine::builder();
        for _ in 0..MAX_ROUTINE_STEPS {
            builder = builder.power_toggle(PowerIndex::one());
        }

        assert_eq!(builder.remaining_capacity(), 0);

        let result = builder.build();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), MAX_ROUTINE_STEPS);
    }

    #[test]
    fn routine_exceeds_max_capacity() {
        let mut builder = Routine::builder();
        for _ in 0..=MAX_ROUTINE_STEPS {
            builder = builder.power_toggle(PowerIndex::one());
        }

        let result = builder.build();
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(matches!(
            err,
            Error::Device(DeviceError::InvalidConfiguration(msg)) if msg.contains("exceeds maximum")
        ));
    }

    #[test]
    fn builder_remaining_capacity() {
        let builder = Routine::builder()
            .power_on(PowerIndex::one())
            .power_off(PowerIndex::one());

        assert_eq!(builder.remaining_capacity(), MAX_ROUTINE_STEPS - 2);
    }

    #[test]
    fn routine_is_cloneable() {
        let routine = Routine::builder()
            .power_on(PowerIndex::one())
            .build()
            .unwrap();

        let cloned = routine.clone();
        assert_eq!(routine.len(), cloned.len());
        assert_eq!(routine.to_backlog_command(), cloned.to_backlog_command());
    }

    #[test]
    fn builder_is_cloneable() {
        let builder = Routine::builder().power_on(PowerIndex::one());

        let cloned = builder.clone();
        assert_eq!(builder.len(), cloned.len());
    }

    #[test]
    fn complex_wakeup_routine() {
        let routine = Routine::builder()
            .power_on(PowerIndex::one())
            .enable_fade()
            .set_fade_speed(FadeSpeed::SLOW)
            .set_dimmer(Dimmer::new(10).unwrap())
            .set_color_temperature(ColorTemperature::WARM)
            .delay(Duration::from_secs(60))
            .set_dimmer(Dimmer::new(50).unwrap())
            .delay(Duration::from_secs(60))
            .set_dimmer(Dimmer::new(100).unwrap())
            .set_color_temperature(ColorTemperature::NEUTRAL)
            .build()
            .unwrap();

        assert_eq!(routine.len(), 10);
    }
}
