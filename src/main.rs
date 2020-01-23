#![no_std]
#![no_main]

use embedded_graphics::{pixelcolor::BinaryColor, prelude::*, primitives::Rectangle, text_6x8};
use embedded_hal::digital::v2::OutputPin;
use panic_semihosting as _;
use rtfm::app;
use ssd1306::prelude::*;
use ssd1306::Builder;
use stm32f1xx_hal::{
    device, gpio,
    gpio::{gpioc::PC13, Edge, ExtiPin, Output, PushPull, State},
    i2c::{BlockingI2c, DutyCycle, Mode},
    pac::{self, I2C2},
    prelude::*,
    stm32,
    timer::{CountDownTimer, Event, Timer},
};

type Display = ssd1306::mode::graphics::GraphicsMode<
    ssd1306::interface::i2c::I2cInterface<
        stm32f1xx_hal::i2c::BlockingI2c<
            I2C2,
            (
                gpio::gpiob::PB10<gpio::Alternate<gpio::OpenDrain>>,
                gpio::gpiob::PB11<gpio::Alternate<gpio::OpenDrain>>,
            ),
        >,
    >,
>;

#[app(device = stm32f1xx_hal::pac, peripherals = true)]
const APP: () = {
    struct Resources {
        display: Display,
        // x1: gpio::gpioa::PA5<gpio::Input<gpio::PullUp>>,
        // x10: gpio::gpioa::PA6<gpio::Input<gpio::PullUp>>,
        x100: gpio::gpioa::PA7<gpio::Input<gpio::PullUp>>,
    }

    #[init]
    fn init(cx: init::Context) -> init::LateResources {
        // let cp = cortex_m::peripheral::Peripherals::take().unwrap();
        let dp = cx.device;

        let mut flash = dp.FLASH.constrain();
        let mut rcc = dp.RCC.constrain();

        let clocks = rcc
            .cfgr
            .use_hse(8.mhz())
            .sysclk(72.mhz())
            .pclk1(36.mhz())
            .freeze(&mut flash.acr);

        let mut afio = dp.AFIO.constrain(&mut rcc.apb2);

        let mut gpioc = dp.GPIOC.split(&mut rcc.apb2);
        let mut gpiob = dp.GPIOB.split(&mut rcc.apb2);
        let mut gpioa = dp.GPIOA.split(&mut rcc.apb2);

        let led = gpioc.pc13.into_push_pull_output(&mut gpioc.crh);

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

        disp.draw(text_6x8!(
            "Welcome",
            stroke = Some(BinaryColor::On),
            fill = Some(BinaryColor::Off)
        ));

        disp.flush().unwrap();

        // let mut x1 = gpioa.pa5.into_pull_up_input(&mut gpioa.crl);
        // let x10 = gpioa.pa6.into_pull_up_input(&mut gpioa.crl);
        let mut x100 = gpioa.pa7.into_pull_up_input(&mut gpioa.crl);

        x100.make_interrupt_source(&mut afio);
        x100.trigger_on_edge(&dp.EXTI, Edge::FALLING);
        x100.enable_interrupt(&dp.EXTI);

        // Init the static resources to use them later through RTFM
        init::LateResources {
            display: disp,
            // x1,
            // x10,
            x100,
        }
    }

    #[idle]
    fn idle(_cx: idle::Context) -> ! {
        loop {
            cortex_m::asm::wfi();
        }
    }

    #[task(binds = EXTI9_5, resources = [display, x100])]
    fn axis(cx: axis::Context) {
        if cx.resources.x100.check_interrupt() {
            cx.resources.display.draw(text_6x8!(
                "Interrupted",
                stroke = Some(BinaryColor::On),
                fill = Some(BinaryColor::Off)
            ));

            cx.resources.display.flush().unwrap();

            // if we don't clear this bit, the ISR would trigger indefinitely
            cx.resources.x100.clear_interrupt_pending_bit();
        }
    }
};
