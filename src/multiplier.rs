use core::fmt;
use embedded_hal::digital::v2::InputPin;
use stm32f1xx_hal::gpio::*;

#[derive(Copy, Clone, Debug)]
pub enum SelectedMultiplier {
    X1 = 1,
    X10 = 10,
    X100 = 100,
}

impl fmt::Display for SelectedMultiplier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "X{}", *self as u8)
    }
}

pub struct MultiplierPins {
    pin_x1: gpioa::PA5<Input<PullUp>>,
    pin_x10: gpioa::PA6<Input<PullUp>>,
    pin_x100: gpioa::PA7<Input<PullUp>>,
}

impl MultiplierPins {
    pub fn new(
        pin_x1: gpioa::PA5<Input<PullUp>>,
        pin_x10: gpioa::PA6<Input<PullUp>>,
        pin_x100: gpioa::PA7<Input<PullUp>>,
    ) -> Self {
        Self {
            pin_x1,
            pin_x10,
            pin_x100,
        }
    }

    /// Return the currently selected multiplier
    ///
    /// If no multiplier is selected (no pin is high), `None` will be returned.
    pub fn selection(&self) -> Option<SelectedMultiplier> {
        if self.pin_x1.is_low().unwrap() {
            Some(SelectedMultiplier::X1)
        } else if self.pin_x10.is_low().unwrap() {
            Some(SelectedMultiplier::X10)
        } else if self.pin_x100.is_low().unwrap() {
            Some(SelectedMultiplier::X100)
        } else {
            None
        }
    }
}
