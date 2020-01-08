#![no_std]
#![no_main]

use arrayvec::ArrayString;
use core::{fmt, fmt::Write};
use cortex_m_rt::ExceptionFrame;
use cortex_m_rt::{entry, exception};
use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{Circle, Rectangle, Triangle},
    text_6x8,
};
use embedded_hal::digital::v2::InputPin;
use panic_semihosting as _;
use ssd1306::prelude::*;
use ssd1306::Builder;
use stm32f1xx_hal::gpio;
use stm32f1xx_hal::i2c::{BlockingI2c, DutyCycle, Mode};
use stm32f1xx_hal::prelude::*;
use stm32f1xx_hal::stm32;

struct MulPins {
    x1: gpio::gpioa::PA5<gpio::Input<gpio::PullUp>>,
    x10: gpio::gpioa::PA6<gpio::Input<gpio::PullUp>>,
    x100: gpio::gpioa::PA7<gpio::Input<gpio::PullUp>>,
}

impl MulPins {
    pub fn multiplier(&self) -> Option<Multiplier> {
        if self.x1.is_low().unwrap() {
            Some(Multiplier::X1)
        } else if self.x10.is_low().unwrap() {
            Some(Multiplier::X10)
        } else if self.x100.is_low().unwrap() {
            Some(Multiplier::X100)
        } else {
            None
        }
    }
}

#[derive(Copy, Clone, Debug)]
enum Multiplier {
    X1 = 1,
    X10 = 10,
    X100 = 100,
}

impl fmt::Display for Multiplier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "X{}", *self as u8)
    }
}

struct AxisPins {
    x: gpio::gpioa::PA0<gpio::Input<gpio::PullUp>>,
    y: gpio::gpioa::PA1<gpio::Input<gpio::PullUp>>,
    z: gpio::gpioa::PA2<gpio::Input<gpio::PullUp>>,
    a: gpio::gpioa::PA3<gpio::Input<gpio::PullUp>>,
}

impl AxisPins {
    pub fn axis(&self) -> Option<Axis> {
        if self.x.is_low().unwrap() {
            Some(Axis::X)
        } else if self.y.is_low().unwrap() {
            Some(Axis::Y)
        } else if self.z.is_low().unwrap() {
            Some(Axis::Z)
        } else if self.a.is_low().unwrap() {
            Some(Axis::A)
        } else {
            None
        }
    }
}

#[derive(Copy, Clone, Debug)]
enum Axis {
    X,
    Y,
    Z,
    A,
}

impl fmt::Display for Axis {
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

#[entry]
fn main() -> ! {
    let dp = stm32::Peripherals::take().unwrap();

    let mut flash = dp.FLASH.constrain();
    let mut rcc = dp.RCC.constrain();

    let clocks = rcc.cfgr.freeze(&mut flash.acr);

    let mut afio = dp.AFIO.constrain(&mut rcc.apb2);

    let mut gpiob = dp.GPIOB.split(&mut rcc.apb2);
    let mut gpioa = dp.GPIOA.split(&mut rcc.apb2);

    let scl = gpiob.pb10.into_alternate_open_drain(&mut gpiob.crh);
    let sda = gpiob.pb11.into_alternate_open_drain(&mut gpiob.crh);

    let i2c = BlockingI2c::i2c2(
        dp.I2C2,
        (scl, sda),
        Mode::Fast {
            frequency: 400_000.hz(),
            duty_cycle: DutyCycle::Ratio2to1,
        },
        clocks,
        &mut rcc.apb1,
        1000,
        10,
        1000,
        1000,
    );

    let mut disp: GraphicsMode<_> = Builder::new()
        .with_rotation(DisplayRotation::Rotate180)
        .connect_i2c(i2c)
        .into();

    disp.init().unwrap();
    disp.flush().unwrap();

    let mul_pins = MulPins {
        x1: gpioa.pa5.into_pull_up_input(&mut gpioa.crl),
        x10: gpioa.pa6.into_pull_up_input(&mut gpioa.crl),
        x100: gpioa.pa7.into_pull_up_input(&mut gpioa.crl),
    };

    let axis_pins = AxisPins {
        x: gpioa.pa0.into_pull_up_input(&mut gpioa.crl),
        y: gpioa.pa1.into_pull_up_input(&mut gpioa.crl),
        z: gpioa.pa2.into_pull_up_input(&mut gpioa.crl),
        a: gpioa.pa3.into_pull_up_input(&mut gpioa.crl),
    };

    let mut mul_buf = ArrayString::<[_; 16]>::new();
    let mut axis_buf = ArrayString::<[_; 16]>::new();

    loop {
        mul_buf.clear();
        axis_buf.clear();

        if let Some(mul) = mul_pins.multiplier() {
            write!(mul_buf, "Mul: {}   ", mul).unwrap();
        } else {
            write!(mul_buf, "Mul: Off  ").unwrap();
        }

        disp.draw(text_6x8!(
            &mul_buf,
            stroke = Some(BinaryColor::On),
            fill = Some(BinaryColor::Off)
        ));

        if let Some(axis) = axis_pins.axis() {
            write!(axis_buf, "Axis: {}  ", axis).unwrap();
        } else {
            write!(axis_buf, "Axis: Off").unwrap();
        }

        disp.draw(
            text_6x8!(
                &axis_buf,
                stroke = Some(BinaryColor::On),
                fill = Some(BinaryColor::Off)
            )
            .translate(Point::new(0, 8)),
        );

        disp.flush().unwrap();
    }
}

#[exception]
fn HardFault(ef: &ExceptionFrame) -> ! {
    panic!("{:#?}", ef);
}
