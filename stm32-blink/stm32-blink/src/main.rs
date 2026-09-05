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

defmt::timestamp!("{=u32}", 0_u32);

// Registradores principais do SX1278
const REG_FIFO: u8 = 0x00;
const REG_OP_MODE: u8 = 0x01;
const REG_FRF_MSB: u8 = 0x06;
const REG_FRF_MID: u8 = 0x07;
const REG_FRF_LSB: u8 = 0x08;
const REG_PA_CONFIG: u8 = 0x09;
const REG_FIFO_ADDR_PTR: u8 = 0x0D;
const REG_FIFO_TX_BASE_ADDR: u8 = 0x0E;
const REG_IRQ_FLAGS: u8 = 0x12;
const REG_MODEM_CONFIG_1: u8 = 0x1D;
const REG_MODEM_CONFIG_2: u8 = 0x1E;
const REG_PREAMBLE_MSB: u8 = 0x20;
const REG_PREAMBLE_LSB: u8 = 0x21;
const REG_PAYLOAD_LENGTH: u8 = 0x22;
const REG_MODEM_CONFIG_3: u8 = 0x26;
const REG_SYNC_WORD: u8 = 0x39;
const REG_DIO_MAPPING_1: u8 = 0x40;
const REG_VERSION: u8 = 0x42;

// Bits do registrador RegIrqFlags
const IRQ_TX_DONE: u8 = 0x08;

// Modos do SX1278 para frequência abaixo de 525 MHz
const MODE_LORA_SLEEP: u8 = 0x88;
const MODE_LORA_STANDBY: u8 = 0x89;
const MODE_LORA_TX: u8 = 0x8B;

const SX127X_EXPECTED_VERSION: u8 = 0x12;

const PACKET_LENGTH: usize = 24;

const PACKET_TEMPLATE: &[u8; PACKET_LENGTH] = b"SEQ=00000;MSG=STM32-LORA";

fn build_packet(sequence: u32) -> [u8; PACKET_LENGTH] {
    let mut packet = *PACKET_TEMPLATE;

    /*
     * Mantém somente cinco dígitos:
     * 1     -> 00001
     * 42    -> 00042
     * 12345 -> 12345
     */
    let mut value = sequence % 100_000;

    /*
     * Os dígitos ficam entre os índices 4 e 8:
     *
     * SEQ=00000;MSG=STM32-LORA
     *     ^^^^^
     */
    for index in (4..9).rev() {
        packet[index] = b'0' + (value % 10) as u8;
        value /= 10;
    }

    packet
}

