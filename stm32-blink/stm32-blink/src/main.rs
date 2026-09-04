#![no_std]
#![no_main]

use cortex_m::delay::Delay;
use cortex_m_rt::entry;
use defmt_rtt as _;
use panic_probe as _;
use stm32f4xx_hal::{
    pac,
    prelude::*,
    rcc::Config,
    spi::{Mode, Phase, Polarity, Spi},
};

// Timestamp simples para as mensagens do defmt.
// Ainda não precisamos medir o tempo real.
defmt::timestamp!("{=u32}", 0_u32);

// Registrador que identifica a versão do SX1278.
const REG_VERSION: u8 = 0x42;

// Valor esperado para a família SX1276/77/78/79.
const SX127X_EXPECTED_VERSION: u8 = 0x12;

#[entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();
    let cp = cortex_m::Peripherals::take().unwrap();

    // Utiliza o clock interno HSI.
    let mut rcc = dp.RCC.freeze(Config::DEFAULT);

    // Separa os bancos de GPIO utilizados.
    let gpioa = dp.GPIOA.split(&mut rcc);
    let gpiob = dp.GPIOB.split(&mut rcc);
    let gpioc = dp.GPIOC.split(&mut rcc);

    /*
     * SPI1:
     *
     * PA5 = D13 = SCK
     * PA6 = D12 = MISO
     * PA7 = D11 = MOSI
     */
    let sck = gpioa.pa5;
    let miso = gpioa.pa6;
    let mosi = gpioa.pa7;

    /*
     * Controle do SX1278:
     *
     * PB6 = D10 = NSS/CS
     * PC7 = D9  = RESET
     */
    let mut cs = gpiob.pb6.into_push_pull_output();
    let mut reset = gpioc.pc7.into_push_pull_output();

    // Temporizador baseado no SysTick do Cortex-M4.
    let mut delay = Delay::new(cp.SYST, rcc.clocks.hclk().raw());

    /*
     * O SX1278 só aceita comandos SPI quando NSS/CS está baixo.
     * Fora de uma transferência, mantemos o pino em nível alto.
     */
    let _ = cs.set_high();

    /*
     * Reset físico do SX1278.
     *
     * RESET baixo  -> rádio em reset
     * RESET alto   -> funcionamento normal
     */
    let _ = reset.set_low();
    delay.delay_ms(10_u32);

    let _ = reset.set_high();
    delay.delay_ms(20_u32);

    /*
     * SPI modo 0:
     *
     * CPOL = 0: clock permanece baixo quando ocioso.
     * CPHA = 0: dados capturados na primeira transição.
     */
    let spi_mode = Mode {
        polarity: Polarity::IdleLow,
        phase: Phase::CaptureOnFirstTransition,
    };

    /*
     * Configura o SPI1 em 500 kHz.
     * A frequência baixa facilita o primeiro teste da montagem.
     */
    let mut spi = Spi::new(
        dp.SPI1,
        (Some(sck), Some(miso), Some(mosi)),
        spi_mode,
        500.kHz(),
        &mut rcc,
    );

    /*
     * Leitura do RegVersion:
     *
     * bit 7 do endereço = 0 -> operação de leitura
     * endereço 0x42     -> RegVersion
     *
     * Durante o primeiro byte, enviamos o endereço.
     * Durante o segundo byte, recebemos o valor do registrador.
     */
    let mut buffer = [REG_VERSION & 0x7F, 0x00];

    let _ = cs.set_low();

    let transfer_ok = spi.transfer_in_place(&mut buffer).is_ok();

    let _ = cs.set_high();

    // O primeiro byte recebido é descartado.
    let version = buffer[1];

    let radio_detectado =
        transfer_ok && version == SX127X_EXPECTED_VERSION;

    /*
     * PA5 também está conectado ao LED verde LD2 da Nucleo.
     *
     * Depois da leitura, liberamos o SPI e recuperamos o PA5
     * para indicar visualmente o resultado.
     */
    let (_spi1, (sck, _miso, _mosi)) = spi.release();

    let mut led = match sck.unwrap() {
        stm32f4xx_hal::gpio::alt::spi1::Sck::PA5(pin) => {
            pin.into_push_pull_output()
        }
        _ => unreachable!(),
    };

    let _ = led.set_low();

    loop {
        /*
         * As mensagens são repetidas porque as primeiras mensagens RTT
         * podem acontecer antes de o probe-rs começar a monitorá-las.
         */
        defmt::info!(
            "Transferencia SPI concluida: {=bool}",
            transfer_ok
        );

        defmt::info!(
            "RegVersion em decimal: {=u8}",
            version
        );

        defmt::info!(
            "Valor esperado em decimal: 18"
        );

        if radio_detectado {
            /*
             * Sucesso:
             * uma piscada longa seguida de uma pausa.
             */
            let _ = led.set_high();
            delay.delay_ms(500_u32);

            let _ = led.set_low();
            delay.delay_ms(1500_u32);
        } else {
            /*
             * Falha:
             * três piscadas rápidas seguidas de uma pausa.
             */
            for _ in 0..3 {
                let _ = led.set_high();
                delay.delay_ms(100_u32);

                let _ = led.set_low();
                delay.delay_ms(100_u32);
            }

            delay.delay_ms(1400_u32);
        }
    }
}