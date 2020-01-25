#![no_std]
#![no_main]

mod multiplier;
mod pendant;

use crate::{multiplier::MultiplierPins, pendant::Pendant};
use core::fmt::Write;
use cortex_m_semihosting::hprintln;
use embedded_graphics::{
    egcircle, egrectangle, pixelcolor::BinaryColor, prelude::*, primitives::Rectangle, text_6x8,
};
use embedded_hal::{
    digital::v2::{InputPin, ToggleableOutputPin},
    Direction,
};
use heapless::consts::*;
use panic_semihosting as _;
use rtfm::app;
use rtfm::cyccnt::{Instant, U32Ext};
use ssd1306::prelude::*;
use ssd1306::Builder;
use stm32f1xx_hal::{
    device, gpio,
    gpio::{gpioc::PC13, Edge, ExtiPin, Output, PullDown, PushPull, State},
    i2c::{BlockingI2c, DutyCycle, Mode},
    pac::{self, I2C2},
    prelude::*,
    qei::{self, QeiOptions, SlaveMode},
    stm32,
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
        led: PC13<Output<PushPull>>,
        display: Display,
        inputs_timer: CountDownTimer<pac::TIM2>,
        estop: gpio::gpioa::PA4<gpio::Input<gpio::PullDown>>,
        pendant: Pendant,
        multiplier_pins: MultiplierPins,
        update_period: u32,
        #[init(false)]
        pinger_state: bool,
        qei: Encoder
        // x1: gpio::gpioa::PA5<gpio::Input<gpio::PullUp>>,
        // x10: gpio::gpioa::PA6<gpio::Input<gpio::PullUp>>,
        // x100: gpio::gpioa::PA7<gpio::Input<gpio::PullUp>>,
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

        disp.draw(text_6x8!(
            "Initialising...",
            stroke = Some(BinaryColor::On),
            fill = Some(BinaryColor::Off)
        ));

        disp.flush().unwrap();

        let mut x1 = gpioa.pa5.into_pull_up_input(&mut gpioa.crl);
        let mut x10 = gpioa.pa6.into_pull_up_input(&mut gpioa.crl);
        let mut x100 = gpioa.pa7.into_pull_up_input(&mut gpioa.crl);

        let multiplier_pins = MultiplierPins::new(x1, x10, x100);

        let mut estop = gpioa.pa4.into_pull_down_input(&mut gpioa.crl);

        // estop.make_interrupt_source(&mut afio);
        // estop.trigger_on_edge(&dp.EXTI, Edge::RISING);
        // estop.enable_interrupt(&dp.EXTI);

        // TIM1 QEI
        let c1 = gpioa.pa8;
        let c2 = gpioa.pa9;

        // Set QEI up for 200 PPR (2 pulses per click) with overflow to zero on full rev
        let qei = Timer::tim1(dp.TIM1, &clocks, &mut rcc.apb2).qei((c1, c2), &mut afio.mapr, QeiOptions {
                slave_mode: SlaveMode::EncoderMode1,
                auto_reload_value: 199
            });

        let mut inputs_timer =
            Timer::tim2(dp.TIM2, &clocks, &mut rcc.apb1).start_count_down(10.hz());
        inputs_timer.listen(Event::Update);

        estop.make_interrupt_source(&mut afio);
        estop.trigger_on_edge(&dp.EXTI, Edge::RISING_FALLING);
        estop.enable_interrupt(&dp.EXTI);

        let update_period = 4_000_000;

        // Schedule `update` task
        cx.schedule
            .update(cx.start + update_period.cycles())
            .unwrap();

        hprintln!("Init complete").unwrap();

        // Init the static resources to use them later through RTFM
        init::LateResources {
            display: disp,
            estop,
            inputs_timer,
            pendant: Pendant::new(),
            multiplier_pins,
            led,
            update_period,
            qei
        }
    }

    #[task(schedule = [update], resources = [qei, pendant, display, update_period, pinger_state])]
    fn update(cx: update::Context) {
        let update::Resources {
            pendant,
            display,
            update_period,
            pinger_state,
            qei
        } = cx.resources;

        let mut line_buf: heapless::String<U21> = heapless::String::new();

        display.clear();

        // Pinger
        display.draw(egcircle!(
            (127 - 5, 5),
            5,
            stroke = Some(BinaryColor::On),
            fill = if *pinger_state { Some(BinaryColor::On) } else { Some(BinaryColor::Off)}
        ));

        // *pinger_state = !*pinger_state;

        write!(line_buf, "Estop: {:?}", pendant.estopped()).expect("Estop write");

        display.draw(text_6x8!(
            &line_buf,
            stroke = Some(BinaryColor::On),
            fill = Some(BinaryColor::Off)
        ));

        line_buf.clear();
        write!(line_buf, "Mul: {:?}", pendant.multiplier()).expect("Mul write");

        display.draw(
            text_6x8!(
                &line_buf,
                stroke = Some(BinaryColor::On),
                fill = Some(BinaryColor::Off)
            )
            .translate((0, 8).into()),
        );

         line_buf.clear();
        write!(line_buf, "QEI: {:?} {}", qei.count(), match qei.direction() {
            Direction::Downcounting => 'V',
            Direction::Upcounting => '^'
        }).expect("Mul write");

        display.draw(
            text_6x8!(
                &line_buf,
                stroke = Some(BinaryColor::On),
                fill = Some(BinaryColor::Off)
            )
            .translate((0, 8).into()),
        );

        display.flush().unwrap();

        cx.schedule
            .update(cx.scheduled + update_period.cycles())
            .unwrap();
    }

    #[task(binds = TIM2, resources = [pinger_state, display, pendant, estop, multiplier_pins, inputs_timer])]
    fn poll_inputs(cx: poll_inputs::Context) {


        cx.resources
            .pendant
            .set_multiplier(cx.resources.multiplier_pins.selection());

        *cx.resources.pinger_state = !*cx.resources.pinger_state;

        // Clears the update flag
        cx.resources.inputs_timer.clear_update_interrupt_flag();
    }

    #[task(binds = EXTI4, resources = [pendant, estop])]
    fn estop(cx: estop::Context) {
        if cx.resources.estop.check_interrupt() {
            if cx.resources.estop.is_low().expect("Estop pin") {
                cx.resources.pendant.set_estop();
            } else {
                cx.resources.pendant.clear_estop();
            }

            // if we don't clear this bit, the ISR would trigger indefinitely
            cx.resources.estop.clear_interrupt_pending_bit();
        }
    }

    extern "C" {
        fn EXTI0();
    }
};