#[entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();
    let cp = cortex_m::Peripherals::take().unwrap();

    let mut rcc = dp.RCC.freeze(Config::DEFAULT);

    let gpioa = dp.GPIOA.split(&mut rcc);
    let gpiob = dp.GPIOB.split(&mut rcc);
    let gpioc = dp.GPIOC.split(&mut rcc);

    /*
     * SPI1:
     * PA5 = D13 = SCK
     * PA6 = D12 = MISO
     * PA7 = D11 = MOSI
     */
    let sck = gpioa.pa5;
    let miso = gpioa.pa6;
    let mosi = gpioa.pa7;

    /*
     * Controle do SX1278:
     * PB6  = D10 = NSS
     * PC7  = D9  = RESET
     * PA10 = D2  = DIO0
     */
    let mut cs = gpiob.pb6.into_push_pull_output();
    let mut reset = gpioc.pc7.into_push_pull_output();
    let dio0 = gpioa.pa10.into_pull_down_input();

    let mut delay = Delay::new(cp.SYST, rcc.clocks.hclk().raw());

    let _ = cs.set_high();

    // Reset físico do SX1278
    let _ = reset.set_low();
    delay.delay_ms(10_u32);

    let _ = reset.set_high();
    delay.delay_ms(20_u32);

    let spi_mode = Mode {
        polarity: Polarity::IdleLow,
        phase: Phase::CaptureOnFirstTransition,
    };

    let mut spi = Spi::new(
        dp.SPI1,
        (Some(sck), Some(miso), Some(mosi)),
        spi_mode,
        500.kHz(),
        &mut rcc,
    );

    /*
     * Macro para escrever em um registrador.
     * O bit 7 do endereço deve ser 1 para escrita.
     */
    macro_rules! write_register {
        ($register:expr, $value:expr) => {{
            let mut data = [$register | 0x80, $value];

            let _ = cs.set_low();
            let success = spi.transfer_in_place(&mut data).is_ok();
            let _ = cs.set_high();

            success
        }};
    }

    /*
     * Macro para ler um registrador.
     * O bit 7 do endereço deve ser 0 para leitura.
     */
    macro_rules! read_register {
        ($register:expr) => {{
            let mut data = [$register & 0x7F, 0x00];

            let _ = cs.set_low();
            let success = spi.transfer_in_place(&mut data).is_ok();
            let _ = cs.set_high();

            (success, data[1])
        }};
    }

    // Confirma novamente a comunicação SPI
    let (transfer_ok, version) = read_register!(REG_VERSION);

    defmt::info!("Transferencia SPI concluida: {=bool}", transfer_ok);

    defmt::info!("RegVersion: {=u8}", version);

    if !transfer_ok || version != SX127X_EXPECTED_VERSION {
        loop {
            defmt::error!("SX1278 nao detectado. RegVersion: {=u8}", version);

            delay.delay_ms(2000_u32);
        }
    }

    defmt::info!("SX1278 detectado com sucesso");

    /*
     * Coloca o rádio em modo LoRa Sleep.
     *
     * Bit 7 = LoRa
     * Bit 3 = Low Frequency Mode, necessário para 433 MHz
     * Bits 2:0 = Sleep
     */
    write_register!(REG_OP_MODE, MODE_LORA_SLEEP);
    delay.delay_ms(10_u32);

    /*
     * Frequência de 433 MHz.
     *
     * FRF = 433 MHz × 2^19 / 32 MHz
     * Resultado: 0x6C4000
     */
    write_register!(REG_FRF_MSB, 0x6C);
    write_register!(REG_FRF_MID, 0x40);
    write_register!(REG_FRF_LSB, 0x00);

    /*
     * Potência aproximada de 12 dBm usando PA_BOOST.
     * É suficiente para o primeiro teste em bancada.
     */
    write_register!(REG_PA_CONFIG, 0x8A);

    /*
     * ModemConfig1:
     * Bandwidth = 125 kHz
     * Coding Rate = 4/5
     * Header explícito
     */
    write_register!(REG_MODEM_CONFIG_1, 0x72);

    /*
     * ModemConfig2:
     * Spreading Factor = SF7
     * CRC ativado
     */
    write_register!(REG_MODEM_CONFIG_2, 0x74);

    /*
     * ModemConfig3:
     * Low Data Rate Optimize desativado
     * AGC automático ativado
     */
    write_register!(REG_MODEM_CONFIG_3, 0x04);

    // Preâmbulo de 8 símbolos
    write_register!(REG_PREAMBLE_MSB, 0x00);
    write_register!(REG_PREAMBLE_LSB, 0x08);

    // Sync Word da rede LoRa privada
    write_register!(REG_SYNC_WORD, 0x12);

    /*
     * DIO0 = TxDone.
     * Bits 7:6 = 01.
     */
    write_register!(REG_DIO_MAPPING_1, 0x40);

    // Início da região de transmissão do FIFO
    write_register!(REG_FIFO_TX_BASE_ADDR, 0x00);

    // Limpa todas as interrupções anteriores
    write_register!(REG_IRQ_FLAGS, 0xFF);

    // Coloca o rádio em Standby
    write_register!(REG_OP_MODE, MODE_LORA_STANDBY);
    delay.delay_ms(10_u32);

    defmt::info!("SX1278 configurado em 433 MHz");
    defmt::info!("Iniciando transmissoes");

    let mut packet_number: u32 = 1;

    loop {
        // Retorna ao modo Standby antes de preparar o pacote
        write_register!(REG_OP_MODE, MODE_LORA_STANDBY);

        // Aponta o FIFO para o início da área de transmissão
        write_register!(REG_FIFO_ADDR_PTR, 0x00);

        /*
         * Primeiro byte da transferência é o endereço do FIFO.
         * Os demais bytes são a mensagem.
         */
        let packet = build_packet(packet_number);

        let mut fifo_data = [0_u8; PACKET_LENGTH + 1];

        fifo_data[0] = REG_FIFO | 0x80;
        fifo_data[1..].copy_from_slice(&packet);

        let _ = cs.set_low();
        let fifo_ok = spi.transfer_in_place(&mut fifo_data).is_ok();
        let _ = cs.set_high();

        // Informa ao rádio o tamanho da mensagem
        write_register!(REG_PAYLOAD_LENGTH, PACKET_LENGTH as u8);

        // Limpa as interrupções antes de transmitir
        write_register!(REG_IRQ_FLAGS, 0xFF);

        defmt::info!("Transmitindo pacote: {=u32}", packet_number);

        defmt::info!("FIFO preenchido: {=bool}", fifo_ok);

        // Inicia a transmissão
        write_register!(REG_OP_MODE, MODE_LORA_TX);

        /*
         * Aguarda DIO0 ficar alto.
         * Existe um timeout de 2 segundos para evitar travamento.
         */
        let mut timeout_ms: u32 = 0;

        while dio0.is_low() && timeout_ms < 2000 {
            delay.delay_ms(1_u32);
            timeout_ms += 1;
        }

        // Também verifica o TxDone diretamente no registrador
        let (irq_read_ok, irq_flags) = read_register!(REG_IRQ_FLAGS);

        let tx_done = irq_read_ok && (irq_flags & IRQ_TX_DONE) != 0;

        if tx_done {
            defmt::info!("TxDone confirmado. Pacote: {=u32}", packet_number);
        } else {
            defmt::warn!("TxDone nao confirmado. IRQ flags: {=u8}", irq_flags);
        }

        // Limpa TxDone e outras interrupções
        write_register!(REG_IRQ_FLAGS, 0xFF);

        // Retorna para Standby
        write_register!(REG_OP_MODE, MODE_LORA_STANDBY);

        packet_number = packet_number.wrapping_add(1);

        // Um novo pacote a cada dois segundos
        delay.delay_ms(2000_u32);
    }
}
