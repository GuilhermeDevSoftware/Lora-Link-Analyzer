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
const IRQ_RX_DONE: u8 = 0x40;
const IRQ_CRC_ERROR: u8 = 0x20;
const REG_FIFO_RX_BASE_ADDR: u8 = 0x0F;
const REG_FIFO_RX_CURRENT_ADDR: u8 = 0x10;
const REG_RX_NB_BYTES: u8 = 0x13;
const REG_HOP_CHANNEL: u8 = 0x1C;
const MODE_LORA_RX: u8 = 0x8D;
// Limite por sondagens com delay de 1 ms; nao e cronometro de latencia.
const ACK_WAIT_POLLS: u32 = 1500;
// Uma transmissão inicial e até dois reenvios.
const MAX_ATTEMPTS: u32 = 3;

// Pausa adicional antes de cada reenvio.
const RETRY_DELAY_MS: u32 = 200;


// Modos do SX1278 para frequência abaixo de 525 MHz
const MODE_LORA_SLEEP: u8 = 0x88;
const MODE_LORA_STANDBY: u8 = 0x89;
const MODE_LORA_TX: u8 = 0x8B;

const SX127X_EXPECTED_VERSION: u8 = 0x12;

const PACKET_LENGTH: usize = b"SEQ=00000;MSG=STM32-LORA".len();

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
    let _dio0 = gpioa.pa10.into_pull_down_input(); // IRQ consultada por SPI

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

            assert!(success, "Falha SPI ao escrever registrador");
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

            assert!(success, "Falha SPI ao ler registrador");
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

    write_register!(REG_FIFO_RX_BASE_ADDR, 0x00);

    // Limpa todas as interrupções anteriores
    write_register!(REG_IRQ_FLAGS, 0xFF);

    // Coloca o rádio em Standby
    write_register!(REG_OP_MODE, MODE_LORA_STANDBY);
    delay.delay_ms(10_u32);

    defmt::info!("SX1278 configurado em 433 MHz");
    defmt::info!("Iniciando transmissoes");

    let mut packet_number: u32 = 1;
    let mut tx_ok: u32 = 0;
    let mut tx_errors: u32 = 0;
    let mut ack_ok: u32 = 0;
    let mut ack_timeouts: u32 = 0;
    let mut ack_rejected: u32 = 0;
    let mut retries: u32 = 0;
    let mut unconfirmed_packets: u32 = 0;

    loop {
    // Reinicia o resultado ao começar um novo pacote.
    let mut delivered = false;

    for attempt in 1..=MAX_ATTEMPTS {
        if attempt > 1 {
            retries += 1;
            delay.delay_ms(RETRY_DELAY_MS);
        }

        write_register!(REG_OP_MODE, MODE_LORA_STANDBY);
        write_register!(REG_DIO_MAPPING_1, 0x40);
        write_register!(REG_FIFO_ADDR_PTR, 0x00);
        let packet = build_packet(packet_number);
        let mut fifo_data = [0_u8; PACKET_LENGTH + 1];
        fifo_data[0] = REG_FIFO | 0x80;
        fifo_data[1..].copy_from_slice(&packet);
        let _ = cs.set_low();
        let fifo_ok = spi.transfer_in_place(&mut fifo_data).is_ok();
        let _ = cs.set_high();
        assert!(fifo_ok, "Falha SPI ao preencher FIFO");
        write_register!(REG_PAYLOAD_LENGTH, PACKET_LENGTH as u8);
        write_register!(REG_IRQ_FLAGS, 0xFF);
        defmt::info!(
        "Transmitindo pacote: {=u32}; tentativa: {=u32}/{=u32}",
        packet_number,
        attempt,
        MAX_ATTEMPTS
        );
        write_register!(REG_OP_MODE, MODE_LORA_TX);

        let mut tx_done = false;
        for _ in 0..2000 {
            let (_, flags) = read_register!(REG_IRQ_FLAGS);
            if flags & IRQ_TX_DONE != 0 {
                tx_done = true;
                break;
            }
            delay.delay_ms(1_u32);
        }

        if tx_done {
            // Entrar em RX ANTES de imprimir logs para nao atrasar o ACK.
            write_register!(REG_OP_MODE, MODE_LORA_STANDBY);
            write_register!(REG_DIO_MAPPING_1, 0x00);
            write_register!(REG_FIFO_ADDR_PTR, 0x00);
            write_register!(REG_IRQ_FLAGS, 0xFF);
            write_register!(REG_OP_MODE, MODE_LORA_RX);
            tx_ok += 1;

            let mut expected = *b"ACK=00000";
            expected[4..9].copy_from_slice(&packet[4..9]);
            let mut confirmed = false;
            for _ in 0..ACK_WAIT_POLLS {
                let (_, flags) = read_register!(REG_IRQ_FLAGS);
                if flags & IRQ_RX_DONE != 0 {
                    write_register!(REG_OP_MODE, MODE_LORA_STANDBY);
                    let (_, length) = read_register!(REG_RX_NB_BYTES);
                    let (_, hop) = read_register!(REG_HOP_CHANNEL);
                    if flags & IRQ_CRC_ERROR == 0 && hop & 0x40 != 0 && length == 9 {
                        let (_, address) = read_register!(REG_FIFO_RX_CURRENT_ADDR);
                        write_register!(REG_FIFO_ADDR_PTR, address);
                        let mut buffer = [0_u8; 10];
                        buffer[0] = REG_FIFO & 0x7F;
                        let _ = cs.set_low();
                        let ok = spi.transfer_in_place(&mut buffer).is_ok();
                        let _ = cs.set_high();
                        assert!(ok, "Falha SPI ao ler ACK");
                        confirmed = buffer[1..] == expected[..];
                    }
                    write_register!(REG_IRQ_FLAGS, 0xFF);
                    if confirmed {
                        break;
                    }
                    ack_rejected += 1;
                    write_register!(REG_OP_MODE, MODE_LORA_RX);
                }
                delay.delay_ms(1_u32);
            }
            write_register!(REG_OP_MODE, MODE_LORA_STANDBY);
            if confirmed {
                delivered = true;
                ack_ok += 1;

                defmt::info!(
                "ACK confirmado. Pacote: {=u32}",
                packet_number
        );
            } else {
                ack_timeouts += 1;
                defmt::warn!("Timeout ACK. Pacote: {=u32}", packet_number);
            }
        } else {
            tx_errors += 1;
            defmt::warn!("TxDone nao confirmado. Pacote: {=u32}", packet_number);
        }
                write_register!(REG_OP_MODE, MODE_LORA_STANDBY);
        write_register!(REG_IRQ_FLAGS, 0xFF);

        // Recebeu ACK: não precisa realizar as próximas tentativas.
        if delivered {
            break;
        }
    } // Fecha o for de tentativas.

    // Só chega aqui sem confirmação após esgotar as tentativas.
    if !delivered {
        unconfirmed_packets += 1;

        defmt::warn!(
            "Pacote sem confirmacao apos {=u32} tentativas: {=u32}",
            MAX_ATTEMPTS,
            packet_number
        );
        }

        defmt::info!(
        "Pacotes finalizados: {=u32}; confirmados: {=u32}; sem confirmacao: {=u32}; reenvios: {=u32}",
        packet_number,
        ack_ok,
        unconfirmed_packets,
        retries
        );

        defmt::info!(
        "TX OK: {=u32}; falhas TX: {=u32}; timeouts ACK: {=u32}; RX rejeitados: {=u32}",
        tx_ok,
        tx_errors,
        ack_timeouts,
        ack_rejected
            );

        // Ensaio limitado: evita reutilizar sequencias e aceitar ACK antigo.
        if packet_number == 99_999 {
            defmt::info!("Ensaio concluido. Reinicie as duas placas para novo ensaio.");
            loop { delay.delay_ms(1000_u32); }
        }
        packet_number += 1;
        // Pausa apos o resultado: periodo total inclui TX e espera pelo ACK.
        delay.delay_ms(2000_u32);
    }
}
