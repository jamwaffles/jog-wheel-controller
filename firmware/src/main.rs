#![no_std]
#![no_main]

mod multiplier;
mod pendant;

use core::fmt::Write;
use cortex_m::asm;
use cortex_m_semihosting::hprintln;
use embedded_hal::{Direction, Qei};
use heapless::consts::*;
use panic_semihosting as _;
use rtfm::app;
use rtfm::cyccnt::{Duration, Instant};
use ssd1306::prelude::*;
use ssd1306::Builder;
use stm32f1xx_hal::{
    gpio,
    gpio::State,
    i2c::{BlockingI2c, DutyCycle, Mode},
    pac::{self, I2C2},
    prelude::*,
    qei::{self},
    timer::{self, CountDownTimer, Event, Timer},
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

type Encoder = qei::Qei<
    pac::TIM1,
    timer::Tim1NoRemap,
    (
        gpio::gpioa::PA8<gpio::Input<gpio::Floating>>,
        gpio::gpioa::PA9<gpio::Input<gpio::Floating>>,
    ),
>;

#[app(device = stm32f1xx_hal::pac, peripherals = true, monotonic = rtfm::cyccnt::CYCCNT)]
const APP: () = {
    struct Resources {
        display: Display,
        jog_velocity_timer: CountDownTimer<pac::TIM2>,
        update_period: Duration,
        qei: Encoder,
        prev_qei_sample_time: Instant,
        #[init(0)]
        prev_qei_count: u16,
    }

    #[init(schedule = [update])]
    fn init(mut cx: init::Context) -> init::LateResources {
        let dp = cx.device;

        // Enable cycle counter
        let mut core = cx.core;
        core.DWT.enable_cycle_counter();

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

        let led = gpioc
            .pc13
            .into_push_pull_output_with_state(&mut gpioc.crh, State::High);

        let scl = gpiob.pb10.into_alternate_open_drain(&mut gpiob.crh);
        let sda = gpiob.pb11.into_alternate_open_drain(&mut gpiob.crh);

        let i2c = BlockingI2c::i2c2(
            dp.I2C2,
            (scl, sda),
            Mode::Fast {
                frequency: 100_000.hz(),
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

        // TIM1 QEI
        let c1 = gpioa.pa8;
        let c2 = gpioa.pa9;

        // Set QEI up for 400 PPR (4 pulses per click) with overflow to zero on full rev
        let qei = Timer::tim1(dp.TIM1, &clocks, &mut rcc.apb2).qei((c1, c2), &mut afio.mapr);

        let mut jog_velocity_timer =
            Timer::tim2(dp.TIM2, &clocks, &mut rcc.apb1).start_count_down(10.hz());
        jog_velocity_timer.listen(Event::Update);

        let update_period = Duration::from_cycles(clocks.sysclk().0 / 2.hz().0);

        // Schedule `update` task
        cx.schedule.update(cx.start + update_period).unwrap();

        // Init the static resources to use them later through RTFM
        init::LateResources {
            display: disp,
            jog_velocity_timer,
            update_period,
            qei,
            prev_qei_sample_time: cx.start,
        }
    }

    #[idle(resources = [prev_qei_count])]
    fn idle(mut cx: idle::Context) -> ! {
        loop {
            cx.resources
                .prev_qei_count
                .lock(|prev_qei_count| hprintln!("{}", prev_qei_count));
        }
    }

    #[task(schedule = [update], priority = 3, resources = [prev_qei_count, update_period])]
    fn update(mut cx: update::Context) {
        let update::Resources {
            update_period,
            prev_qei_count,
            ..
        } = cx.resources;

        cx.schedule.update(cx.scheduled + *update_period).unwrap();
    }

    #[task(binds = TIM2, priority = 2, resources = [jog_velocity_timer, qei, prev_qei_count])]
    fn jog_velocity(mut cx: jog_velocity::Context) {
        let count = cx.resources.qei.count();

        let prev_qei_count = cx
            .resources
            .prev_qei_count
            .lock(|prev_qei_count| *prev_qei_count = count);

        cx.resources
            .jog_velocity_timer
            .clear_update_interrupt_flag();
    }

    extern "C" {
        fn EXTI0();
    }
};
