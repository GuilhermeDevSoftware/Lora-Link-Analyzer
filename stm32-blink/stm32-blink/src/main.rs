#![no_std]
#![no_main]

use cortex_m::delay::Delay;
use cortex_m_rt::entry;
use panic_halt as _;
use stm32f4xx_hal::{pac, prelude::*, rcc::Config};

#[entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();
    let cp = cortex_m::Peripherals::take().unwrap();

    // Clock interno HSI; nao depende de cristal ou clock externo.
    let mut rcc = dp.RCC.freeze(Config::DEFAULT);

    // Na Nucleo-F446RE, o LED verde LD2 esta ligado ao PA5.
    let gpioa = dp.GPIOA.split(&mut rcc);
    let mut led = gpioa.pa5.into_push_pull_output();

    // SysTick conta o tempo usando a frequencia real configurada para HCLK.
    let mut delay = Delay::new(cp.SYST, rcc.clocks.hclk().raw());

    loop {
    // Duas piscadas rápidas.
    for _ in 0..2 {
        led.set_high();
        delay.delay_ms(100_u32);

        led.set_low();
        delay.delay_ms(100_u32);
    }

    // Pausa de dois segundos com o LED apagado.
    delay.delay_ms(2000_u32);
}
}
