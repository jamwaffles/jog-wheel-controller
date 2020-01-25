#![no_std]
#![no_main]

use arrayvec::ArrayString;
use core::{fmt, fmt::Write};
use cortex_m_rt::ExceptionFrame;
use cortex_m_rt::{entry, exception};
use embedded_graphics::{
    pixelcolor::BinaryColor, prelude::*, primitives::Rectangle, text_6x12, text_8x16,
};
use embedded_hal::digital::v2::InputPin;
use panic_semihosting as _;
use ssd1306::prelude::*;
use ssd1306::Builder;
use stm32f1xx_hal::{
    gpio,
    i2c::{BlockingI2c, DutyCycle, Mode},
    prelude::*,
    stm32,
    timer::Timer,
};

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

#[derive(Copy, Clone, Debug)]
enum EmergencyStop {
    Enabled,
    Disabled,
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

    let clocks = rcc
        .cfgr
        .use_hse(8.mhz())
        .sysclk(72.mhz())
        .pclk1(36.mhz())
        .freeze(&mut flash.acr);

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

    // EStop is always trying to pull to ground, but in normal operation will be pulled up. A short
    // to ground (case) is more likely than short to +VE so this adds a bit more safety than pulling
    // high on estop.
    let estop: gpio::gpioa::PA4<gpio::Input<gpio::PullDown>> =
        gpioa.pa4.into_pull_down_input(&mut gpioa.crl);

    // TIM2
    let c1 = gpioa.pa8;
    let c2 = gpioa.pa9;

    let qei = Timer::tim1(dp.TIM1, &clocks, &mut rcc.apb2).qei((c1, c2), &mut afio.mapr);

    let mut mul_buf = ArrayString::<[_; 16]>::new();
    let mut axis_buf = ArrayString::<[_; 16]>::new();
    let mut qei_buf = ArrayString::<[_; 32]>::new();

    loop {
        mul_buf.clear();
        axis_buf.clear();
        qei_buf.clear();

        if let Some(mul) = mul_pins.multiplier() {
            write!(mul_buf, "Mul: {}   ", mul).unwrap();
        } else {
            write!(mul_buf, "Mul: Off  ").unwrap();
        }

        disp.draw(text_6x12!(
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
            text_6x12!(
                &axis_buf,
                stroke = Some(BinaryColor::On),
                fill = Some(BinaryColor::Off)
            )
            .translate(Point::new(0, 14)),
        );

        write!(qei_buf, "Jog: {:05}", qei.count()).expect("Fmt jog");

        disp.draw(
            text_6x12!(
                &qei_buf,
                stroke = Some(BinaryColor::On),
                fill = Some(BinaryColor::Off)
            )
            .translate(Point::new(0, 28)),
        );

        let (estop_fg, estop_bg) = if estop.is_low().unwrap() {
            (Some(BinaryColor::Off), Some(BinaryColor::On))
        } else {
            (Some(BinaryColor::On), Some(BinaryColor::Off))
        };

        let estop_text = text_8x16!(
            if estop.is_low().unwrap() {
                "MACHINE ESTOP"
            } else {
                "MACHINE ENABLE"
            },
            stroke = estop_fg,
            fill = estop_bg
        );

        let (w, h) = disp.get_dimensions();
        let estop_top = h as i32 - estop_text.size().height as i32;

        disp.draw(
            Rectangle::new(Point::new(0, estop_top), Point::new(w as i32, h as i32))
                .fill(estop_bg)
                .into_iter()
                .chain(estop_text.translate(Point::new(
                    ((w as u32 / 2) - (estop_text.size().width / 2)) as i32,
                    estop_top,
                ))),
        );

        disp.flush().unwrap();
    }
}

#[exception]
fn HardFault(ef: &ExceptionFrame) -> ! {
    panic!("{:#?}", ef);
}
