use core::fmt;
use embedded_hal::digital::v2::InputPin;

#[derive(Copy, Clone, Debug)]
enum SelectedAxis {
    X,
    Y,
    Z,
    A,
}

impl fmt::Display for SelectedAxis {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::X => 'X',
                Self::Y => 'Y',
                Self::Z => 'Z',
                Self::A => 'A',
            }
        )
    }
}

pub struct Axis<X, Y, Z, A> {
    x: X,
    y: Y,
    z: Z,
    a: A,
}

impl<X, Y, Z, A> Axis<X, Y, Z, A>
where
    X: InputPin,
    Y: InputPin,
    Z: InputPin,
    A: InputPin,
{
    pub fn new(x: X, y: Y, z: Z, a: A) -> Self {
        Self { x, y, z, a }
    }

    /// Get the currently selected axis (if any)
    ///
    /// If no axis is selected, `None` will be returned. This indicates that the axis selector
    /// switch is in the "Off" position, or that a connection to the pendant has been lost.
    pub fn axis(&self) -> Option<SelectedAxis> {
        if self.x.is_low().unwrap() {
            Some(SelectedAxis::X)
        } else if self.y.is_low().unwrap() {
            Some(SelectedAxis::Y)
        } else if self.z.is_low().unwrap() {
            Some(SelectedAxis::Z)
        } else if self.a.is_low().unwrap() {
            Some(SelectedAxis::A)
        } else {
            None
        }
    }
}
